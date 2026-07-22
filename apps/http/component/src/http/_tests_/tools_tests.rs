use std::collections::BTreeMap;

use serde_json::{Value, json};

#[test]
fn normalizes_defaults() {
    let input = serde_json::from_value(json!({
        "url": "https://example.com"
    }))
    .unwrap();

    let (request, fallback_url) = super::normalize_input(input).unwrap();

    assert_eq!(request.method, "GET");
    assert_eq!(request.url, "https://example.com");
    assert_eq!(fallback_url, "https://example.com");
    assert_eq!(request.timeout_seconds, Some(60));
    assert_eq!(request.query, BTreeMap::new());
    assert_eq!(request.body_json, None);
    assert_eq!(request.body_text, None);
    assert_eq!(request.credential, None);
    assert_eq!(request.credential_injection, None);
}

#[test]
fn normalizes_query_headers_json_body_and_timeout() {
    let input = serde_json::from_value(json!({
        "url": "https://example.com/api",
        "method": "post",
        "query": {"q": "rust"},
        "headers": {"x-test": "true"},
        "body_json": {"ok": true},
        "timeout_seconds": 42
    }))
    .unwrap();

    let (request, _) = super::normalize_input(input).unwrap();

    assert_eq!(request.method, "POST");
    assert_eq!(request.query.get("q"), Some(&String::from("rust")));
    assert_eq!(request.headers.get("x-test"), Some(&String::from("true")));
    assert_eq!(request.body_json, Some(json!({"ok": true})));
    assert_eq!(request.body_text, None);
    assert_eq!(request.timeout_seconds, Some(42));
}

#[test]
fn normalizes_text_body() {
    let input = serde_json::from_value(json!({
        "url": "https://example.com",
        "method": "PUT",
        "body_text": "hello"
    }))
    .unwrap();

    let (request, _) = super::normalize_input(input).unwrap();

    assert_eq!(request.method, "PUT");
    assert_eq!(request.body_json, None);
    assert_eq!(request.body_text, Some(String::from("hello")));
}

#[test]
fn rejects_invalid_method() {
    let input = serde_json::from_value(json!({
        "url": "https://example.com",
        "method": "TRACE"
    }))
    .unwrap();

    assert_eq!(super::normalize_input(input).unwrap_err(), "invalid_method");
}

#[test]
fn rejects_multiple_body_fields() {
    let input = serde_json::from_value(json!({
        "url": "https://example.com",
        "body_json": {"a": true},
        "body_text": "hello"
    }))
    .unwrap();

    assert_eq!(
        super::normalize_input(input).unwrap_err(),
        "multiple_body_fields"
    );
}

#[test]
fn rejects_invalid_timeout() {
    let input = serde_json::from_value(json!({
        "url": "https://example.com",
        "timeout_seconds": 301
    }))
    .unwrap();

    assert_eq!(
        super::normalize_input(input).unwrap_err(),
        "invalid_timeout_seconds"
    );
}

#[test]
fn normalizes_success_and_truncated_responses() {
    let output = super::normalize_response(
        crate::http::host::HostHttpResponse {
            ok: true,
            status: Some(206),
            url: Some(String::from("https://example.com/final")),
            headers: BTreeMap::from([(String::from("content-type"), String::from("text/plain"))]),
            content_type: Some(String::from("text/plain")),
            body_json: Some(json!("body...")),
            body_truncated: true,
            error: None,
        },
        "https://example.com",
    );

    assert_eq!(output["status"], 206);
    assert_eq!(output["ok"], true);
    assert_eq!(output["url"], "https://example.com/final");
    assert_eq!(output["content_type"], "text/plain");
    assert_eq!(output["body"], "body...");
    assert_eq!(output["body_truncated"], true);
}

#[test]
fn call_tool_reports_invalid_request_without_host_call() {
    let output = super::call_tool(
        &json!({
            "tool_name": "http_request",
            "input": {"url": "https://example.com", "method": "TRACE"}
        })
        .to_string(),
    );

    assert_eq!(decode(&output)["reason"], "invalid_method");
}

#[test]
fn host_validation_errors_report_invalid_request() {
    for code in [
        "invalid_host_http_request",
        "invalid_url",
        "host_http_scheme_denied",
        "host_http_https_required",
        "host_http_host_denied",
        "host_http_capability_denied",
        "host_http_credentials_denied",
        "multiple_body_fields",
        "invalid_method",
        "invalid_timeout_seconds",
        "credential_scope_mismatch",
        "credential_required",
        "credential_header_reserved",
        "credential_header_required",
        "invalid_credential_header",
        "credential_header_denied",
        "credential_header_conflict",
    ] {
        let output = crate::http::host::host_error(code);

        assert_eq!(output["error"], "invalid_request", "{code}");
        assert_eq!(output["reason"], code, "{code}");
    }
}

#[test]
fn host_transport_errors_report_provider_unavailable() {
    let output = crate::http::host::host_error("invalid_host_http_response");

    assert_eq!(output["error"], "provider_unavailable");
}

fn decode(output: &str) -> Value {
    serde_json::from_str(output).unwrap()
}
