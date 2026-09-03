//! Typed shape validation for published GitHub events and lifecycle controls.

use crate::github::webhook_host::WebhookError;
use crate::github::webhook_types::{GitHubWebhookPayload, PullRequest};

pub(super) fn published(
    event_type: &str,
    payload: &GitHubWebhookPayload,
) -> Result<(), WebhookError> {
    require(content_identity(payload))?;
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
        "check_run" => {
            require(action(payload) && payload.check_run.as_ref().is_some_and(valid_check_run))
        }
        "check_suite" => require(
            action(payload)
                && payload.check_suite.as_ref().is_some_and(|suite| {
                    suite.id > 0 && nonempty(&suite.status) && sha(&suite.head_sha)
                }),
        ),
        "status" => require(
            payload.sha.as_deref().is_some_and(sha)
                && payload.state.as_deref().is_some_and(nonempty),
        ),
        "workflow_job" => require(
            action(payload)
                && payload.workflow_job.as_ref().is_some_and(|job| {
                    job.id > 0 && nonempty(&job.name) && nonempty(&job.status) && sha(&job.head_sha)
                }),
        ),
        "workflow_run" => require(
            action(payload)
                && payload.workflow_run.as_ref().is_some_and(|run| {
                    run.id > 0 && nonempty(&run.name) && nonempty(&run.status) && sha(&run.head_sha)
                }),
        ),
        "merge_group" => require(
            action(payload)
                && payload.merge_group.as_ref().is_some_and(|group| {
                    nonempty(&group.head_ref)
                        && sha(&group.head_sha)
                        && nonempty(&group.base_ref)
                        && sha(&group.base_sha)
                }),
        ),
        "branch_protection_configuration"
        | "branch_protection_rule"
        | "repository_ruleset"
        | "security_and_analysis" => require(action(payload)),
        _ => Err(WebhookError::UnsupportedEvent),
    }
}

pub(super) fn control(
    event_type: &str,
    payload: &GitHubWebhookPayload,
) -> Result<(), WebhookError> {
    match event_type {
        "ping" => require(
            payload.zen.as_deref().is_some_and(nonempty)
                && payload.hook.as_ref().is_some_and(|hook| hook.id > 0),
        ),
        "installation" => require(
            valid_installation(payload)
                && matches!(
                    action_value(payload),
                    Some(
                        "created"
                            | "deleted"
                            | "suspend"
                            | "unsuspend"
                            | "new_permissions_accepted"
                    )
                ),
        ),
        "installation_repositories" => require(
            valid_installation(payload)
                && matches!(action_value(payload), Some("added" | "removed"))
                && (!payload.repositories_added.is_empty()
                    || !payload.repositories_removed.is_empty()),
        ),
        "installation_target" => require(
            valid_installation(payload)
                && matches!(action_value(payload), Some("renamed" | "transferred")),
        ),
        "github_app_authorization" => require(
            matches!(action_value(payload), Some("revoked"))
                && payload.sender.as_ref().is_some_and(|sender| sender.id > 0),
        ),
        _ => Err(WebhookError::UnsupportedEvent),
    }
}

fn valid_check_run(run: &crate::github::webhook_signal_types::CheckRun) -> bool {
    run.id > 0 && nonempty(&run.name) && nonempty(&run.status) && sha(&run.head_sha)
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

fn content_identity(payload: &GitHubWebhookPayload) -> bool {
    valid_installation(payload)
        && payload
            .repository
            .as_ref()
            .is_some_and(|repository| repository.id > 0)
        && payload.sender.as_ref().is_some_and(|sender| sender.id > 0)
}

fn valid_installation(payload: &GitHubWebhookPayload) -> bool {
    payload
        .installation
        .as_ref()
        .is_some_and(|installation| installation.id > 0 && installation.account.id > 0)
}

fn action(payload: &GitHubWebhookPayload) -> bool {
    action_value(payload).is_some()
}

pub(super) fn action_value(payload: &GitHubWebhookPayload) -> Option<&str> {
    payload.action.as_deref().filter(|action| {
        !action.is_empty()
            && action.len() <= 64
            && action
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte == b'_')
    })
}

fn nonempty(value: &str) -> bool {
    !value.is_empty() && value.len() <= 2_048 && !value.chars().any(char::is_control)
}

fn sha(value: &str) -> bool {
    !value.is_empty() && value.len() <= 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn require(condition: bool) -> Result<(), WebhookError> {
    if condition {
        Ok(())
    } else {
        Err(WebhookError::EventTypeDisagreement)
    }
}
