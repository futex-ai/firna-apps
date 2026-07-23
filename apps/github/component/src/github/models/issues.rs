//! Issue and issue-comment response models.

use crate::github::models::{ProviderUser, RequiredNullable, required_nullable};
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct IssueDetail {
    pub(crate) number: u64,
    pub(crate) title: String,
    #[serde(default)]
    pub(crate) body: Option<String>,
    pub(crate) state: String,
    #[serde(default)]
    pub(crate) state_reason: Option<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub(crate) user: RequiredNullable<ProviderUser>,
    pub(crate) labels: Vec<IssueLabel>,
    #[serde(default)]
    pub(crate) assignees: Vec<ProviderUser>,
    #[serde(deserialize_with = "required_nullable")]
    pub(crate) milestone: RequiredNullable<IssueMilestone>,
    pub(crate) comments: u64,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    #[serde(deserialize_with = "required_nullable")]
    pub(crate) closed_at: RequiredNullable<String>,
    pub(crate) html_url: String,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct IssueLabel {
    pub(crate) name: String,
    pub(crate) color: String,
    pub(crate) description: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct IssueMilestone {
    pub(crate) number: u64,
    pub(crate) title: String,
    pub(crate) state: String,
    pub(crate) html_url: String,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct IssueComment {
    pub(crate) id: u64,
    #[serde(deserialize_with = "required_nullable")]
    pub(crate) user: RequiredNullable<ProviderUser>,
    pub(crate) body: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    pub(crate) html_url: String,
}
