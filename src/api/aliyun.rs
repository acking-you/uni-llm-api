use anyhow::{bail, Context};
use axum::{
    body::Body,
    http::{HeaderMap, HeaderValue},
    response::Response,
};
use reqwest::{
    header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
    Client,
};
use serde_json::json;

use crate::common::stream::get_ollama_stream;

use super::uni_ollama::chat::ChatRequest;

pub async fn chat_completion(
    chat_req: ChatRequest,
    model_id: String,
    model_name: String,
    api_key: String,
    client: Client,
) -> anyhow::Result<Response> {
    let mut headers = HeaderMap::new();
    let api_key = format!("Bearer {}", api_key);
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&api_key)?);
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    // Add Accept header to receive stream response
    headers.insert(ACCEPT, HeaderValue::from_static("text/event-stream"));

    // Construct request body
    let mut body = json!({
        "model": model_name,
        "messages": chat_req.messages,
        "stream": chat_req.stream,
        "tools": chat_req.tools,
    });

    if let Some(options) = chat_req.options {
        options.into_iter().for_each(|(k, v)| {
            body.as_object_mut()
                .expect("as object nerver fails")
                .insert(k, v);
        });
    }

    let api_resp = client
        .post("https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions") // API URL
        .headers(headers)
        .json(&body)
        .send()
        .await?;

    // Check response status
    if !api_resp.status().is_success() {
        let error_text = api_resp.text().await?;
        tracing::error!("Failed to request API: {}", error_text);
        bail!("error:{error_text}")
    }

    // Process api response
    if chat_req.stream {
        process_streaming(model_id, api_resp).await
    } else {
        todo!()
    }
}

pub async fn process_streaming(
    model_id: String,
    api_resp: reqwest::Response,
) -> anyhow::Result<Response> {
    let stream = api_resp.bytes_stream();

    let ollama_resp_stream = get_ollama_stream(model_id, stream);

    let mut response_builder = Response::builder().status(200);
    let mut header = HeaderMap::new();
    header.append(
        CONTENT_TYPE,
        HeaderValue::from_static("application/x-ndjson"),
    );
    *response_builder.headers_mut().unwrap() = header;
    let res = response_builder
        .body(Body::from_stream(ollama_resp_stream))
        .context("Construct response")?;
    Ok(res)
}
