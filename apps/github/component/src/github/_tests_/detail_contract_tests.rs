//! Contract tests for pull-request and issue detail projections.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use super::detail_tests::{issue_detail, pull_request_detail};
use super::support::{call, response, response_with_headers, user};

#[test]
fn skips_optional_detail_pages_when_include_flags_are_false() {
    let (pull_request, requests) = call(
        "github_read_pr",
        json!({
            "owner": "octo",
            "repository": "repo",
            "number": 7,
            "include_files": false
        }),
        vec![response(200, pull_request_detail())],
    );
    assert_eq!(requests.len(), 1);
    assert_eq!(pull_request["files"], json!([]));
    assert!(pull_request.get("next_files_page").is_none());

    let (issue, requests) = call(
        "github_read_issue",
        json!({
            "owner": "octo",
            "repository": "repo",
            "number": 9,
            "include_comments": false
        }),
        vec![response(200, issue_detail())],
    );
    assert_eq!(requests.len(), 1);
    assert_eq!(issue["comments"], json!([]));
    assert!(issue.get("next_comments_page").is_none());
}

#[test]
fn projects_all_documented_nullable_and_optional_detail_states() {
    let mut issue = issue_detail();
    let object = issue.as_object_mut().expect("fixture should be an object");
    object.remove("body");
    object.remove("state_reason");
    object.remove("assignees");
    let (issue, _) = call(
        "github_read_issue",
        json!({
            "owner": "octo",
            "repository": "repo",
            "number": 9,
            "include_comments": false
        }),
        vec![response(200, issue)],
    );
    assert_eq!(issue["body"], Value::Null);
    assert_eq!(issue["state_reason"], Value::Null);
    assert_eq!(issue["assignees"], json!([]));
    assert_eq!(issue["closed_at"], Value::Null);

    let (pull_request, _) = call(
        "github_read_pr",
        json!({
            "owner": "octo",
            "repository": "repo",
            "number": 7,
            "include_files": false
        }),
        vec![response(200, pull_request_detail())],
    );
    for field in ["body", "draft", "mergeable", "closed_at", "merged_at"] {
        assert_eq!(pull_request[field], Value::Null, "field {field}");
    }
}

#[test]
fn bounds_issue_labels_and_assignees_without_inventing_rows() {
    let mut detail = issue_detail();
    detail["labels"] = Value::Array(
        (0..101)
            .map(|index| {
                json!({
                    "name": format!("label-{index}"),
                    "color": "ffffff",
                    "description": null
                })
            })
            .collect(),
    );
    detail["assignees"] = Value::Array(
        (0..21)
            .map(|index| user(&format!("user-{index}")))
            .collect(),
    );
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
    assert_eq!(output["labels"].as_array().unwrap().len(), 100);
    assert_eq!(output["assignees"].as_array().unwrap().len(), 20);
    assert_eq!(output["labels_truncated"], true);
    assert_eq!(output["assignees_truncated"], true);
}

#[test]
fn returns_numeric_comment_pagination_and_bounded_retry_metadata() {
    let headers = BTreeMap::from([(
        String::from("link"),
        String::from(
            "<https://api.github.com/repos/octo/repo/issues/9/comments?page=2>; rel=\"next\"",
        ),
    )]);
    let (output, _) = call(
        "github_read_issue",
        json!({ "owner": "octo", "repository": "repo", "number": 9 }),
        vec![
            response(200, issue_detail()),
            response_with_headers(200, headers, json!([])),
        ],
    );
    assert_eq!(output["next_comments_page"], 2);

    for (per_page, retry) in [(9, json!(4)), (1, Value::Null)] {
        let mut truncated = response(200, json!([]));
        truncated.body_truncated = true;
        let (error, _) = call(
            "github_read_issue",
            json!({
                "owner": "octo",
                "repository": "repo",
                "number": 9,
                "comments_page": 3,
                "comments_per_page": per_page
            }),
            vec![response(200, issue_detail()), truncated],
        );
        assert_eq!(error["error"], "provider_response_too_large");
        if per_page == 1 {
            assert_eq!(error["retry_input"], retry);
        } else {
            assert_eq!(error["retry_input"]["comments_page"], 3);
            assert_eq!(error["retry_input"]["comments_per_page"], retry);
        }
    }
}

#[test]
fn suppresses_pull_request_next_page_at_the_3000_file_window() {
    let headers = BTreeMap::from([(
        String::from("link"),
        String::from(
            "<https://api.github.com/repos/octo/repo/pulls/7/files?page=301>; rel=\"next\"",
        ),
    )]);
    let (output, _) = call(
        "github_read_pr",
        json!({
            "owner": "octo",
            "repository": "repo",
            "number": 7,
            "files_page": 300,
            "files_per_page": 10
        }),
        vec![
            response(200, pull_request_detail()),
            response_with_headers(200, headers, json!([])),
        ],
    );
    assert!(output.get("next_files_page").is_none());
}

#[test]
fn rejects_provider_pages_that_exceed_requested_cardinality() {
    let files = Value::Array((0..11).map(|_| pull_request_file()).collect());
    let (error, _) = call(
        "github_read_pr",
        json!({ "owner": "octo", "repository": "repo", "number": 7 }),
        vec![response(200, pull_request_detail()), response(200, files)],
    );
    assert_eq!(error["error"], "provider_contract_error");

    let comments = Value::Array((0..11).map(issue_comment).collect());
    let (error, _) = call(
        "github_read_issue",
        json!({ "owner": "octo", "repository": "repo", "number": 9 }),
        vec![response(200, issue_detail()), response(200, comments)],
    );
    assert_eq!(error["error"], "provider_contract_error");
}

pub(super) fn pull_request_file() -> Value {
    json!({
        "filename": "src/lib.rs",
        "status": "modified",
        "additions": 2,
        "deletions": 1,
        "changes": 3,
        "sha": null,
        "blob_url": "https://github.com/octo/repo/blob/abc/src/lib.rs"
    })
}

pub(super) fn issue_comment(id: u64) -> Value {
    json!({
        "id": id,
        "user": null,
        "body": "comment",
        "created_at": "2026-01-01T00:00:00Z",
        "updated_at": "2026-01-01T00:00:00Z",
        "html_url": "https://github.com/octo/repo/issues/9#issuecomment-10"
    })
}
