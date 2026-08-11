use std::collections::BTreeMap;

use serde_json::json;
use unimock::Unimock;

use super::support::{
    assert_error, call_with_response, host_error, invoke, invoke_raw, response,
    response_with_headers,
};

#[test]
fn malformed_and_unknown_calls_return_stable_invalid_input() {
    let http = Unimock::new(());

    let malformed = invoke_raw(&http, String::from("not-json"));
    assert_error(&malformed, "invalid_request");
    assert_eq!(malformed["reason"], "malformed_tool_call");

    let unknown = invoke(&http, "x_unknown", json!({}));
    assert_error(&unknown, "invalid_request");
    assert_eq!(unknown["reason"], "unknown_tool");
}

#[test]
fn provider_statuses_map_without_exposing_provider_bodies() {
    let cases = [
        (401, "auth_required"),
        (403, "missing_scope"),
        (404, "not_found"),
        (503, "provider_unavailable"),
    ];
    for (status, code) in cases {
        let output = call_with_response(
            "x_search_recent_posts",
            json!({"query": "rust", "max_results": 10}),
            response(
                status,
                Some(json!({"detail": "provider-secret-detail", "token": "never-leak"})),
            ),
        );
        assert_error(&output, code);
        let encoded = output.to_string();
        assert!(!encoded.contains("provider-secret-detail"));
        assert!(!encoded.contains("never-leak"));
    }
}

#[test]
fn unknown_provider_4xx_responses_are_non_retryable_and_redacted() {
    let cases = [
        (
            "x_search_recent_posts",
            json!({"query": "invalid:query", "max_results": 10}),
            400,
        ),
        ("x_create_post", json!({"text": "Hello X"}), 422),
    ];
    for (tool, input, status) in cases {
        let output = call_with_response(
            tool,
            input,
            response(
                status,
                Some(json!({"detail": "provider-secret-detail", "token": "never-leak"})),
            ),
        );
        assert_error(&output, "invalid_request");
        assert_eq!(output["reason"], "provider_rejected_request");
        let encoded = output.to_string();
        assert!(!encoded.contains("provider-secret-detail"));
        assert!(!encoded.contains("never-leak"));
    }
}

#[test]
fn rate_limit_and_usage_cap_are_distinct_stable_errors() {
    let limited = call_with_response(
        "x_search_recent_posts",
        json!({"query": "rust", "max_results": 10}),
        response_with_headers(
            429,
            BTreeMap::from([(String::from("Retry-After"), String::from("45"))]),
            Some(json!({"type": "https://api.x.com/2/problems/rate-limit-exceeded"})),
        ),
    );
    assert_error(&limited, "rate_limited");
    assert_eq!(limited["retry_after_seconds"], 45);

    let capped = call_with_response(
        "x_search_recent_posts",
        json!({"query": "rust", "max_results": 10}),
        response(
            429,
            Some(json!({"type": "https://api.x.com/2/problems/usage-capped"})),
        ),
    );
    assert_error(&capped, "provider_budget_exhausted");
}

#[test]
fn malformed_truncated_and_transport_read_responses_are_stable() {
    let malformed = call_with_response(
        "x_search_recent_posts",
        json!({"query": "rust", "max_results": 10}),
        response(200, Some(json!({"data": "not-an-array"}))),
    );
    assert_error(&malformed, "provider_contract_error");

    let mut truncated_response = response(200, Some(json!({"data": []})));
    truncated_response.body_truncated = true;
    let truncated = call_with_response(
        "x_search_recent_posts",
        json!({"query": "rust", "max_results": 10}),
        truncated_response,
    );
    assert_error(&truncated, "provider_contract_error");

    let unavailable = call_with_response(
        "x_search_recent_posts",
        json!({"query": "rust", "max_results": 10}),
        host_error("provider_transport_failed"),
    );
    assert_error(&unavailable, "provider_unavailable");
}

#[test]
fn provider_collections_cannot_exceed_the_requested_billing_bound() {
    let posts = (1..=11)
        .map(|id| json!({"id": id.to_string(), "text": "bounded"}))
        .collect::<Vec<_>>();
    let output = call_with_response(
        "x_search_recent_posts",
        json!({"query": "rust", "max_results": 10}),
        response(200, Some(json!({"data": posts}))),
    );

    assert_error(&output, "provider_contract_error");
}

#[test]
fn missing_or_terminal_credentials_require_reauthorization() {
    for error in [
        "credential_not_found",
        "credential_unavailable",
        "auth_required",
    ] {
        let output = call_with_response(
            "x_search_recent_posts",
            json!({"query": "rust", "max_results": 10}),
            host_error(error),
        );
        assert_error(&output, "auth_required");
    }
}
