//! Bounded projections for source, pull-request, review, issue, and comment events.

use crate::github::webhook_host::WebhookError;
use crate::github::webhook_projection_common::{
    bounded, canonical_url, comment_projection, commit_projection, issue_projection,
    pull_request_projection, required, review_projection,
};
use crate::github::webhook_projection_types::EventProjection;
use crate::github::webhook_types::GitHubWebhookPayload;

const MAX_COMMITS: usize = 20;

pub(super) fn event(
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
