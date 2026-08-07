//! Bounded GitHub webhook normalization tests.

use serde_json::{Value, json};

use crate::github::webhooks::normalize_event;

use super::webhook_support::{fixture, valid_verification};

const EVENTS: [&str; 6] = [
    "push",
    "pull_request",
    "pull_request_review",
    "pull_request_review_comment",
    "issues",
    "issue_comment",
];

#[test]
fn normalizes_every_subscribed_event_fixture() {
    for event_type in EVENTS {
        let body = fixture(event_type);
        let output = normalize(&body, event_type);

        assert_eq!(output["app_id"], "github");
        assert_eq!(output["provider"], "github");
        assert_eq!(output["provider_event_type"], event_type);
        assert_eq!(output["provider_account_id"], "2001");
        assert_eq!(output["source"]["installation_id"], "1001");
        assert_eq!(output["source"]["repository_id"], "3001");
        assert_eq!(output["payload"]["event"]["kind"], event_type);
    }
}

#[test]
fn bounds_commit_lists_and_model_visible_text() {
    let mut push: Value = serde_json::from_str(&fixture("push")).expect("fixture should be JSON");
    let commit = push["commits"][0].clone();
    push["commits"] = Value::Array((0..25).map(|_| commit.clone()).collect());
    push["commits"][0]["message"] = json!("m".repeat(800));
    let output = normalize(&push.to_string(), "push");
    let commits = output["payload"]["event"]["commits"]
        .as_array()
        .expect("commits should be an array");
    assert_eq!(commits.len(), 20);
    assert_eq!(
        commits[0]["message"]
            .as_str()
            .expect("message should be text")
            .chars()
            .count(),
        512
    );

    let mut issue: Value =
        serde_json::from_str(&fixture("issue_comment")).expect("fixture should be JSON");
    issue["issue"]["title"] = json!("t".repeat(400));
    issue["issue"]["body"] = json!("b".repeat(3_000));
    issue["comment"]["body"] = json!("c".repeat(3_000));
    let output = normalize(&issue.to_string(), "issue_comment");
    assert_eq!(
        output["payload"]["event"]["issue"]["title"]
            .as_str()
            .expect("title should be text")
            .chars()
            .count(),
        256
    );
    assert_eq!(
        output["payload"]["event"]["comment"]["body"]
            .as_str()
            .expect("body should be text")
            .chars()
            .count(),
        2_000
    );
}

#[test]
fn omits_secrets_patches_unknown_fields_and_noncanonical_urls() {
    let mut issue: Value =
        serde_json::from_str(&fixture("issue_comment")).expect("fixture should be JSON");
    issue["private_key"] = json!("PRIVATE-MARKER");
    issue["token"] = json!("TOKEN-MARKER");
    issue["comment"]["patch"] = json!("PATCH-MARKER");
    issue["repository"]["html_url"] = json!("https://evil.example/repo");
    issue["comment"]["html_url"] = json!("https://api.github.com/repos/private");

    let output = normalize(&issue.to_string(), "issue_comment");
    let encoded = output.to_string();
    assert!(!encoded.contains("PRIVATE-MARKER"));
    assert!(!encoded.contains("TOKEN-MARKER"));
    assert!(!encoded.contains("PATCH-MARKER"));
    assert_eq!(output["payload"]["repository"]["url"], Value::Null);
    assert_eq!(output["payload"]["event"]["comment"]["url"], Value::Null);
}

#[test]
fn refuses_to_normalize_control_or_unsupported_events() {
    let body = fixture("installation");
    let output = normalize(&body, "installation");

    assert_eq!(output["error"], "invalid_request");
    assert_eq!(output["reason"], "unsupported_github_event");
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
