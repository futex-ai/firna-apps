use serde_json::json;
use unimock::Unimock;

use super::support::{
    assert_create_cost, assert_error, capturing_http, invoke, response, success_output,
};

#[test]
fn create_post_sends_one_request_without_operation_id() {
    let (http, requests) = capturing_http(response(
        201,
        Some(json!({"data": {"id": "44", "text": "Hello X"}})),
    ));

    let output = invoke(
        &http,
        "x_create_post",
        json!({"text": "Hello X", "allow_link": false}),
    );

    assert_eq!(
        success_output(&output),
        &json!({"post": {"id": "44", "text": "Hello X"}})
    );
    assert_create_cost(&output, 15_000);
    let requests = requests.lock().expect("request capture lock");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, "POST");
    assert_eq!(requests[0].url, "https://api.x.com/2/tweets");
    assert_eq!(requests[0].headers["content-type"], "application/json");
    let body = requests[0].body_json.as_ref().expect("create body");
    assert_eq!(body, &json!({"text": "Hello X"}));
    assert!(!body.to_string().contains("durable-operation-id"));
}

#[test]
fn create_reply_uses_only_the_documented_reply_shape() {
    let (http, requests) = capturing_http(response(
        201,
        Some(json!({"data": {"id": "45", "text": "Reply"}})),
    ));

    let output = invoke(
        &http,
        "x_create_post",
        json!({"text": "Reply", "reply_to_post_id": "44"}),
    );

    assert_eq!(success_output(&output)["post"]["id"], "45");
    assert_create_cost(&output, 15_000);
    let requests = requests.lock().expect("request capture lock");
    assert_eq!(
        requests[0].body_json,
        Some(json!({
            "text": "Reply",
            "reply": {"in_reply_to_tweet_id": "44"}
        }))
    );
}

#[test]
fn link_posts_require_explicit_cost_acknowledgement() {
    let http = Unimock::new(());

    let rejected = invoke(
        &http,
        "x_create_post",
        json!({"text": "See HTTPS://example.com"}),
    );

    assert_error(&rejected, "invalid_request");
    assert_eq!(rejected["reason"], "link_acknowledgement_required");
}

#[test]
fn successful_link_post_reports_the_capped_link_rate() {
    let (http, _) = capturing_http(response(
        201,
        Some(json!({"data": {"id": "46", "text": "See https://example.com"}})),
    ));

    let output = invoke(
        &http,
        "x_create_post",
        json!({"text": "See HTTPS://example.com", "allow_link": true}),
    );

    assert_create_cost(&output, 200_000);
}

#[test]
fn ambiguous_create_transport_failure_is_not_retried() {
    let (http, requests) = capturing_http(crate::x::host::HostHttpResponse {
        error: Some(String::from("provider_transport_failed")),
        ..crate::x::host::HostHttpResponse::default()
    });

    let output = invoke(&http, "x_create_post", json!({"text": "Hello X"}));

    assert_error(&output, "write_outcome_unknown");
    assert_eq!(requests.lock().expect("request capture lock").len(), 1);
}

#[test]
fn ambiguous_create_provider_results_fail_closed() {
    let mut truncated = response(201, Some(json!({"data": {"id": "44", "text": "Hello X"}})));
    truncated.body_truncated = true;
    let cases = [
        response(201, Some(json!({"data": "not-a-post"}))),
        truncated,
        crate::x::host::HostHttpResponse {
            ok: true,
            status: None,
            body_json: Some(json!({"data": {"id": "44", "text": "Hello X"}})),
            ..crate::x::host::HostHttpResponse::default()
        },
        response(503, Some(json!({"title": "temporarily unavailable"}))),
    ];

    for provider_response in cases {
        let (http, requests) = capturing_http(provider_response);
        let output = invoke(&http, "x_create_post", json!({"text": "Hello X"}));

        assert_error(&output, "write_outcome_unknown");
        assert_eq!(requests.lock().expect("request capture lock").len(), 1);
    }
}

#[test]
fn create_validation_rejects_invalid_text_and_reply_before_dispatch() {
    let http = Unimock::new(());

    assert_error(
        &invoke(&http, "x_create_post", json!({"text": "  "})),
        "invalid_request",
    );
    assert_error(
        &invoke(
            &http,
            "x_create_post",
            json!({"text": "Reply", "reply_to_post_id": "not-an-id"}),
        ),
        "invalid_request",
    );
}
