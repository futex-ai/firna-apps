//! Content-analysis and domain input DTOs.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub(crate) enum PageType {
    #[serde(rename = "ecommerce")]
    Ecommerce,
    #[serde(rename = "news")]
    News,
    #[serde(rename = "blogs")]
    Blogs,
    #[serde(rename = "message-boards")]
    MessageBoards,
    #[serde(rename = "organization")]
    Organization,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ContentSearchMode {
    #[default]
    AsIs,
    OnePerDomain,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Sentiment {
    Positive,
    Negative,
    Neutral,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ContentSearchInput {
    pub(crate) keyword: String,
    pub(crate) page_types: Option<Vec<PageType>>,
    pub(crate) country_code: Option<String>,
    pub(crate) language_code: Option<String>,
    pub(crate) min_domain_rank: Option<u64>,
    pub(crate) timeout_seconds: Option<u64>,
    pub(crate) search_mode: Option<ContentSearchMode>,
    pub(crate) sentiment: Option<Sentiment>,
    pub(crate) limit: Option<u64>,
    pub(crate) offset: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ContentSentimentInput {
    pub(crate) keyword: String,
    pub(crate) page_types: Option<Vec<PageType>>,
    pub(crate) country_code: Option<String>,
    pub(crate) language_code: Option<String>,
    pub(crate) min_domain_rank: Option<u64>,
    pub(crate) timeout_seconds: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DomainTechnologiesInput {
    pub(crate) hostname: String,
    pub(crate) timeout_seconds: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DomainWhoisInput {
    pub(crate) hostname: String,
    pub(crate) timeout_seconds: Option<u64>,
}
