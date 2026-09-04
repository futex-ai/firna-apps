//! Bounded, redacted family routing for model-visible GitHub events.

use std::collections::BTreeMap;

use crate::github::webhook_content_projection;
use crate::github::webhook_effects;
use crate::github::webhook_host::WebhookError;
use crate::github::webhook_projection_common::{
    MAX_TITLE_CHARS, actor_projection, bounded, repository_projection,
};
use crate::github::webhook_projection_types::{EventPayload, EventProjection, NormalizedEvent};
use crate::github::webhook_signal_projection;
use crate::github::webhook_types::{GitHubWebhookPayload, VerifiedProviderEvent};

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
    let platform_effects =
        webhook_effects::repository_changes(&verified.verification.provider_event_type, &body);
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
        platform_effects,
    })
}

fn event_projection(
    event_type: &str,
    body: &GitHubWebhookPayload,
) -> Result<EventProjection, WebhookError> {
    match event_type {
        "push"
        | "pull_request"
        | "pull_request_review"
        | "pull_request_review_comment"
        | "issues"
        | "issue_comment" => webhook_content_projection::event(event_type, body),
        "check_run"
        | "check_suite"
        | "status"
        | "workflow_job"
        | "workflow_run"
        | "merge_group"
        | "branch_protection_configuration"
        | "branch_protection_rule"
        | "repository_ruleset"
        | "security_and_analysis" => webhook_signal_projection::event(event_type, body),
        _ => Err(WebhookError::UnsupportedEvent),
    }
}
