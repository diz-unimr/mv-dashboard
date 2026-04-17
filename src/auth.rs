use axum::Form;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect};
use axum_login::{AuthSession, AuthUser, AuthnBackend, UserId};
use bcrypt::hash;
use serde::Deserialize;
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct Credentials {
    username: String,
    password: String,
}

#[derive(Clone, Debug)]
pub(crate) struct User {
    username: String,
    password: String,
    hash: String,
}

impl AuthUser for User {
    type Id = String;

    fn id(&self) -> Self::Id {
        self.username.clone()
    }

    fn session_auth_hash(&self) -> &[u8] {
        self.hash.as_bytes()
    }
}

impl Default for User {
    fn default() -> Self {
        Self {
            username: "anonymous".to_string(),
            password: String::new(),
            hash: String::new(),
        }
    }
}

impl User {
    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn password(&self) -> &str {
        &self.password
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Backend {
    users: Arc<RwLock<HashMap<String, User>>>,
    http_client: reqwest::Client,
    onkostar_url: String,
}

impl Backend {
    pub fn new(onkostar_url: &str) -> Self {
        let http_client = reqwest::ClientBuilder::new()
            .user_agent("mv-dashboard/0.1.0")
            .build()
            .unwrap_or_default();

        Self {
            users: Arc::new(RwLock::new(HashMap::new())),
            http_client,
            onkostar_url: onkostar_url.to_string(),
        }
    }
}

impl AuthnBackend for Backend {
    type User = User;
    type Credentials = Credentials;
    type Error = Infallible;

    async fn authenticate(
        &self,
        credentials: Self::Credentials,
    ) -> Result<Option<Self::User>, Self::Error> {
        match self
            .http_client
            .get(format!("{}/x-api/me", self.onkostar_url))
            .basic_auth(&credentials.username, Some(&credentials.password))
            .send()
            .await
        {
            Ok(response) => {
                if !response.status().is_success() {
                    return Ok(None);
                }
                response.text().await.unwrap_or_default()
            }
            Err(_) => return Ok(None),
        };

        let Ok(hash) = hash(credentials.password.clone(), 10) else {
            return Ok(None);
        };

        let user = User {
            username: credentials.username,
            password: credentials.password,
            hash,
        };

        self.users
            .write()
            .await
            .insert(user.username.clone(), user.clone());

        Ok(Some(user))
    }

    async fn get_user(&self, user_id: &UserId<Self>) -> Result<Option<Self::User>, Self::Error> {
        match self.users.read().await.get(user_id) {
            Some(user) => Ok(Some(user.clone())),
            None => Ok(None),
        }
    }
}

pub(crate) async fn handle_login(
    mut auth_session: AuthSession<Backend>,
    Form(credentials): Form<Credentials>,
) -> impl IntoResponse {
    let user = match auth_session.authenticate(credentials).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return Redirect::to("/mv-dashboard/login").into_response();
        }
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    if auth_session.login(&user).await.is_err() {
        return StatusCode::INTERNAL_SERVER_ERROR.into_response();
    }

    Redirect::to("/mv-dashboard").into_response()
}

pub(crate) async fn handle_logout(mut auth_session: AuthSession<Backend>) -> impl IntoResponse {
    match auth_session.logout().await {
        Ok(_) => Redirect::to("/mv-dashboard/login").into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

#[cfg(test)]
#[allow(clippy::panic)]
mod tests {
    use crate::auth::{Backend, Credentials};
    use axum_login::AuthnBackend;
    use httpmock::Method::GET;
    use httpmock::MockServer;

    #[tokio::test]
    async fn test_should_authenticate_user() {
        let mock_server = MockServer::start();
        let mock = mock_server.mock(|when, then| {
            when.method(GET)
                .path("/x-api/me")
                // Authorization: Basic {ptsr00:test}
                .header("Authorization", "Basic cHRzcjAwOnRlc3Q=");
            then.status(200).body("ptsr00");
        });

        let credentials = Credentials {
            username: "ptsr00".to_string(),
            password: "test".to_string(),
        };

        let backend = Backend::new(&mock_server.base_url());

        let result = backend.authenticate(credentials).await;

        assert!(result.is_ok());
        match result.unwrap() {
            Some(user) => {
                assert_eq!(user.username(), "ptsr00");
                assert_eq!(user.password(), "test");
            }
            None => panic!("User not authenticated"),
        }

        mock.assert();
    }

    #[tokio::test]
    async fn test_should_not_authenticate_user() {
        let mock_server = MockServer::start();
        let mock = mock_server.mock(|when, then| {
            when.method(GET).path("/x-api/me");
            then.status(401);
        });

        let credentials = Credentials {
            username: "ptsr00".to_string(),
            password: "test".to_string(),
        };

        let backend = Backend::new(&mock_server.base_url());

        let result = backend.authenticate(credentials).await;

        assert!(result.is_ok());
        assert!(result.unwrap().is_none(), "No user expected!");

        mock.assert();
    }
}
