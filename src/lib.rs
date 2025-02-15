//! implements the API for the Uni Llama project
use api::uni_ollama::{chat::api_chat, config::UniModelInfoRef};
use axum::Json;
use parking_lot::RwLock;
use reqwest::Client;
use reqwest::ClientBuilder;
use reqwest::Proxy;
use serde_json::json;
use serde_json::Value;
use std::fmt::Debug;
use tokio::net::ToSocketAddrs;
use tower_http::trace::DefaultMakeSpan;
use tower_http::trace::TraceLayer;

pub use api::uni_ollama::config::ApiKeyInfo;
pub use api::uni_ollama::config::ApiKeyProvider;
pub use api::uni_ollama::config::ModelInfo;
pub use api::uni_ollama::config::UniModelsInfo;
use api::uni_ollama::tag::api_tags;
use axum::{
    routing::{get, post},
    Router,
};

mod api;
pub(crate) mod common;

#[derive(Clone)]
pub(crate) struct SharedState {
    pub proxy_client: Option<Client>,
    pub client: Client,
    pub model_config: UniModelInfoRef,
}

/// Run the server
pub async fn run_server<A: ToSocketAddrs + Debug>(
    init_models_info: UniModelsInfo,
    addr: A,
) -> anyhow::Result<()> {
    let client = ClientBuilder::new().no_proxy().build()?;
    let proxy_client = init_models_info.proxy_url.as_ref().map(|url| {
        ClientBuilder::new()
            .proxy(
                Proxy::https(url)
                    .expect("proxy url must be valid when crate Proxy::https"),
            )
            .proxy(
                Proxy::http(url).expect("proxy url must be valid when crate Proxy::http"),
            )
            .build()
            .expect("construct proxy client nerver fails")
    });

    let model_config = UniModelInfoRef::new(RwLock::new(init_models_info));
    let shared_state = SharedState {
        client,
        model_config,
        proxy_client,
    };

    async fn api_version() -> Json<Value> {
        Json(json!({
            "version":"0.5.1"
        }))
    }

    let api_routes: Router = Router::new()
        .route("/tags", get(api_tags))
        .route("/chat", post(api_chat))
        .route("/version", get(api_version))
        .with_state(shared_state);

    let app = Router::new()
        .nest("/api", api_routes) // logging so we can see whats going on
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        );

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("Listening on: {:?}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}
