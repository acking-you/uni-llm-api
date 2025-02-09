use std::{collections::HashMap, sync::Arc};

use parking_lot::RwLock;

pub mod chat;
pub mod tag;

/// A struct for make a request to the chat api
#[derive(Debug, Clone)]
pub struct ModelInfo {
    /// Model name for the api call
    pub name: String,
    /// To find actual api_key in [`UniModelsInfo::api_keys`]
    pub api_key_id: String,
}

#[derive(Debug, Clone)]
pub enum ApiKeyProvider {
    Aliyun,
    Tencent,
    Bytedance,
    DeepSeek,
}
impl Default for ApiKeyProvider {
    fn default() -> Self {
        Self::Aliyun
    }
}

/// A struct that contains the api_key and the provider of the api_key
#[derive(Debug, Default, Clone)]
pub struct ApiKeyInfo {
    pub api_key: String,
    pub provider: ApiKeyProvider,
}

/// A struct that contains all the information about the models and their api_keys
#[derive(Debug, Default, Clone)]
pub struct UniModelsInfo {
    /// A mapping of unique names representing api_keys to their actual api_key values,
    /// such as `aliyun: ApiKeyInfo { api_key: xxx, provider: Aliyun }`
    pub api_keys: HashMap<String, ApiKeyInfo>,
    /// A mapping of the unique name of the model to its specific invocation details,
    /// such as `aliyun/deepseek: ModelInfo { name: "deepseek", api_key_id: "aliyun" }`
    pub models: HashMap<String, ModelInfo>,
}

pub type UniModelInfoRef = Arc<RwLock<UniModelsInfo>>;
