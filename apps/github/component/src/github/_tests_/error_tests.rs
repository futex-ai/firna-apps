//! Stable GitHub provider and component error tests.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{Value, json};
use unimock::{MockFn as _, Unimock, matching};

use crate::github::call_tool_with;
use crate::github::error::GitHubError;
use crate::github::provider::{Clock, GitHubProviderGet};

use super::support::{FixedClock, call, call_with_clock, response, response_with_headers};

struct AdvancingClock(AtomicU64);

impl Clock for AdvancingClock {
    fn now_unix_seconds(&self) -> u64 {
        self.0.fetch_add(1, Ordering::Relaxed)
    }
}

#[test]
fn dispatches_with_installation_scope_without_effective_user() {
    let provider = Unimock::new(
        GitHubProviderGet
            .next_call(matching!(_))
            .returns(Ok(response(
                200,
                json!({ "total_count": 0, "repositories": [] }),
            ))),
    );
    let output: Value = serde_json::from_str(&call_tool_with(
        &json!({
            "installation_id": "install",
            "tool_name": "github_list_repositories",
            "input": {}
        })
        .to_string(),
        &provider,
        &FixedClock(1_000),
    ))
    .expect("output should decode");
    assert_eq!(output["repositories"], json!([]));
}

#[test]
fn maps_provider_statuses_without_leaking_bodies() {
    let cases = [
        (401, "auth_required"),
        (403, "provider_access_denied"),
        (404, "invalid_request"),
        (409, "invalid_request"),
        (422, "invalid_request"),
        (500, "provider_unavailable"),
    ];
    for (status, expected) in cases {
        let (output, _) = call(
            "github_list_repositories",
            json!({}),
            vec![response(
                status,
                json!({ "message": "private-secret-body" }),
            )],
        );
        assert_eq!(output["error"], expected);
        if status == 404 {
            assert_eq!(output["reason"], "not_found_or_not_accessible");
        }
        assert!(!output.to_string().contains("private-secret-body"));
    }
}

#[test]
fn maps_primary_and_secondary_rate_limits_with_bounded_timing() {
    let primary = BTreeMap::from([
        (String::from("x-ratelimit-remaining"), String::from("0")),
        (String::from("x-ratelimit-reset"), String::from("1100")),
    ]);
    let (output, _) = call(
        "github_list_repositories",
        json!({}),
        vec![response_with_headers(
            403,
            primary,
            json!({ "message": "denied" }),
        )],
    );
    assert_eq!(output["retry_after_seconds"], 100);

    let precedence = BTreeMap::from([
        (String::from("retry-after"), String::from("999999")),
        (String::from("x-ratelimit-remaining"), String::from("0")),
        (String::from("x-ratelimit-reset"), String::from("1100")),
    ]);
    let (output, _) = call(
        "github_list_repositories",
        json!({}),
        vec![response_with_headers(429, precedence, json!({}))],
    );
    assert_eq!(output["retry_after_seconds"], 86_400);

    let (output, _) = call(
        "github_list_repositories",
        json!({}),
        vec![response(
            403,
            json!({ "message": "You have exceeded a secondary rate limit." }),
        )],
    );
    assert_eq!(output["retry_after_seconds"], 60);
}

#[test]
fn samples_the_clock_once_for_primary_rate_limit_reset_math() {
    let headers = BTreeMap::from([
        (String::from("x-ratelimit-remaining"), String::from("0")),
        (String::from("x-ratelimit-reset"), String::from("1100")),
    ]);
    let clock = AdvancingClock(AtomicU64::new(1_000));
    let (output, _) = call_with_clock(
        "github_list_repositories",
        json!({}),
        vec![response_with_headers(403, headers, json!({}))],
        &clock,
    );
    assert_eq!(output["retry_after_seconds"], 100);
    assert_eq!(clock.0.load(Ordering::Relaxed), 1_001);
}

#[test]
fn ignores_malformed_or_past_rate_limit_headers() {
    let malformed = BTreeMap::from([
        (String::from("x-ratelimit-remaining"), String::from("0")),
        (String::from("x-ratelimit-reset"), String::from("999")),
        (String::from("retry-after"), String::from("1.5")),
    ]);
    let (output, _) = call(
        "github_list_repositories",
        json!({}),
        vec![response_with_headers(403, malformed, json!({}))],
    );
    assert_eq!(output["error"], "rate_limited");
    assert_eq!(output["retry_after_seconds"], json!(null));
}

#[test]
fn covers_secondary_and_429_retry_metadata_precedence() {
    let secondary_with_retry = BTreeMap::from([(String::from("retry-after"), String::from("30"))]);
    let (output, _) = call(
        "github_list_repositories",
        json!({}),
        vec![response_with_headers(
            403,
            secondary_with_retry,
            json!({ "message": "Secondary rate limit" }),
        )],
    );
    assert_eq!(output["retry_after_seconds"], 30);

    let excessive_reset = BTreeMap::from([
        (String::from("x-ratelimit-remaining"), String::from("0")),
        (String::from("x-ratelimit-reset"), String::from("999999")),
    ]);
    let (output, _) = call(
        "github_list_repositories",
        json!({}),
        vec![response_with_headers(
            403,
            excessive_reset,
            json!({ "message": "Secondary rate limit" }),
        )],
    );
    assert_eq!(output["retry_after_seconds"], 86_400);

    let (output, _) = call(
        "github_list_repositories",
        json!({}),
        vec![response(429, json!({ "message": "private" }))],
    );
    assert_eq!(output["error"], "rate_limited");
    assert_eq!(output["retry_after_seconds"], Value::Null);
    assert!(!output.to_string().contains("private"));
}

#[test]
fn keeps_unknown_structured_forbidden_responses_as_access_denied() {
    for body in [
        json!({ "message": "permission denied" }),
        json!({ "message": "This was not a secondary rate limit" }),
        json!({ "message": 7 }),
        json!({ "documentation_url": "private" }),
    ] {
        let (output, _) = call(
            "github_list_repositories",
            json!({}),
            vec![response(403, body)],
        );
        assert_eq!(output["error"], "provider_access_denied");
    }
}

#[test]
fn maps_trait_backed_provider_transport_failure_without_leaking_details() {
    let provider = Unimock::new(
        GitHubProviderGet
            .next_call(matching!(_))
            .returns(Err(GitHubError::ProviderUnavailable)),
    );
    let output: Value = serde_json::from_str(&call_tool_with(
        &json!({
            "installation_id": "install",
            "tool_name": "github_list_repositories",
            "input": {},
            "effective_user_id": "user"
        })
        .to_string(),
        &provider,
        &FixedClock(1_000),
    ))
    .unwrap();
    assert_eq!(
        output,
        json!({ "ok": false, "error": "provider_unavailable" })
    );
}

#[test]
fn gives_no_retry_hint_for_a_truncated_detail_response() {
    let mut truncated = response(200, json!({ "private": "body" }));
    truncated.body_truncated = true;
    let (output, _) = call(
        "github_read_pr",
        json!({ "owner": "octo", "repository": "repo", "number": 7 }),
        vec![truncated],
    );
    assert_eq!(output["error"], "provider_response_too_large");
    assert_eq!(output["retry_input"], Value::Null);
    assert!(!output.to_string().contains("private"));
}

#[test]
fn rejects_host_truncation_before_provider_parsing() {
    let mut truncated = response(200, json!([{"not": "complete"}]));
    truncated.body_truncated = true;
    let (output, _) = call("github_list_repositories", json!({}), vec![truncated]);
    assert_eq!(output["error"], "provider_response_too_large");
    assert_eq!(output["retry_input"], json!(null));
}
