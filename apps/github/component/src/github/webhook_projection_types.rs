//! Serializable bounded GitHub event projection types.

use std::collections::BTreeMap;

use serde::Serialize;

#[derive(Serialize)]
pub(super) struct NormalizedEvent {
    pub(super) app_id: String,
    pub(super) installation_id: String,
    pub(super) provider: &'static str,
    pub(super) provider_event_id: String,
    pub(super) provider_event_type: String,
    pub(super) provider_account_id: String,
    pub(super) source: BTreeMap<String, String>,
    pub(super) payload: EventPayload,
}

#[derive(Serialize)]
pub(super) struct EventPayload {
    pub(super) installation_id: u64,
    pub(super) repository: RepositoryProjection,
    pub(super) actor: ActorProjection,
    pub(super) action: Option<String>,
    pub(super) event: EventProjection,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(super) enum EventProjection {
    Push {
        git_ref: String,
        before: String,
        after: String,
        compare_url: Option<String>,
        created: bool,
        deleted: bool,
        forced: bool,
        commits: Vec<CommitProjection>,
        head_commit: Option<CommitProjection>,
    },
    PullRequest {
        pull_request: PullRequestProjection,
    },
    PullRequestReview {
        pull_request: PullRequestProjection,
        review: ReviewProjection,
    },
    PullRequestReviewComment {
        pull_request: PullRequestProjection,
        comment: CommentProjection,
    },
    Issues {
        issue: IssueProjection,
    },
    IssueComment {
        issue: IssueProjection,
        comment: CommentProjection,
    },
}

#[derive(Serialize)]
pub(super) struct RepositoryProjection {
    pub(super) id: u64,
    pub(super) name: Option<String>,
    pub(super) full_name: Option<String>,
    pub(super) url: Option<String>,
    pub(super) private: Option<bool>,
}

#[derive(Serialize)]
pub(super) struct ActorProjection {
    pub(super) id: u64,
    pub(super) login: Option<String>,
    pub(super) url: Option<String>,
}

#[derive(Serialize)]
pub(super) struct CommitProjection {
    pub(super) sha: String,
    pub(super) message: Option<String>,
    pub(super) url: Option<String>,
    pub(super) author_name: Option<String>,
    pub(super) author_login: Option<String>,
}

#[derive(Serialize)]
pub(super) struct PullRequestProjection {
    pub(super) id: u64,
    pub(super) number: u64,
    pub(super) title: String,
    pub(super) body: Option<String>,
    pub(super) state: String,
    pub(super) draft: Option<bool>,
    pub(super) merged: Option<bool>,
    pub(super) url: Option<String>,
    pub(super) author: ActorProjection,
    pub(super) head_ref: String,
    pub(super) head_sha: String,
    pub(super) base_ref: String,
    pub(super) base_sha: String,
}

#[derive(Serialize)]
pub(super) struct ReviewProjection {
    pub(super) id: u64,
    pub(super) state: String,
    pub(super) body: Option<String>,
    pub(super) url: Option<String>,
    pub(super) submitted_at: Option<String>,
    pub(super) commit_sha: Option<String>,
    pub(super) author: ActorProjection,
}

#[derive(Serialize)]
pub(super) struct CommentProjection {
    pub(super) id: u64,
    pub(super) body: Option<String>,
    pub(super) url: Option<String>,
    pub(super) created_at: Option<String>,
    pub(super) updated_at: Option<String>,
    pub(super) author: ActorProjection,
}

#[derive(Serialize)]
pub(super) struct IssueProjection {
    pub(super) id: u64,
    pub(super) number: u64,
    pub(super) title: String,
    pub(super) body: Option<String>,
    pub(super) state: String,
    pub(super) url: Option<String>,
    pub(super) locked: Option<bool>,
    pub(super) author: ActorProjection,
}
