use std::collections::BTreeMap;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use serde_json::json;

use crate::github::provider::ProviderMediaType;

use super::support::{
    FILE_COMMIT_SHA, call, file_responses, installation_repositories, repository, response,
    response_with_headers,
};

#[test]
fn lists_repositories_with_canonical_defaults_and_nullable_fields() {
    let headers = BTreeMap::from([(
        String::from("link"),
        String::from("<https://api.github.com/installation/repositories?page=2>; rel=\"next\""),
    )]);
    let (output, requests) = call(
        "github_list_repositories",
        json!({}),
        vec![response_with_headers(
            200,
            headers,
            installation_repositories(vec![repository()]),
        )],
    );

    assert_eq!(output["page"], 1);
    assert_eq!(output["next_page"], 2);
    assert_eq!(output["repositories"][0]["description"], json!(null));
    assert_eq!(output["repositories"][0]["description_truncated"], false);
    let request = &requests[0];
    assert_eq!(request.path, "/installation/repositories");
    assert_eq!(request.query["page"], "1");
    assert_eq!(request.query["per_page"], "30");
    assert_eq!(request.media_type, ProviderMediaType::Json);
}

#[test]
fn accepts_installation_repository_pagination() {
    let (_, requests) = call(
        "github_list_repositories",
        json!({ "page": 7, "per_page": 1 }),
        vec![response(200, installation_repositories(vec![]))],
    );
    assert_eq!(requests[0].query["page"], "7");
    assert_eq!(requests[0].query["per_page"], "1");
}

#[test]
fn builds_fixed_order_literal_search_query_once() {
    let headers = BTreeMap::from([(
        String::from("Link"),
        String::from("<https://api.github.com/search/code?page=2>; rel=\"next\""),
    )]);
    let body = json!({
        "total_count": 1,
        "incomplete_results": false,
        "items": [{
            "name": "lib.rs",
            "path": "src/lib.rs",
            "sha": "abc",
            "html_url": "https://github.com/octo/repo/blob/abc/src/lib.rs",
            "repository": { "full_name": "octo/repo" },
            "text_matches": [{ "fragment": "fn main() {}" }]
        }]
    });
    let (output, requests) = call(
        "github_search_code",
        json!({
            "query": "repo:other \\\"literal",
            "owner": "octo",
            "repository": "repo",
            "language": "Rust",
            "path": "src/*.rs"
        }),
        vec![response_with_headers(200, headers, body)],
    );

    assert_eq!(output["matches"][0]["fragments"][0]["truncated"], false);
    assert_eq!(output["next_page"], 2);
    assert_eq!(
        requests[0].query["q"],
        r#""repo:other \\\"literal" repo:"octo/repo" language:"Rust" path:"src/*.rs""#
    );
    assert_eq!(requests[0].media_type, ProviderMediaType::TextMatch);
}

#[test]
fn enforces_search_pairing_and_finite_window() {
    let (pairing, requests) = call(
        "github_search_code",
        json!({ "query": "rust", "repository": "repo" }),
        vec![],
    );
    assert_eq!(pairing["reason"], "repository_qualifier_requires_owner");
    assert!(requests.is_empty());

    let (window, requests) = call(
        "github_search_code",
        json!({ "query": "rust", "page": 51, "per_page": 20 }),
        vec![],
    );
    assert_eq!(window["reason"], "result_window_exceeded");
    assert!(requests.is_empty());
}

#[test]
fn suppresses_search_next_page_outside_provider_window() {
    let headers = BTreeMap::from([(
        String::from("link"),
        String::from("<https://api.github.com/search/code?page=51>; rel=\"next\""),
    )]);
    let (output, _) = call(
        "github_search_code",
        json!({ "query": "rust", "page": 50, "per_page": 20 }),
        vec![response_with_headers(
            200,
            headers,
            json!({ "total_count": 5000, "incomplete_results": false, "items": [] }),
        )],
    );
    assert!(output.get("next_page").is_none());
}

#[test]
fn reads_exact_text_file_and_rejects_missing_required_provider_fields() {
    let encoded = STANDARD.encode("hello\n");
    let (output, requests) = call(
        "github_read_file",
        json!({ "owner": "octo", "repository": "repo", "path": "docs/read me.md", "ref": "main" }),
        file_responses(
            "docs/read me.md",
            json!({
                "type": "file",
                "path": "docs/read me.md",
                "sha": "abc",
                "size": 6,
                "html_url": null,
                "encoding": "base64",
                "content": encoded
            }),
        ),
    );
    assert_eq!(output["content"], "hello\n");
    assert_eq!(output["ref"], "main");
    assert_eq!(requests[0].path, "/repos/octo/repo/commits");
    assert_eq!(requests[0].query["sha"], "main");
    assert_eq!(
        requests[3].path,
        "/repos/octo/repo/contents/docs/read%20me%2Emd"
    );
    assert_eq!(requests[3].query["ref"], FILE_COMMIT_SHA);

    let mut incomplete = repository();
    incomplete
        .as_object_mut()
        .expect("fixture should be an object")
        .remove("description");
    let (error, _) = call(
        "github_list_repositories",
        json!({}),
        vec![response(200, installation_repositories(vec![incomplete]))],
    );
    assert_eq!(error["error"], "invalid_provider_response");
}

#[test]
fn rejects_structurally_invalid_https_provider_urls() {
    let mut invalid = repository();
    invalid["html_url"] = json!("https://?query");
    let (error, _) = call(
        "github_list_repositories",
        json!({}),
        vec![response(200, installation_repositories(vec![invalid]))],
    );
    assert_eq!(error["error"], "invalid_provider_response");
}
