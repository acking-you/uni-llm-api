//! A binary for the Uni LLM project
use std::{fs::OpenOptions, path::PathBuf};

use clap::Parser;
use shellexpand::tilde;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use uni_llm_api::{run_server, UniModelsInfo};

/// Custom command-line parsing structure
#[derive(Parser, Debug)]
#[command(
    author = "L_B__",
    version,
    about = "https://github.com/acking-you/uni-llm-api"
)]
struct Cli {
    /// Port number for the service to run on
    #[arg(
        short,
        long,
        default_value = "12345",
        value_parser = clap::value_parser!(u16).range(1..=65535)
    )]
    port: u16,
    /// Root directory of the configuration file (support `~` expand)
    #[arg(short, long)]
    config_dir: Option<String>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or(EnvFilter::new("info")),
        )
        .with(tracing_subscriber::fmt::layer().pretty())
        .init();
    let cli = Cli::parse();

    let config_dir = cli
        .config_dir
        .map(|p| PathBuf::from(tilde(&p).into_owned()))
        .unwrap_or_else(|| {
            dirs::home_dir()
                .expect("Unable to retrieve the user's home directory")
                .join(".uni-llm")
        });

    if !config_dir.exists() {
        std::fs::create_dir_all(&config_dir)
            .unwrap_or_else(|_| panic!("Unable to create directory: {:?}", config_dir));
        tracing::info!(
            "Configuration directory automatically created: {:?}",
            config_dir
        );
    }
    let config_path = config_dir.join("config.json");
    if !config_path.exists() {
        let config = UniModelsInfo::default();
        let writer = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&config_path)
            .expect("Unable to create config file");
        serde_json::to_writer_pretty(writer, &config)
            .expect("Unable to serialize config");
        return;
    }
    let reader = OpenOptions::new()
        .read(true)
        .open(&config_path)
        .expect("Unable to read config file");
    let mut models_info: UniModelsInfo =
        serde_json::from_reader(reader).expect("Unable to parse config file");
    models_info.insert_latest_tag_for_openwebui();

    run_server(models_info, format!("0.0.0.0:{}", cli.port))
        .await
        .expect("Run Server error");
}
