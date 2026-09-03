//! Provider-neutral repository-change effect regressions.

use serde_json::{Value, json};

use crate::github::webhooks::normalize_event;

use super::webhook_support::{fixture, valid_verification};

#[test]
fn status_and_policy_events_emit_bounded_repository_change_effects() {
    let cases = [
        ("push", "source", "branches", Some("main"), None),
        (
            "pull_request",
            "pull_request",
            "branches",
            Some("events"),
            Some(17),
        ),
        (
            "pull_request_review",
            "review",
            "branches",
            Some("events"),
            Some(17),
        ),
        ("check_run", "check", "branches", Some("events"), Some(17)),
        ("check_suite", "check", "branches", Some("events"), Some(17)),
        ("status", "check", "branches", Some("events"), None),
        (
            "workflow_job",
            "workflow",
            "branches",
            Some("events"),
            Some(17),
        ),
        (
            "workflow_run",
            "workflow",
            "branches",
            Some("events"),
            Some(17),
        ),
        ("merge_group", "merge_queue", "repository", None, None),
        (
            "branch_protection_configuration",
            "repository_policy",
            "repository",
            None,
            None,
        ),
        (
            "branch_protection_rule",
            "repository_policy",
            "repository",
            None,
            None,
        ),
        (
            "repository_ruleset",
            "repository_policy",
            "repository",
            None,
            None,
        ),
        (
            "security_and_analysis",
            "security",
            "repository",
            None,
            None,
        ),
    ];

    for (event_type, change_kind, scope, branch, pull_request_number) in cases {
        let output = normalize(&fixture(event_type), event_type);
        let effects = output["platform_effects"]
            .as_array()
            .expect("published status event should contain effects");
        assert_eq!(effects.len(), 1, "unexpected effect count for {event_type}");
        let effect = &effects[0];
        assert_eq!(effect["kind"], "repository_change");
        assert_eq!(effect["contract_version"], 1);
        assert_eq!(effect["provider_repository_id"], "3001");
        assert_eq!(effect["change_kind"], change_kind);
        assert_eq!(effect["scope"], scope);
        match branch {
            Some(branch) => assert_eq!(effect["branches"], json!([branch])),
            None => assert!(effect.get("branches").is_none()),
        }
        match pull_request_number {
            Some(number) => assert_eq!(effect["pull_request_number"], number),
            None => assert!(effect.get("pull_request_number").is_none()),
        }
    }
}

#[test]
fn tag_pushes_and_subscriber_only_events_emit_no_platform_effect() {
    let mut tag: Value = serde_json::from_str(&fixture("push")).expect("fixture should be JSON");
    tag["ref"] = json!("refs/tags/v2.1.0");
    let output = normalize(&tag.to_string(), "push");
    assert!(output.get("platform_effects").is_none());

    for event_type in ["pull_request_review_comment", "issues", "issue_comment"] {
        let output = normalize(&fixture(event_type), event_type);
        assert!(output.get("platform_effects").is_none());
    }
}

#[test]
fn status_branch_hints_are_unique_and_bounded_to_eight() {
    let mut body: Value = serde_json::from_str(&fixture("status")).expect("fixture should be JSON");
    body["branches"] = Value::Array(
        (0..12)
            .map(|index| json!({"name": format!("branch-{index}")}))
            .chain([json!({"name": "branch-0"})])
            .collect(),
    );

    let output = normalize(&body.to_string(), "status");
    let branches = output["platform_effects"][0]["branches"]
        .as_array()
        .expect("branches should be present");
    assert_eq!(branches.len(), 8);
    assert_eq!(branches[0], "branch-0");
    assert_eq!(branches[7], "branch-7");
}

fn normalize(body: &str, event_type: &str) -> Value {
    let (envelope, verification) = valid_verification(body, event_type);
    let request = json!({
        "workspace_id": "018f0000-0000-7000-8000-000000000001",
        "installation_id": "018f0000-0000-7000-8000-000000000002",
        "envelope": envelope,
        "verification": verification
    });
    serde_json::from_str(&normalize_event(&request.to_string()))
        .expect("normalization output should be JSON")
}
