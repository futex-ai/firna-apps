use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use fna_apps_interface::Error;
use fna_apps_interface::runtime::AppToolUsageReport;
use fna_apps_wasm::{HostHttpRequest, WasmHostMock};
use serde_json::json;
use unimock::{MockFn as _, Unimock, matching};
use uuid::Uuid;

use crate::x_runtime_support::runtime_with_host;
use crate::x_test_support::{
    assert_no_null_fields, call_tool_error, call_tool_result, host_error, provider_response,
};

#[tokio::test]
async fn x_component_creates_one_reply_without_forwarding_operation_id() {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let captured = Arc::clone(&requests);
    let runtime = runtime_with_host(Arc::new(Unimock::new(
        WasmHostMock::http_request
            .next_call(matching!(_))
            .answers_arc(Arc::new(move |_, request: HostHttpRequest| {
                captured.lock().expect("request capture lock").push(request);
                provider_response(201, Some(json!({"data": {"id": "45", "text": "Hello X"}})))
            })),
    )));
    let installation_id = Uuid::now_v7();

    let result = call_tool_result(
        &runtime,
        installation_id,
        "x_create_post",
        "x.create_post",
        Some("durable-operation-id"),
        json!({"text": "Hello X", "reply_to_post_id": "44"}),
    )
    .await
    .expect("X reply creation should complete");

    assert_eq!(
        result.usage,
        Some(AppToolUsageReport::ReportedCost {
            cost_usd_micros: 15_000,
        })
    );
    let output = result.output;
    assert_eq!(output, json!({"post": {"id": "45", "text": "Hello X"}}));
    assert!(output.get("usage").is_none());
    assert_no_null_fields(&output);
    let requests = requests.lock().expect("request capture lock");
    assert_eq!(requests.len(), 1);
    let request = &requests[0];
    assert_eq!(request.method, "POST");
    assert_eq!(request.url, "https://api.x.com/2/tweets");
    assert_eq!(request.query, BTreeMap::new());
    assert_eq!(
        request.headers,
        BTreeMap::from([(
            String::from("content-type"),
            String::from("application/json")
        )])
    );
    assert_eq!(
        request.body_json,
        Some(json!({
            "text": "Hello X",
            "reply": {"in_reply_to_tweet_id": "44"}
        }))
    );
    assert!(
        !request
            .body_json
            .as_ref()
            .expect("body")
            .to_string()
            .contains("durable-operation-id")
    );
    assert_eq!(
        request
            .credential
            .as_ref()
            .expect("credential")
            .installation_id,
        Some(installation_id)
    );
}

#[tokio::test]
async fn x_component_requires_link_cost_acknowledgement_before_dispatch() {
    let runtime = runtime_with_host(Arc::new(Unimock::new(())));

    let error = call_tool_error(
        &runtime,
        Uuid::now_v7(),
        "x_create_post",
        "x.create_post",
        None,
        json!({"text": "See https://example.com"}),
    )
    .await;

    assert!(matches!(
        error,
        Error::InvalidRequest { app_id, reason }
            if app_id == "x" && reason == "link_acknowledgement_required"
    ));
}

#[tokio::test]
async fn x_component_reports_the_capped_link_post_cost() {
    let runtime = runtime_with_host(Arc::new(Unimock::new(
        WasmHostMock::http_request
            .next_call(matching!(_))
            .returns(provider_response(
                201,
                Some(json!({
                    "data": {"id": "46", "text": "See https://example.com"}
                })),
            )),
    )));

    let result = call_tool_result(
        &runtime,
        Uuid::now_v7(),
        "x_create_post",
        "x.create_post",
        None,
        json!({"text": "See HTTPS://example.com", "allow_link": true}),
    )
    .await
    .expect("X link Post creation should complete");

    assert_eq!(
        result.usage,
        Some(AppToolUsageReport::ReportedCost {
            cost_usd_micros: 200_000,
        })
    );
}

#[tokio::test]
async fn x_component_never_retries_an_ambiguous_write() {
    let requests = Arc::new(Mutex::new(0_u8));
    let captured = Arc::clone(&requests);
    let runtime = runtime_with_host(Arc::new(Unimock::new(
        WasmHostMock::http_request
            .next_call(matching!(_))
            .answers_arc(Arc::new(move |_, _| {
                let mut count = captured.lock().expect("request counter lock");
                *count += 1;
                host_error("provider_transport_failed")
            })),
    )));

    let error = call_tool_error(
        &runtime,
        Uuid::now_v7(),
        "x_create_post",
        "x.create_post",
        Some("ambiguous-operation"),
        json!({"text": "Hello X"}),
    )
    .await;

    assert!(matches!(
        error,
        Error::RuntimeRejected { operation, reason }
            if operation == "call-tool" && reason == "write_outcome_unknown"
    ));
    assert_eq!(*requests.lock().expect("request counter lock"), 1);
}
