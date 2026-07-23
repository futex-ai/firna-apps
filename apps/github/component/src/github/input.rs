//! Tool input DTOs and centralized validation.

use serde::Deserialize;
use serde_json::Value;

pub(crate) use crate::github::input_validation::{
    encoded_path, encoded_segment, git_ref, language, owner, page, page_size, path,
    positive_number, quoted_search_literal, repository, result_offset, search_path, search_term,
};

#[derive(Debug, Deserialize)]
pub(crate) struct AppToolCall {
    pub(crate) installation_id: String,
    pub(crate) tool_name: String,
    pub(crate) input: Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ListRepositoriesInput {
    #[serde(default)]
    pub(crate) page: Option<i64>,
    #[serde(default)]
    pub(crate) per_page: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SearchCodeInput {
    pub(crate) query: String,
    #[serde(default)]
    pub(crate) owner: Option<String>,
    #[serde(default)]
    pub(crate) repository: Option<String>,
    #[serde(default)]
    pub(crate) language: Option<String>,
    #[serde(default)]
    pub(crate) path: Option<String>,
    #[serde(default)]
    pub(crate) page: Option<i64>,
    #[serde(default)]
    pub(crate) per_page: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReadFileInput {
    pub(crate) owner: String,
    pub(crate) repository: String,
    pub(crate) path: String,
    #[serde(rename = "ref", default)]
    pub(crate) git_ref: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReadPullRequestInput {
    pub(crate) owner: String,
    pub(crate) repository: String,
    pub(crate) number: i64,
    #[serde(default)]
    pub(crate) include_files: Option<bool>,
    #[serde(default)]
    pub(crate) files_page: Option<i64>,
    #[serde(default)]
    pub(crate) files_per_page: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReadIssueInput {
    pub(crate) owner: String,
    pub(crate) repository: String,
    pub(crate) number: i64,
    #[serde(default)]
    pub(crate) include_comments: Option<bool>,
    #[serde(default)]
    pub(crate) comments_page: Option<i64>,
    #[serde(default)]
    pub(crate) comments_per_page: Option<i64>,
}
