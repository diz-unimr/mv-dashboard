use crate::auth::Backend;
use crate::server::routes::routes;
use crate::{CONFIG, config};

mod handlers;
mod routes;

pub(crate) async fn start_server(config: &config::Config) -> Result<(), String> {
    match tokio::net::TcpListener::bind(&config.listen).await {
        Ok(listener) => {
            log::info!("Starting application listening on '{}'", config.listen);
            if let Err(err) = axum::serve(
                listener,
                routes(
                    Backend::new(&CONFIG.onkostar_url),
                    CONFIG.cookie_domain.clone(),
                ),
            )
            .await
            {
                Err(err.to_string())
            } else {
                Ok(())
            }
        }
        Err(err) => Err(format!("Cannot listening on '{}': {}", config.listen, err)),
    }
}
