use std::{collections::HashMap, sync::Arc};

use anyhow::Context;
use axum::{extract::State, response::Response, Json};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::{
    api::{self, aliyun, AppError},
    SharedState,
};

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(default)]
    pub tools: Vec<Tool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<HashMap<String, serde_json::Value>>,
    #[serde(default = "default_stream")]
    pub stream: bool,
    #[serde(default = "default_keep_alive")]
    pub keep_alive: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub function: FunctionCall,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Tool {
    #[serde(rename = "type")]
    pub type_: String,
    pub function: ToolFunction,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolFunction {
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

/// Handle chat requests. This function is called when a POST request is made to `/api/chat`.
/// See [ollama chat api](https://github.com/ollama/ollama/blob/main/docs/api.md#generate-a-chat-completion)
pub async fn api_chat(
    State(state): State<SharedState>,
    Json(payload): Json<ChatRequest>,
) -> Result<Response, AppError> {
    let SharedState {
        client,
        model_config,
    } = state;
    tracing::info!("Received chat request: {:?}", payload);
    // Retrieve specific information about the calling model,
    // and invoke the corresponding interface to complete the API call based on the API provider
    let (model_id, model_name, api_info) = {
        let guard = model_config.read();
        let value = guard
            .models
            .get(&payload.model)
            .context("Invalid model id")?;
        let api_info = guard
            .api_keys
            .get(&value.api_key_id)
            .context("Invalid api_key_id")?;
        (payload.model.clone(), value.name.clone(), api_info.clone())
    };
    // Make a request to the corresponding cloud provider's API
    let local_status = Arc::new(RwLock::new(LocalChatStatus { is_thinking: false }));
    let res = match api_info.provider {
        api::uni_ollama::ApiKeyProvider::Aliyun => {
            aliyun::chat_completion(
                payload,
                model_id,
                model_name,
                api_info.api_key,
                client,
            )
            .await?
        }
        api::uni_ollama::ApiKeyProvider::Tencent => todo!(),
        api::uni_ollama::ApiKeyProvider::Bytedance => todo!(),
        api::uni_ollama::ApiKeyProvider::DeepSeek => todo!(),
    };
    Ok(res)
}
