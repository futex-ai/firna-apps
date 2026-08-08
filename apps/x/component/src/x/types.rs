//! Typed X tool and provider data transfer objects.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::x::metrics_types::GetPostMetricsOutput;

#[derive(Debug, Deserialize)]
pub(crate) struct AppToolCall {
    pub(crate) installation_id: String,
    pub(crate) tool_name: String,
    pub(crate) input: Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GetPostsInput {
    pub(crate) ids: Vec<String>,
    #[serde(default)]
    pub(crate) include_authors: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SearchRecentPostsInput {
    pub(crate) query: String,
    pub(crate) max_results: u64,
    #[serde(default)]
    pub(crate) next_token: Option<String>,
    #[serde(default)]
    pub(crate) include_authors: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreatePostInput {
    pub(crate) text: String,
    #[serde(default)]
    pub(crate) reply_to_post_id: Option<String>,
    #[serde(default)]
    pub(crate) allow_link: bool,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub(crate) enum ToolSuccess {
    GetPosts(GetPostsOutput),
    GetPostMetrics(GetPostMetricsOutput),
    SearchRecentPosts(SearchRecentPostsOutput),
    CreatePost(CreatePostOutput),
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

#[derive(Debug, Serialize)]
pub(crate) struct GetPostsOutput {
    pub(crate) posts: Vec<CompactPost>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) authors: Vec<CompactAuthor>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) missing_ids: Vec<String>,
    pub(crate) result_count: usize,
}

#[derive(Debug, Serialize)]
pub(crate) struct SearchRecentPostsOutput {
    pub(crate) posts: Vec<CompactPost>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) authors: Vec<CompactAuthor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) next_token: Option<String>,
    pub(crate) result_count: usize,
}

#[derive(Debug, Serialize)]
pub(crate) struct CreatePostOutput {
    pub(crate) post: CompactPost,
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
pub(crate) struct CompactAuthor {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) username: String,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct ProviderReadResponse {
    #[serde(default)]
    pub(crate) data: Vec<CompactPost>,
    #[serde(default)]
    pub(crate) includes: ProviderIncludes,
    #[serde(default)]
    pub(crate) meta: ProviderMeta,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct ProviderIncludes {
    #[serde(default)]
    pub(crate) users: Vec<CompactAuthor>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct ProviderMeta {
    pub(crate) next_token: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProviderCreateResponse {
    pub(crate) data: CompactPost,
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

#[derive(Debug, Serialize)]
pub(crate) struct CreatePostBody {
    pub(crate) text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reply: Option<CreateReplyBody>,
}

#[derive(Debug, Serialize)]
pub(crate) struct CreateReplyBody {
    pub(crate) in_reply_to_tweet_id: String,
}
