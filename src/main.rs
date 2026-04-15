use crate::auth::Backend;
use crate::routes::routes;
use clap::Parser;
use include_dir::{Dir, include_dir};
use std::sync::LazyLock;

pub mod api_client;
pub mod auth;
pub mod config;
mod routes;

static CONFIG: LazyLock<config::Config> = LazyLock::new(config::Config::parse);

static ASSETS: Dir = include_dir!("resources/assets");

static API_CLIENT: LazyLock<api_client::ApiClient> =
    LazyLock::new(|| api_client::ApiClient::new(&CONFIG.onkostar_url.clone()));

#[tokio::main]
async fn main() -> Result<(), String> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let conf = config::Config::parse();

    match tokio::net::TcpListener::bind(&conf.listen).await {
        Ok(listener) => {
            log::info!("Starting application listening on '{}'", &conf.listen);
            if let Err(err) = axum::serve(
                listener,
                routes(
                    Backend::new(&CONFIG.onkostar_url),
                    CONFIG.cookie_domain.clone(),
                ),
            )
            .await
            {
                return Err(err.to_string());
            }
        }
        Err(err) => return Err(format!("Cannot listening on '{}': {}", &conf.listen, err)),
    }

    Ok(())
}
