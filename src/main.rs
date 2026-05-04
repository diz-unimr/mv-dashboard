use clap::Parser;
use std::sync::LazyLock;

mod api_client;
mod auth;
mod config;
mod dashboard;
mod server;

static CONFIG: LazyLock<config::Config> = LazyLock::new(config::Config::parse);

#[tokio::main]
async fn main() -> Result<(), String> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let conf = config::Config::parse();
    server::start_server(&conf).await
}
