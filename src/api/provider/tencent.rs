use axum::response::Response;
use reqwest::Client;

use crate::api::uni_ollama::message::OllamaChatRequest;

pub(crate) async fn chat_completion(
    chat_req: OllamaChatRequest,
    model_id: String,
    model_name: String,
    api_key: String,
    client: Client,
) -> anyhow::Result<Response> {
    super::common::chat_completion(
        "https://api.lkeap.cloud.tencent.com/v1/chat/completions",
        chat_req,
        model_id,
        model_name,
        api_key,
        client,
    )
    .await
}
