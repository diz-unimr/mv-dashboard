use crate::api_client::Case;
use crate::auth::{Backend, handle_login, handle_logout};
use crate::{API_CLIENT, ASSETS};
use askama::Template;
use axum::body::Body;
use axum::extract::Path;
use axum::http::header::CONTENT_TYPE;
use axum::http::{Request, StatusCode};
use axum::middleware::{Next, from_fn};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use axum_login::tower_sessions::SessionManagerLayer;
use axum_login::{AuthManagerLayerBuilder, AuthSession, login_required};
use log::error;
use std::path;
use tower_sessions::cookie::time::Duration;
use tower_sessions::{Expiry, MemoryStore};

pub(crate) fn routes(auth_backend: Backend, cookie_domain: Option<String>) -> axum::Router {
    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store)
        .with_name("mv-dashboard-session")
        .with_path("/mv-dashboard")
        .with_secure(false)
        .with_expiry(Expiry::OnInactivity(Duration::minutes(30)))
        .with_always_save(true);

    let session_layer = if let Some(cookie_domain) = cookie_domain {
        log::info!("Using cookie domain: {}", cookie_domain);
        session_layer.with_domain(cookie_domain.clone())
    } else {
        session_layer
    };

    let auth_layer = AuthManagerLayerBuilder::new(auth_backend, session_layer).build();

    let protected_routes = axum::Router::new()
        .route("/mv-dashboard", get(handle_index_request))
        .layer(login_required!(Backend, login_url = "/mv-dashboard/login"));

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

    let ajax_routes = axum::Router::new()
        .route("/mv-dashboard/cases", get(handle_cases_request))
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
            get(|path| async { serve_asset(path).await }),
        )
        .merge(protected_routes)
        .merge(ajax_routes)
        .layer(auth_layer)
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    username: String,
}

#[derive(Template)]
#[template(path = "fragments/cases.html")]
struct CasesTemplate {
    cases: Vec<Case>,
}

impl CasesTemplate {
    fn case_count(&self) -> usize {
        self.cases.len()
    }

    fn valid_case_count(&self) -> usize {
        self.cases.iter().filter(|case| case.is_valid()).count()
    }

    fn invalid_case_count(&self) -> usize {
        self.cases.iter().filter(|case| !case.is_valid()).count()
    }

    fn hnummer_case_count(&self) -> usize {
        self.cases
            .iter()
            .filter(|case| case.has_valid_case_number())
            .count()
    }
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {}

async fn show_login() -> Result<impl IntoResponse, String> {
    let template = LoginTemplate {};
    Ok(Html(template.render().unwrap()))
}

async fn handle_index_request(auth: AuthSession<Backend>) -> Result<impl IntoResponse, String> {
    let user = auth.user.clone().unwrap_or_default();

    let template = IndexTemplate {
        username: user.username().to_string(),
    };
    Ok(Html(template.render().unwrap()))
}

async fn handle_cases_request(auth: AuthSession<Backend>) -> Result<impl IntoResponse, String> {
    let user = auth.user.clone().unwrap_or_default();

    let response = match API_CLIENT.dashboard(user.clone()).await {
        Ok(data) => data,
        Err(e) => {
            error!("{e}");
            return Ok(Response::builder()
                .status(500)
                .body("Cannot connect to X-API".to_string())
                .unwrap_or_default()
                .into_response());
        }
    };

    let template = CasesTemplate {
        cases: response.cases,
    };
    Ok(Html(template.render().unwrap()).into_response())
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

#[cfg(test)]
mod tests {
    use crate::auth::Backend;
    use crate::routes::routes;
    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode};
    use httpmock::Method::GET;
    use httpmock::MockServer;
    use tower::ServiceExt;

    #[tokio::test]
    async fn should_redirect_from_root_to_mv_dashboard() {
        let response = routes(Backend::new("http://localhost:8080/onkostar"), None)
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/")
                    .body(Body::empty())
                    .expect("request built"),
            )
            .await;

        match response {
            Ok(response) => {
                assert_eq!(response.status(), StatusCode::FOUND);
                assert_eq!(response.headers().get("Location").unwrap(), "/mv-dashboard");
            }
            Err(err) => panic!("Error: {:?}", err),
        }
    }

    #[tokio::test]
    async fn should_redirect_to_login_if_not_logged_in() {
        let response = routes(Backend::new("http://localhost:8080/onkostar"), None)
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/mv-dashboard")
                    .body(Body::empty())
                    .expect("request built"),
            )
            .await;

        match response {
            Ok(response) => {
                assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
                assert_eq!(
                    response.headers().get("Location").unwrap(),
                    "/mv-dashboard/login?next=%2Fmv-dashboard"
                );
            }
            Err(err) => panic!("Error: {:?}", err),
        }
    }

    #[tokio::test]
    async fn should_send_cookie_on_login() {
        let mock_server = MockServer::start();
        let mock = mock_server.mock(|when, then| {
            when.method(GET).path("/x-api/me");
            then.status(200).body("ptsr00");
        });

        let response = routes(Backend::new(&mock_server.base_url()), None)
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/mv-dashboard/login?next=%2Fmv-dashboard")
                    .header("Content-Type", "application/x-www-form-urlencoded")
                    .body(Body::from("username=ptsr00&password=test"))
                    .expect("request built"),
            )
            .await;

        match response {
            Ok(response) => {
                assert_eq!(response.status(), StatusCode::SEE_OTHER);
                assert_eq!(response.headers().get("Location").unwrap(), "/mv-dashboard");
                assert!(response.headers().get("Set-Cookie").is_some());
            }
            Err(err) => panic!("Error: {:?}", err),
        }

        mock.assert();
    }

    #[tokio::test]
    async fn should_login_again() {
        let mock_server = MockServer::start();
        let mock = mock_server.mock(|when, then| {
            when.method(GET).path("/x-api/me");
            then.status(401);
        });

        let response = routes(Backend::new(&mock_server.base_url()), None)
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/mv-dashboard/login?next=%2Fmv-dashboard")
                    .header("Content-Type", "application/x-www-form-urlencoded")
                    .body(Body::from("username=ptsr00&password=test"))
                    .expect("request built"),
            )
            .await;

        match response {
            Ok(response) => {
                assert_eq!(response.status(), StatusCode::SEE_OTHER);
                assert_eq!(
                    response.headers().get("Location").unwrap(),
                    "/mv-dashboard/login"
                );
            }
            Err(err) => panic!("Error: {:?}", err),
        }

        mock.assert();
    }
}
