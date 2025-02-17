//! message for ollama api
use std::collections::HashMap;

use crate::api::common::default_chat_resp_role;
use chrono::{Local, SecondsFormat};
use serde::{Deserialize, Serialize};

use crate::api::provider::message::Usage;

#[derive(Debug, Serialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub(crate) enum DoneReason {
    Stop,
}

/// Ollama response, see [link](https://github.com/ollama/ollama/blob/main/docs/api.md#response-10)
#[derive(Debug, Serialize)]
pub(crate) struct OllamaChatResponse {
    pub model: String,
    pub created_at: String,
    pub message: RespMessage,
    pub done: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done_reason: Option<DoneReason>,
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
    /// Total time consumed by streaming API calls
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_duration: Option<u32>,
}

impl OllamaChatResponse {
    pub(crate) fn add_modle_and_message(&mut self, model: &str, message: RespMessage) {
        self.model = model.to_string();
        self.message = message;
    }

    pub(crate) fn add_usage(&mut self, usage: &Usage) {
        self.total_duration = Some(usage.total_tokens);
        self.prompt_eval_count = Some(usage.prompt_tokens);
        self.eval_count = Some(usage.completion_tokens);
    }

    pub(crate) fn fill_option(&mut self) {
        self.done_reason = Some(DoneReason::Stop);
        self.total_duration = Some(0);
        self.load_duration = Some(0);
        self.prompt_eval_count = Some(0);
        self.prompt_eval_duration = Some(0);
        self.eval_count = Some(0);
        self.eval_duration = Some(0);
    }
}

pub(crate) fn gen_ollama_think_start_message(model_id: &str) -> String {
    gen_ollama_message(
        model_id,
        RespMessage {
            role: Role::Assistant,
            content: "<think>".to_string(),
            images: None,
        },
    )
}

pub(crate) fn gen_ollama_think_end_message(model_id: &str) -> String {
    gen_ollama_message(
        model_id,
        RespMessage {
            role: Role::Assistant,
            content: "</think>".to_string(),
            images: None,
        },
    )
}

pub(crate) fn gen_last_message(
    model_id: &str,
    message: Option<RespMessage>,
    usage: &Usage,
    eval_dur: u32,
) -> String {
    let mut resp = OllamaChatResponse::default();
    if let Some(msg) = message {
        resp.add_modle_and_message(model_id, msg);
    } else {
        resp.model = model_id.to_string();
    }
    resp.fill_option();
    resp.add_usage(usage);
    resp.done = true;
    resp.eval_duration = Some(eval_dur);
    serde_json::to_string(&resp).expect("gen last message never fails")
}

pub(crate) fn gen_ollama_message(model_id: &str, msg: RespMessage) -> String {
    let mut resp = OllamaChatResponse::default();
    resp.add_modle_and_message(model_id, msg);
    serde_json::to_string(&resp).expect("gen ollama response nerver fails")
}

impl Default for OllamaChatResponse {
    fn default() -> Self {
        Self {
            model: Default::default(),
            created_at: Local::now().to_rfc3339_opts(SecondsFormat::Nanos, true),
            message: Default::default(),
            done: false,
            done_reason: None,
            total_duration: Default::default(),
            load_duration: Default::default(),
            prompt_eval_count: Default::default(),
            prompt_eval_duration: Default::default(),
            eval_count: Default::default(),
            eval_duration: Default::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub(crate) struct RespMessage {
    #[serde(default = "default_chat_resp_role")]
    pub role: Role,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct OllamaChatRequest {
    pub model: String,
    pub messages: Vec<ReqMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub tools: Vec<Tool>,
    #[allow(unused)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<HashMap<String, serde_json::Value>>,
    #[serde(default = "default_stream")]
    pub stream: bool,
    #[allow(unused)]
    #[serde(default = "default_keep_alive")]
    pub keep_alive: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ReqMessage {
    pub role: Role,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Role {
    System,
    User,
    Assistant,
    Tool,
}

impl Default for Role {
    fn default() -> Self {
        Self::User
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub function: FunctionCall,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct FunctionCall {
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Tool {
    #[serde(rename = "type")]
    pub type_: String,
    pub function: ToolFunction,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ToolFunction {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub parameters: serde_json::Value,
}

fn default_stream() -> bool {
    true
}

fn default_keep_alive() -> String {
    "5m".to_string()
}
