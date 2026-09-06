//! Typed source, pull-request, review, issue, and comment webhook objects.

use serde::Deserialize;

use crate::github::webhook_types::Actor;

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct Commit {
    pub(crate) id: String,
    pub(crate) message: Option<String>,
    pub(crate) url: Option<String>,
    pub(crate) author: Option<CommitAuthor>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct CommitAuthor {
    pub(crate) name: Option<String>,
    pub(crate) username: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct PullRequest {
    pub(crate) id: u64,
    pub(crate) number: u64,
    pub(crate) title: String,
    pub(crate) body: Option<String>,
    pub(crate) state: String,
    pub(crate) draft: Option<bool>,
    pub(crate) merged: Option<bool>,
    pub(crate) html_url: String,
    pub(crate) user: Actor,
    pub(crate) head: GitReference,
    pub(crate) base: GitReference,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct GitReference {
    #[serde(rename = "ref")]
    pub(crate) name: String,
    pub(crate) sha: String,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct Review {
    pub(crate) id: u64,
    pub(crate) state: String,
    pub(crate) body: Option<String>,
    pub(crate) html_url: String,
    pub(crate) submitted_at: Option<String>,
    pub(crate) commit_id: Option<String>,
    pub(crate) user: Actor,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct Comment {
    pub(crate) id: u64,
    pub(crate) body: Option<String>,
    pub(crate) html_url: String,
    pub(crate) created_at: Option<String>,
    pub(crate) updated_at: Option<String>,
    pub(crate) user: Actor,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct Issue {
    pub(crate) id: u64,
    pub(crate) number: u64,
    pub(crate) title: String,
    pub(crate) body: Option<String>,
    pub(crate) state: String,
    pub(crate) html_url: String,
    pub(crate) locked: Option<bool>,
    pub(crate) user: Actor,
}
