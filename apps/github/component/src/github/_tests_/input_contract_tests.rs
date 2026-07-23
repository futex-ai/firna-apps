use serde_json::{Value, json};

use crate::github::input::{
    git_ref, language, owner, page, path, repository, result_offset, search_path, search_term,
};

use super::support::{call, response};

#[test]
fn rejects_wrong_scalar_types_and_unknown_input_fields_before_dispatch() {
    let cases = [
        ("github_list_repositories", json!({ "page": "1" })),
        ("github_list_repositories", json!({ "per_page": 1.5 })),
        ("github_list_repositories", json!({ "visibility": "all" })),
        ("github_search_code", json!({})),
        ("github_search_code", json!({ "query": 7 })),
        (
            "github_read_file",
            json!({ "owner": "octo", "repository": "repo", "path": 7 }),
        ),
        (
            "github_read_pr",
            json!({ "owner": "octo", "repository": "repo", "number": 1.5 }),
        ),
        (
            "github_read_pr",
            json!({ "owner": "octo", "repository": "repo", "number": 1, "include_files": 1 }),
        ),
        (
            "github_read_issue",
            json!({ "owner": "octo", "repository": "repo", "number": 1, "include_comments": "yes" }),
        ),
        ("github_list_repositories", json!({ "unknown": true })),
    ];

    for (tool, input) in cases {
        let (output, requests) = call(tool, input, vec![]);
        assert_eq!(output["reason"], "invalid_tool_call", "tool {tool}");
        assert!(requests.is_empty());
    }
}

#[test]
fn enforces_string_byte_scalar_and_shape_boundaries() {
    assert!(owner(&"a".repeat(100)).is_ok());
    assert!(owner(&"a".repeat(101)).is_err());
    assert!(owner("octo-").is_err());
    assert!(repository(&"a".repeat(100)).is_ok());
    assert!(repository(&"a".repeat(101)).is_err());
    assert!(repository("repo/name").is_err());
    assert!(path(&"a".repeat(1_024)).is_ok());
    assert!(path(&"a".repeat(1_025)).is_err());
    assert!(path("/absolute").is_err());
    assert!(git_ref(Some("a".repeat(255))).is_ok());
    assert!(git_ref(Some("a".repeat(256))).is_err());
    assert!(search_term(&"é".repeat(256)).is_ok());
    assert!(search_term(&"é".repeat(257)).is_err());
    assert!(language(Some("é".repeat(100))).is_ok());
    assert!(language(Some("é".repeat(101))).is_err());
    assert!(search_path(Some("a".repeat(256))).is_ok());
    assert!(search_path(Some("a".repeat(257))).is_err());
}

#[test]
fn enforces_numeric_endpoints_and_checked_finite_windows() {
    assert_eq!(page(Some(2_147_483_647)).unwrap(), 2_147_483_647);
    assert!(page(Some(2_147_483_648)).is_err());
    assert_eq!(result_offset(2_147_483_647, 50).unwrap(), 107_374_182_300);

    assert_invalid(
        "github_search_code",
        json!({ "query": "rust", "page": 51, "per_page": 20 }),
        "result_window_exceeded",
    );
    let (search, _) = call(
        "github_search_code",
        json!({ "query": "rust", "page": 50, "per_page": 20 }),
        vec![response(
            200,
            json!({ "total_count": 0, "incomplete_results": false, "items": [] }),
        )],
    );
    assert_eq!(search["page"], 50);

    assert_invalid(
        "github_read_pr",
        json!({
            "owner": "octo",
            "repository": "repo",
            "number": 1,
            "files_page": 301,
            "files_per_page": 10
        }),
        "result_window_exceeded",
    );
}

#[test]
fn rejects_all_zero_negative_excessive_page_and_number_values() {
    let cases = [
        (
            "github_list_repositories",
            json!({ "page": 0 }),
            "invalid_page",
        ),
        (
            "github_list_repositories",
            json!({ "per_page": 51 }),
            "invalid_page_size",
        ),
        (
            "github_search_code",
            json!({ "query": "rust", "per_page": 21 }),
            "invalid_page_size",
        ),
        (
            "github_read_pr",
            json!({ "owner": "octo", "repository": "repo", "number": 0 }),
            "invalid_number",
        ),
        (
            "github_read_pr",
            json!({ "owner": "octo", "repository": "repo", "number": 2147483648_i64 }),
            "invalid_number",
        ),
        (
            "github_read_issue",
            json!({ "owner": "octo", "repository": "repo", "number": -1 }),
            "invalid_number",
        ),
        (
            "github_read_issue",
            json!({ "owner": "octo", "repository": "repo", "number": 1, "comments_per_page": 11 }),
            "invalid_page_size",
        ),
    ];
    for (tool, input, reason) in cases {
        assert_invalid(tool, input, reason);
    }
}

#[test]
fn requires_both_search_repository_qualifiers() {
    assert_invalid(
        "github_search_code",
        json!({ "query": "rust", "owner": "octo" }),
        "owner_qualifier_requires_repository",
    );
    assert_invalid(
        "github_search_code",
        json!({ "query": "rust", "repository": "repo" }),
        "repository_qualifier_requires_owner",
    );
}

fn assert_invalid(tool: &str, input: Value, reason: &str) {
    let (output, requests) = call(tool, input, vec![]);
    assert_eq!(output["error"], "invalid_request");
    assert_eq!(output["reason"], reason);
    assert!(requests.is_empty());
}
