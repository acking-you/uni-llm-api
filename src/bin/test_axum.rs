//! This example demonstrates how to stream data from a reqwest response to an axum response
use axum::{
    body::{Body, Bytes},
    extract::State,
    http::{HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use reqwest::{header::CONTENT_TYPE, Client};
use std::{convert::Infallible, time::Duration};
use tokio_stream::StreamExt;
use tower_http::trace::TraceLayer;
use tracing::Span;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}=debug,tower_http=debug", env!("CARGO_CRATE_NAME")).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let client = Client::new();

    let app = Router::new()
        .route("/", get(stream_reqwest_response))
        .route("/stream", get(stream_some_data))
        // Add some logging so we can see the streams going through
        .layer(TraceLayer::new_for_http().on_body_chunk(
            |chunk: &Bytes, _latency: Duration, _span: &Span| {
                tracing::debug!("streaming {} bytes", chunk.len());
            },
        ))
        .with_state(client);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:12345")
        .await
        .unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn stream_reqwest_response(State(client): State<Client>) -> Response {
    let reqwest_response = match client.get("http://127.0.0.1:12345/stream").send().await
    {
        Ok(res) => res,
        Err(err) => {
            tracing::error!(%err, "request failed");
            return (StatusCode::BAD_REQUEST, Body::empty()).into_response();
        }
    };

    let mut response_builder = Response::builder().status(reqwest_response.status());
    let mut header = reqwest_response.headers().clone();
    header.append(
        CONTENT_TYPE,
        HeaderValue::from_static("application/x-ndjson"),
    );
    *response_builder.headers_mut().unwrap() = header;
    response_builder
        .body(Body::from_stream(
            reqwest_response
                .bytes_stream()
                .map(|v| v.map(|_|{
                        let resp = r#"{"model":"qwen2.5:latest","created_at":"2025-02-08T15:55:15.1845709Z","message":{"role":"assistant","content":"The"},"done":false}"#;
                        Bytes::from(format!("{resp}\n"))
                    } )),
        ))
        // This unwrap is fine because the body is empty here
        .unwrap()
}

async fn stream_some_data() -> Body {
    let stream = tokio_stream::iter(0..5)
        .throttle(Duration::from_secs(1))
        .map(|n| n.to_string())
        .map(Ok::<_, Infallible>);
    Body::from_stream(stream)
}
