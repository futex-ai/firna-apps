//! Shared validation and family routing for published GitHub events.

use crate::github::webhook_content_validation;
use crate::github::webhook_host::WebhookError;
use crate::github::webhook_signal_validation;
use crate::github::webhook_types::GitHubWebhookPayload;

pub(super) fn published(
    event_type: &str,
    payload: &GitHubWebhookPayload,
) -> Result<(), WebhookError> {
    require(content_identity(payload))?;
    match event_type {
        "push"
        | "pull_request"
        | "pull_request_review"
        | "pull_request_review_comment"
        | "issues"
        | "issue_comment" => webhook_content_validation::published(event_type, payload),
        "check_run"
        | "check_suite"
        | "status"
        | "workflow_job"
        | "workflow_run"
        | "merge_group"
        | "branch_protection_configuration"
        | "branch_protection_rule"
        | "repository_ruleset"
        | "security_and_analysis" => webhook_signal_validation::published(event_type, payload),
        _ => Err(WebhookError::UnsupportedEvent),
    }
}

pub(super) fn content_identity(payload: &GitHubWebhookPayload) -> bool {
    valid_installation(payload)
        && payload
            .repository
            .as_ref()
            .is_some_and(|repository| repository.id > 0)
        && payload.sender.as_ref().is_some_and(|sender| sender.id > 0)
}

pub(super) fn valid_installation(payload: &GitHubWebhookPayload) -> bool {
    payload
        .installation
        .as_ref()
        .is_some_and(|installation| installation.id > 0 && installation.account.id > 0)
}

pub(super) fn action(payload: &GitHubWebhookPayload) -> bool {
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

pub(super) fn nonempty(value: &str) -> bool {
    !value.is_empty() && value.len() <= 2_048 && !value.chars().any(char::is_control)
}

pub(super) fn sha(value: &str) -> bool {
    !value.is_empty() && value.len() <= 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(super) fn require(condition: bool) -> Result<(), WebhookError> {
    if condition {
        Ok(())
    } else {
        Err(WebhookError::EventTypeDisagreement)
    }
}
