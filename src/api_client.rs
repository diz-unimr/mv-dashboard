use crate::CONFIG;
use crate::auth::User;
use chrono::NaiveDate;
use itertools::{Itertools, sorted};
use log::error;
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
    pub(crate) deceased: Option<bool>,
    pub(crate) deceased_at_first_mtb: Option<bool>,
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
        self.is_first_mtb_after_mv_consent()
            && self.broad_consent.is_some()
            && self.mtb.is_some()
            && self.clinical_submission.is_some()
            && self.genomic_submission.is_some()
            && match &self.mv_consent {
                Some(mv_consent) => mv_consent.is_valid(),
                None => false,
            }
            && match &self.deceased_at_first_mtb {
                Some(deceased_at_first_mtb) => !*deceased_at_first_mtb,
                None => true,
            }
    }

    pub fn has_valid_case_number(&self) -> bool {
        !self.case_id.starts_with("!")
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

    pub fn is_first_mtb_after_mv_consent(&self) -> bool {
        if let Some(mv_consent) = &self.mv_consent
            && let Some(mtb) = &self.mtb
        {
            let Some(care_plans) = &mtb.care_plans else {
                return false;
            };

            if let Some(first_care_plan) = sorted(care_plans).collect_vec().first() {
                let Ok(mv_consent_date) =
                    NaiveDate::parse_from_str(&mv_consent.consent_date, "%Y-%m-%d")
                else {
                    return false;
                };
                let Ok(first_care_plan_date) =
                    NaiveDate::parse_from_str(&first_care_plan.date, "%Y-%m-%d")
                else {
                    return false;
                };
                return mv_consent_date <= first_care_plan_date;
            }
            return false;
        }
        false
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Mtb {
    pub(crate) registration_date: String,
    pub(crate) care_plans: Option<Vec<CarePlan>>,
}

#[derive(serde::Deserialize, Debug, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CarePlan {
    pub(crate) date: String,
}

impl PartialOrd for CarePlan {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CarePlan {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.naive_date().cmp(&other.naive_date())
    }
}

impl CarePlan {
    pub fn naive_date(&self) -> Option<NaiveDate> {
        NaiveDate::parse_from_str(&self.date, "%Y-%m-%d").ok()
    }
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

#[cfg(test)]
mod tests {
    use crate::api_client::{BroadConsent, CarePlan, Case, Mtb, MvConsent, Submission};
    use itertools::{Itertools, sorted};

    #[test]
    fn test_should_find_first_mtb_before_mv_consent() {
        let case = Case {
            case_id: "H1234-26".to_string(),
            guid: Some("TESTGUID".to_string()),
            deceased: None,
            deceased_at_first_mtb: None,
            mv_consent: Some(MvConsent {
                consent_date: "2026-04-01".to_string(),
                sequencing: true,
                case_identification: true,
                re_identification: true,
            }),
            broad_consent: Some(BroadConsent {
                consent_date: "2026-04-01".to_string(),
                electronic_available: true,
            }),
            mtb: Some(Mtb {
                registration_date: "2026-04-13".to_string(),
                care_plans: Some(vec![CarePlan {
                    date: "2026-04-13".to_string(),
                }]),
            }),
            clinical_submission: Some(Submission {
                id: "KDK1234567".to_string(),
                date: "2026-04-13".to_string(),
            }),
            genomic_submission: Some(Submission {
                id: "KDK1234567".to_string(),
                date: "13.04.2026".to_string(),
            }),
        };

        assert!(case.is_valid());
        assert!(case.is_first_mtb_after_mv_consent())
    }

    #[test]
    fn test_should_find_first_mtb_same_day_mv_consent() {
        let case = Case {
            case_id: "H1234-26".to_string(),
            guid: Some("TESTGUID".to_string()),
            deceased: None,
            deceased_at_first_mtb: None,
            mv_consent: Some(MvConsent {
                consent_date: "2026-04-01".to_string(),
                sequencing: true,
                case_identification: true,
                re_identification: true,
            }),
            broad_consent: Some(BroadConsent {
                consent_date: "2026-04-13".to_string(),
                electronic_available: true,
            }),
            mtb: Some(Mtb {
                registration_date: "2026-04-13".to_string(),
                care_plans: Some(vec![
                    CarePlan {
                        date: "2026-04-12".to_string(),
                    },
                    CarePlan {
                        date: "2026-04-14".to_string(),
                    },
                ]),
            }),
            clinical_submission: Some(Submission {
                id: "KDK1234567".to_string(),
                date: "2026-04-13".to_string(),
            }),
            genomic_submission: Some(Submission {
                id: "KDK1234567".to_string(),
                date: "2026-04-13".to_string(),
            }),
        };

        assert!(case.is_valid());
        assert!(case.is_first_mtb_after_mv_consent())
    }

    #[test]
    fn test_should_find_first_mtb_after_mv_consent() {
        let case = Case {
            case_id: "H1234-26".to_string(),
            guid: Some("TESTGUID".to_string()),
            deceased: None,
            deceased_at_first_mtb: None,
            mv_consent: Some(MvConsent {
                consent_date: "2026-04-01".to_string(),
                sequencing: true,
                case_identification: true,
                re_identification: true,
            }),
            broad_consent: Some(BroadConsent {
                consent_date: "2026-04-01".to_string(),
                electronic_available: true,
            }),
            mtb: Some(Mtb {
                registration_date: "2026-03-31".to_string(),
                care_plans: Some(vec![CarePlan {
                    date: "2026-03-31".to_string(),
                }]),
            }),
            clinical_submission: Some(Submission {
                id: "KDK1234567".to_string(),
                date: "2026-04-13".to_string(),
            }),
            genomic_submission: Some(Submission {
                id: "KDK1234567".to_string(),
                date: "2026-04-13".to_string(),
            }),
        };

        assert!(!case.is_valid());
        assert!(!case.is_first_mtb_after_mv_consent())
    }

    #[test]
    fn test_should_not_any_mtb_after_mv_consent() {
        let case = Case {
            case_id: "H1234-26".to_string(),
            guid: Some("TESTGUID".to_string()),
            deceased: None,
            deceased_at_first_mtb: None,
            mv_consent: Some(MvConsent {
                consent_date: "2026-04-01".to_string(),
                sequencing: true,
                case_identification: true,
                re_identification: true,
            }),
            broad_consent: Some(BroadConsent {
                consent_date: "2026-04-01".to_string(),
                electronic_available: true,
            }),
            mtb: Some(Mtb {
                registration_date: "2026-03-31".to_string(),
                care_plans: None,
            }),
            clinical_submission: Some(Submission {
                id: "KDK1234567".to_string(),
                date: "2026-04-13".to_string(),
            }),
            genomic_submission: Some(Submission {
                id: "KDK1234567".to_string(),
                date: "2026-04-13".to_string(),
            }),
        };

        assert!(!case.is_valid());
        assert!(!case.is_first_mtb_after_mv_consent())
    }

    #[test]
    fn test_should_sort_care_plans_by_date() {
        let care_plans = vec![
            CarePlan {
                date: "2026-04-13".to_string(),
            },
            CarePlan {
                date: "2026-04-12".to_string(),
            },
        ];

        let actual = sorted(care_plans).collect_vec();
        assert_eq!(
            actual,
            vec![
                CarePlan {
                    date: "2026-04-12".to_string(),
                },
                CarePlan {
                    date: "2026-04-13".to_string(),
                },
            ]
        );
    }
}
