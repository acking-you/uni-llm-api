//! implements the API for the Uni Llama project
use api::uni_ollama::{chat::api_chat, UniModelInfoRef};
use parking_lot::RwLock;
use reqwest::Client;
use std::fmt::Debug;
use tokio::net::ToSocketAddrs;

use api::uni_ollama::tag::api_tags;
pub use api::uni_ollama::ApiKeyInfo;
pub use api::uni_ollama::ApiKeyProvider;
pub use api::uni_ollama::ModelInfo;
pub use api::uni_ollama::UniModelsInfo;
use axum::{
    routing::{get, post},
    Router,
};

mod api;
pub(crate) mod common;

#[derive(Clone)]
pub(crate) struct SharedState {
    pub client: Client,
    pub model_config: UniModelInfoRef,
}

/// Run the server
pub async fn run_server<A: ToSocketAddrs + Debug>(
    init_models_info: UniModelsInfo,
    addr: A,
) -> anyhow::Result<()> {
    let client = Client::new();
    let model_config = UniModelInfoRef::new(RwLock::new(init_models_info));
    let shared_state = SharedState {
        client,
        model_config,
    };

    let api_routes: Router = Router::new()
        .route("/tags", get(api_tags))
        .route("/chat", post(api_chat))
        .with_state(shared_state);

    let app = Router::new().nest("/api", api_routes);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("Listening on: {:?}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}
