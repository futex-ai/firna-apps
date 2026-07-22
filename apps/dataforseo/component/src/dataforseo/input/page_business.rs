//! Instant-page and business input DTOs.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InstantPageAuditInput {
    pub(crate) url: String,
    pub(crate) timeout_seconds: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BusinessSearchInput {
    pub(crate) latitude: f64,
    pub(crate) longitude: f64,
    pub(crate) radius_km: f64,
    pub(crate) query: Option<String>,
    pub(crate) categories: Option<Vec<String>>,
    pub(crate) is_claimed: Option<bool>,
    pub(crate) min_rating: Option<f64>,
    pub(crate) limit: Option<u64>,
    pub(crate) offset: Option<u64>,
    pub(crate) timeout_seconds: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BusinessInfoInput {
    pub(crate) business_name: Option<String>,
    pub(crate) cid: Option<String>,
    pub(crate) place_id: Option<String>,
    pub(crate) location_name: Option<String>,
    pub(crate) location_code: Option<i64>,
    pub(crate) language_name: Option<String>,
    pub(crate) language_code: Option<String>,
    pub(crate) timeout_seconds: Option<u64>,
}
