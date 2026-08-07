//! Pull-request response models.

use serde::Deserialize;

use crate::github::models::{ProviderUser, RequiredNullable, required_nullable};

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct PullRequestDetail {
    pub(crate) number: u64,
    pub(crate) title: String,
    #[serde(deserialize_with = "required_nullable")]
    pub(crate) body: RequiredNullable<String>,
    pub(crate) state: String,
    #[serde(default)]
    pub(crate) draft: Option<bool>,
    pub(crate) merged: bool,
    #[serde(deserialize_with = "required_nullable")]
    pub(crate) mergeable: RequiredNullable<bool>,
    pub(crate) user: ProviderUser,
    pub(crate) base: PullRequestRef,
    pub(crate) head: PullRequestRef,
    pub(crate) additions: u64,
    pub(crate) deletions: u64,
    pub(crate) changed_files: u64,
    pub(crate) commits: u64,
    pub(crate) comments: u64,
    pub(crate) review_comments: u64,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    #[serde(deserialize_with = "required_nullable")]
    pub(crate) closed_at: RequiredNullable<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub(crate) merged_at: RequiredNullable<String>,
    pub(crate) html_url: String,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct PullRequestRef {
    #[serde(rename = "ref")]
    pub(crate) git_ref: String,
    pub(crate) sha: String,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct PullRequestFile {
    pub(crate) filename: String,
    pub(crate) status: String,
    pub(crate) additions: u64,
    pub(crate) deletions: u64,
    pub(crate) changes: u64,
    #[serde(deserialize_with = "required_nullable")]
    pub(crate) sha: RequiredNullable<String>,
    pub(crate) blob_url: String,
    #[serde(default)]
    pub(crate) previous_filename: Option<String>,
    #[serde(default)]
    pub(crate) patch: Option<String>,
}
