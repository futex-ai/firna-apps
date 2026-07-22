use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use fna_apps_interface::{
    Error,
    runtime::{AppRuntime, AppToolCall, AppToolResult},
};
use fna_apps_wasm::{HostHttpRequest, HostHttpResponse, WasmComponentRuntime, WasmHostMock};
use serde_json::{Value, json};
use unimock::{MockFn as _, Unimock, matching};
use uuid::Uuid;

use crate::exa_runtime_support::runtime_with_host;

#[tokio::test]
async fn exa_component_sends_search_through_host_http() {
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
                    url: Some(String::from("https://api.exa.ai/search")),
                    headers: BTreeMap::new(),
                    content_type: Some(String::from("application/json")),
                    body_json: Some(json!({
                        "results": [{
                            "title": "Rust",
                            "url": "https://www.rust-lang.org",
                            "highlights": ["Rust language"]
                        }]
                    })),
                    body_truncated: false,
                    error: None,
                }
            })),
    )));

    let output = call_tool(
        &runtime,
        json!({
            "query": " rust language ",
            "search_type": "deep-lite",
            "num_results": 3,
            "category": "research paper",
            "include_domains": ["rust-lang.org", ""],
            "exclude_domains": ["example.com"],
            "start_published_date": "2024-01-01",
            "end_published_date": "2026-01-01",
            "livecrawl": true,
            "highlights_max_characters": 2000,
            "text_max_characters": 5000,
            "timeout_seconds": 42
        }),
    )
    .await;

    assert_eq!(output["provider"], "exa");
    assert_eq!(output["status"], 200);
    assert_eq!(output["results"][0]["title"], "Rust");
    let requests = requests.lock().unwrap();
    let request = &requests[0];
    assert_eq!(request.url, "https://api.exa.ai/search");
    assert_eq!(request.timeout_seconds, Some(42));
    assert_eq!(request.credential.as_ref().unwrap().app_id, "exa");
    assert_eq!(
        request.credential.as_ref().unwrap().credential_kind,
        "api_key"
    );
    assert_eq!(
        request
            .credential_injection
            .as_ref()
            .unwrap()
            .header_name
            .as_deref(),
        Some("x-api-key")
    );
    assert_eq!(request.headers.get("x-api-key"), None);
    assert_provider_body(request.body_json.as_ref().unwrap());
}

#[tokio::test]
async fn exa_component_rejects_invalid_request_input() {
    let runtime = runtime_with_host(Arc::new(Unimock::new(())));

    let error = runtime
        .call_tool(AppToolCall {
            workspace_id: Uuid::now_v7(),
            installation_id: Uuid::now_v7(),
            tool_name: String::from("exa_web_search"),
            operation: String::from("exa.search"),
            operation_id: None,
            input: json!({ "query": "   " }),
            effective_user_id: None,
            output_hints: None,
        })
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        fna_apps_interface::Error::InvalidRequest { .. }
    ));
}

#[tokio::test]
async fn exa_component_maps_rate_limited_provider_response() {
    let error = call_with_host_response(provider_response_with_headers(
        Some(429),
        BTreeMap::from([(String::from("retry-after"), String::from("45"))]),
        Some(json!({ "error": "rate_limit" })),
    ))
    .await
    .unwrap_err();

    assert!(matches!(
        error,
        Error::RateLimited {
            retry_after_seconds: Some(45),
            ..
        }
    ));
}

#[tokio::test]
async fn exa_component_maps_provider_unavailable_responses() {
    let cases = vec![
        provider_response(Some(401), Some(json!({ "error": "unauthorized" }))),
        provider_response(Some(403), Some(json!({ "error": "forbidden" }))),
        provider_response(Some(503), Some(json!({ "error": "unavailable" }))),
        provider_response(Some(200), None),
        HostHttpResponse {
            ok: false,
            status: None,
            url: None,
            headers: BTreeMap::new(),
            content_type: None,
            body_json: None,
            body_truncated: false,
            error: Some(String::from("credential_not_found")),
        },
    ];

    for response in cases {
        let error = call_with_host_response(response).await.unwrap_err();

        assert!(matches!(error, Error::ProviderUnavailable { .. }));
    }
}

async fn call_tool(runtime: &WasmComponentRuntime, input: Value) -> Value {
    call_tool_result(runtime, input).await.unwrap().output
}

async fn call_with_host_response(
    response: HostHttpResponse,
) -> fna_apps_interface::Result<AppToolResult> {
    let runtime = runtime_with_host(Arc::new(Unimock::new(
        WasmHostMock::http_request
            .next_call(matching!(_))
            .returns(response),
    )));
    call_tool_result(&runtime, json!({ "query": "rust language" })).await
}

async fn call_tool_result(
    runtime: &WasmComponentRuntime,
    input: Value,
) -> fna_apps_interface::Result<AppToolResult> {
    runtime.call_tool(exa_tool_call(input)).await
}

fn exa_tool_call(input: Value) -> AppToolCall {
    AppToolCall {
        workspace_id: Uuid::now_v7(),
        installation_id: Uuid::now_v7(),
        tool_name: String::from("exa_web_search"),
        operation: String::from("exa.search"),
        operation_id: None,
        input,
        effective_user_id: None,
        output_hints: None,
    }
}

fn provider_response(status: Option<u16>, body_json: Option<Value>) -> HostHttpResponse {
    provider_response_with_headers(status, BTreeMap::new(), body_json)
}

fn provider_response_with_headers(
    status: Option<u16>,
    headers: BTreeMap<String, String>,
    body_json: Option<Value>,
) -> HostHttpResponse {
    HostHttpResponse {
        ok: true,
        status,
        url: Some(String::from("https://api.exa.ai/search")),
        headers,
        content_type: Some(String::from("application/json")),
        body_json,
        body_truncated: false,
        error: None,
    }
}

fn assert_provider_body(body: &Value) {
    assert_eq!(body["query"], "rust language");
    assert_eq!(body["type"], "deep-lite");
    assert_eq!(body["numResults"], 3);
    assert_eq!(body["category"], "research paper");
    assert_eq!(body["includeDomains"], json!(["rust-lang.org"]));
    assert_eq!(body["excludeDomains"], json!(["example.com"]));
    assert_eq!(body["startPublishedDate"], "2024-01-01");
    assert_eq!(body["endPublishedDate"], "2026-01-01");
    assert_eq!(body["contents"]["highlights"]["maxCharacters"], 2000);
    assert_eq!(body["contents"]["text"]["maxCharacters"], 5000);
    assert_eq!(body["contents"]["text"]["verbosity"], "compact");
    assert_eq!(body["contents"]["maxAgeHours"], 0);
}
