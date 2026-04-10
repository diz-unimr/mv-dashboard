use crate::CONFIG;
use crate::auth::User;
use regex::Regex;

pub(crate) struct ApiClient {
    base_url: String,
    http_client: reqwest::Client,
}

impl ApiClient {
    pub fn new(base_url: &str) -> Self {
        ApiClient {
            base_url: Self::clean_base_url(base_url),
            http_client: reqwest::ClientBuilder::new()
                .user_agent("mv-dashboard/0.1.0")
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_default(),
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

    pub async fn dashboard(&self, user: User) -> Result<DashboardResponse, String> {
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
            .map_err(|_| "Cannot read X-API response".to_string())?;

        cases.sort_unstable_by_key(|item| item.formatted_case_id());

        Ok(DashboardResponse { cases })
    }
}

pub(crate) struct DashboardResponse {
    pub(crate) cases: Vec<Case>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Case {
    pub(crate) case_id: String,
    pub(crate) guid: Option<String>,
    pub(crate) mtb: Option<Mtb>,
    pub(crate) mv_consent: Option<MvConsent>,
    pub(crate) broad_consent: Option<BroadConsent>,
    pub(crate) clinical_submission: Option<Submission>,
    pub(crate) genomic_submission: Option<Submission>,
}

impl Case {
    pub fn formatted_case_id(&self) -> String {
        let re = Regex::new(r"^H(?<number>\d+)-(?<year>\d{2})$").expect("Invalid regex pattern");

        let caps = match re.captures(&self.case_id) {
            Some(caps) => caps,
            None => return self.case_id.clone(),
        };

        let number = match caps.name("number") {
            Some(number) => number.as_str(),
            None => return self.case_id.clone(),
        };
        let year = match caps.name("year") {
            Some(year) => year.as_str(),
            None => return self.case_id.clone(),
        };

        format!("H/20{}/{}", year, number)
    }

    pub fn is_valid(&self) -> bool {
        self.mtb.is_some()
            && self.mv_consent.is_some()
            && self.broad_consent.is_some()
            && self.mv_consent.is_some()
            && self.genomic_submission.is_some()
    }

    pub fn onkostar_url(&self) -> Option<String> {
        if let Some(guid) = &self.guid {
            return Some(format!(
                "{}/index.html?procedureId={}",
                &CONFIG.onkostar_url, guid
            ));
        };

        None
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Mtb {
    pub(crate) registration_date: String,
    pub(crate) care_plans: Option<Vec<CarePlan>>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CarePlan {
    pub(crate) date: String,
}

trait Consent {
    fn is_valid(&self) -> bool;
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MvConsent {
    pub(crate) consent_date: String,
    pub(crate) sequencing: bool,
    pub(crate) case_identification: bool,
    pub(crate) re_identification: bool,
}

impl Consent for MvConsent {
    fn is_valid(&self) -> bool {
        self.sequencing
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BroadConsent {
    pub(crate) consent_date: String,
    pub(crate) electronic_available: bool,
}

impl Consent for BroadConsent {
    fn is_valid(&self) -> bool {
        self.electronic_available
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Submission {
    #[serde(default = "String::new")]
    pub(crate) id: String,
    pub(crate) date: String,
}
