use crate::auth::{Backend, handle_login, handle_logout};
use crate::server::handlers::{
    handle_cases_request, handle_followup_request, handle_index_request, serve_asset, show_login,
};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::middleware::{Next, from_fn};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum_login::tower_sessions::SessionManagerLayer;
use axum_login::{AuthManagerLayerBuilder, AuthSession, login_required};
use tower_sessions::cookie::time::Duration;
use tower_sessions::{Expiry, MemoryStore};

pub(crate) fn routes(auth_backend: Backend, cookie_domain: Option<String>) -> axum::Router {
    async fn check_ajax_auth(
        auth: AuthSession<Backend>,
        req: Request<Body>,
        next: Next,
    ) -> Response {
        if auth.user.is_some() {
            return next.run(req).await;
        }

        Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body("Not logged in".to_string())
            .unwrap_or_default()
            .into_response()
    }

    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_name("mv-dashboard-session")
        .with_path("/mv-dashboard")
        .with_secure(false)
        .with_expiry(Expiry::OnInactivity(Duration::minutes(30)))
        .with_always_save(true);

    let session_layer = if let Some(cookie_domain) = cookie_domain {
        log::info!("Using cookie domain: {cookie_domain}");
        session_layer.with_domain(cookie_domain.clone())
    } else {
        session_layer
    };

    let auth_layer = AuthManagerLayerBuilder::new(auth_backend, session_layer).build();

    let protected_routes = axum::Router::new()
        .route("/mv-dashboard", get(handle_index_request))
        .layer(login_required!(Backend, login_url = "/mv-dashboard/login"));

    let ajax_routes = axum::Router::new()
        .route("/mv-dashboard/cases", get(handle_cases_request))
        .route("/mv-dashboard/followups", get(handle_followup_request))
        .layer(from_fn(check_ajax_auth));

    axum::Router::new()
        .route(
            "/",
            get(|| async {
                Response::builder()
                    .status(StatusCode::FOUND)
                    .header("Location", "/mv-dashboard")
                    .body(Body::empty())
                    .unwrap_or_default()
                    .into_response()
            }),
        )
        .route("/mv-dashboard/login", get(show_login).post(handle_login))
        .route("/mv-dashboard/logout", get(handle_logout))
        .route(
            "/mv-dashboard/assets/{*path}",
            get(|path| async { serve_asset(path) }),
        )
        .merge(protected_routes)
        .merge(ajax_routes)
        .layer(auth_layer)
}
