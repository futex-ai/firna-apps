//! Typed GitHub webhook and app-runtime ABI data transfer objects.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct WebhookEnvelope {
    pub(crate) app_id: String,
    pub(crate) ingress_id: String,
    pub(crate) headers: Vec<WebhookHeader>,
    pub(crate) body: Vec<u8>,
    pub(crate) received_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct WebhookHeader {
    pub(crate) name: String,
    pub(crate) value: Vec<u8>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct VerifiedProviderEvent {
    pub(crate) installation_id: String,
    pub(crate) envelope: WebhookEnvelope,
    pub(crate) verification: WebhookVerification,
}

#[derive(Debug, Deserialize)]
pub(crate) struct WebhookResponseRequest {
    pub(crate) verification: WebhookVerification,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct WebhookVerification {
    pub(crate) provider_account_id: String,
    pub(crate) provider_installation_id: Option<String>,
    pub(crate) provider_event_id: String,
    pub(crate) provider_event_type: String,
    pub(crate) provider_user_id: Option<String>,
    pub(crate) installation_lifecycle: Option<ProviderInstallationLifecycle>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProviderInstallationLifecycle {
    Reconcile,
    Revoke,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct GitHubWebhookPayload {
    pub(crate) action: Option<String>,
    pub(crate) zen: Option<String>,
    pub(crate) hook: Option<Hook>,
    pub(crate) installation: Option<Installation>,
    pub(crate) repository: Option<Repository>,
    pub(crate) sender: Option<Actor>,
    #[serde(rename = "ref")]
    pub(crate) git_ref: Option<String>,
    pub(crate) before: Option<String>,
    pub(crate) after: Option<String>,
    #[serde(default)]
    pub(crate) commits: Vec<Commit>,
    pub(crate) head_commit: Option<Commit>,
    pub(crate) compare: Option<String>,
    pub(crate) created: Option<bool>,
    pub(crate) deleted: Option<bool>,
    pub(crate) forced: Option<bool>,
    pub(crate) pull_request: Option<PullRequest>,
    pub(crate) review: Option<Review>,
    pub(crate) comment: Option<Comment>,
    pub(crate) issue: Option<Issue>,
    #[serde(default)]
    pub(crate) repositories_added: Vec<Repository>,
    #[serde(default)]
    pub(crate) repositories_removed: Vec<Repository>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct Hook {
    pub(crate) id: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct Installation {
    pub(crate) id: u64,
    pub(crate) account: Actor,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Actor {
    pub(crate) id: u64,
    pub(crate) login: Option<String>,
    pub(crate) html_url: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Repository {
    pub(crate) id: u64,
    pub(crate) name: Option<String>,
    pub(crate) full_name: Option<String>,
    pub(crate) html_url: Option<String>,
    pub(crate) private: Option<bool>,
}

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
