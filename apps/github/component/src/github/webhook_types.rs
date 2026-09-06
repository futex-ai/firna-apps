//! Typed GitHub webhook and app-runtime ABI data transfer objects.

use serde::{Deserialize, Serialize};

use crate::github::webhook_content_types::{Comment, Commit, Issue, PullRequest, Review};
use crate::github::webhook_signal_types::{
    CheckRun, CheckSuite, MergeGroup, StatusBranch, WorkflowJob, WorkflowRun,
};

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) provider_account_label: Option<String>,
    pub(crate) provider_event_id: String,
    pub(crate) provider_event_type: String,
    pub(crate) provider_user_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) provider_repository_id: Option<String>,
    pub(crate) installation_lifecycle: Option<ProviderInstallationLifecycle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) user_authorization_lifecycle: Option<ProviderUserAuthorizationLifecycle>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProviderInstallationLifecycle {
    Reconcile,
    Revoke,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProviderUserAuthorizationLifecycle {
    Revoke,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct AcknowledgedWebhookPayload {
    pub(crate) installation: Option<Installation>,
    pub(crate) repository: Option<Repository>,
    pub(crate) sender: Option<Actor>,
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
    pub(crate) check_run: Option<CheckRun>,
    pub(crate) check_suite: Option<CheckSuite>,
    pub(crate) workflow_job: Option<WorkflowJob>,
    pub(crate) workflow_run: Option<WorkflowRun>,
    pub(crate) merge_group: Option<MergeGroup>,
    pub(crate) sha: Option<String>,
    pub(crate) state: Option<String>,
    pub(crate) context: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) target_url: Option<String>,
    #[serde(default)]
    pub(crate) branches: Vec<StatusBranch>,
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
