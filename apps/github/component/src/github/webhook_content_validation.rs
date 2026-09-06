//! Validation for published source, pull-request, review, issue, and comment events.

use crate::github::webhook_content_types::PullRequest;
use crate::github::webhook_event_validation::{action, nonempty, require, sha};
use crate::github::webhook_host::WebhookError;
use crate::github::webhook_types::GitHubWebhookPayload;

pub(super) fn published(
    event_type: &str,
    payload: &GitHubWebhookPayload,
) -> Result<(), WebhookError> {
    match event_type {
        "push" => require(
            payload.git_ref.as_deref().is_some_and(nonempty)
                && payload.before.as_deref().is_some_and(sha)
                && payload.after.as_deref().is_some_and(sha),
        ),
        "pull_request" => require(action(payload) && valid_pr(payload.pull_request.as_ref())),
        "pull_request_review" => require(
            action(payload)
                && valid_pr(payload.pull_request.as_ref())
                && payload.review.as_ref().is_some_and(|review| review.id > 0)
                && payload.comment.is_none(),
        ),
        "pull_request_review_comment" => require(
            action(payload)
                && valid_pr(payload.pull_request.as_ref())
                && payload
                    .comment
                    .as_ref()
                    .is_some_and(|comment| comment.id > 0),
        ),
        "issues" => require(
            action(payload)
                && payload
                    .issue
                    .as_ref()
                    .is_some_and(|issue| issue.id > 0 && issue.number > 0)
                && payload.comment.is_none(),
        ),
        "issue_comment" => require(
            action(payload)
                && payload
                    .issue
                    .as_ref()
                    .is_some_and(|issue| issue.id > 0 && issue.number > 0)
                && payload
                    .comment
                    .as_ref()
                    .is_some_and(|comment| comment.id > 0),
        ),
        _ => Err(WebhookError::UnsupportedEvent),
    }
}

fn valid_pr(value: Option<&PullRequest>) -> bool {
    value.is_some_and(|pull_request| {
        pull_request.id > 0
            && pull_request.number > 0
            && nonempty(&pull_request.title)
            && nonempty(&pull_request.state)
            && nonempty(&pull_request.head.name)
            && sha(&pull_request.head.sha)
            && nonempty(&pull_request.base.name)
            && sha(&pull_request.base.sha)
    })
}
