//! Repository, code-search, and content response models.

use serde::Deserialize;

use crate::github::models::{RequiredNullable, required_nullable};

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct Repository {
    pub(crate) id: u64,
    pub(crate) full_name: String,
    #[serde(deserialize_with = "required_nullable")]
    pub(crate) description: RequiredNullable<String>,
    pub(crate) visibility: String,
    pub(crate) archived: bool,
    pub(crate) fork: bool,
    pub(crate) default_branch: String,
    #[serde(deserialize_with = "required_nullable")]
    pub(crate) language: RequiredNullable<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub(crate) pushed_at: RequiredNullable<String>,
    pub(crate) html_url: String,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct InstallationRepositoriesResponse {
    pub(crate) total_count: u64,
    pub(crate) repositories: Vec<Repository>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct CodeSearchResponse {
    pub(crate) total_count: u64,
    pub(crate) incomplete_results: bool,
    pub(crate) items: Vec<CodeSearchItem>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct CodeSearchItem {
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) sha: String,
    pub(crate) html_url: String,
    pub(crate) repository: RepositoryRef,
    #[serde(default)]
    pub(crate) text_matches: Vec<TextMatch>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct RepositoryRef {
    pub(crate) full_name: String,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct TextMatch {
    pub(crate) fragment: String,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct FileContent {
    pub(crate) path: String,
    pub(crate) sha: String,
    pub(crate) size: u64,
    #[serde(deserialize_with = "required_nullable")]
    pub(crate) html_url: RequiredNullable<String>,
    pub(crate) encoding: String,
    pub(crate) content: String,
}
