use anyhow::Context;
use axum::response::{IntoResponse, Response};
use chrono::{Local, SecondsFormat};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

pub(crate) mod uni_ollama;

pub(super) mod aliyun;
pub(super) mod bytedance;
pub(super) mod deepseek;
pub(super) mod siliconflow;

#[derive(Deserialize, Debug)]
pub(crate) struct Delta {
    pub content: String,
    pub reasoning_content: Option<String>,
    pub role: String,
}

#[derive(Deserialize, Debug)]
pub(crate) struct Choice {
    pub delta: Delta,
    pub finish_reason: Option<String>,
    pub index: u32,
    pub logprobs: Option<String>,
}

#[derive(Deserialize, Debug)]
pub(crate) struct Usage {
    pub completion_tokens: u32,
    pub prompt_tokens: u32,
    pub total_tokens: u32,
}

/// See [deepseek api](https://api-docs.deepseek.com/zh-cn/api/create-chat-completion#responses)
#[derive(Deserialize, Debug)]
pub(crate) struct ApiResponse {
    pub choices: Vec<Choice>,
    pub object: Option<String>,
    pub usage: Option<Usage>,
    #[serde(default)]
    pub created: u64,
    pub system_fingerprint: Option<String>,
    pub model: Option<String>,
    pub id: Option<String>,
}

/// Ollama response, see [link](https://github.com/ollama/ollama/blob/main/docs/api.md#response-10)
#[derive(Debug, Serialize)]
pub(crate) struct OllamaResponse {
    pub model: String,
    pub created_at: String,
    pub message: Option<Message>,
    pub done: bool,
    /// The meaning of this value is now changed to [`Usage::total_tokens`] here
    pub total_duration: Option<u32>,
    pub load_duration: Option<u32>,
    /// The meaning of this value is now changed to [`Usage::prompt_tokens`] here
    pub prompt_eval_count: Option<u32>,
    pub prompt_eval_duration: Option<u32>,
    /// The meaning of this value is now changed to [`Usage::completion_tokens`] here
    pub eval_count: Option<u32>,
    pub eval_duration: Option<u32>,
}

impl OllamaResponse {
    pub(crate) fn thinking_start(model: &str) -> Self {
        let mut ret = Self::default();
        ret.model = model.to_string();
        ret.message = Some(Message {
            role: "asistant".to_string(),
            content: "<think>".to_string(),
            images: None,
        });
        ret
    }

    pub(crate) fn thinking_end(model: &str) -> Self {
        let mut ret = Self::default();
        ret.model = model.to_string();
        ret.message = Some(Message {
            role: "asistant".to_string(),
            content: "</think>".to_string(),
            images: None,
        });
        ret
    }

    pub(crate) fn add_modle_and_message(&mut self, model: &str, message: Message) {
        self.model = model.to_string();
        self.message = Some(message);
    }

    pub(crate) fn add_usage(&mut self, usage: &Usage) {
        self.total_duration = Some(usage.total_tokens);
        self.prompt_eval_count = Some(usage.prompt_tokens);
        self.eval_count = Some(usage.completion_tokens);
    }
}

pub(crate) fn gen_ollama_message(
    model_id: &str,
    api_resp: ApiResponse,
) -> anyhow::Result<String> {
    let delta = api_resp
        .choices
        .first()
        .context("choice in response must not be empty")?
        .delta;
    let mut resp = OllamaResponse::default();
    resp.add_modle_and_message(
        model_id,
        Message {
            role: delta.role,
            content: delta.content,
            images: None,
        },
    );
    if let Some(usage) = api_resp.usage {
        resp.add_usage(&usage);
    }
    Ok()
}

impl Default for OllamaResponse {
    fn default() -> Self {
        Self {
            model: Default::default(),
            created_at: Local::now().to_rfc3339_opts(SecondsFormat::Nanos, true),
            message: Default::default(),
            done: false,
            total_duration: Default::default(),
            load_duration: Default::default(),
            prompt_eval_count: Default::default(),
            prompt_eval_duration: Default::default(),
            eval_count: Default::default(),
            eval_duration: Default::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    images: Option<String>,
}

// Make our own error that wraps `anyhow::Error`.
pub(crate) struct AppError(anyhow::Error);

// Tell axum how to convert `AppError` into a response.
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

// This enables using `?` on functions that return `Result<_, anyhow::Error>` to turn them into
// `Result<_, AppError>`. That way you don't need to do that manually.
impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
