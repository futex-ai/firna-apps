//! AI visibility input DTOs.

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AiKeywordVolumeInput {
    pub(crate) keywords: Vec<String>,
    pub(crate) location_name: Option<String>,
    pub(crate) location_code: Option<i64>,
    pub(crate) language_name: Option<String>,
    pub(crate) language_code: Option<String>,
    pub(crate) timeout_seconds: Option<u64>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LlmPlatform {
    Google,
    ChatGpt,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LlmMentionsInput {
    pub(crate) platform: LlmPlatform,
    pub(crate) location_name: Option<String>,
    pub(crate) location_code: Option<i64>,
    pub(crate) language_name: Option<String>,
    pub(crate) language_code: Option<String>,
    pub(crate) targets: Vec<TargetEntity>,
    pub(crate) timeout_seconds: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub(crate) enum TargetEntity {
    Domain(DomainTarget),
    Keyword(KeywordTarget),
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DomainTarget {
    pub(crate) domain: String,
    pub(crate) search_filter: Option<SearchFilter>,
    pub(crate) search_scope: Option<Vec<DomainScope>>,
    pub(crate) include_subdomains: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct KeywordTarget {
    pub(crate) keyword: String,
    pub(crate) search_filter: Option<SearchFilter>,
    pub(crate) search_scope: Option<Vec<KeywordScope>>,
    pub(crate) match_type: Option<MatchType>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SearchFilter {
    #[default]
    Include,
    Exclude,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DomainScope {
    Any,
    Sources,
    SearchResults,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum KeywordScope {
    Any,
    Question,
    Answer,
    BrandEntities,
    FanOutQueries,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MatchType {
    #[default]
    WordMatch,
    PartialMatch,
}
