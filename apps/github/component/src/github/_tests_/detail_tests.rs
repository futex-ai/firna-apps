use std::collections::BTreeMap;

use serde_json::{Value, json};

use super::support::{call, response, response_with_headers, user};

#[test]
fn reads_pull_request_counts_and_one_bounded_file_page() {
    let headers = BTreeMap::from([(
        String::from("link"),
        String::from("<https://api.github.com/repos/octo/repo/pulls/7/files?page=2>; rel=\"next\""),
    )]);
    let (output, requests) = call(
        "github_read_pr",
        json!({ "owner": "octo", "repository": "repo", "number": 7 }),
        vec![
            response(200, pull_request_detail()),
            response_with_headers(
                200,
                headers,
                json!([{
                    "filename": "src/lib.rs",
                    "status": "modified",
                    "additions": 2,
                    "deletions": 1,
                    "changes": 3,
                    "sha": null,
                    "blob_url": "https://github.com/octo/repo/blob/abc/src/lib.rs"
                }]),
            ),
        ],
    );

    assert_eq!(output["commit_count"], 4);
    assert_eq!(output["issue_comment_count"], 5);
    assert_eq!(output["review_comment_count"], 6);
    assert_eq!(output["body"], json!(null));
    assert_eq!(output["draft"], json!(null));
    assert_eq!(output["mergeable"], json!(null));
    assert_eq!(output["files"][0]["sha"], json!(null));
    assert!(output["files"][0].get("patch").is_none());
    assert_eq!(output["next_files_page"], 2);
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[1].query["per_page"], "10");
}

#[test]
fn returns_halved_retry_input_only_for_oversized_file_pages() {
    let mut truncated = response(200, json!([]));
    truncated.body_truncated = true;
    let (error, _) = call(
        "github_read_pr",
        json!({
            "owner": "octo",
            "repository": "repo",
            "number": 7,
            "files_page": 3,
            "files_per_page": 9
        }),
        vec![response(200, pull_request_detail()), truncated],
    );
    assert_eq!(error["error"], "provider_response_too_large");
    assert_eq!(error["retry_input"]["files_page"], 3);
    assert_eq!(error["retry_input"]["files_per_page"], 4);

    let mut one_row = response(200, json!([]));
    one_row.body_truncated = true;
    let (error, _) = call(
        "github_read_pr",
        json!({
            "owner": "octo",
            "repository": "repo",
            "number": 7,
            "files_per_page": 1
        }),
        vec![response(200, pull_request_detail()), one_row],
    );
    assert_eq!(error["retry_input"], json!(null));
}

#[test]
fn reads_issue_nulls_labels_assignees_and_comment_authors() {
    let (output, requests) = call(
        "github_read_issue",
        json!({ "owner": "octo", "repository": "repo", "number": 9 }),
        vec![
            response(200, issue_detail()),
            response(
                200,
                json!([{
                    "id": 10,
                    "user": null,
                    "body": "comment",
                    "created_at": "2026-01-01T00:00:00Z",
                    "updated_at": "2026-01-01T00:00:00Z",
                    "html_url": "https://github.com/octo/repo/issues/9#issuecomment-10"
                }]),
            ),
        ],
    );

    assert_eq!(output["body"], json!(null));
    assert_eq!(output["state_reason"], json!(null));
    assert_eq!(output["author"], json!(null));
    assert_eq!(output["milestone"], json!(null));
    assert_eq!(output["comments"][0]["author"], json!(null));
    assert_eq!(output["labels"][0]["name"], "bug");
    assert_eq!(output["assignees"].as_array().expect("array").len(), 1);
    assert_eq!(requests.len(), 2);
}

#[test]
fn issue_pull_request_discriminator_fails_closed() {
    let mut pull_request = issue_detail();
    pull_request["pull_request"] = json!({ "url": "https://api.github.com/example" });
    let (error, requests) = call(
        "github_read_issue",
        json!({ "owner": "octo", "repository": "repo", "number": 9 }),
        vec![response(200, pull_request)],
    );
    assert_eq!(error["error"], "invalid_request");
    assert_eq!(error["reason"], "use_github_read_pr");
    assert_eq!(requests.len(), 1);

    let mut wrong = issue_detail();
    wrong["pull_request"] = Value::Null;
    let (error, _) = call(
        "github_read_issue",
        json!({ "owner": "octo", "repository": "repo", "number": 9 }),
        vec![response(200, wrong)],
    );
    assert_eq!(error["error"], "provider_contract_error");
}

pub(super) fn pull_request_detail() -> Value {
    json!({
        "number": 7,
        "title": "Improve parser",
        "body": null,
        "state": "open",
        "merged": false,
        "mergeable": null,
        "user": user("octo"),
        "base": { "ref": "main", "sha": "base" },
        "head": { "ref": "feature", "sha": "head" },
        "additions": 20,
        "deletions": 3,
        "changed_files": 2,
        "commits": 4,
        "comments": 5,
        "review_comments": 6,
        "created_at": "2026-01-01T00:00:00Z",
        "updated_at": "2026-01-02T00:00:00Z",
        "closed_at": null,
        "merged_at": null,
        "html_url": "https://github.com/octo/repo/pull/7"
    })
}

pub(super) fn issue_detail() -> Value {
    json!({
        "number": 9,
        "title": "Bug report",
        "body": null,
        "state": "open",
        "state_reason": null,
        "user": null,
        "labels": [{ "name": "bug", "color": "ff0000", "description": null }],
        "assignees": [user("octo")],
        "milestone": null,
        "comments": 1,
        "created_at": "2026-01-01T00:00:00Z",
        "updated_at": "2026-01-02T00:00:00Z",
        "closed_at": null,
        "html_url": "https://github.com/octo/repo/issues/9"
    })
}
