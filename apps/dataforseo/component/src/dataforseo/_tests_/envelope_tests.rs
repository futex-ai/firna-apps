use std::collections::BTreeMap;

use serde_json::json;

use super::super::envelope::decode;
use super::super::error::Error;
use super::super::host::HostHttpResponse;

#[test]
fn successful_envelope_preserves_only_safe_metadata_and_results() {
    let response = response(
        200,
        json!({
            "status_code": 20000,
            "status_message": "Ok.",
            "tasks": [{
                "id": "task-1",
                "status_code": 20000,
                "cost": 0.004,
                "result": [{"items": [{"keyword": "rust"}]}],
                "data": {"login": "must-not-survive"}
            }]
        }),
    );

    let result = decode(response).unwrap();

    assert_eq!(result.task_id.as_deref(), Some("task-1"));
    assert_eq!(result.cost_usd, Some(0.004));
    assert_eq!(result.rate_limit.limit_per_minute, Some(2_000));
    assert_eq!(result.rate_limit.remaining, Some(1_999));
    assert_eq!(
        result.results,
        vec![json!({"items": [{"keyword": "rust"}]})]
    );
}

#[test]
fn no_results_codes_are_empty_successes() {
    let general = decode(response(200, json!({"status_code": 40102, "cost": 0.0}))).unwrap();
    assert!(general.results.is_empty());

    let task = decode(response(
        200,
        json!({
            "status_code": 20000,
            "tasks": [{"status_code": 40102, "result": []}]
        }),
    ))
    .unwrap();
    assert!(task.results.is_empty());
}

#[test]
fn rate_limit_header_spellings_are_decoded() {
    for prefix in ["ratelimit", "x-ratelimit", "x-rate-limit"] {
        let mut response = response(
            200,
            json!({
                "status_code": 20000,
                "tasks": [{"status_code": 20000, "result": []}]
            }),
        );
        response.headers = BTreeMap::from([
            (format!("{prefix}-limit"), String::from("2000")),
            (format!("{prefix}-remaining"), String::from("1999")),
        ]);

        let result = decode(response).unwrap();

        assert_eq!(result.rate_limit.limit_per_minute, Some(2_000));
        assert_eq!(result.rate_limit.remaining, Some(1_999));
    }
}

#[test]
fn response_header_names_are_case_insensitive() {
    let mut success = response(
        200,
        json!({
            "status_code": 20000,
            "tasks": [{"status_code": 20000, "result": []}]
        }),
    );
    success.headers = BTreeMap::from([
        (String::from("X-RateLimit-Limit"), String::from("2000")),
        (String::from("x-RaTe-LiMiT-Remaining"), String::from("1999")),
    ]);

    let result = decode(success).unwrap();

    assert_eq!(result.rate_limit.limit_per_minute, Some(2_000));
    assert_eq!(result.rate_limit.remaining, Some(1_999));

    let mut limited = response(429, json!({"status_code": 20000}));
    limited.headers = BTreeMap::from([(String::from("Retry-After"), String::from("12"))]);

    assert!(matches!(
        decode(limited),
        Err(Error::RateLimited {
            retry_after_seconds: Some(12),
            ..
        })
    ));
}

#[test]
fn status_matrix_is_stable_and_redacted() {
    let cases = [
        (401, 20000, "authentication"),
        (403, 20000, "access"),
        (402, 20000, "budget"),
        (404, 20000, "contract"),
        (429, 20000, "rate"),
        (200, 40100, "authentication"),
        (200, 40201, "access"),
        (200, 40200, "budget"),
        (200, 40202, "rate"),
        (200, 40000, "invalid"),
        (503, 20000, "unavailable"),
        (200, 59999, "unavailable"),
    ];

    for (http_status, provider_code, expected) in cases {
        let error = decode(response(
            http_status,
            json!({"status_code": provider_code, "tasks": []}),
        ))
        .unwrap_err();
        assert_error_kind(error, expected);
    }
}

#[test]
fn malformed_and_truncated_responses_fail_closed() {
    let mut truncated = response(200, json!({"status_code": 20000, "tasks": []}));
    truncated.body_truncated = true;
    assert!(matches!(
        decode(truncated),
        Err(Error::ProviderResponseTooLarge)
    ));

    let malformed = HostHttpResponse {
        ok: true,
        status: Some(200),
        headers: BTreeMap::new(),
        body_json: None,
        body_truncated: false,
    };
    assert!(matches!(
        decode(malformed),
        Err(Error::ProviderUnavailable(None))
    ));
}

fn response(status: u16, body: serde_json::Value) -> HostHttpResponse {
    HostHttpResponse {
        ok: true,
        status: Some(status),
        headers: BTreeMap::from([
            (String::from("x-ratelimit-limit"), String::from("2000")),
            (String::from("x-ratelimit-remaining"), String::from("1999")),
            (
                String::from("authorization"),
                String::from("must-not-surface"),
            ),
        ]),
        body_json: Some(body),
        body_truncated: false,
    }
}

fn assert_error_kind(error: Error, expected: &str) {
    let matches = match expected {
        "authentication" => matches!(error, Error::ProviderAuthenticationFailed(_)),
        "access" => matches!(error, Error::ProviderAccessDenied(_)),
        "budget" => matches!(error, Error::ProviderBudgetExhausted(_)),
        "contract" => matches!(error, Error::ProviderContract),
        "rate" => matches!(error, Error::RateLimited { .. }),
        "invalid" => matches!(error, Error::InvalidRequest(_)),
        "unavailable" => matches!(error, Error::ProviderUnavailable(_)),
        _ => false,
    };
    assert!(matches, "unexpected error {error:?} for {expected}");
}
