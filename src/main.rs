use crate::api_client::Case;
use crate::auth::{Backend, handle_login, handle_logout};
use askama::Template;
use axum::body::Body;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use axum_login::tower_sessions::SessionManagerLayer;
use axum_login::{AuthManagerLayerBuilder, AuthSession, login_required};
use clap::Parser;
use include_dir::{Dir, include_dir};
use std::path;
use std::sync::LazyLock;
use tower_sessions::MemoryStore;

pub mod api_client;
pub mod auth;
pub mod config;

static CONFIG: LazyLock<config::Config> = LazyLock::new(config::Config::parse);

static ASSETS: Dir = include_dir!("resources/assets");

static API_CLIENT: LazyLock<api_client::ApiClient> =
    LazyLock::new(|| api_client::ApiClient::new(&CONFIG.onkostar_url.clone()));

fn routes() -> axum::Router {
    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store);
    let backend = Backend::default();

    let auth_layer = AuthManagerLayerBuilder::new(backend, session_layer).build();

    axum::Router::new()
        .route("/mv-dashboard", get(handle_request))
        .layer(login_required!(Backend, login_url = "/mv-dashboard/login"))
        .route("/mv-dashboard/login", get(show_login).post(handle_login))
        .route("/mv-dashboard/logout", get(handle_logout))
        .route(
            "/mv-dashboard/assets/{*path}",
            get(|path| async { serve_asset(path).await }),
        )
        .layer(auth_layer)
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    username: String,
    cases: Vec<Case>,
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {}

async fn show_login() -> Result<impl IntoResponse, String> {
    let template = LoginTemplate {};
    Ok(Html(template.render().unwrap()))
}

async fn handle_request(auth: AuthSession<Backend>) -> Result<impl IntoResponse, String> {
    let user = auth.user.clone().unwrap_or_default();

    let response = match API_CLIENT.dashboard(user.clone()).await {
        Ok(data) => data,
        Err(_) => return Ok(Html("Cannot connect to X-API".to_string())),
    };

    let template = IndexTemplate {
        username: user.username().to_string(),
        cases: response.cases,
    };
    Ok(Html(template.render().unwrap()))
}

async fn serve_asset(path: Option<Path<String>>) -> impl IntoResponse {
    fn get_mimetype(path: &path::Path) -> Option<&str> {
        if let Some(extension) = path.extension() {
            return match extension.to_str() {
                Some("css") => Some("text/css"),
                Some("js") => Some("application/javascript"),
                _ => None,
            };
        }
        None
    }

    match path {
        Some(path) => match ASSETS.get_file(path.to_string()) {
            Some(file) => {
                if let Some(mime_type) = get_mimetype(file.path()) {
                    Response::builder()
                        .status(StatusCode::OK)
                        .header(CONTENT_TYPE, mime_type)
                        .body(Body::from(file.contents()))
                } else {
                    Response::builder()
                        .status(StatusCode::OK)
                        .body(Body::from(file.contents()))
                }
            }
            None => Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("".as_bytes())),
        },
        None => Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from("".as_bytes())),
    }
    .unwrap()
}

#[tokio::main]
async fn main() -> Result<(), String> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let conf = config::Config::parse();

    match tokio::net::TcpListener::bind(&conf.listen).await {
        Ok(listener) => {
            log::info!("Starting application listening on '{}'", &conf.listen);
            if let Err(err) = axum::serve(listener, routes()).await {
                return Err(err.to_string());
            }
        }
        Err(err) => return Err(format!("Cannot listening on '{}': {}", &conf.listen, err)),
    }

    Ok(())
}
