use anyhow::anyhow;
use anyhow::Context;
use bytes::Bytes;
use bytes::BytesMut;
use futures::Stream;
use futures::StreamExt;
use pin_project::pin_project;
use tracing::instrument;

use crate::api::gen_ollama_message;
use crate::api::ApiResponse;

pub enum ChatRespStatus {
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

pub struct OllamaBytesState<S> {
    status: ChatRespStatus,
    inner: S,
}

pub type ReqwestResult = reqwest::Result<Bytes>;

impl<S: Stream<Item = ReqwestResult> + std::marker::Unpin> OllamaBytesState<S> {
    pub async fn poll_next(mut self) -> Option<(anyhow::Result<bytes::Bytes>, Self)> {
        let chunk = if let Some(chunk) = self.inner.next().await {
            chunk
        } else {
            return None;
        };
        match self.status {
            ChatRespStatus::Init => Some((self.process_init(chunk).await, self)),
            ChatRespStatus::ContentThinking | ChatRespStatus::ReasoningThinking => {
                Some((self.process_thinking(chunk).await, self))
            }
            ChatRespStatus::ThinkFinished => {
                Some((self.process_think_finished(chunk).await, self))
            }
            ChatRespStatus::ChatFinished => None,
        }
    }

    #[instrument(skip(self))]
    pub async fn process_init(
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
        let mut resp_chunk_buf = BytesMut::with_capacity(128);
        // Handle SSE format data (possibly multiple events in one chunk)
        for line in chunk_str.split('\n') {
            if let Some(event_data) = line.strip_prefix("data: ") {
                // Check the end tag
                if event_data.trim() == "[DONE]" {
                    tracing::info!("DONE completion with chunk:\n {chunk_str}");
                    break;
                }

                // Parse JSON
                let response = serde_json::from_str::<ApiResponse>(event_data)
                    .context("parsing ApiResponse")?;
                let choice = response
                    .choices
                    .first()
                    .context("choices.first() never emtpy")?;
                match &self.status {
                    ChatRespStatus::Init => {
                        if choice.delta.content.contains("<think>") {
                            // TODO: add <think> and content to buffer
                            self.status = ChatRespStatus::ContentThinking;
                        } else if choice.delta.reasoning_content.is_some() {
                            self.status = ChatRespStatus::ReasoningThinking;
                        } else if !choice.delta.content.is_empty() {
                            self.status = ChatRespStatus::ThinkFinished;
                        } else {
                            tracing::error!("nerver reach here: {chunk_str}");
                        }
                    }
                    ChatRespStatus::ContentThinking => {
                        if choice.delta.content.contains("</think>") {
                            // TODO: add </think> and content to buffer
                            self.status = ChatRespStatus::ThinkFinished;
                        } else {
                            todo!()
                        }
                    }
                    ChatRespStatus::ReasoningThinking => {
                        if !choice.delta.content.is_empty() {
                            // TODO: add </think> and content to buffer
                            self.status = ChatRespStatus::ThinkFinished;
                        } else {
                            todo!()
                        }
                    }
                    ChatRespStatus::ThinkFinished => todo!(),
                    ChatRespStatus::ChatFinished => todo!(),
                }
            }
        }
        Ok()
    }

    #[instrument(skip(self))]
    pub async fn process_thinking(
        &mut self,
        chunk: ReqwestResult,
    ) -> anyhow::Result<bytes::Bytes> {
        todo!()
    }

    #[instrument(skip(self))]
    pub async fn process_think_finished(
        &mut self,
        chunk: ReqwestResult,
    ) -> anyhow::Result<bytes::Bytes> {
        todo!()
    }
}

pub struct OllamaBytesStream<S: Stream<Item = reqwest::Result<Bytes>>> {
    inner: OllamaBytesState<S>,
}
