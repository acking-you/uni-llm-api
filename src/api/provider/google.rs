use std::collections::HashMap;

use anyhow::{bail, Context};
use axum::{
    body::Body,
    http::{HeaderMap, HeaderValue},
    response::Response,
};
use reqwest::{
    header::{ACCEPT, CONTENT_TYPE},
    Client,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::instrument;

use crate::{
    api::uni_ollama::message::{
        OllamaChatRequest, OllamaChatResponse, RespMessage, Role,
    },
    common::gemini_stream::get_ollama_stream,
};

#[derive(Debug, Serialize)]
pub(crate) struct GeminiRequest {
    pub contents: Vec<Content>,
    pub system_instruction: Option<Content>,
    pub generation_config: Option<HashMap<String, Value>>,
}

#[derive(Debug, Serialize)]
pub(crate) struct Content {
    pub role: Option<String>,
    pub parts: Vec<Part>,
}

#[derive(Debug, Serialize)]
pub(crate) struct Part {
    pub text: String,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GeminiResponse {
    pub candidates: Vec<Candidate>,
    pub usage_metadata: UsageMetadata,
    #[allow(unused)]
    pub model_version: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Candidate {
    pub content: ContentDetails,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ContentDetails {
    pub parts: Vec<PartDetails>,
    #[allow(unused)]
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PartDetails {
    pub text: String,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UsageMetadata {
    pub prompt_token_count: usize,
    pub total_token_count: usize,
}

pub(crate) async fn chat_completion(
    chat_req: OllamaChatRequest,
    model_id: String,
    model_name: String,
    api_key: String,
    client: Client,
) -> anyhow::Result<Response> {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    // Add Accept header to receive stream response
    if chat_req.stream {
        headers.insert(ACCEPT, HeaderValue::from_static("text/event-stream"));
    }

    // gen url
    let url = if chat_req.stream {
        format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:streamGenerateContent?alt=sse&key={}",
        model_name, api_key
    )
    } else {
        format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model_name, api_key
    )
    };
    let (contents, system_instruction) = {
        let mut contents = Vec::new();
        let mut system_instruction: Option<Content> = None;
        for msg in chat_req.messages.into_iter() {
            if let Role::System = msg.role {
                match system_instruction.as_mut() {
                    Some(instruct) => {
                        instruct.parts.push(Part { text: msg.content });
                    }
                    None => {
                        system_instruction = Some(Content {
                            role: None,
                            parts: vec![Part { text: msg.content }],
                        });
                    }
                }
            } else if let Role::Assistant = msg.role {
                contents.push(Content {
                    role: Some("model".to_string()),
                    parts: vec![Part { text: msg.content }],
                });
            } else {
                contents.push(Content {
                    role: Some("user".to_string()),
                    parts: vec![Part { text: msg.content }],
                });
            }
        }
        (contents, system_instruction)
    };
    // Construct request body
    let req = GeminiRequest {
        contents,
        system_instruction,
        generation_config: None, // TODO: Modify `chat_req.options` based on [doc](https://ai.google.dev/gemini-api/docs/text-generation?hl=zh-cn&lang=rest#configure)
    };

    tracing::info!("url:{url:?}\nheaders:{headers:?}\nbody:{req:?}");

    let api_resp = client
        .post(url) // API URL
        .headers(headers)
        .json(&req)
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

pub(crate) fn gen_ollama_message(model_id: &str, msg: RespMessage) -> String {
    let mut resp = OllamaChatResponse::default();
    resp.add_modle_and_message(model_id, msg);
    serde_json::to_string(&resp).expect("gen ollama response nerver fails")
}
pub(crate) fn gen_last_ollama_message(
    model_id: &str,
    usage: UsageMetadata,
    done_dur: u32,
) -> String {
    let mut resp = OllamaChatResponse::default();

    resp.fill_option();
    let UsageMetadata {
        prompt_token_count,
        total_token_count,
    } = usage;
    resp.model = model_id.to_string();
    resp.done = true;
    resp.eval_count = Some(total_token_count as u32);
    resp.prompt_eval_count = Some(prompt_token_count as u32);
    resp.eval_duration = Some(done_dur);

    serde_json::to_string(&resp).expect("gen ollama response nerver fails")
}

#[instrument(skip(api_resp))]
async fn process_streaming(
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
async fn process_non_streaming(
    model_id: String,
    api_resp: reqwest::Response,
) -> anyhow::Result<Response> {
    let api_resp = api_resp
        .json::<GeminiResponse>()
        .await
        .context("process_non_streaming::parse_json")?;
    let mut content = String::new();
    api_resp
        .candidates
        .first()
        .context("Must have at least one choice")?
        .content
        .parts
        .iter()
        .for_each(|c| content.push_str(&c.text));

    let mut resp = OllamaChatResponse::default();

    resp.fill_option();
    let UsageMetadata {
        prompt_token_count,
        total_token_count,
    } = api_resp.usage_metadata;
    resp.model = model_id.to_string();
    resp.done = true;
    resp.eval_count = Some(total_token_count as u32);
    resp.prompt_eval_count = Some(prompt_token_count as u32);
    resp.message = RespMessage {
        role: Role::Assistant,
        content,
        images: None,
    };

    let ollama_resp =
        serde_json::to_string(&resp).expect("gen ollama response nerver fails");

    tracing::debug!("response_body:{ollama_resp}");
    let mut header = HeaderMap::new();
    header.append(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    let mut resp = Response::builder()
        .status(200)
        .body(Body::from(ollama_resp))
        .context("Construct response")?;
    *resp.headers_mut() = header;
    Ok(resp)
}
