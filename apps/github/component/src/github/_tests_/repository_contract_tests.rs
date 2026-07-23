use serde_json::{Value, json};

use super::support::{call, installation_repositories, repository, response};

#[test]
fn applies_search_defaults_and_projects_absent_text_matches() {
    let (output, requests) = call(
        "github_search_code",
        json!({ "query": "rust" }),
        vec![response(
            200,
            json!({
                "total_count": 1,
                "incomplete_results": true,
                "items": [{
                    "name": "lib.rs",
                    "path": "src/lib.rs",
                    "sha": "abc",
                    "html_url": "https://github.com/octo/repo/blob/abc/src/lib.rs",
                    "repository": { "full_name": "octo/repo" }
                }]
            }),
        )],
    );
    assert_eq!(output["page"], 1);
    assert_eq!(output["incomplete_results"], true);
    assert_eq!(output["matches"][0]["fragments"], json!([]));
    assert_eq!(requests[0].query["page"], "1");
    assert_eq!(requests[0].query["per_page"], "20");
}

#[test]
fn treats_search_syntax_globs_quotes_and_backslashes_as_one_literal_query() {
    let (_, requests) = call(
        "github_search_code",
        json!({
            "query": "a\"b\\c OR path:*.rs",
            "owner": "octo",
            "repository": "repo.name",
            "language": "C++",
            "path": "src/[ab]*.rs"
        }),
        vec![response(
            200,
            json!({ "total_count": 0, "incomplete_results": false, "items": [] }),
        )],
    );
    assert_eq!(
        requests[0].query["q"],
        r#""a\"b\\c OR path:*.rs" repo:"octo/repo.name" language:"C++" path:"src/[ab]*.rs""#
    );
    assert!(!requests[0].query["q"].contains("%22"));
}

#[test]
fn honors_repository_page_boundaries() {
    let (_, requests) = call(
        "github_list_repositories",
        json!({
            "page": 2147483647_i64,
            "per_page": 50
        }),
        vec![response(200, installation_repositories(vec![]))],
    );
    let query = &requests[0].query;
    assert_eq!(query["page"], "2147483647");
    assert_eq!(query["per_page"], "50");
}

#[test]
fn rejects_repository_and_search_provider_cardinality_overruns() {
    let repositories = installation_repositories((0..2).map(|_| repository()).collect());
    let (output, _) = call(
        "github_list_repositories",
        json!({ "per_page": 1 }),
        vec![response(200, repositories)],
    );
    assert_eq!(output["error"], "invalid_provider_response");

    let items = Value::Array((0..2).map(|_| search_item(0)).collect());
    let (output, _) = call(
        "github_search_code",
        json!({ "query": "rust", "per_page": 1 }),
        vec![response(
            200,
            json!({ "total_count": 2, "incomplete_results": false, "items": items }),
        )],
    );
    assert_eq!(output["error"], "invalid_provider_response");

    let (output, _) = call(
        "github_search_code",
        json!({ "query": "rust" }),
        vec![response(
            200,
            json!({
                "total_count": 1,
                "incomplete_results": false,
                "items": [search_item(6)]
            }),
        )],
    );
    assert_eq!(output["error"], "invalid_provider_response");
}

#[test]
fn fails_closed_for_missing_null_and_wrong_typed_required_list_fields() {
    let mut missing = repository();
    missing.as_object_mut().unwrap().remove("full_name");
    let mut null = repository();
    null["archived"] = Value::Null;
    let mut wrong = repository();
    wrong["id"] = json!("one");
    for row in [missing, null, wrong] {
        let (output, _) = call(
            "github_list_repositories",
            json!({}),
            vec![response(200, installation_repositories(vec![row]))],
        );
        assert_eq!(output["error"], "invalid_provider_response");
    }

    for body in [
        json!({ "incomplete_results": false, "items": [] }),
        json!({ "total_count": 0, "incomplete_results": null, "items": [] }),
        json!({ "total_count": 0, "incomplete_results": false, "items": {} }),
    ] {
        let (output, _) = call(
            "github_search_code",
            json!({ "query": "rust" }),
            vec![response(200, body)],
        );
        assert_eq!(output["error"], "invalid_provider_response");
    }
}

fn search_item(fragment_count: usize) -> Value {
    json!({
        "name": "lib.rs",
        "path": "src/lib.rs",
        "sha": "abc",
        "html_url": "https://github.com/octo/repo/blob/abc/src/lib.rs",
        "repository": { "full_name": "octo/repo" },
        "text_matches": (0..fragment_count)
            .map(|_| json!({ "fragment": "match" }))
            .collect::<Vec<_>>()
    })
}
