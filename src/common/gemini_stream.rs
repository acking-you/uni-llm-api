//! Implement a streaming ollama API for Google Gemini
use std::future::Future;
use std::task::ready;
use std::task::Poll;
use std::time::Instant;

use anyhow::anyhow;
use anyhow::Context;
use bytes::Bytes;
use bytes::BytesMut;
use futures::stream::Unfold;
use futures::Stream;
use futures::StreamExt;
use pin_project::pin_project;
use tracing::instrument;

use crate::api::provider::google::gen_last_ollama_message;
use crate::api::provider::google::gen_ollama_message;
use crate::api::provider::google::GeminiResponse;
use crate::api::uni_ollama::message::RespMessage;
use crate::api::uni_ollama::message::Role;

#[derive(Debug)]
enum ChatRespStatus {
    /// Chatting status
    Chatting,
    /// Chat Finished state
    ChatFinished,
}

struct OllamaBytesState<S> {
    status: ChatRespStatus,
    model_id: String,
    ins: Instant,
    inner: S,
}

type ReqwestResult = reqwest::Result<Bytes>;

impl<S: Stream<Item = ReqwestResult> + Unpin> OllamaBytesState<S> {
    async fn poll_next(mut self) -> Option<(anyhow::Result<bytes::Bytes>, Self)> {
        let chunk = self.inner.next().await?;
        match self.status {
            ChatRespStatus::Chatting => Some((self.process_msg(chunk).await, self)),
            ChatRespStatus::ChatFinished => None,
        }
    }

    #[instrument(skip(self, chunk), err)]
    pub async fn process_msg(
        &mut self,
        chunk: ReqwestResult,
    ) -> anyhow::Result<bytes::Bytes> {
        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(e) => {
                tracing::error!("Failed to get bytes: {e}");
                return Err(anyhow!("error:{e}"));
            }
        };

        let chunk_str = String::from_utf8_lossy(&chunk);
        tracing::debug!("chunk_str:{chunk_str}");
        let mut resp_chunk_buf = BytesMut::with_capacity(128);
        // Handle SSE format data (possibly multiple events in one chunk)
        for line in chunk_str.split('\n') {
            if let Some(event_data) = line.strip_prefix("data: ") {
                // Parse JSON
                let response = serde_json::from_str::<GeminiResponse>(event_data)?;

                let candidate = response
                    .candidates
                    .first()
                    .context("candidates.first() never emtpy")?;

                let text = candidate
                    .content
                    .parts
                    .first()
                    .context("parts.fisrt() nerver empty")?
                    .text
                    .clone();

                macro_rules! append_msg {
                    ($msg:expr) => {{
                        let msg = gen_ollama_message(
                            &self.model_id,
                            RespMessage {
                                role: Role::Assistant,
                                content: $msg,
                                images: None,
                            },
                        );
                        resp_chunk_buf.extend_from_slice(msg.as_bytes());
                        resp_chunk_buf.extend_from_slice(b"\n");
                    }};
                    ($usage:expr,$dur:expr) => {{
                        let msg = gen_last_ollama_message(&self.model_id, $usage, $dur);
                        resp_chunk_buf.extend_from_slice(msg.as_bytes());
                        resp_chunk_buf.extend_from_slice(b"\n");
                    }};
                }

                match &self.status {
                    ChatRespStatus::Chatting => {
                        if candidate.finish_reason.is_none() {
                            append_msg!(text);
                        } else {
                            let dur = self.ins.elapsed().as_millis() as u32;
                            append_msg!(text);
                            append_msg!(response.usage_metadata, dur + 1);
                            tracing::info!("finished chatting: chunk:{chunk_str}");
                            self.status = ChatRespStatus::ChatFinished;
                        }
                    }
                    // do nothing
                    ChatRespStatus::ChatFinished => {}
                }
            }
        }
        Ok(resp_chunk_buf.freeze())
    }
}

type OllamaBytesStateFold<S, Fut> =
    Unfold<OllamaBytesState<S>, fn(OllamaBytesState<S>) -> Fut, Fut>;

/// Used to convert the response stream of third-party APIs into a unified ollama format response stream
#[pin_project]
struct OllamaBytesStream<
    S: Stream<Item = ReqwestResult>,
    Fut: Future<Output = Option<(anyhow::Result<Bytes>, OllamaBytesState<S>)>>,
> {
    #[pin]
    inner: OllamaBytesStateFold<S, Fut>,
    /// Fuse
    is_done: bool,
}

impl<
        S: Stream<Item = ReqwestResult>,
        Fut: Future<Output = Option<(anyhow::Result<Bytes>, OllamaBytesState<S>)>>,
    > Stream for OllamaBytesStream<S, Fut>
{
    type Item = anyhow::Result<Bytes>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = self.project();
        if *this.is_done {
            return Poll::Ready(None);
        }
        match ready!(this.inner.poll_next(cx)) {
            Some(item) => Poll::Ready(Some(item)),
            None => {
                *this.is_done = true;
                Poll::Ready(None)
            }
        }
    }
}

pub(crate) fn get_ollama_stream<S: Stream<Item = ReqwestResult> + Unpin + 'static>(
    model_id: String,
    bytes_stream: S,
) -> impl Stream<Item = anyhow::Result<Bytes>> {
    OllamaBytesStream {
        inner: futures::stream::unfold(
            OllamaBytesState {
                status: ChatRespStatus::Chatting,
                model_id,
                inner: bytes_stream,
                ins: Instant::now(),
            },
            OllamaBytesState::poll_next,
        ),
        is_done: false,
    }
}
