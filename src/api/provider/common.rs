use std::fmt::Debug;

use anyhow::{bail, Context};
use axum::{
    body::Body,
    http::{HeaderMap, HeaderValue},
    response::Response,
};
use reqwest::{
    header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
    Client, IntoUrl,
};
use serde::Serialize;
use tracing::instrument;

use crate::{
    api::uni_ollama::message::{
        gen_last_message, OllamaChatRequest, ReqMessage, RespMessage, Tool,
    },
    common::stream::get_ollama_stream,
};

use super::message::{ApiResponse, Usage};

#[derive(Debug, Serialize)]
pub(crate) struct CommonReq {
    pub model: String,
    pub messages: Vec<ReqMessage>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<Tool>,
}

pub(crate) async fn chat_completion<U: IntoUrl + Debug>(
    url: U,
    chat_req: OllamaChatRequest,
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
    if chat_req.stream {
        headers.insert(ACCEPT, HeaderValue::from_static("text/event-stream"));
    }

    // Construct request body
    let req = CommonReq {
        model: model_name,
        messages: chat_req.messages,
        stream: chat_req.stream,
        tools: chat_req.tools,
    };
    let mut body = serde_json::to_value(&req).context("construct common req")?;

    if let Some(options) = chat_req.options {
        // TODO: Insert options based on [doc](https://api-docs.deepseek.com/zh-cn/api/create-chat-completion)
        options.into_iter().for_each(|(k, v)| {
            body.as_object_mut()
                .expect("as object nerver fails")
                .insert(k, v);
        });
    }

    tracing::info!("url:{url:?}\nheaders:{headers:?}\nbody:{body}");

    let api_resp = client
        .post(url) // API URL
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
async fn process_streaming(
    model_id: String,
    api_resp: reqwest::Response,
) -> anyhow::Result<Response> {
    let stream = api_resp.bytes_stream();

    let ollama_resp_stream = get_ollama_stream(model_id, stream);
    let mut header = HeaderMap::new();
    header.append(
        CONTENT_TYPE,
        HeaderValue::from_static("application/x-ndjson"),
    );
    let mut res = Response::builder()
        .status(200)
        .body(Body::from_stream(ollama_resp_stream))
        .context("Construct response")?;
    *res.headers_mut() = header;
    Ok(res)
}

#[instrument(skip(api_resp))]
async fn process_non_streaming(
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
    if !delta.reasoning_content.is_empty() {
        content.push_str("<think>\n");
        content.push_str(delta.reasoning_content.as_str());
        content.push_str("</think>\n");
    }
    content.push_str(&delta.content);

    let ollama_resp = gen_last_message(
        &model_id,
        Some(RespMessage {
            role: delta.role,
            content,
            images: None,
        }),
        api_resp.usage.as_ref().unwrap_or(&Usage::default()),
        0,
    );
    tracing::debug!("response_body:{ollama_resp}");
    let mut header = HeaderMap::new();
    header.append(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    let mut res = Response::builder()
        .status(200)
        .body(Body::from(ollama_resp))
        .context("Construct response")?;
    *res.headers_mut() = header;
    Ok(res)
}
