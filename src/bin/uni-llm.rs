//! A binary for the Uni LLM project
use std::fs::OpenOptions;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use uni_llm_api::{run_server, UniModelsInfo};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or(EnvFilter::new("info")),
        )
        .with(tracing_subscriber::fmt::layer().pretty())
        .init();

    let reader = OpenOptions::new()
        .read(true)
        .open("./config/test.json")
        .unwrap();
    let models_info: UniModelsInfo = serde_json::from_reader(reader).unwrap();

    run_server(models_info, "0.0.0.0:12345").await.unwrap();
}
