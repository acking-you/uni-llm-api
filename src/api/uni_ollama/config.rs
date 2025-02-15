//! Config for the UniOllama api

use std::{collections::HashMap, sync::Arc};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// A struct for make a request to the chat api
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelInfo {
    /// Model name for the api call
    pub name: String,
    /// To find actual api_key in [`UniModelsInfo::api_keys`]
    pub api_key_id: String,
}

/// A struct for make a request to the tag api
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApiKeyProvider {
    /// See [`crate::api::provider::aliyun`]
    Aliyun,
    /// See [`crate::api::provider::tencent`]
    Tencent,
    /// See [`crate::api::provider::bytedance`]
    Bytedance,
    /// See [`crate::api::provider::deepseek`]
    DeepSeek,
    /// See [`crate::api::provider::google`]
    Google,
    /// See [`crate::api::provider::siliconflow`]
    Siliconflow,
    /// URL for the custom api_key provider
    Custom(String),
}

impl Default for ApiKeyProvider {
    fn default() -> Self {
        Self::Aliyun
    }
}

/// A struct that contains the api_key and the provider of the api_key
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ApiKeyInfo {
    /// The actual api_key value
    pub api_key: String,
    /// The provider of the api_key, such as `aliyun`, `tencent`, `bytedance`, `deepseek`
    ///
    /// ## See Also
    ///
    /// - [`ApiKeyProvider`]
    ///
    pub provider: ApiKeyProvider,
    /// Whether the [`self`] needs a proxy to make a request
    #[serde(default)]
    pub need_proxy: bool,
}

/// A struct that contains all the information about the models and their api_keys
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniModelsInfo {
    /// Used as [`reqwest::Proxy::http`]
    pub proxy_url: Option<String>,
    /// A mapping of unique names representing api_keys to their actual api_key values,
    /// such as `aliyun: ApiKeyInfo { api_key: xxx, provider: Aliyun }`
    pub api_keys: HashMap<String, ApiKeyInfo>,
    /// A mapping of the unique name of the model to its specific invocation details,
    /// such as `aliyun/deepseek: ModelInfo { name: "deepseek", api_key_id: "aliyun" }`
    pub models: HashMap<String, ModelInfo>,
}

impl Default for UniModelsInfo {
    fn default() -> Self {
        Self {
            proxy_url: Some("http://127.0.0.1:11111".to_string()),
            api_keys: {
                let mut map = HashMap::new();
                map.insert(
                    "aliyun".to_string(),
                    ApiKeyInfo {
                        api_key: "[YOUR-API-KEY]".to_string(),
                        provider: ApiKeyProvider::Aliyun,
                        need_proxy: false,
                    },
                );
                map.insert(
                    "bytedance".to_string(),
                    ApiKeyInfo {
                        api_key: "[YOUR-API-KEY]".to_string(),
                        provider: ApiKeyProvider::Bytedance,
                        need_proxy: false,
                    },
                );
                map.insert(
                    "tencent".to_string(),
                    ApiKeyInfo {
                        api_key: "[YOUR-API-KEY]".to_string(),
                        provider: ApiKeyProvider::Tencent,
                        need_proxy: false,
                    },
                );
                map.insert(
                    "siliconflow".to_string(),
                    ApiKeyInfo {
                        api_key: "[YOUR-API-KEY]".to_string(),
                        provider: ApiKeyProvider::Siliconflow,
                        need_proxy: false,
                    },
                );
                map.insert(
                    "google".to_string(),
                    ApiKeyInfo {
                        api_key: "[YOUR-API-KEY]".to_string(),
                        provider: ApiKeyProvider::Google,
                        need_proxy: true,
                    },
                );
                map
            },
            models: {
                let mut map = HashMap::new();
                map.insert(
                    "aliyun-r1".to_string(),
                    ModelInfo {
                        name: "deepseek-r1".to_string(),
                        api_key_id: "aliyun".to_string(),
                    },
                );
                map.insert(
                    "aliyun-qwen-max-latest".to_string(),
                    ModelInfo {
                        name: "qwen-max-latest".to_string(),
                        api_key_id: "aliyun".to_string(),
                    },
                );
                map.insert(
                    "bytedance-r1".to_string(),
                    ModelInfo {
                        name: "ep-20250207154718-64blv".to_string(),
                        api_key_id: "bytedance".to_string(),
                    },
                );
                map.insert(
                    "tencent-r1".to_string(),
                    ModelInfo {
                        name: "deepseek-r1".to_string(),
                        api_key_id: "tencent".to_string(),
                    },
                );
                map.insert(
                    "siliconflow-r1".to_string(),
                    ModelInfo {
                        name: "deepseek-ai/DeepSeek-R1".to_string(),
                        api_key_id: "siliconflow".to_string(),
                    },
                );
                map.insert(
                    "gemini-1.5-flash".to_string(),
                    ModelInfo {
                        name: "gemini-1.5-flash".to_string(),
                        api_key_id: "google".to_string(),
                    },
                );
                map.insert(
                    "gemini-2.0-flash".to_string(),
                    ModelInfo {
                        name: "gemini-2.0-flash".to_string(),
                        api_key_id: "google".to_string(),
                    },
                );
                map.insert(
                    "gemini-2.0-flash-thinking-exp".to_string(),
                    ModelInfo {
                        name: "gemini-2.0-flash-thinking-exp".to_string(),
                        api_key_id: "google".to_string(),
                    },
                );
                map
            },
        }
    }
}

impl UniModelsInfo {
    /// Insert the latest tag for compatible with [OpenWebUI](https://github.com/open-webui/open-webui)
    pub fn insert_latest_tag_for_openwebui(&mut self) {
        let latest_tagged_key_values = self
            .models
            .iter()
            .map(|(k, v)| (format!("{k}:latest"), v.clone()))
            .collect::<Vec<_>>();
        self.models.extend(latest_tagged_key_values);
    }
}

pub(crate) type UniModelInfoRef = Arc<RwLock<UniModelsInfo>>;

#[cfg(test)]
mod tests {
    use std::fs::OpenOptions;

    use super::UniModelsInfo;

    #[test]
    fn test_json() {
        let models_info = UniModelsInfo::default();
        let writer = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open("./config/test.json")
            .unwrap();
        serde_json::to_writer_pretty(writer, &models_info).unwrap();
    }
}
