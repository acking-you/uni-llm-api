//! Implement a unified streaming ollama API for (OpenAI Compatible)
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

use crate::api::provider::message::ApiResponse;
use crate::api::provider::message::Usage;
use crate::api::uni_ollama::message::gen_last_message;
use crate::api::uni_ollama::message::gen_ollama_message;
use crate::api::uni_ollama::message::gen_ollama_think_end_message;
use crate::api::uni_ollama::message::gen_ollama_think_start_message;
use crate::api::uni_ollama::message::RespMessage;

#[derive(Debug)]
enum ChatRespStatus {
    /// Initial state
    Init,
    /// Thinking state
    ReasoningThinking,
    ContentThinking,
    /// Finished thinking or no thinking state
    ThinkFinished,
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
            ChatRespStatus::Init
            | ChatRespStatus::ContentThinking
            | ChatRespStatus::ReasoningThinking
            | ChatRespStatus::ThinkFinished => {
                Some((self.process_msg(chunk).await, self))
            }
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
        let mut response = ApiResponse::default();
        for line in chunk_str.split('\n') {
            if let Some(event_data) = line.strip_prefix("data: ") {
                // Check the end tag
                if event_data.trim() == "[DONE]" {
                    tracing::info!("DONE completion with chunk:\n {chunk_str}");
                    self.status = ChatRespStatus::ChatFinished;
                    let msg = gen_last_message(
                        &self.model_id,
                        &response.usage.unwrap_or(Usage::default()),
                        self.ins.elapsed().as_millis() as u32,
                    );
                    resp_chunk_buf.extend_from_slice(msg.as_bytes());
                    resp_chunk_buf.extend_from_slice(b"\n");
                    break;
                } else {
                    // Parse JSON
                    response = serde_json::from_str::<ApiResponse>(event_data)?;
                }

                let choice = response
                    .choices
                    .first()
                    .context("choices.first() never emtpy")?;
                macro_rules! append_msg {
                    ($msg:expr) => {{
                        let msg = gen_ollama_message(
                            &self.model_id,
                            RespMessage {
                                role: choice.delta.role,
                                content: $msg,
                                images: None,
                            },
                            response.usage.as_ref(),
                        );
                        resp_chunk_buf.extend_from_slice(msg.as_bytes());
                        resp_chunk_buf.extend_from_slice(b"\n");
                    }};
                }
                macro_rules! append_thinking_start_msg {
                    ($msg:expr) => {{
                        let think_tag = gen_ollama_think_start_message(&self.model_id);
                        resp_chunk_buf.extend_from_slice(think_tag.as_bytes());
                        resp_chunk_buf.extend_from_slice(b"\n");
                        if !$msg.is_empty() {
                            append_msg!($msg);
                        }
                    }};
                }
                macro_rules! append_thinking_end_msg {
                    ($msg:expr) => {{
                        let think_end_tag = gen_ollama_think_end_message(&self.model_id);
                        resp_chunk_buf.extend_from_slice(think_end_tag.as_bytes());
                        resp_chunk_buf.extend_from_slice(b"\n");
                        if !$msg.is_empty() {
                            append_msg!($msg);
                        }
                    }};
                }
                match &self.status {
                    ChatRespStatus::Init => {
                        if choice.delta.content.contains("<think>") {
                            let msg = choice.delta.content.replace("<think>", "");
                            append_thinking_start_msg!(msg);
                            self.status = ChatRespStatus::ContentThinking;
                        } else if choice.delta.reasoning_content.is_some() {
                            let msg = choice
                                .delta
                                .reasoning_content
                                .clone()
                                .expect("nerver none checked by `is_some`");
                            append_thinking_start_msg!(msg);
                            self.status = ChatRespStatus::ReasoningThinking;
                        } else if !choice.delta.content.is_empty() {
                            self.status = ChatRespStatus::ThinkFinished;
                        } else {
                            tracing::info!(
                                "We will do nothing when it is empty msg: {chunk_str}"
                            );
                        }
                    }
                    ChatRespStatus::ContentThinking => {
                        if choice.delta.content.contains("</think>") {
                            let msg = choice.delta.content.replace("</think>", "");
                            append_thinking_end_msg!(msg);
                            self.status = ChatRespStatus::ThinkFinished;
                        } else {
                            append_msg!(choice.delta.content.clone());
                        }
                    }
                    ChatRespStatus::ReasoningThinking => {
                        if !choice.delta.content.is_empty()
                            || choice.delta.reasoning_content.is_none()
                        {
                            append_thinking_end_msg!(choice.delta.content.clone());
                            self.status = ChatRespStatus::ThinkFinished;
                        } else {
                            let msg = choice
                                .delta
                                .reasoning_content.clone()
                                .context("As it is `ChatRespStatus::ReasoningThinking` state, `reasoning_content` should be `Some`")?;
                            append_msg!(msg);
                        }
                    }
                    ChatRespStatus::ThinkFinished => {
                        append_msg!(choice.delta.content.clone());
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
                status: ChatRespStatus::Init,
                model_id,
                inner: bytes_stream,
                ins: Instant::now(),
            },
            OllamaBytesState::poll_next,
        ),
        is_done: false,
    }
}
