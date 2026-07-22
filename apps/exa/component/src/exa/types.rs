//! JSON DTOs used by the Exa component.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Deserialize)]
pub(crate) struct AppToolCall {
    pub(crate) tool_name: String,
    pub(crate) input: Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExaSearchInput {
    pub(crate) query: String,
    #[serde(default)]
    pub(crate) search_type: Option<SearchType>,
    #[serde(default)]
    pub(crate) num_results: Option<u64>,
    #[serde(default)]
    pub(crate) category: Option<ExaCategory>,
    #[serde(default)]
    pub(crate) include_domains: Vec<String>,
    #[serde(default)]
    pub(crate) exclude_domains: Vec<String>,
    #[serde(default)]
    pub(crate) start_published_date: Option<String>,
    #[serde(default)]
    pub(crate) end_published_date: Option<String>,
    #[serde(default)]
    pub(crate) livecrawl: Option<bool>,
    #[serde(default)]
    pub(crate) highlights_max_characters: Option<u64>,
    #[serde(default)]
    pub(crate) text_max_characters: Option<u64>,
    #[serde(default)]
    pub(crate) timeout_seconds: Option<u64>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum SearchType {
    #[default]
    Auto,
    Fast,
    Instant,
    DeepLite,
    Deep,
    DeepReasoning,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub(crate) enum ExaCategory {
    #[serde(rename = "company")]
    Company,
    #[serde(rename = "people")]
    People,
    #[serde(rename = "research paper")]
    ResearchPaper,
    #[serde(rename = "news")]
    News,
    #[serde(rename = "personal site")]
    PersonalSite,
    #[serde(rename = "financial report")]
    FinancialReport,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExaProviderRequest {
    pub(crate) query: String,
    #[serde(rename = "type")]
    pub(crate) search_type: SearchType,
    pub(crate) num_results: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) category: Option<ExaCategory>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) include_domains: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) exclude_domains: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) start_published_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) end_published_date: Option<String>,
    pub(crate) contents: ExaContentsRequest,
}

impl ExaProviderRequest {
    pub(crate) fn into_value(self) -> Value {
        match serde_json::to_value(self) {
            Ok(value) => value,
            Err(_) => json!({}),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExaContentsRequest {
    pub(crate) highlights: ExaMaxCharacters,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) text: Option<ExaTextRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) max_age_hours: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExaMaxCharacters {
    pub(crate) max_characters: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExaTextRequest {
    pub(crate) max_characters: u64,
    pub(crate) verbosity: String,
}
