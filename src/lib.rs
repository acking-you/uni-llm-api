use anyhow::Context;
use api::uni_ollama::{api_chat, api_tags};
use axum::{
    routing::{get, post},
    Router,
};

mod api;

pub async fn run_server<A: ToSocketAddrs + Debug>(addr: A) -> anyhow::Result<()> {
    let api_routes: Router = Router::new()
        .route("/tags", get(api_tags))
        .route("chat", post(api_chat));
    let app = Router::new().nest("/api", api_routes);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("with addr:{addr:?}"))?;
    tracing::info!("Listening on: {:?}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}
