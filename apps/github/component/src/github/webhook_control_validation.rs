//! Closed validation for GitHub webhook handshake and lifecycle controls.

use crate::github::webhook_event_validation::{
    action_value, nonempty, require, valid_installation,
};
use crate::github::webhook_host::WebhookError;
use crate::github::webhook_types::GitHubWebhookPayload;

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
                && matches!(action_value(payload), Some("added" | "removed")),
        ),
        "installation_target" => {
            require(valid_installation(payload) && matches!(action_value(payload), Some("renamed")))
        }
        "github_app_authorization" => require(
            matches!(action_value(payload), Some("revoked"))
                && payload.sender.as_ref().is_some_and(|sender| sender.id > 0),
        ),
        _ => Err(WebhookError::UnsupportedEvent),
    }
}
