//! Backlink input DTOs.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum BacklinksStatus {
    #[default]
    Live,
    Lost,
    All,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum BacklinkMode {
    #[default]
    AsIs,
    OnePerDomain,
    OnePerAnchor,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BacklinksSummaryInput {
    pub(crate) target: String,
    pub(crate) include_subdomains: Option<bool>,
    pub(crate) backlinks_status: Option<BacklinksStatus>,
    pub(crate) dofollow_only: Option<bool>,
    pub(crate) timeout_seconds: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BacklinksInput {
    pub(crate) target: String,
    pub(crate) include_subdomains: Option<bool>,
    pub(crate) backlinks_status: Option<BacklinksStatus>,
    pub(crate) dofollow_only: Option<bool>,
    pub(crate) limit: Option<u64>,
    pub(crate) offset: Option<u64>,
    pub(crate) mode: Option<BacklinkMode>,
    pub(crate) timeout_seconds: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReferringDomainsInput {
    pub(crate) target: String,
    pub(crate) include_subdomains: Option<bool>,
    pub(crate) backlinks_status: Option<BacklinksStatus>,
    pub(crate) dofollow_only: Option<bool>,
    pub(crate) limit: Option<u64>,
    pub(crate) offset: Option<u64>,
    pub(crate) min_backlinks: Option<u64>,
    pub(crate) timeout_seconds: Option<u64>,
}
