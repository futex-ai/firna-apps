//! Shared bounded projections for published GitHub webhook objects.

use crate::github::webhook_content_types::{Comment, Commit, Issue, PullRequest, Review};
use crate::github::webhook_projection_types::{
    ActorProjection, CommentProjection, CommitProjection, IssueProjection, PullRequestProjection,
    RepositoryProjection, ReviewProjection,
};
use crate::github::webhook_types::{Actor, Repository};

pub(super) const MAX_TITLE_CHARS: usize = 256;
const MAX_BODY_CHARS: usize = 2_000;
const MAX_COMMIT_MESSAGE_CHARS: usize = 512;

pub(super) fn repository_projection(repository: &Repository) -> RepositoryProjection {
    RepositoryProjection {
        id: repository.id,
        name: repository.name.as_deref().map(|value| bounded(value, 256)),
        full_name: repository
            .full_name
            .as_deref()
            .map(|value| bounded(value, 256)),
        url: repository.html_url.as_deref().and_then(canonical_url),
        private: repository.private,
    }
}

pub(super) fn actor_projection(actor: &Actor) -> ActorProjection {
    ActorProjection {
        id: actor.id,
        login: actor.login.as_deref().map(|value| bounded(value, 256)),
        url: actor.html_url.as_deref().and_then(canonical_url),
    }
}

pub(super) fn commit_projection(commit: &Commit) -> CommitProjection {
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

pub(super) fn pull_request_projection(value: &PullRequest) -> PullRequestProjection {
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

pub(super) fn review_projection(value: &Review) -> ReviewProjection {
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

pub(super) fn comment_projection(value: &Comment) -> CommentProjection {
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

pub(super) fn issue_projection(value: &Issue) -> IssueProjection {
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

pub(super) fn bounded(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

pub(super) fn canonical_url(value: &str) -> Option<String> {
    value
        .starts_with("https://github.com/")
        .then(|| bounded(value, 2_048))
}

pub(super) fn required<T>(
    value: Option<&T>,
) -> Result<&T, crate::github::webhook_host::WebhookError> {
    value.ok_or(crate::github::webhook_host::WebhookError::EventTypeDisagreement)
}
