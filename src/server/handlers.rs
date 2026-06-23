use crate::CONFIG;
use crate::api_client::XApiClient;
use crate::auth::Backend;
use crate::dashboard::{ApiClient, Case, CaseState, SequencingType};
use askama::Template;
use axum::body::Body;
use axum::extract::{Path, Query};
use axum::http::header::CONTENT_TYPE;
use axum::http::{Response, StatusCode};
use axum::response::{Html, IntoResponse};
use axum_login::AuthSession;
use chrono::{Datelike, Utc};
use include_dir::{Dir, include_dir};
use itertools::Itertools;
use log::error;
use moka::future::Cache;
use serde::Deserialize;
use serde_json::json;
use std::path;
use std::range::RangeInclusive;
use std::sync::LazyLock;
use std::time::Duration;

static ASSETS: Dir = include_dir!("resources/assets");

static API_CLIENT: LazyLock<XApiClient> = LazyLock::new(|| {
    if let Some(cache_duration) = CONFIG.cache_duration {
        let cache = Cache::builder()
            .max_capacity(1)
            .time_to_live(Duration::from_secs(cache_duration.as_secs()))
            .build();
        XApiClient::new(&CONFIG.onkostar_url.clone(), Some(cache))
    } else {
        XApiClient::new(&CONFIG.onkostar_url.clone(), None)
    }
});

#[allow(unused)]
struct LatestMvConsent {
    days_past: i64,
    date: String,
    hnummer_days_past: i64,
    hnummer_date: String,
}

struct SubmissionReport {
    required: usize,
    kdk_only: usize,
    grz_only: usize,
    missing_ongoing: usize,
    missing_all: usize,
}

struct SubmissionSequencingTypeReport {
    none: usize,
    panel: usize,
    wes: usize,
    wgs: usize,
}

#[derive(Deserialize)]
pub(crate) struct CaseFilter {
    year: Option<i32>,
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
    selected_year: Option<i32>,
}

impl CasesTemplate {
    fn case_count(&self) -> usize {
        self.cases.len()
    }

    fn valid_case_count(&self) -> usize {
        self.cases.iter().filter(|case| case.is_valid()).count()
    }

    fn invalid_case_count(&self) -> usize {
        self.submission_report().missing_ongoing
            + self.submission_report().kdk_only
            + self.submission_report().grz_only
    }

    fn hnummer_case_count(&self) -> usize {
        self.cases
            .iter()
            .filter(|case| case.has_valid_case_number())
            .count()
    }

    fn submission_report(&self) -> SubmissionReport {
        SubmissionReport {
            required: self
                .cases
                .iter()
                .filter(|case| {
                    (case.clinical_submission.is_some() && case.genomic_submission.is_some())
                        || case.has_valid_500_submissions()
                })
                .count(),
            kdk_only: self
                .cases
                .iter()
                .filter(|case| {
                    case.clinical_submission.is_some()
                        && case.genomic_submission.is_none()
                        && !case.has_valid_500_submissions()
                })
                .count(),
            grz_only: self
                .cases
                .iter()
                .filter(|case| {
                    case.clinical_submission.is_none() && case.genomic_submission.is_some()
                })
                .count(),
            missing_ongoing: self
                .cases
                .iter()
                .filter(|case| {
                    case.has_valid_case_number()
                        && case.clinical_submission.is_none()
                        && case.genomic_submission.is_none()
                })
                .count(),
            missing_all: self
                .cases
                .iter()
                .filter(|case| {
                    case.clinical_submission.is_none() && case.genomic_submission.is_none()
                })
                .count(),
        }
    }

    fn submission_sequencing_type_report(&self) -> SubmissionSequencingTypeReport {
        fn count(cases: &[Case], sequencing_type: &SequencingType) -> usize {
            cases
                .iter()
                .filter(|case| case.is_valid())
                .filter(|case| {
                    if let Some(submission) = &case.clinical_submission {
                        &submission.sequencing_type == sequencing_type
                    } else {
                        false
                    }
                })
                .count()
        }

        SubmissionSequencingTypeReport {
            none: count(&self.cases, &SequencingType::None),
            panel: count(&self.cases, &SequencingType::Panel),
            wes: count(&self.cases, &SequencingType::Wes)
                + count(&self.cases, &SequencingType::WesLr),
            wgs: count(&self.cases, &SequencingType::Wgs),
        }
    }

    fn json_data(&self) -> String {
        let data = json!({
            "cases": {
                "case_count": self.case_count(),
                "hnumber_case_count": self.hnummer_case_count(),
                "valid_case_count": self.valid_case_count(),
                "invalid_case_count": self.invalid_case_count(),
            },
            "submission_reports": {
                "required": self.submission_report().required,
                "kdk_only": self.submission_report().kdk_only,
                "grz_only": self.submission_report().grz_only,
                "missing_ongoing": self.submission_report().missing_ongoing,
                "missing_all": self.submission_report().missing_all,
            }
        });

        serde_json::to_string(&data).unwrap_or_default()
    }

