//! Bounded, redacted projections for model-visible GitHub events.

use std::collections::BTreeMap;

use crate::github::webhook_host::WebhookError;
use crate::github::webhook_projection_types::{
    ActorProjection, CommentProjection, CommitProjection, EventPayload, EventProjection,
    IssueProjection, NormalizedEvent, PullRequestProjection, RepositoryProjection,
    ReviewProjection,
};
use crate::github::webhook_types::{
    Actor, Comment, Commit, GitHubWebhookPayload, Issue, PullRequest, Repository, Review,
    VerifiedProviderEvent,
};

const MAX_COMMITS: usize = 20;
const MAX_TITLE_CHARS: usize = 256;
const MAX_BODY_CHARS: usize = 2_000;
const MAX_COMMIT_MESSAGE_CHARS: usize = 512;

pub(super) fn normalize(
    verified: VerifiedProviderEvent,
    body: GitHubWebhookPayload,
) -> Result<NormalizedEvent, WebhookError> {
    let installation = body
        .installation
        .as_ref()
        .ok_or(WebhookError::MissingInstallation)?;
    let repository = body
        .repository
        .as_ref()
        .ok_or(WebhookError::MissingRepository)?;
    let actor = body.sender.as_ref().ok_or(WebhookError::MissingAccount)?;
    let event = event_projection(&verified.verification.provider_event_type, &body)?;
    let mut source = BTreeMap::from([
        (String::from("installation_id"), installation.id.to_string()),
        (String::from("repository_id"), repository.id.to_string()),
        (String::from("actor_id"), actor.id.to_string()),
    ]);
    if let Some(full_name) = repository.full_name.as_ref() {
        source.insert(
            String::from("repository"),
            bounded(full_name, MAX_TITLE_CHARS),
        );
    }
    if let Some(login) = actor.login.as_ref() {
        source.insert(String::from("actor"), bounded(login, MAX_TITLE_CHARS));
    }
    Ok(NormalizedEvent {
        app_id: verified.envelope.app_id,
        installation_id: verified.installation_id,
        provider: "github",
        provider_event_id: verified.verification.provider_event_id,
        provider_event_type: verified.verification.provider_event_type,
        provider_account_id: verified.verification.provider_account_id,
        source,
        payload: EventPayload {
            installation_id: installation.id,
            repository: repository_projection(repository),
            actor: actor_projection(actor),
            action: body.action.as_deref().map(|value| bounded(value, 64)),
            event,
        },
    })
}

fn event_projection(
    event_type: &str,
    body: &GitHubWebhookPayload,
) -> Result<EventProjection, WebhookError> {
    match event_type {
        "push" => Ok(EventProjection::Push {
            git_ref: bounded(required(body.git_ref.as_ref())?, 512),
            before: bounded(required(body.before.as_ref())?, 64),
            after: bounded(required(body.after.as_ref())?, 64),
            compare_url: body.compare.as_deref().and_then(canonical_url),
            created: body.created.unwrap_or(false),
            deleted: body.deleted.unwrap_or(false),
            forced: body.forced.unwrap_or(false),
            commits: body
                .commits
                .iter()
                .take(MAX_COMMITS)
                .map(commit_projection)
                .collect(),
            head_commit: body.head_commit.as_ref().map(commit_projection),
        }),
        "pull_request" => Ok(EventProjection::PullRequest {
            pull_request: pull_request_projection(required(body.pull_request.as_ref())?),
        }),
        "pull_request_review" => Ok(EventProjection::PullRequestReview {
            pull_request: pull_request_projection(required(body.pull_request.as_ref())?),
            review: review_projection(required(body.review.as_ref())?),
        }),
        "pull_request_review_comment" => Ok(EventProjection::PullRequestReviewComment {
            pull_request: pull_request_projection(required(body.pull_request.as_ref())?),
            comment: comment_projection(required(body.comment.as_ref())?),
        }),
        "issues" => Ok(EventProjection::Issues {
            issue: issue_projection(required(body.issue.as_ref())?),
        }),
        "issue_comment" => Ok(EventProjection::IssueComment {
            issue: issue_projection(required(body.issue.as_ref())?),
            comment: comment_projection(required(body.comment.as_ref())?),
        }),
        _ => Err(WebhookError::UnsupportedEvent),
    }
}

