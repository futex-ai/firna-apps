//! Validation for published CI, status, merge-queue, and policy events.

use crate::github::webhook_event_validation::{action, nonempty, require, sha};
use crate::github::webhook_host::WebhookError;
use crate::github::webhook_signal_types::CheckRun;
use crate::github::webhook_types::GitHubWebhookPayload;

pub(super) fn published(
    event_type: &str,
    payload: &GitHubWebhookPayload,
) -> Result<(), WebhookError> {
    match event_type {
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

fn valid_check_run(run: &CheckRun) -> bool {
    run.id > 0 && nonempty(&run.name) && nonempty(&run.status) && sha(&run.head_sha)
}
