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
use serde::Serialize;
use tracing::instrument;

use crate::common::stream::get_ollama_stream;

use super::{
    gen_ollama_message,
    uni_ollama::chat::{ChatRequest, Message, Tool},
    ApiResponse,
};

#[derive(Debug, Serialize)]
struct AliyunReq {
    model: String,
    messages: Vec<Message>,
    stream: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<Tool>,
}

pub(crate) async fn chat_completion(
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
    let req = AliyunReq {
        model: model_name,
        messages: chat_req.messages,
        stream: chat_req.stream,
        tools: chat_req.tools,
    };
    let mut body = serde_json::to_value(&req).context("construct aliyun req")?;

    if let Some(options) = chat_req.options {
        options.into_iter().for_each(|(k, v)| {
            body.as_object_mut()
                .expect("as object nerver fails")
                .insert(k, v);
        });
    }

    tracing::info!("headers:{headers:?}\nbody:{body}");

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
        process_non_streaming(model_id, api_resp).await
    }
}

#[instrument(skip(api_resp))]
pub(crate) async fn process_streaming(
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

#[instrument(skip(api_resp))]
pub(crate) async fn process_non_streaming(
    model_id: String,
    api_resp: reqwest::Response,
) -> anyhow::Result<Response> {
    let api_resp = api_resp
        .json::<ApiResponse>()
        .await
        .context("process_non_streaming::parse_json")?;
    let mut content = String::new();
    let delta = &api_resp
        .choices
        .first()
        .context("Must have at least one choice")?
        .delta;
    if let Some(reason_content) = delta.reasoning_content.as_ref() {
        content.push_str("<think>\n");
        content.push_str(reason_content);
        content.push_str("</think>\n");
    }
    content.push_str(&delta.content);

    let ollama_resp = gen_ollama_message(
        &model_id,
        super::Message {
            role: delta.role.clone(),
            content,
            images: None,
        },
        api_resp.usage.as_ref(),
    );

    let mut response_builder = Response::builder().status(200);
    let mut header = HeaderMap::new();
    header.append(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    *response_builder.headers_mut().unwrap() = header;
    let res = response_builder
        .body(Body::from(ollama_resp))
        .context("Construct response")?;
    Ok(res)
}
