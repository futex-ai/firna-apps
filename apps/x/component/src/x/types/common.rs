//! Shared call, result, usage, and provider response types.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::x::types::success::ToolSuccess;

#[derive(Debug, Deserialize)]
pub(crate) struct AppToolCall {
    pub(crate) installation_id: String,
    pub(crate) tool_name: String,
    pub(crate) input: Value,
}

#[derive(Debug, Serialize)]
pub(crate) struct PricedToolSuccess {
    pub(crate) output: ToolSuccess,
    pub(crate) usage: ToolUsageReport,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum ToolUsageReport {
    Metered { units: Vec<ToolUsageUnit> },
    ReportedCost { cost_usd_micros: u64 },
}

#[derive(Debug, Serialize)]
pub(crate) struct ToolUsageUnit {
    pub(crate) unit: &'static str,
    pub(crate) quantity: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct CompactPost {
    pub(crate) id: String,
    pub(crate) text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) author_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) created_at: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct CompactUser {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) created_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) profile_image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) protected: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) verified: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) verified_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) public_metrics: Option<UserPublicMetrics>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct UserPublicMetrics {
    pub(crate) followers_count: u64,
    pub(crate) following_count: u64,
    #[serde(alias = "tweet_count")]
    pub(crate) post_count: u64,
    pub(crate) listed_count: u64,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct ProviderIncludes {
    #[serde(default)]
    pub(crate) users: Vec<CompactUser>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct ProviderMeta {
    pub(crate) next_token: Option<String>,
    pub(crate) total_tweet_count: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
pub(crate) struct ProviderCollection<T> {
    #[serde(default)]
    pub(crate) data: Vec<T>,
    #[serde(default)]
    pub(crate) includes: ProviderIncludes,
    #[serde(default)]
    pub(crate) meta: ProviderMeta,
}

#[derive(Debug, Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
pub(crate) struct ProviderSingle<T> {
    #[serde(default)]
    pub(crate) data: Option<T>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct ProviderErrorResponse {
    #[serde(rename = "type")]
    pub(crate) problem_type: Option<String>,
    #[serde(default)]
    pub(crate) errors: Vec<ProviderProblem>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProviderProblem {
    #[serde(rename = "type")]
    pub(crate) problem_type: Option<String>,
}

impl ProviderErrorResponse {
    pub(crate) fn is_usage_capped(&self) -> bool {
        self.problem_type
            .iter()
            .chain(
                self.errors
                    .iter()
                    .filter_map(|problem| problem.problem_type.as_ref()),
            )
            .any(|kind| kind.ends_with("/usage-capped"))
    }
}
