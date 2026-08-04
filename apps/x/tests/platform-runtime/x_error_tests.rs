use std::collections::BTreeMap;
use std::sync::Arc;

use fna_apps_interface::Error;
use fna_apps_interface::provider_error::ProviderError;
use fna_apps_wasm::{HostHttpResponse, WasmHostMock};
use serde_json::json;
use unimock::{MockFn as _, Unimock, matching};
use uuid::Uuid;

use crate::x_runtime_support::runtime_with_host;
use crate::x_test_support::{
    call_tool_error, host_error, provider_response, provider_response_with_headers,
};

#[tokio::test]
async fn x_component_maps_missing_credentials_and_provider_statuses() {
    let cases = [
        (
            host_error("credential_not_found"),
            ExpectedError::AuthRequired,
        ),
        (provider_failure(401), ExpectedError::AuthRequired),
        (provider_failure(403), ExpectedError::MissingReadScope),
        (provider_failure(404), ExpectedError::NotFound),
        (provider_failure(503), ExpectedError::ProviderUnavailable),
    ];

    for (response, expected) in cases {
        let error = call_search(response).await;
        assert_expected_error(&error, expected);
        let encoded = format!("{error:?}");
        assert!(!encoded.contains("provider-secret-detail"));
        assert!(!encoded.contains("never-leak-token"));
    }
}

#[tokio::test]
async fn x_component_maps_unknown_4xx_to_non_retryable_rejections() {
    let errors = [
        call_search(provider_failure(400)).await,
        call_create(provider_failure(422)).await,
    ];

    for error in errors {
        let encoded = format!("{error:?}");
        assert!(matches!(
            error,
            Error::InvalidRequest { app_id, reason }
                if app_id == "x" && reason == "provider_rejected_request"
        ));
        assert!(!encoded.contains("provider-secret-detail"));
        assert!(!encoded.contains("never-leak-token"));
    }
}

#[tokio::test]
async fn x_component_distinguishes_rate_limits_from_credit_exhaustion() {
    let limited = call_search(provider_response_with_headers(
        429,
        BTreeMap::from([(String::from("Retry-After"), String::from("45"))]),
        Some(json!({"type": "https://api.x.com/2/problems/rate-limit-exceeded"})),
    ))
    .await;
    assert!(matches!(
        limited,
        Error::RateLimited {
            ref app_id,
            retry_after_seconds: Some(45),
        } if app_id == "x"
    ));

    let capped = call_search(provider_response(
        429,
        Some(json!({
            "type": "https://api.x.com/2/problems/usage-capped",
            "detail": "account billing id must remain private"
        })),
    ))
    .await;
    assert!(matches!(
        capped,
        Error::Provider(ProviderError::BudgetExhausted {
            ref app_id,
            provider_code: None,
        }) if app_id == "x"
    ));
    assert!(!format!("{capped:?}").contains("billing id"));
}

#[tokio::test]
async fn x_component_rejects_malformed_and_truncated_provider_json() {
    let malformed = call_search(provider_response(
        200,
        Some(json!({"data": "not-an-array"})),
    ))
    .await;
    assert_provider_contract_error(&malformed);

    let mut truncated = provider_response(200, Some(json!({"data": []})));
    truncated.body_truncated = true;
    let error = call_search(truncated).await;
    assert_provider_contract_error(&error);
}

#[tokio::test]
async fn x_component_fails_ambiguous_create_responses_closed() {
    let mut truncated =
        provider_response(201, Some(json!({"data": {"id": "45", "text": "Hello X"}})));
    truncated.body_truncated = true;
    let cases = [
        provider_response(201, Some(json!({"data": "not-a-post"}))),
        truncated,
        HostHttpResponse {
            ok: true,
            status: None,
            url: Some(String::from("https://api.x.com/2/tweets")),
            headers: BTreeMap::new(),
            content_type: Some(String::from("application/json")),
            body_json: Some(json!({"data": {"id": "45", "text": "Hello X"}})),
            body_truncated: false,
            error: None,
        },
        provider_response(503, Some(json!({"title": "temporarily unavailable"}))),
    ];

    for response in cases {
        let error = call_create(response).await;
        assert!(matches!(
            error,
            Error::RuntimeRejected { operation, reason }
                if operation == "call-tool" && reason == "write_outcome_unknown"
        ));
    }
}

async fn call_search(response: HostHttpResponse) -> Error {
    let runtime = runtime_with_host(Arc::new(Unimock::new(
        WasmHostMock::http_request
            .next_call(matching!(_))
            .returns(response),
    )));
    call_tool_error(
        &runtime,
        Uuid::now_v7(),
        "x_search_recent_posts",
        "x.search_recent_posts",
        None,
        json!({"query": "rust", "max_results": 10}),
    )
    .await
}

async fn call_create(response: HostHttpResponse) -> Error {
    let runtime = runtime_with_host(Arc::new(Unimock::new(
        WasmHostMock::http_request
            .next_call(matching!(_))
            .returns(response),
    )));
    call_tool_error(
        &runtime,
        Uuid::now_v7(),
        "x_create_post",
        "x.create_post",
        None,
        json!({"text": "Hello X"}),
    )
    .await
}

#[derive(Clone, Copy)]
enum ExpectedError {
    AuthRequired,
    MissingReadScope,
    NotFound,
    ProviderUnavailable,
}

fn assert_expected_error(error: &Error, expected: ExpectedError) {
    match expected {
        ExpectedError::AuthRequired => assert!(matches!(
            error,
            Error::AuthRequired { app_id, auth_ids }
                if app_id == "x" && auth_ids == &[String::from("x_workspace")]
        )),
        ExpectedError::MissingReadScope => assert!(matches!(
            error,
            Error::MissingProviderScope { app_id, scope }
                if app_id == "x" && scope == "tweet.read"
        )),
        ExpectedError::NotFound => assert!(matches!(
            error,
            Error::RuntimeRejected { operation, reason }
                if *operation == "call-tool" && reason == "not_found"
        )),
        ExpectedError::ProviderUnavailable => assert!(matches!(
            error,
            Error::ProviderUnavailable(app_id) if app_id == "x"
        )),
    }
}

fn assert_provider_contract_error(error: &Error) {
    assert!(matches!(
        error,
        Error::Provider(ProviderError::Contract { app_id }) if app_id == "x"
    ));
}

fn provider_failure(status: u16) -> HostHttpResponse {
    provider_response(
        status,
        Some(json!({
            "detail": "provider-secret-detail",
            "token": "never-leak-token"
        })),
    )
}
