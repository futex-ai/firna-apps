//! Complete closed GitHub webhook event catalog tests.

use serde_json::{Value, json};

use super::webhook_support::{DIGEST, envelope, fixture, verify_with_digest};

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

pub(super) const ACKNOWLEDGED: [&str; 29] = [
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

#[test]
fn verifies_every_published_event_fixture() {
    for event_type in PUBLISHED {
        let result = verify_with_digest(
            &envelope(
                &fixture(event_type),
                event_type,
                Some(&format!("sha256={DIGEST}")),
            ),
            DIGEST,
        );
        assert_eq!(result["provider_event_type"], event_type);
        assert_eq!(result["provider_repository_id"], "3001");
    }
}

#[test]
fn authenticates_every_acknowledged_event_without_specific_content() {
    let body = fixture("acknowledged");
    for event_type in ACKNOWLEDGED {
        let result = verify_with_digest(
            &envelope(&body, event_type, Some(&format!("sha256={DIGEST}"))),
            DIGEST,
        );
        assert_eq!(result["provider_event_type"], event_type);
        assert_eq!(result["provider_repository_id"], "3001");
    }
}

#[test]
fn every_published_event_rejects_a_missing_required_object() {
    let cases = [
        ("push", "ref"),
        ("pull_request", "pull_request"),
        ("pull_request_review", "review"),
        ("pull_request_review_comment", "comment"),
        ("issues", "issue"),
        ("issue_comment", "comment"),
        ("check_run", "check_run"),
        ("check_suite", "check_suite"),
        ("status", "sha"),
        ("workflow_job", "workflow_job"),
        ("workflow_run", "workflow_run"),
        ("merge_group", "merge_group"),
        ("branch_protection_configuration", "action"),
        ("branch_protection_rule", "action"),
        ("repository_ruleset", "action"),
        ("security_and_analysis", "action"),
    ];
    for (event_type, required_field) in cases {
        let mut body: Value =
            serde_json::from_str(&fixture(event_type)).expect("fixture should be JSON");
        body[required_field] = Value::Null;
        let result = verify_with_digest(
            &envelope(
                &body.to_string(),
                event_type,
                Some(&format!("sha256={DIGEST}")),
            ),
            DIGEST,
        );
        assert_eq!(
            result["reason"], "github_event_type_disagreement",
            "missing {required_field} was accepted for {event_type}"
        );
    }
}

#[test]
fn acknowledged_events_require_only_signed_common_identity() {
    let mut body: Value =
        serde_json::from_str(&fixture("acknowledged")).expect("fixture should be JSON");
    body["action"] = json!("future_provider_action");
    body["arbitrary"] = json!({"nested": ["SECRET-MARKER", {"patch": "PATCH-MARKER"}]});
    for event_type in ACKNOWLEDGED {
        let result = verify_with_digest(
            &envelope(
                &body.to_string(),
                event_type,
                Some(&format!("sha256={DIGEST}")),
            ),
            DIGEST,
        );
        assert_eq!(result["provider_event_type"], event_type);
    }
}
