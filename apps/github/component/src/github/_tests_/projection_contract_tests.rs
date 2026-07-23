use serde_json::{Value, json};

use crate::github::projection::{ExactKind, OUTPUT_BUDGET_BYTES, exact};

use super::detail_contract_tests::{issue_comment, pull_request_file};
use super::detail_tests::{issue_detail, pull_request_detail};
use super::support::{call, installation_repositories, repository, response};

#[test]
fn accepts_exact_field_maxima_and_rejects_oversized_or_invalid_values() {
    for (kind, maximum) in [
        (ExactKind::Identifier, 512),
        (ExactKind::PathOrTitle, 1_024),
        (ExactKind::EnumOrTimestamp, 64),
    ] {
        assert!(exact(&"a".repeat(maximum), kind).is_ok());
        assert!(exact(&"a".repeat(maximum + 1), kind).is_err());
    }
    let valid_url = format!("https://github.com/{}", "a".repeat(2_028));
    assert_eq!(valid_url.len(), 2_047);
    assert!(exact(&valid_url, ExactKind::Url).is_ok());
    assert!(exact(&(valid_url + "aa"), ExactKind::Url).is_err());
    for value in [
        "http://github.com/octo",
        "https://",
        "https://user@github.com",
    ] {
        assert!(exact(value, ExactKind::Url).is_err(), "url {value}");
    }
}

#[test]
fn maximum_repository_page_uses_multibyte_previews_within_budget() {
    let description = "é".repeat(1_025);
    let rows = Value::Array(
        (0..50)
            .map(|index| {
                let mut row = repository();
                row["id"] = json!(index);
                row["description"] = json!(description);
                row
            })
            .collect(),
    );
    let (output, _) = call(
        "github_list_repositories",
        json!({ "per_page": 50 }),
        vec![response(
            200,
            installation_repositories(rows.as_array().cloned().unwrap_or_default()),
        )],
    );
    assert_eq!(output["repositories"].as_array().unwrap().len(), 50);
    assert!(
        output["repositories"]
            .as_array()
            .unwrap()
            .iter()
            .all(|row| row["description_truncated"] == true)
    );
    assert_within_budget(&output);
}

#[test]
fn maximum_search_page_bounds_rows_fragments_and_serialized_output() {
    let fragment = "é".repeat(1_001);
    let items = Value::Array(
        (0..20)
            .map(|index| {
                json!({
                    "name": format!("file-{index}.rs"),
                    "path": format!("src/file-{index}.rs"),
                    "sha": format!("sha-{index}"),
                    "html_url": format!("https://github.com/octo/repo/blob/sha-{index}/src/file.rs"),
                    "repository": { "full_name": "octo/repo" },
                    "text_matches": (0..5)
                        .map(|_| json!({ "fragment": fragment }))
                        .collect::<Vec<_>>()
                })
            })
            .collect(),
    );
    let (output, _) = call(
        "github_search_code",
        json!({ "query": "rust", "per_page": 20 }),
        vec![response(
            200,
            json!({
                "total_count": 20,
                "incomplete_results": false,
                "items": items
            }),
        )],
    );
    assert_eq!(output["matches"].as_array().unwrap().len(), 20);
    assert!(
        output["matches"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|row| row["fragments"].as_array().unwrap())
            .all(|fragment| fragment["truncated"] == true)
    );
    assert_within_budget(&output);
}

#[test]
fn maximum_pull_request_and_issue_pages_set_deterministic_preview_flags() {
    let mut pull_request = pull_request_detail();
    pull_request["body"] = json!("é".repeat(32_769));
    let files = Value::Array(
        (0..10)
            .map(|index| {
                let mut file = pull_request_file();
                file["filename"] = json!(format!("src/file-{index}.rs"));
                file["previous_filename"] = json!(format!("old/file-{index}.rs"));
                file["patch"] = json!("é".repeat(4_097));
                file
            })
            .collect(),
    );
    let (pull_request, _) = call(
        "github_read_pr",
        json!({ "owner": "octo", "repository": "repo", "number": 7 }),
        vec![response(200, pull_request), response(200, files)],
    );
    assert_eq!(pull_request["body_truncated"], true);
    assert!(
        pull_request["files"]
            .as_array()
            .unwrap()
            .iter()
            .all(|file| file["patch_truncated"] == true)
    );
    assert_within_budget(&pull_request);

    let mut issue = issue_detail();
    issue["body"] = json!("é".repeat(32_769));
    let comments = Value::Array(
        (0..10)
            .map(|index| {
                let mut comment = issue_comment(index);
                comment["body"] = json!("é".repeat(4_097));
                comment
            })
            .collect(),
    );
    let (issue, _) = call(
        "github_read_issue",
        json!({ "owner": "octo", "repository": "repo", "number": 9 }),
        vec![response(200, issue), response(200, comments)],
    );
    assert_eq!(issue["body_truncated"], true);
    assert!(
        issue["comments"]
            .as_array()
            .unwrap()
            .iter()
            .all(|comment| comment["body_truncated"] == true)
    );
    assert_within_budget(&issue);
}

fn assert_within_budget(value: &Value) {
    assert!(serde_json::to_vec(value).unwrap().len() <= OUTPUT_BUDGET_BYTES);
}
