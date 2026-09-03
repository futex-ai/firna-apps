//! Closed GitHub webhook event and control catalog.

pub(super) const PUBLISHED_EVENTS: [&str; 16] = [
    "push",
    "pull_request",
    "pull_request_review",
    "pull_request_review_comment",
    "merge_group",
    "issues",
    "issue_comment",
    "check_run",
    "check_suite",
    "status",
    "workflow_job",
    "workflow_run",
    "branch_protection_configuration",
    "branch_protection_rule",
    "repository_ruleset",
    "security_and_analysis",
];

pub(super) const ACKNOWLEDGED_EVENTS: [&str; 29] = [
    "create",
    "delete",
    "commit_comment",
    "fork",
    "gollum",
    "release",
    "repository",
    "repository_dispatch",
    "member",
    "public",
    "star",
    "watch",
    "pull_request_review_thread",
    "merge_queue_entry",
    "issue_dependencies",
    "sub_issues",
    "label",
    "milestone",
    "workflow_dispatch",
    "deployment",
    "deployment_status",
    "deployment_review",
    "deployment_protection_rule",
    "registry_package",
    "page_build",
    "discussion",
    "discussion_comment",
    "deploy_key",
    "exemption_request_push_ruleset",
];

pub(super) const CONTROL_EVENTS: [&str; 5] = [
    "ping",
    "installation",
    "installation_repositories",
    "installation_target",
    "github_app_authorization",
];

pub(super) fn is_published(event_type: &str) -> bool {
    PUBLISHED_EVENTS.contains(&event_type)
}

pub(super) fn is_acknowledged(event_type: &str) -> bool {
    ACKNOWLEDGED_EVENTS.contains(&event_type)
}

pub(super) fn is_control(event_type: &str) -> bool {
    CONTROL_EVENTS.contains(&event_type)
}

pub(super) fn is_supported(event_type: &str) -> bool {
    is_published(event_type) || is_acknowledged(event_type) || is_control(event_type)
}