    fn latest_mv_consent(&self) -> Option<LatestMvConsent> {
        if self.cases.is_empty() {
            return None;
        }

        let latest = self
            .cases
            .iter()
            .filter_map(|case| case.clone().mv_consent)
            .filter_map(|mv_consent| {
                chrono::NaiveDate::parse_from_str(&mv_consent.consent_date, "%Y-%m-%d").ok()
            })
            .sorted()
            .last();

        let hnummer_latest = self
            .cases
            .iter()
            .filter(|case| case.has_valid_case_number())
            .filter_map(|case| case.clone().mv_consent)
            .filter_map(|mv_consent| {
                chrono::NaiveDate::parse_from_str(&mv_consent.consent_date, "%Y-%m-%d").ok()
            })
            .sorted()
            .last();

        Some(LatestMvConsent {
            days_past: match latest {
                Some(date) => Utc::now()
                    .date_naive()
                    .signed_duration_since(date)
                    .num_days(),
                None => 0,
            },
            date: match latest {
                Some(date) => date.to_string(),
                None => String::new(),
            },
            hnummer_days_past: match hnummer_latest {
                Some(date) => Utc::now()
                    .date_naive()
                    .signed_duration_since(date)
                    .num_days(),
                None => 0,
            },
            hnummer_date: match hnummer_latest {
                Some(date) => date.to_string(),
                None => String::new(),
            },
        })
    }

    fn years(&self) -> RangeInclusive<i32> {
        RangeInclusive::from(2025..=Utc::now().year())
    }
}

#[derive(Template)]
#[template(path = "fragments/followup.html")]
struct FollowUpTemplate {
    cases: Vec<Case>,
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {}

#[allow(clippy::expect_used)]
pub(super) async fn show_login() -> Result<impl IntoResponse, String> {
    let template = LoginTemplate {};
    Ok(Html(template.render().expect("Could not render template")))
}

#[allow(clippy::expect_used)]
pub(super) async fn handle_index_request(
    auth: AuthSession<Backend>,
) -> Result<impl IntoResponse, String> {
    let user = auth.user.clone().unwrap_or_default();

    let template = IndexTemplate {
        username: user.username().to_string(),
    };
    Ok(Html(template.render().expect("Could not render template")))
}

#[allow(clippy::expect_used)]
pub(super) async fn handle_cases_request(
    auth: AuthSession<Backend>,
    filter: Query<CaseFilter>,
) -> Result<impl IntoResponse, String> {
    let user = auth.user.clone().unwrap_or_default();

    let response = match API_CLIENT.request_dashboard_data(user.clone()).await {
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

    let cases = match filter.year {
        Some(year) => response
            .cases
            .into_iter()
            .filter(|case| match case.leistungsdatum() {
                Some(date) => date.year() == year,
                None => false,
            })
            .collect_vec(),
        None => response.cases.clone(),
    };

    let template = CasesTemplate {
        cases,
        selected_year: filter.year,
    };
    Ok(Html(template.render().expect("Could not render template")).into_response())
}

#[allow(clippy::expect_used)]
pub(super) async fn handle_followup_request(
    auth: AuthSession<Backend>,
) -> Result<impl IntoResponse, String> {
    let user = auth.user.clone().unwrap_or_default();

    let response = match API_CLIENT.request_dashboard_data(user.clone()).await {
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

    let template = FollowUpTemplate {
        cases: response
            .cases
            .into_iter()
            .filter(|case| case.next_follow_up_due.is_some())
            .sorted_unstable_by_key(|case| {
                case.next_follow_up_due
                    .as_ref()
                    .expect("no next follow up date")
                    .clone()
            })
            .collect_vec(),
    };
    Ok(Html(template.render().expect("Could not render template")).into_response())
}

#[allow(clippy::expect_used)]
pub(super) fn serve_asset(path: Option<Path<String>>) -> impl IntoResponse {
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
    .expect("Could not serve asset")
}

#[cfg(test)]
#[allow(clippy::expect_used)]
#[allow(clippy::panic)]
mod tests {
    use crate::auth::Backend;
    use crate::server::routes::routes;
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
                assert_eq!(
                    response
                        .headers()
                        .get("Location")
                        .expect("Could not get Location header"),
                    "/mv-dashboard"
                );
            }
            Err(err) => panic!("Error: {err:?}"),
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
                    response
                        .headers()
                        .get("Location")
                        .expect("Could not get Location header"),
                    "/mv-dashboard/login?next=%2Fmv-dashboard"
                );
            }
            Err(err) => panic!("Error: {err:?}"),
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
                assert_eq!(
                    response
                        .headers()
                        .get("Location")
                        .expect("Could not get Location header"),
                    "/mv-dashboard"
                );
                assert!(response.headers().get("Set-Cookie").is_some());
            }
            Err(err) => panic!("Error: {err:?}"),
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
                    response
                        .headers()
                        .get("Location")
                        .expect("Could not get Location header"),
                    "/mv-dashboard/login"
                );
            }
            Err(err) => panic!("Error: {err:?}"),
        }

        mock.assert();
    }
}
