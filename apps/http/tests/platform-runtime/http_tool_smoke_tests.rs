use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use fna_apps_interface::{
    Error,
    runtime::{AppRuntime, AppToolCall},
};
use fna_apps_wasm::{HostHttpRequest, HostHttpResponse, WasmComponentRuntime, WasmHostMock};
use serde_json::{Value, json};
use unimock::{MockFn as _, Unimock, matching};
use uuid::Uuid;

use crate::http_runtime_support::runtime_with_host;

#[tokio::test]
async fn http_component_sends_request_through_host_http() {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let captured_requests = requests.clone();
    let runtime = runtime_with_host(Arc::new(Unimock::new(
        WasmHostMock::http_request
            .next_call(matching!(_))
            .answers_arc(Arc::new(move |_, request: HostHttpRequest| {
                captured_requests.lock().unwrap().push(request);
                HostHttpResponse {
                    ok: true,
                    status: Some(201),
                    url: Some(String::from("https://api.example.com/final")),
                    headers: BTreeMap::from([(
                        String::from("content-type"),
                        String::from("application/json"),
                    )]),
                    content_type: Some(String::from("application/json")),
                    body_json: Some(json!({"accepted": true})),
                    body_truncated: false,
                    error: None,
                }
            })),
    )));

    let output = call_tool(
        &runtime,
        json!({
            "url": "https://api.example.com/jobs",
            "method": "post",
            "query": {"mode": "test"},
            "headers": {"x-request": "true"},
            "body_json": {"name": "build"},
            "timeout_seconds": 42
        }),
    )
    .await;

    assert_eq!(output["status"], 201);
    assert_eq!(output["ok"], true);
    assert_eq!(output["url"], "https://api.example.com/final");
    assert_eq!(output["content_type"], "application/json");
    assert_eq!(output["body"]["accepted"], true);
    let requests = requests.lock().unwrap();
    let request = &requests[0];
    assert_eq!(request.method, "POST");
    assert_eq!(request.url, "https://api.example.com/jobs");
    assert_eq!(request.query.get("mode"), Some(&String::from("test")));
    assert_eq!(
        request.headers.get("x-request"),
        Some(&String::from("true"))
    );
    assert_eq!(request.body_json, Some(json!({"name": "build"})));
    assert_eq!(request.timeout_seconds, Some(42));
    assert_eq!(request.credential, None);
    assert_eq!(request.credential_injection, None);
}

#[tokio::test]
async fn http_component_sends_default_timeout_to_host() {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let captured_requests = requests.clone();
    let runtime = runtime_with_host(Arc::new(Unimock::new(
        WasmHostMock::http_request
            .next_call(matching!(_))
            .answers_arc(Arc::new(move |_, request: HostHttpRequest| {
                captured_requests.lock().unwrap().push(request);
                HostHttpResponse {
                    ok: true,
                    status: Some(200),
                    url: Some(String::from("https://example.com/status")),
                    headers: BTreeMap::new(),
                    content_type: Some(String::from("application/json")),
                    body_json: Some(json!({"ok": true})),
                    body_truncated: false,
                    error: None,
                }
            })),
    )));

    let output = call_tool(&runtime, json!({"url": "https://example.com/status"})).await;

    assert_eq!(output["status"], 200);
    let requests = requests.lock().unwrap();
    assert_eq!(requests[0].timeout_seconds, Some(60));
}

#[tokio::test]
async fn http_component_returns_non_2xx_as_normal_result() {
    let runtime = runtime_with_host(Arc::new(Unimock::new(
        WasmHostMock::http_request
            .next_call(matching!(_))
            .returns(HostHttpResponse {
                ok: true,
                status: Some(404),
                url: Some(String::from("https://example.com/missing")),
                headers: BTreeMap::new(),
                content_type: Some(String::from("text/plain")),
                body_json: Some(json!("not found")),
                body_truncated: false,
                error: None,
            }),
    )));

    let output = call_tool(&runtime, json!({"url": "https://example.com/missing"})).await;

    assert_eq!(output["status"], 404);
    assert_eq!(output["ok"], false);
    assert_eq!(output["body"], "not found");
}

#[tokio::test]
async fn http_component_rejects_invalid_request_input() {
    let runtime = runtime_with_host(Arc::new(Unimock::new(())));

    let error = runtime
        .call_tool(http_tool_call(json!({
            "url": "https://example.com",
            "body_json": {"a": true},
            "body_text": "hello"
        })))
        .await
        .unwrap_err();

    assert!(matches!(error, Error::InvalidRequest { .. }));
}

#[tokio::test]
async fn http_component_maps_host_failures() {
    let runtime = runtime_with_host(Arc::new(Unimock::new(
        WasmHostMock::http_request
            .next_call(matching!(_))
            .returns(HostHttpResponse {
                ok: false,
                status: None,
                url: None,
                headers: BTreeMap::new(),
                content_type: None,
                body_json: None,
                body_truncated: false,
                error: Some(String::from("provider_unavailable")),
            }),
    )));

    let error = runtime
        .call_tool(http_tool_call(json!({"url": "https://example.com"})))
        .await
        .unwrap_err();

    assert!(matches!(error, Error::ProviderUnavailable { .. }));
}

#[tokio::test]
async fn http_component_maps_host_validation_failures_to_invalid_request() {
    let runtime = runtime_with_host(Arc::new(Unimock::new(
        WasmHostMock::http_request
            .next_call(matching!(_))
            .returns(HostHttpResponse {
                ok: false,
                status: None,
                url: None,
                headers: BTreeMap::new(),
                content_type: None,
                body_json: None,
                body_truncated: false,
                error: Some(String::from("host_http_credentials_denied")),
            }),
    )));

    let error = runtime
        .call_tool(http_tool_call(json!({"url": "https://example.com"})))
        .await
        .unwrap_err();

    assert!(
        matches!(error, Error::InvalidRequest { reason, .. } if reason == "host_http_credentials_denied")
    );
}

async fn call_tool(runtime: &WasmComponentRuntime, input: Value) -> Value {
    runtime
        .call_tool(http_tool_call(input))
        .await
        .unwrap()
        .output
}

fn http_tool_call(input: Value) -> AppToolCall {
    AppToolCall {
        workspace_id: Uuid::now_v7(),
        installation_id: Uuid::now_v7(),
        tool_name: String::from("http_request"),
        operation: String::from("http.request"),
        operation_id: None,
        input,
        effective_user_id: None,
        agent_id: None,
        output_hints: None,
    }
}