fn repository_projection(repository: &Repository) -> RepositoryProjection {
    RepositoryProjection {
        id: repository.id,
        name: repository
            .name
            .as_deref()
            .map(|value| bounded(value, MAX_TITLE_CHARS)),
        full_name: repository
            .full_name
            .as_deref()
            .map(|value| bounded(value, MAX_TITLE_CHARS)),
        url: repository.html_url.as_deref().and_then(canonical_url),
        private: repository.private,
    }
}

fn actor_projection(actor: &Actor) -> ActorProjection {
    ActorProjection {
        id: actor.id,
        login: actor
            .login
            .as_deref()
            .map(|value| bounded(value, MAX_TITLE_CHARS)),
        url: actor.html_url.as_deref().and_then(canonical_url),
    }
}

fn commit_projection(commit: &Commit) -> CommitProjection {
    CommitProjection {
        sha: bounded(&commit.id, 64),
        message: commit
            .message
            .as_deref()
            .map(|value| bounded(value, MAX_COMMIT_MESSAGE_CHARS)),
        url: commit.url.as_deref().and_then(canonical_url),
        author_name: commit
            .author
            .as_ref()
            .and_then(|author| author.name.as_deref())
            .map(|value| bounded(value, MAX_TITLE_CHARS)),
        author_login: commit
            .author
            .as_ref()
            .and_then(|author| author.username.as_deref())
            .map(|value| bounded(value, MAX_TITLE_CHARS)),
    }
}

fn pull_request_projection(value: &PullRequest) -> PullRequestProjection {
    PullRequestProjection {
        id: value.id,
        number: value.number,
        title: bounded(&value.title, MAX_TITLE_CHARS),
        body: value
            .body
            .as_deref()
            .map(|body| bounded(body, MAX_BODY_CHARS)),
        state: bounded(&value.state, 32),
        draft: value.draft,
        merged: value.merged,
        url: canonical_url(&value.html_url),
        author: actor_projection(&value.user),
        head_ref: bounded(&value.head.name, 256),
        head_sha: bounded(&value.head.sha, 64),
        base_ref: bounded(&value.base.name, 256),
        base_sha: bounded(&value.base.sha, 64),
    }
}

fn review_projection(value: &Review) -> ReviewProjection {
    ReviewProjection {
        id: value.id,
        state: bounded(&value.state, 32),
        body: value
            .body
            .as_deref()
            .map(|body| bounded(body, MAX_BODY_CHARS)),
        url: canonical_url(&value.html_url),
        submitted_at: value.submitted_at.clone(),
        commit_sha: value.commit_id.as_deref().map(|sha| bounded(sha, 64)),
        author: actor_projection(&value.user),
    }
}

fn comment_projection(value: &Comment) -> CommentProjection {
    CommentProjection {
        id: value.id,
        body: value
            .body
            .as_deref()
            .map(|body| bounded(body, MAX_BODY_CHARS)),
        url: canonical_url(&value.html_url),
        created_at: value.created_at.clone(),
        updated_at: value.updated_at.clone(),
        author: actor_projection(&value.user),
    }
}

fn issue_projection(value: &Issue) -> IssueProjection {
    IssueProjection {
        id: value.id,
        number: value.number,
        title: bounded(&value.title, MAX_TITLE_CHARS),
        body: value
            .body
            .as_deref()
            .map(|body| bounded(body, MAX_BODY_CHARS)),
        state: bounded(&value.state, 32),
        url: canonical_url(&value.html_url),
        locked: value.locked,
        author: actor_projection(&value.user),
    }
}

fn required<T>(value: Option<&T>) -> Result<&T, WebhookError> {
    value.ok_or(WebhookError::EventTypeDisagreement)
}

fn bounded(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn canonical_url(value: &str) -> Option<String> {
    value
        .starts_with("https://github.com/")
        .then(|| bounded(value, 2_048))
}
