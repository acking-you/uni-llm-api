use crate::api::common::default_chat_resp_role;
use crate::api::common::null_to_default;
use crate::api::uni_ollama::message::Role;
use serde::Deserialize;

#[derive(Deserialize, Debug, Default)]
pub(crate) struct Delta {
    #[serde(deserialize_with = "null_to_default")]
    #[serde(default)]
    pub content: String,
    pub reasoning_content: Option<String>,
    #[serde(default = "default_chat_resp_role")]
    pub role: Role,
}

#[derive(Deserialize, Debug, Default)]
pub(crate) struct Choice {
    /// `delta` For streaming api
    /// `message` For non streaming api
    #[serde(alias = "message")]
    pub delta: Delta,
    #[allow(unused)]
    pub finish_reason: Option<String>,
    #[allow(unused)]
    pub index: u32,
    #[allow(unused)]
    pub logprobs: Option<serde_json::Value>,
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
