//! Example for using reqwest to send a POST request to DeepSeek API and receive a stream response.
//!
//! ## Http streaming
//! An HTTP stream generally comes in [two forms](https://majdibo.com/blog/Understanding%20HTTP%20Streaming/):
//! [SSE](https://bunny.net/academy/http/what-is-sse-server-sent-events-and-how-do-they-work/) and
//! [chunked](https://bunny.net/academy/http/what-is-chunked-encoding/).
//! The stream implementation of most model responses adopts `SSE` because it is more suitable for text streams.
//! You can refer to [this link](https://stackoverflow.com/questions/39889052/why-use-server-sent-events-instead-of-simple-http-chunked-streaming)
//! for more details.
//!
//! Setting `Transfer-Encoding: chunked` implies that all data will be in the format of `length + chunked` data, which is not convenient for handling text streams.
//! If it is necessary to process text streams in a chunked manner, additional constraints via `Content-Type` are usually applied. For instance, Ollama's API adds `Content-Type: application/x-ndjson`.
//! This way, data is separated by `\n` instead of being presented in the `length + chunked` data format.

use futures::StreamExt;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, Deserialize)]
struct ApiResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    delta: Delta,
}

#[derive(Debug, Deserialize)]
struct Delta {
    content: Option<String>,
}

#[derive(Serialize)]
struct RequestBody {
    model: String,
    messages: Vec<Message>,
    stream: bool,
    temperature: f32,
    max_tokens: i32,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Load environment variable from .env file in project directory
    dotenvy::dotenv().ok().unwrap();
    // 1. Create HTTP client
    let client = reqwest::Client::new();

    // 2. Prepare request header
    let mut headers = HeaderMap::new();
    let api_key = format!("Bearer {}", std::env::var("API_KEY").unwrap());
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&api_key)?);
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    // Add Accept header to receive stream response
    headers.insert(ACCEPT, HeaderValue::from_static("text/event-stream"));

    // 3. Construct request body
    let request_body = RequestBody {
        model: "qwen-max-2025-01-25".to_string(), // Confirm the model name according to the document
        messages: vec![Message {
            role: "user".to_string(),
            content:
                "role: the role of the message, either system, user, assistant, or tool
            上面对大模型API请求中每个角色有什么不同？以及上下文一般怎么通过API塞进去的？"
                    .to_string(),
        }],
        stream: true,
        temperature: 0.7,
        max_tokens: 1024,
    };

    // 4. Send request and get response
    let response = client
        .post("https://dashscope.aliyuncs.com/compatible-mode/v1/chat/completions") // API URL
        .headers(headers)
        .json(&request_body)
        .send()
        .await?;

    // 5. Check response status
    if !response.status().is_success() {
        let error_text = response.text().await?;
        eprintln!("Failed to request API: {}", error_text);
        return Ok(());
    }

    // 6. Process stream response
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        let chunk_str = String::from_utf8_lossy(&chunk);

        // Handle SSE format data (possibly multiple events in one chunk)
        for line in chunk_str.split('\n') {
            if let Some(event_data) = line.strip_prefix("data: ") {
                // Check the end tag
                if event_data.trim() == "[DONE]" {
                    println!("\n\n[DONE]");
                    break;
                }

                // Parse JSON
                match serde_json::from_str::<ApiResponse>(event_data) {
                    Ok(response) => {
                        if let Some(content) = &response.choices[0].delta.content {
                            print!("{}", content); // Real-time output
                        }
                    }
                    Err(e) => eprintln!("Parsing json error: {}", e),
                }
            }
        }
    }
    Ok(())
}
