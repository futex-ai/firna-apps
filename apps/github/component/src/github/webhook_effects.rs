//! Provider-neutral effects derived from authenticated published GitHub events.

use std::collections::HashSet;

use crate::github::webhook_projection_types::RepositoryChangeEffect;
use crate::github::webhook_signal_types::PullRequestReference;
use crate::github::webhook_types::{GitHubWebhookPayload, PullRequest};

const MAX_BRANCHES: usize = 8;
const MAX_BRANCH_BYTES: usize = 255;

pub(super) fn repository_changes(
    event_type: &str,
    payload: &GitHubWebhookPayload,
) -> Vec<RepositoryChangeEffect> {
    let Some(repository_id) = payload
        .repository
        .as_ref()
        .map(|value| value.id.to_string())
    else {
        return Vec::new();
    };
    let effect = match event_type {
        "push" => push(payload, repository_id),
        "pull_request" => {
            pull_request(payload.pull_request.as_ref(), repository_id, "pull_request")
        }
        "pull_request_review" => {
            pull_request(payload.pull_request.as_ref(), repository_id, "review")
        }
        "check_run" => payload.check_run.as_ref().map(|run| {
            branch_effect(
                repository_id,
                "check",
                run.check_suite
                    .as_ref()
                    .and_then(|suite| suite.name.as_deref()),
                &run.pull_requests,
                Some(&run.head_sha),
            )
        }),
        "check_suite" => payload.check_suite.as_ref().map(|suite| {
            branch_effect(
                repository_id,
                "check",
                suite.head_branch.as_deref(),
                &suite.pull_requests,
                Some(&suite.head_sha),
            )
        }),
        "status" => Some(status(payload, repository_id)),
        "workflow_job" => payload.workflow_job.as_ref().map(|job| {
            branch_effect(
                repository_id,
                "workflow",
                job.head_branch.as_deref(),
                &job.pull_requests,
                Some(&job.head_sha),
            )
        }),
        "workflow_run" => payload.workflow_run.as_ref().map(|run| {
            branch_effect(
                repository_id,
                "workflow",
                run.head_branch.as_deref(),
                &run.pull_requests,
                Some(&run.head_sha),
            )
        }),
        "merge_group" => payload
            .merge_group
            .as_ref()
            .map(|group| repository_effect(repository_id, "merge_queue", Some(&group.head_sha))),
        "branch_protection_configuration" | "branch_protection_rule" | "repository_ruleset" => {
            Some(repository_effect(repository_id, "repository_policy", None))
        }
        "security_and_analysis" => Some(repository_effect(repository_id, "security", None)),
        _ => None,
    };
    effect.into_iter().collect()
}

fn push(payload: &GitHubWebhookPayload, repository_id: String) -> Option<RepositoryChangeEffect> {
    let branch = payload.git_ref.as_deref()?.strip_prefix("refs/heads/")?;
    Some(effect(
        repository_id,
        "source",
        vec![branch.to_owned()],
        None,
        payload.after.as_deref(),
    ))
}

fn pull_request(
    pull_request: Option<&PullRequest>,
    repository_id: String,
    change_kind: &'static str,
) -> Option<RepositoryChangeEffect> {
    let pull_request = pull_request?;
    Some(effect(
        repository_id,
        change_kind,
        vec![pull_request.head.name.clone()],
        Some(pull_request.number),
        Some(&pull_request.head.sha),
    ))
}

fn branch_effect(
    repository_id: String,
    change_kind: &'static str,
    direct_branch: Option<&str>,
    pull_requests: &[PullRequestReference],
    head_sha: Option<&str>,
) -> RepositoryChangeEffect {
    let mut branches = direct_branch
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    branches.extend(
        pull_requests
            .iter()
            .filter_map(|pull_request| pull_request.head.as_ref())
            .filter_map(|head| head.name.clone()),
    );
    let pull_request_number = (pull_requests.len() == 1).then(|| pull_requests[0].number);
    let pull_request_head_sha = (pull_requests.len() == 1)
        .then(|| {
            pull_requests[0]
                .head
                .as_ref()
                .and_then(|head| head.sha.as_deref())
        })
        .flatten();
    effect(
        repository_id,
        change_kind,
        bounded_branches(branches),
        pull_request_number,
        pull_request_head_sha.or(head_sha),
    )
}

fn status(payload: &GitHubWebhookPayload, repository_id: String) -> RepositoryChangeEffect {
    let branches = bounded_branches(payload.branches.iter().map(|value| value.name.clone()));
    effect(
        repository_id,
        "check",
        branches,
        None,
        payload.sha.as_deref(),
    )
}

fn repository_effect(
    repository_id: String,
    change_kind: &'static str,
    head_sha: Option<&str>,
) -> RepositoryChangeEffect {
    effect(repository_id, change_kind, Vec::new(), None, head_sha)
}

fn effect(
    repository_id: String,
    change_kind: &'static str,
    branches: Vec<String>,
    pull_request_number: Option<u64>,
    head_sha: Option<&str>,
) -> RepositoryChangeEffect {
    RepositoryChangeEffect {
        kind: "repository_change",
        contract_version: 1,
        provider_repository_id: repository_id,
        change_kind,
        scope: if branches.is_empty() {
            "repository"
        } else {
            "branches"
        },
        branches,
        pull_request_number,
        head_sha: head_sha.filter(|value| valid_sha(value)).map(str::to_owned),
    }
}

fn bounded_branches(branches: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut seen = HashSet::new();
    branches
        .into_iter()
        .filter(|branch| valid_branch(branch))
        .filter(|branch| seen.insert(branch.clone()))
        .take(MAX_BRANCHES)
        .collect()
}

fn valid_branch(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= MAX_BRANCH_BYTES
        && !value.chars().any(char::is_control)
}

fn valid_sha(value: &str) -> bool {
    !value.is_empty() && value.len() <= 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
