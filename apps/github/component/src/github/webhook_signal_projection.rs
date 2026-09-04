//! Bounded projections for CI, status, merge-queue, and repository policy events.

use crate::github::webhook_host::WebhookError;
use crate::github::webhook_projection_common::{bounded, canonical_url, required};
use crate::github::webhook_projection_types::{
    AutomationProjection, EventProjection, StatusProjection,
};
use crate::github::webhook_types::GitHubWebhookPayload;

pub(super) fn event(
    event_type: &str,
    body: &GitHubWebhookPayload,
) -> Result<EventProjection, WebhookError> {
    match event_type {
        "check_run" => {
            let run = required(body.check_run.as_ref())?;
            Ok(EventProjection::CheckRun {
                signal: automation(
                    run.id,
                    Some(&run.name),
                    &run.status,
                    run.conclusion.as_deref(),
                    run.check_suite
                        .as_ref()
                        .and_then(|suite| suite.name.as_deref()),
                    &run.head_sha,
                    run.html_url.as_deref(),
                ),
            })
        }
        "check_suite" => {
            let suite = required(body.check_suite.as_ref())?;
            Ok(EventProjection::CheckSuite {
                signal: automation(
                    suite.id,
                    None,
                    &suite.status,
                    suite.conclusion.as_deref(),
                    suite.head_branch.as_deref(),
                    &suite.head_sha,
                    None,
                ),
            })
        }
        "status" => Ok(EventProjection::Status {
            signal: StatusProjection {
                sha: bounded(required(body.sha.as_ref())?, 64),
                state: bounded(required(body.state.as_ref())?, 64),
                context: body.context.as_deref().map(|value| bounded(value, 256)),
                description: body.description.as_deref().map(|value| bounded(value, 512)),
                target_url: body.target_url.as_deref().and_then(canonical_url),
                branches: body
                    .branches
                    .iter()
                    .take(8)
                    .map(|branch| bounded(&branch.name, 255))
                    .collect(),
            },
        }),
        "workflow_job" => {
            let job = required(body.workflow_job.as_ref())?;
            Ok(EventProjection::WorkflowJob {
                signal: automation(
                    job.id,
                    Some(&job.name),
                    &job.status,
                    job.conclusion.as_deref(),
                    job.head_branch.as_deref(),
                    &job.head_sha,
                    job.html_url.as_deref(),
                ),
            })
        }
        "workflow_run" => {
            let run = required(body.workflow_run.as_ref())?;
            Ok(EventProjection::WorkflowRun {
                signal: automation(
                    run.id,
                    Some(&run.name),
                    &run.status,
                    run.conclusion.as_deref(),
                    run.head_branch.as_deref(),
                    &run.head_sha,
                    run.html_url.as_deref(),
                ),
            })
        }
        "merge_group" => {
            let group = required(body.merge_group.as_ref())?;
            Ok(EventProjection::MergeGroup {
                head_ref: bounded(&group.head_ref, 255),
                head_sha: bounded(&group.head_sha, 64),
                base_ref: bounded(&group.base_ref, 255),
                base_sha: bounded(&group.base_sha, 64),
            })
        }
        "branch_protection_configuration" => Ok(EventProjection::BranchProtectionConfiguration),
        "branch_protection_rule" => Ok(EventProjection::BranchProtectionRule),
        "repository_ruleset" => Ok(EventProjection::RepositoryRuleset),
        "security_and_analysis" => Ok(EventProjection::SecurityAndAnalysis),
        _ => Err(WebhookError::UnsupportedEvent),
    }
}

fn automation(
    id: u64,
    name: Option<&str>,
    status: &str,
    conclusion: Option<&str>,
    head_branch: Option<&str>,
    head_sha: &str,
    url: Option<&str>,
) -> AutomationProjection {
    AutomationProjection {
        id,
        name: name.map(|value| bounded(value, 256)),
        status: bounded(status, 64),
        conclusion: conclusion.map(|value| bounded(value, 64)),
        head_branch: head_branch.map(|value| bounded(value, 255)),
        head_sha: bounded(head_sha, 64),
        url: url.and_then(canonical_url),
    }
}
