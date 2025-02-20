use anyhow::Context;
use axum::{extract::State, response::Response};

use crate::{
    api::{
        self,
        provider::{self, aliyun, bytedance, deepseek, google, siliconflow, tencent},
        uni_ollama::message::OllamaChatRequest,
    },
    SharedStateRef,
};

use super::error::AppError;

/// Handle chat requests. This function is called when a POST request is made to `/api/chat`.
/// See [ollama chat api](https://github.com/ollama/ollama/blob/main/docs/api.md#generate-a-chat-completion)
pub(crate) async fn api_chat(
    State(state): State<SharedStateRef>,
    body: String,
) -> Result<Response, AppError> {
    let payload: OllamaChatRequest =
        serde_json::from_str(&body).context("Get ChatRequest")?;
    // Retrieve specific information about the calling model,
    // and invoke the corresponding interface to complete the API call based on the API provider
    let (model_id, model_name, api_info) = {
        let (model_name, api_key_id) = {
            let guard = state.model_config.read();
            let model_name = guard
                .models
                .get(&payload.model)
                .context("Invalid model id")?
                .name
                .clone();
            let api_key_id = guard
                .models
                .get(&payload.model)
                .context("Invalid model id")?
                .api_key_id
                .clone();
            (model_name, api_key_id)
        };
        let api_info = {
            let mut guard = state.model_config.write();
            let api_key_info = guard
                .api_keys
                .get_mut(&api_key_id)
                .context("Invalid api_key_id")?;
            api_key_info.selected()
        };
        (payload.model.clone(), model_name, api_info)
    };
    // Provide the correct client instance based on whether a proxy is needed
    let client = if api_info.need_proxy {
        tracing::info!("start proxy: model_id:{model_id} model_name:{model_name}");
        state
            .proxy_client
            .clone()
            .with_context(|| "You've chosen to use a proxy but haven't set it up yet")?
    } else {
        state.client.clone()
    };
    // Make a request to the corresponding cloud provider's API
    let res = match api_info.provider {
        api::uni_ollama::config::ApiKeyProvider::Aliyun => {
            aliyun::chat_completion(
                payload,
                model_id,
                model_name,
                api_info.api_key,
                client,
            )
            .await?
        }
        api::uni_ollama::config::ApiKeyProvider::Tencent => {
            tencent::chat_completion(
                payload,
                model_id,
                model_name,
                api_info.api_key,
                client,
            )
            .await?
        }
        api::uni_ollama::config::ApiKeyProvider::Bytedance => {
            bytedance::chat_completion(
                payload,
                model_id,
                model_name,
                api_info.api_key,
                client,
            )
            .await?
        }
        api::uni_ollama::config::ApiKeyProvider::DeepSeek => {
            deepseek::chat_completion(
                payload,
                model_id,
                model_name,
                api_info.api_key,
                client,
            )
            .await?
        }
        api::uni_ollama::config::ApiKeyProvider::Siliconflow => {
            siliconflow::chat_completion(
                payload,
                model_id,
                model_name,
                api_info.api_key,
                client,
            )
            .await?
        }
        api::uni_ollama::config::ApiKeyProvider::Google => {
            google::chat_completion(
                payload,
                model_id,
                model_name,
                api_info.api_key,
                client,
            )
            .await?
        }
        crate::ApiKeyProvider::Custom(url) => {
            provider::common::chat_completion(
                url,
                payload,
                model_id,
                model_name,
                api_info.api_key,
                client,
            )
            .await?
        }
    };
    Ok(res)
}
