use crate::auth::User;
use crate::dashboard::{ApiClient, Case, DashboardData};
use moka::future::Cache;

pub(crate) struct XApiClient {
    base_url: String,
    http_client: reqwest::Client,
    cache: Option<Cache<String, Vec<Case>>>,
}

impl XApiClient {
    pub fn new(base_url: &str, cache: Option<Cache<String, Vec<Case>>>) -> Self {
        XApiClient {
            base_url: Self::clean_base_url(base_url),
            http_client: reqwest::ClientBuilder::new()
                .user_agent(concat!("mv-dashboard", env!("CARGO_PKG_VERSION")))
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
            cache,
        }
    }

    fn clean_base_url(url: &str) -> String {
        if let Some(stripped) = url.strip_suffix('/') {
            return Self::clean_base_url(stripped);
        }
        url.to_string()
    }

    fn full_url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }
}

impl ApiClient for XApiClient
where
    Self: Send + Sync,
{
    async fn request_dashboard_data(&self, user: User) -> Result<DashboardData, String> {
        if let Some(cache) = &self.cache
            && let Some(cases) = cache.get("dashboard").await
        {
            return Ok(DashboardData { cases });
        }

        let response = self
            .http_client
            .get(self.full_url("/x-api/mv-dashboard"))
            .basic_auth(user.username(), Some(&user.password()))
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| format!("Cannot connect to X-API: {e}"))?;

        let mut cases = response
            .json::<Vec<Case>>()
            .await
            .map_err(|e| format!("Cannot read X-API response: {e}"))?;

        cases.sort_unstable_by_key(Case::formatted_case_id);

        if let Some(cache) = &self.cache {
            cache.insert("dashboard".to_string(), cases.clone()).await;
        }

        Ok(DashboardData { cases })
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use crate::api_client::XApiClient;
    use crate::auth::User;
    use crate::dashboard::{ApiClient, Case};
    use httpmock::Method::GET;
    use httpmock::MockServer;
    use moka::future::Cache;
    use std::fs;

    #[tokio::test]
    async fn test_should_request_from_api() {
        let content = fs::read_to_string("testresources/test1.json").expect("Unable to read file");
        let mock_server = MockServer::start();
        let mock = mock_server.mock(|when, then| {
            when.method(GET).path("/x-api/mv-dashboard");
            then.status(200).body(format!("[{content}]"));
        });

        let api_client = XApiClient::new(&mock_server.base_url(), None);
        let response = api_client.request_dashboard_data(User::default()).await;

        assert!(response.is_ok());

        mock.assert();
    }

    #[tokio::test]
    async fn test_should_use_cached_value() {
        let mock_server = MockServer::start();
        let mock = mock_server.mock(|when, then| {
            when.method(GET).path("/x-api/mv-dashboard");
            then.status(500)
                .body("some nonsense for testing purposes - should not get requested");
        });

        let expected_content = serde_json::from_str::<Case>(
            &fs::read_to_string("testresources/test1.json").expect("Unable to read file"),
        )
        .expect("Unable to parse json");
        let cache = Cache::builder().build();
        cache
            .insert("dashboard".to_string(), vec![expected_content.clone()])
            .await;

        let api_client = XApiClient::new(&mock_server.base_url(), Some(cache));
        let response = api_client.request_dashboard_data(User::default()).await;

        assert!(response.is_ok());
        assert!(
            response
                .expect("error response")
                .cases
                .contains(&expected_content)
        );

        mock.assert_calls(0);
    }
}
