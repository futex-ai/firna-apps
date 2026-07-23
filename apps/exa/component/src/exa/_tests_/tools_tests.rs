use serde_json::json;

use super::normalize_input;
use crate::exa::types::ExaSearchInput;

#[test]
fn normalizes_defaults_and_camel_case_provider_body() {
    let input = serde_json::from_value::<ExaSearchInput>(json!({
        "query": "  rust agents  "
    }))
    .expect("input should decode");

    let (request, timeout_seconds) = normalize_input(input).expect("input should normalize");
    let body = request.into_value();

    assert_eq!(timeout_seconds, 60);
    assert_eq!(body["query"], "rust agents");
    assert_eq!(body["type"], "auto");
    assert_eq!(body["numResults"], 10);
    assert_eq!(body["contents"]["highlights"]["maxCharacters"], 4000);
}

#[test]
fn trims_optional_filters_and_propagates_timeout() {
    let input = serde_json::from_value::<ExaSearchInput>(json!({
        "query": "search",
        "search_type": "deep-lite",
        "num_results": 3,
        "include_domains": [" example.com ", ""],
        "exclude_domains": [" spam.example "],
        "start_published_date": " 2026-01-01 ",
        "end_published_date": " ",
        "livecrawl": true,
        "highlights_max_characters": 99,
        "text_max_characters": 123,
        "timeout_seconds": 42
    }))
    .expect("input should decode");

    let (request, timeout_seconds) = normalize_input(input).expect("input should normalize");
    let body = request.into_value();

    assert_eq!(timeout_seconds, 42);
    assert_eq!(body["type"], "deep-lite");
    assert_eq!(body["includeDomains"], json!(["example.com"]));
    assert_eq!(body["excludeDomains"], json!(["spam.example"]));
    assert_eq!(body["startPublishedDate"], "2026-01-01");
    assert!(body.get("endPublishedDate").is_none());
    assert_eq!(body["contents"]["maxAgeHours"], 0);
    assert_eq!(body["contents"]["text"]["maxCharacters"], 123);
}

#[test]
fn rejects_empty_query_and_out_of_range_values() {
    let empty_query = serde_json::from_value::<ExaSearchInput>(json!({
        "query": "  "
    }))
    .expect("input should decode");
    assert_eq!(
        normalize_input(empty_query).expect_err("empty query should fail"),
        "empty_query"
    );

    let invalid_results = serde_json::from_value::<ExaSearchInput>(json!({
        "query": "search",
        "num_results": 0
    }))
    .expect("input should decode");
    assert_eq!(
        normalize_input(invalid_results).expect_err("invalid result count should fail"),
        "invalid_num_results"
    );

    let invalid_timeout = serde_json::from_value::<ExaSearchInput>(json!({
        "query": "search",
        "timeout_seconds": 301
    }))
    .expect("input should decode");
    assert_eq!(
        normalize_input(invalid_timeout).expect_err("invalid timeout should fail"),
        "invalid_timeout_seconds"
    );
}
