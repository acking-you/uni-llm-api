//! implements the API for the Uni Llama project
use std::fmt::Debug;
use tokio::net::ToSocketAddrs;

use api::uni_ollama::{api_chat, api_tags};
use axum::{
    routing::{get, post},
    Router,
};

mod api;

/// Run the server
pub async fn run_server<A: ToSocketAddrs + Debug>(addr: A) -> anyhow::Result<()> {
    let api_routes: Router = Router::new()
        .route("/tags", get(api_tags))
        .route("/chat", post(api_chat));
    let app = Router::new().nest("/api", api_routes);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("Listening on: {:?}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}
