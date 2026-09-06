//! Broad GitHub webhook and permission manifest conformance.

use std::collections::BTreeMap;

use fna_apps_interface::PlatformEffectKind;
use fna_apps_interface::manifest::{IngressEventHandling, InstallationPermissionLevel};

use crate::manifest;

const PACKAGE_OVERVIEW: &str = include_str!("../../../README.md");

const PUBLISHED: [&str; 16] = [
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

const ACKNOWLEDGED: [&str; 28] = [
    "create",
    "delete",
    "commit_comment",
    "fork",
    "gollum",
    "release",
    "repository",
    "repository_dispatch",
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

const EFFECTS: [&str; 13] = [
    "push",
    "pull_request",
    "pull_request_review",
    "merge_group",
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

#[test]
fn manifest_declares_the_exact_published_and_acknowledged_baseline() {
    let manifest = manifest();
    let events = &manifest.ingress[0].events;
    assert_eq!(events.len(), 44);

    for event_type in PUBLISHED {
        let event = events
            .iter()
            .find(|event| event.provider_type == event_type)
            .expect("published event should exist");
        assert_eq!(event.handling, IngressEventHandling::Publish);
        let expected_effects = if EFFECTS.contains(&event_type) {
            vec![PlatformEffectKind::RepositoryChange]
        } else {
            Vec::new()
        };
        assert_eq!(event.platform_effects, expected_effects);
    }
    for event_type in ACKNOWLEDGED {
        let event = events
            .iter()
            .find(|event| event.provider_type == event_type)
            .expect("acknowledged event should exist");
        assert_eq!(event.handling, IngressEventHandling::Acknowledge);
        assert!(event.platform_effects.is_empty());
    }
    let overview = PACKAGE_OVERVIEW
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        overview
            .contains("16 published signed repository events, and 28 authenticated dormant events")
    );
}

#[test]
fn manifest_uses_the_selected_grants_and_narrow_read_tool_subsets() {
    let manifest = manifest();
    let flow = manifest.credential_flows[0]
        .installation_token()
        .expect("GitHub installation-token flow");
    assert_eq!(flow.permissions, expected_permissions());
    assert_eq!(
        manifest.auth_requirements[0].scopes,
        expected_permissions()
            .into_iter()
            .map(|(name, level)| format!("{name}:{}", permission_name(level)))
            .collect::<Vec<_>>()
    );

    let expected_tools = [
        ("github_list_repositories", vec!["metadata"]),
        ("github_search_code", vec!["contents", "metadata"]),
        ("github_read_file", vec!["contents", "metadata"]),
        (
            "github_read_pr",
            vec![
                "checks",
                "contents",
                "metadata",
                "pull_requests",
                "statuses",
            ],
        ),
        ("github_read_issue", vec!["issues", "metadata"]),
    ];
    for (tool_name, permission_names) in expected_tools {
        let tool = manifest
            .tools
            .iter()
            .find(|tool| tool.name == tool_name)
            .expect("tool should exist");
        let permissions = tool
            .installation_permissions
            .as_ref()
            .expect("tool permission subset");
        assert_eq!(permissions.len(), permission_names.len());
        for name in permission_names {
            assert_eq!(
                permissions.get(name),
                Some(&InstallationPermissionLevel::Read)
            );
        }
    }
}

#[test]
fn manifest_excludes_security_alert_grants_and_events() {
    let manifest = manifest();
    let flow = manifest.credential_flows[0]
        .installation_token()
        .expect("GitHub installation-token flow");
    for permission in [
        "code_scanning_alerts",
        "dependabot_alerts",
        "repository_advisories",
        "secret_scanning_alerts",
    ] {
        assert!(!flow.permissions.contains_key(permission));
    }

    let events = &manifest.ingress[0].events;
    for event_type in [
        "code_scanning_alert",
        "dependabot_alert",
        "repository_advisory",
        "repository_vulnerability_alert",
        "secret_scanning_alert",
        "security_advisory",
    ] {
        assert!(
            events.iter().all(|event| event.provider_type != event_type),
            "unexpected security-alert event {event_type}"
        );
    }
}

fn expected_permissions() -> BTreeMap<String, InstallationPermissionLevel> {
    [
        ("actions", InstallationPermissionLevel::Write),
        ("actions_variables", InstallationPermissionLevel::Write),
        ("administration", InstallationPermissionLevel::Read),
        ("checks", InstallationPermissionLevel::Write),
        ("statuses", InstallationPermissionLevel::Write),
        ("contents", InstallationPermissionLevel::Write),
        ("deployments", InstallationPermissionLevel::Write),
        ("discussions", InstallationPermissionLevel::Write),
        ("issues", InstallationPermissionLevel::Write),
        ("merge_queues", InstallationPermissionLevel::Write),
        ("metadata", InstallationPermissionLevel::Read),
        ("packages", InstallationPermissionLevel::Write),
        ("pages", InstallationPermissionLevel::Write),
        ("repository_projects", InstallationPermissionLevel::Write),
        ("pull_requests", InstallationPermissionLevel::Write),
        ("workflows", InstallationPermissionLevel::Write),
    ]
    .into_iter()
    .map(|(name, level)| (name.to_owned(), level))
    .collect()
}

fn permission_name(level: InstallationPermissionLevel) -> &'static str {
    match level {
        InstallationPermissionLevel::Read => "read",
        InstallationPermissionLevel::Write => "write",
    }
}
