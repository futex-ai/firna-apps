//! Bounded, redacted projections for model-visible GitHub events.

use std::collections::BTreeMap;

use crate::github::webhook_effects;
use crate::github::webhook_host::WebhookError;
use crate::github::webhook_projection_common::{
    MAX_TITLE_CHARS, actor_projection, bounded, canonical_url, comment_projection,
    commit_projection, issue_projection, pull_request_projection, repository_projection,
    review_projection,
};
use crate::github::webhook_projection_types::{
    AutomationProjection, EventPayload, EventProjection, NormalizedEvent, StatusProjection,
};
use crate::github::webhook_types::{GitHubWebhookPayload, VerifiedProviderEvent};

const MAX_COMMITS: usize = 20;

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
        "check_run" => {
            let run = required(body.check_run.as_ref())?;
            Ok(EventProjection::CheckRun {
                signal: automation_projection(
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
                signal: automation_projection(
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
                signal: automation_projection(
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
                signal: automation_projection(
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

fn automation_projection(
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

fn required<T>(value: Option<&T>) -> Result<&T, WebhookError> {
    value.ok_or(WebhookError::EventTypeDisagreement)
}
