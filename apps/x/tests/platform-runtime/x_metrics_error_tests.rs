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
async fn x_metrics_maps_auth_scope_rate_budget_and_provider_failures() {
    let cases = [
        (
            host_error("credential_not_found"),
            ExpectedError::AuthRequired,
        ),
        (provider_failure(403), ExpectedError::MissingReadScope),
        (
            provider_response_with_headers(
                429,
                BTreeMap::from([(String::from("Retry-After"), String::from("45"))]),
                Some(json!({"detail": "provider-secret-detail"})),
            ),
            ExpectedError::RateLimited,
        ),
        (
            provider_response(
                429,
                Some(json!({
                    "type": "https://api.x.com/2/problems/usage-capped",
                    "detail": "private billing identifier"
                })),
            ),
            ExpectedError::BudgetExhausted,
        ),
        (provider_failure(503), ExpectedError::ProviderUnavailable),
        (provider_failure(400), ExpectedError::ProviderRejected),
    ];

    for (response, expected) in cases {
        let error = call_metrics(response).await;
        assert_expected_error(&error, expected);
        let encoded = format!("{error:?}");
        assert!(!encoded.contains("provider-secret-detail"));
        assert!(!encoded.contains("private billing identifier"));
        assert!(!encoded.contains("never-leak-token"));
    }
}

#[tokio::test]
async fn x_metrics_rejects_malformed_and_truncated_successes() {
    let malformed = call_metrics(provider_response(
        200,
        Some(json!({
            "data": [{"id": "11", "public_metrics": {"impression_count": 1}}]
        })),
    ))
    .await;
    assert_provider_contract_error(&malformed);

    let mut truncated = provider_response(200, Some(json!({"data": []})));
    truncated.body_truncated = true;
    let error = call_metrics(truncated).await;
    assert_provider_contract_error(&error);
}

#[tokio::test]
async fn x_metrics_rejects_invalid_input_before_dispatch() {
    let runtime = runtime_with_host(Arc::new(Unimock::new(())));
    let error = call_tool_error(
        &runtime,
        Uuid::now_v7(),
        "x_get_post_metrics",
        "x.get_post_metrics",
        None,
        json!({"ids": ["11", "11"]}),
    )
    .await;

    assert!(matches!(
        error,
        Error::InvalidRequest { app_id, reason }
            if app_id == "x" && reason == "invalid_post_ids"
    ));
}

async fn call_metrics(response: HostHttpResponse) -> Error {
    let runtime = runtime_with_host(Arc::new(Unimock::new(
        WasmHostMock::http_request
            .next_call(matching!(_))
            .returns(response),
    )));
    call_tool_error(
        &runtime,
        Uuid::now_v7(),
        "x_get_post_metrics",
        "x.get_post_metrics",
        None,
        json!({"ids": ["11"], "include_private_metrics": true}),
    )
    .await
}

#[derive(Clone, Copy)]
enum ExpectedError {
    AuthRequired,
    MissingReadScope,
    RateLimited,
    BudgetExhausted,
    ProviderUnavailable,
    ProviderRejected,
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
        ExpectedError::RateLimited => assert!(matches!(
            error,
            Error::RateLimited {
                app_id,
                retry_after_seconds: Some(45),
            } if app_id == "x"
        )),
        ExpectedError::BudgetExhausted => assert!(matches!(
            error,
            Error::Provider(ProviderError::BudgetExhausted {
                app_id,
                provider_code: None,
            }) if app_id == "x"
        )),
        ExpectedError::ProviderUnavailable => assert!(matches!(
            error,
            Error::ProviderUnavailable(app_id) if app_id == "x"
        )),
        ExpectedError::ProviderRejected => assert!(matches!(
            error,
            Error::InvalidRequest { app_id, reason }
                if app_id == "x" && reason == "provider_rejected_request"
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
