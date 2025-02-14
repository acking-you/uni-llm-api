//! A binary for the Uni Llama project
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use uni_llm_api::{run_server, UniModelsInfo};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or(EnvFilter::new("debug")),
        )
        .with(tracing_subscriber::fmt::layer().pretty())
        .init();

    run_server(UniModelsInfo::default(), "0.0.0.0:12345")
        .await
        .unwrap();
}
