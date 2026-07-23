use serde_json::{Value, json};

use super::detail_contract_tests::{issue_comment, pull_request_file};
use super::detail_tests::{issue_detail, pull_request_detail};
use super::support::{call, response};

#[test]
fn requires_nullable_pull_request_fields_to_be_present() {
    for field in ["body", "mergeable", "closed_at", "merged_at"] {
        let mut detail = pull_request_detail();
        detail.as_object_mut().unwrap().remove(field);
        assert_invalid_pull_request(detail, &format!("missing {field}"));
    }
}

#[test]
fn rejects_missing_null_and_wrong_typed_pull_request_fields() {
    let mut missing = pull_request_detail();
    missing.as_object_mut().unwrap().remove("title");
    let mut null = pull_request_detail();
    null["user"] = Value::Null;
    let mut wrong = pull_request_detail();
    wrong["commits"] = json!("four");
    for (detail, context) in [
        (missing, "missing title"),
        (null, "null user"),
        (wrong, "wrong commits"),
    ] {
        assert_invalid_pull_request(detail, context);
    }
}

#[test]
fn rejects_missing_nullable_or_malformed_pull_request_file_fields() {
    let mut missing = pull_request_file();
    missing.as_object_mut().unwrap().remove("sha");
    let mut null = pull_request_file();
    null["filename"] = Value::Null;
    let mut wrong = pull_request_file();
    wrong["additions"] = json!("two");
    for file in [missing, null, wrong] {
        let (output, _) = call(
            "github_read_pr",
            json!({ "owner": "octo", "repository": "repo", "number": 7 }),
            vec![
                response(200, pull_request_detail()),
                response(200, json!([file])),
            ],
        );
        assert_eq!(output["error"], "invalid_provider_response");
    }
}

#[test]
fn requires_nullable_issue_fields_to_be_present() {
    for field in ["user", "milestone", "closed_at"] {
        let mut detail = issue_detail();
        detail.as_object_mut().unwrap().remove(field);
        assert_invalid_issue(detail, &format!("missing {field}"));
    }
}

#[test]
fn rejects_missing_null_and_wrong_typed_issue_fields() {
    let mut missing = issue_detail();
    missing.as_object_mut().unwrap().remove("title");
    let mut null = issue_detail();
    null["labels"] = Value::Null;
    let mut wrong = issue_detail();
    wrong["comments"] = json!("one");
    for (detail, context) in [
        (missing, "missing title"),
        (null, "null labels"),
        (wrong, "wrong comments"),
    ] {
        assert_invalid_issue(detail, context);
    }
}

#[test]
fn rejects_missing_nullable_or_malformed_issue_comment_fields() {
    let mut missing = issue_comment(1);
    missing.as_object_mut().unwrap().remove("user");
    let mut null = issue_comment(1);
    null["body"] = Value::Null;
    let mut wrong = issue_comment(1);
    wrong["created_at"] = json!(7);
    for comment in [missing, null, wrong] {
        let (output, _) = call(
            "github_read_issue",
            json!({ "owner": "octo", "repository": "repo", "number": 9 }),
            vec![
                response(200, issue_detail()),
                response(200, json!([comment])),
            ],
        );
        assert_eq!(output["error"], "invalid_provider_response");
    }
}

#[test]
fn rejects_detail_number_mismatches_without_fetching_a_page() {
    let mut pull_request = pull_request_detail();
    pull_request["number"] = json!(8);
    let (output, requests) = call(
        "github_read_pr",
        json!({ "owner": "octo", "repository": "repo", "number": 7 }),
        vec![response(200, pull_request)],
    );
    assert_eq!(output["error"], "invalid_provider_response");
    assert_eq!(requests.len(), 1);

    let mut issue = issue_detail();
    issue["number"] = json!(8);
    let (output, requests) = call(
        "github_read_issue",
        json!({ "owner": "octo", "repository": "repo", "number": 9 }),
        vec![response(200, issue)],
    );
    assert_eq!(output["error"], "invalid_provider_response");
    assert_eq!(requests.len(), 1);
}

fn assert_invalid_pull_request(detail: Value, context: &str) {
    let (output, _) = call(
        "github_read_pr",
        json!({
            "owner": "octo",
            "repository": "repo",
            "number": 7,
            "include_files": false
        }),
        vec![response(200, detail)],
    );
    assert_eq!(output["error"], "invalid_provider_response", "{context}");
}

fn assert_invalid_issue(detail: Value, context: &str) {
    let (output, _) = call(
        "github_read_issue",
        json!({
            "owner": "octo",
            "repository": "repo",
            "number": 9,
            "include_comments": false
        }),
        vec![response(200, detail)],
    );
    assert_eq!(output["error"], "invalid_provider_response", "{context}");
}
