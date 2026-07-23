//! Search and keyword input DTOs.

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GoogleSerpInput {
    pub(crate) keyword: String,
    pub(crate) location_name: Option<String>,
    pub(crate) location_code: Option<i64>,
    pub(crate) language_name: Option<String>,
    pub(crate) language_code: Option<String>,
    pub(crate) device: Option<Device>,
    pub(crate) depth: Option<u64>,
    pub(crate) timeout_seconds: Option<u64>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Device {
    #[default]
    Desktop,
    Mobile,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct KeywordOverviewInput {
    pub(crate) keywords: Vec<String>,
    pub(crate) location_name: Option<String>,
    pub(crate) location_code: Option<i64>,
    pub(crate) language_name: Option<String>,
    pub(crate) language_code: Option<String>,
    pub(crate) timeout_seconds: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct KeywordSuggestionsInput {
    pub(crate) keyword: String,
    pub(crate) location_name: Option<String>,
    pub(crate) location_code: Option<i64>,
    pub(crate) language_name: Option<String>,
    pub(crate) language_code: Option<String>,
    pub(crate) limit: Option<u64>,
    pub(crate) exact_match: Option<bool>,
    pub(crate) ignore_synonyms: Option<bool>,
    pub(crate) min_search_volume: Option<u64>,
    pub(crate) max_keyword_difficulty: Option<u64>,
    pub(crate) timeout_seconds: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RankedKeywordsInput {
    pub(crate) target: String,
    pub(crate) location_name: Option<String>,
    pub(crate) location_code: Option<i64>,
    pub(crate) language_name: Option<String>,
    pub(crate) language_code: Option<String>,
    pub(crate) limit: Option<u64>,
    pub(crate) offset: Option<u64>,
    pub(crate) historical_serp_mode: Option<HistoricalSerpMode>,
    pub(crate) max_rank: Option<u64>,
    pub(crate) min_search_volume: Option<u64>,
    pub(crate) timeout_seconds: Option<u64>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum HistoricalSerpMode {
    #[default]
    Live,
    Lost,
    All,
}
