use axum::response::{IntoResponse, Response};
use chrono::{Local, SecondsFormat};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub(crate) mod uni_ollama;

pub(super) mod aliyun;
pub(super) mod bytedance;
pub(super) mod deepseek;
pub(super) mod siliconflow;
pub(super) mod tencent;

#[derive(Deserialize, Debug, Default)]
pub(crate) struct Delta {
    pub content: String,
    pub reasoning_content: Option<String>,
    pub role: String,
}

#[derive(Deserialize, Debug, Default)]
pub(crate) struct Choice {
    pub delta: Delta,
    #[allow(unused)]
    pub finish_reason: Option<String>,
    #[allow(unused)]
    pub index: u32,
    #[allow(unused)]
    pub logprobs: Option<Value>,
}

#[derive(Deserialize, Debug, Default)]
pub(crate) struct Usage {
    pub completion_tokens: u32,
    pub prompt_tokens: u32,
    pub total_tokens: u32,
}

/// See [deepseek api](https://api-docs.deepseek.com/zh-cn/api/create-chat-completion#responses)
#[derive(Deserialize, Debug, Default)]
pub(crate) struct ApiResponse {
    pub choices: Vec<Choice>,
    #[allow(unused)]
    pub object: Option<String>,
    pub usage: Option<Usage>,
    #[allow(unused)]
    #[serde(default)]
    pub created: u64,
    #[allow(unused)]
    pub system_fingerprint: Option<String>,
    #[allow(unused)]
    pub model: Option<String>,
    #[allow(unused)]
    pub id: Option<String>,
}

/// Ollama response, see [link](https://github.com/ollama/ollama/blob/main/docs/api.md#response-10)
#[derive(Debug, Serialize)]
pub(crate) struct OllamaResponse {
    pub model: String,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<Message>,
    pub done: bool,
    /// The meaning of this value is now changed to [`Usage::total_tokens`] here
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_duration: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_duration: Option<u32>,
    /// The meaning of this value is now changed to [`Usage::prompt_tokens`] here
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_eval_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_eval_duration: Option<u32>,
    /// The meaning of this value is now changed to [`Usage::completion_tokens`] here
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_count: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_duration: Option<u32>,
}

impl OllamaResponse {
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

pub(crate) fn gen_ollama_think_start_message(model_id: &str) -> String {
    gen_ollama_message(
        model_id,
        Message {
            role: "assistant".to_string(),
            content: "<think>".to_string(),
            images: None,
        },
        None,
    )
}

pub(crate) fn gen_ollama_think_end_message(model_id: &str) -> String {
    gen_ollama_message(
        model_id,
        Message {
            role: "assistant".to_string(),
            content: "</think>".to_string(),
            images: None,
        },
        None,
    )
}

pub(crate) fn gen_last_message(model_id: &str, usage: &Usage) -> String {
    let mut resp = OllamaResponse::default();
    resp.add_usage(usage);
    resp.model = model_id.to_string();
    resp.done = true;
    serde_json::to_string(&resp).expect("gen last message never fails")
}

pub(crate) fn gen_ollama_message(
    model_id: &str,
    msg: Message,
    usage: Option<&Usage>,
) -> String {
    let mut resp = OllamaResponse::default();
    resp.add_modle_and_message(model_id, msg);
    if let Some(usage) = usage {
        resp.add_usage(usage);
    }
    serde_json::to_string(&resp).expect("gen ollama response nerver fails")
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
pub(crate) struct Message {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<String>,
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
