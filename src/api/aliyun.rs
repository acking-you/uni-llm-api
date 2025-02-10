use anyhow::{anyhow, bail, Context};
use axum::{
    body::Body,
    http::{HeaderMap, HeaderValue},
    response::Response,
};
use bytes::{Bytes, BytesMut};
use futures::StreamExt;
use reqwest::{
    header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
    Client,
};
use serde_json::{json, Value};

use super::{
    uni_ollama::{
        chat::{ChatRequest, LocalChatStatusRef},
        ApiKeyInfo,
    },
    ApiResponse, OllamaResponse,
};

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
    let mut stream = api_resp.bytes_stream();

    let stream_resp = stream.map(move |chunk| {
        match chunk {
            Ok(chunk) => {
                let chunk_str = String::from_utf8_lossy(&chunk);
                let mut resp_chunk_buf = BytesMut::with_capacity(128);
                // Handle SSE format data (possibly multiple events in one chunk)
                for line in chunk_str.split('\n') {
                    if let Some(event_data) = line.strip_prefix("data: ") {
                        // Check the end tag
                        if event_data.trim() == "[DONE]" {
                            tracing::info!("DONE completion with chunk:\n {chunk_str}");
                            break;
                        }

                        // Parse JSON
                        let response = serde_json::from_str::<ApiResponse>(event_data)
                            .context("parsing ApiResponse")?;

                        // append to buffer
                        resp_chunk_buf.extend_from_slice(extend);
                    }
                }
                Ok()
            }
            Err(e) => Err(anyhow!("error:{e}")),
        }
    });

    let mut response_builder = Response::builder().status(api_resp.status());
    let header = HeaderMap::new();
    header.append(
        CONTENT_TYPE,
        HeaderValue::from_static("application/x-ndjson"),
    );
    *response_builder.headers_mut().unwrap() = header;
    let res = response_builder
        .body(Body::from_stream(stream_resp))
        .context("Construct response")?;
    Ok(res)
}
