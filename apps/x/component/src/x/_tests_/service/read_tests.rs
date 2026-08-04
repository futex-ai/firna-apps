use serde_json::json;
use unimock::Unimock;

use super::support::{
    assert_error, assert_read_usage, capturing_http, invoke, response, success_output,
};

#[test]
fn get_posts_sends_bounded_lookup_and_returns_compact_partial_result() {
    let (http, requests) = capturing_http(response(
        200,
        Some(json!({
            "data": [{
                "id": "11",
                "text": "First",
                "author_id": "7",
                "created_at": "2026-08-01T09:00:00Z",
                "public_metrics": {"like_count": 9000}
            }],
            "includes": {
                "users": [{"id": "7", "name": "Ada", "username": "ada"}]
            }
        })),
    ));

    let output = invoke(
        &http,
        "x_get_posts",
        json!({"ids": ["11", "22"], "include_authors": true}),
    );

    let result = success_output(&output);
    assert_eq!(result["posts"][0]["id"], "11");
    assert_eq!(result["authors"][0]["username"], "ada");
    assert_eq!(result["missing_ids"], json!(["22"]));
    assert_eq!(result["result_count"], 1);
    assert!(result["posts"][0].get("public_metrics").is_none());
    assert_read_usage(&output, 1, 1);
    let requests = requests.lock().expect("request capture lock");
    let request = &requests[0];
    assert_eq!(request.method, "GET");
    assert_eq!(request.url, "https://api.x.com/2/tweets");
    assert_eq!(request.query["ids"], "11,22");
    assert_eq!(request.query["tweet.fields"], "author_id,created_at,text");
    assert_eq!(request.query["expansions"], "author_id");
    assert_eq!(request.query["user.fields"], "id,name,username");
    assert_eq!(request.credential.credential_kind, "access_token");
    assert_eq!(request.credential.installation_id, "018f-installation");
    assert_eq!(request.credential_injection.kind, "bearer_authorization");
    assert_eq!(request.response_body_limit_bytes, 262_144);
    assert_eq!(request.timeout_seconds, 30);
}

#[test]
fn get_posts_omits_author_expansion_and_optional_null_fields() {
    let (http, requests) = capturing_http(response(
        200,
        Some(json!({
            "data": [{"id": "11", "text": "First"}],
            "includes": {"users": [{"id": "7", "name": "Ada", "username": "ada"}]}
        })),
    ));

    let output = invoke(&http, "x_get_posts", json!({"ids": ["11"]}));

    let result = success_output(&output);
    assert!(result.get("authors").is_none());
    assert!(result.get("missing_ids").is_none());
    assert!(result["posts"][0].get("author_id").is_none());
    assert_read_usage(&output, 1, 0);
    let requests = requests.lock().expect("request capture lock");
    assert!(!requests[0].query.contains_key("expansions"));
    assert!(!requests[0].query.contains_key("user.fields"));
}

#[test]
fn recent_search_sends_one_explicit_page_and_returns_next_token() {
    let (http, requests) = capturing_http(response(
        200,
        Some(json!({
            "data": [{"id": "33", "text": "Result"}],
            "meta": {"result_count": 1, "next_token": "next-page"}
        })),
    ));

    let output = invoke(
        &http,
        "x_search_recent_posts",
        json!({
            "query": "  rust lang:en  ",
            "max_results": 10,
            "next_token": " current-page "
        }),
    );

    let result = success_output(&output);
    assert_eq!(result["posts"][0]["id"], "33");
    assert_eq!(result["next_token"], "next-page");
    assert_eq!(result["result_count"], 1);
    assert_read_usage(&output, 1, 0);
    let requests = requests.lock().expect("request capture lock");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].url, "https://api.x.com/2/tweets/search/recent");
    assert_eq!(requests[0].query["query"], "rust lang:en");
    assert_eq!(requests[0].query["max_results"], "10");
    assert_eq!(requests[0].query["next_token"], "current-page");
}

#[test]
fn read_validation_rejects_duplicate_ids_and_invalid_search_bounds() {
    let http = Unimock::new(());

    let duplicate = invoke(&http, "x_get_posts", json!({"ids": ["11", "11"]}));
    assert_error(&duplicate, "invalid_request");

    let short_page = invoke(
        &http,
        "x_search_recent_posts",
        json!({"query": "rust", "max_results": 9}),
    );
    assert_error(&short_page, "invalid_request");

    let blank_token = invoke(
        &http,
        "x_search_recent_posts",
        json!({"query": "rust", "max_results": 10, "next_token": "  "}),
    );
    assert_error(&blank_token, "invalid_request");
}

#[test]
fn empty_recent_search_reports_zero_billable_resources() {
    let output = call_with_empty_search();

    assert_eq!(success_output(&output)["result_count"], 0);
    assert_read_usage(&output, 0, 0);
}

fn call_with_empty_search() -> serde_json::Value {
    let (http, _) = capturing_http(response(200, Some(json!({"data": []}))));
    invoke(
        &http,
        "x_search_recent_posts",
        json!({"query": "nothing", "max_results": 10}),
    )
}
