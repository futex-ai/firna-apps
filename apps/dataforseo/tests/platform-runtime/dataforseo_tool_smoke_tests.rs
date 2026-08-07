use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use fna_apps_interface::{
    Error,
    provider_error::ProviderError,
    runtime::{AppRuntime, AppToolCall},
};
use fna_apps_wasm::{HostCredentialInjectionKind, HostHttpRequest, HostHttpResponse, WasmHostMock};
use serde_json::{Value, json};
use unimock::{MockFn as _, Unimock, matching};
use uuid::Uuid;

use crate::dataforseo_runtime_support::runtime_with_host;
use crate::dataforseo_tool_cases::tool_cases;

#[tokio::test]
async fn dataforseo_component_smoke_calls_all_sixteen_tools() {
    let installation_id = Uuid::now_v7();
    let captured = Arc::new(Mutex::new(Vec::new()));
    let requests = Arc::clone(&captured);
    let runtime = runtime_with_host(Arc::new(Unimock::new(
        WasmHostMock::http_request
            .each_call(matching!(_))
            .answers_arc(Arc::new(move |_, request: HostHttpRequest| {
                requests.lock().unwrap().push(request.clone());
                provider_success(&request)
            })),
    )));

    for tool in tool_cases() {
        let output = runtime
            .call_tool(AppToolCall {
                workspace_id: Uuid::now_v7(),
                installation_id,
                tool_name: tool.name.to_owned(),
                operation: tool.operation.to_owned(),
                operation_id: None,
                input: tool.input,
                effective_user_id: None,
                agent_id: None,
                output_hints: None,
            })
            .await
            .unwrap()
            .output;

        assert_eq!(output["ok"], true, "{}", tool.name);
        assert_eq!(output["provider"], "dataforseo", "{}", tool.name);
        assert_eq!(output["operation"], tool.operation, "{}", tool.name);
        assert_eq!(output["cost_usd"], 0.001, "{}", tool.name);
        assert_eq!(output["rate_limit"]["remaining"], 1999, "{}", tool.name);
        let encoded = output.to_string();
        assert!(!encoded.contains("api-login"));
        assert!(!encoded.contains("api-password"));
        assert!(!encoded.contains("Basic YXBp"));
        assert!(!encoded.contains("provider request echo"));
    }

    let requests = captured.lock().unwrap();
    assert_eq!(requests.len(), 16);
    for (request, expected) in requests.iter().zip(tool_cases()) {
        assert_request(request, expected.path, installation_id);
    }
}

#[tokio::test]
async fn dataforseo_component_maps_truncation_without_decoding_partial_json() {
    let runtime = runtime_with_host(Arc::new(Unimock::new(
        WasmHostMock::http_request
            .next_call(matching!(_))
            .returns(HostHttpResponse {
                ok: true,
                status: Some(200),
                url: None,
                headers: BTreeMap::new(),
                content_type: Some(String::from("application/json")),
                body_json: Some(json!({"status_code": 20000})),
                body_truncated: true,
                error: None,
            }),
    )));
    let tool = tool_cases().remove(0);

    let error = runtime
        .call_tool(AppToolCall {
            workspace_id: Uuid::now_v7(),
            installation_id: Uuid::now_v7(),
            tool_name: tool.name.to_owned(),
            operation: tool.operation.to_owned(),
            operation_id: None,
            input: tool.input,
            effective_user_id: None,
            agent_id: None,
            output_hints: None,
        })
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        Error::Provider(ProviderError::ResponseTooLarge { app_id }) if app_id == "dataforseo"
    ));
}

fn assert_request(request: &HostHttpRequest, expected_path: &str, installation_id: Uuid) {
    assert_eq!(request.method, "POST");
    assert_eq!(
        request.url,
        format!("https://api.dataforseo.com{expected_path}")
    );
    assert!(request.query.is_empty());
    assert!(request.headers.is_empty());
    assert!(request.body_text.is_none());
    assert_eq!(request.response_body_limit_bytes, Some(1_048_576));
    assert!(matches!(request.timeout_seconds, Some(180 | 240)));
    assert!(request.credential.is_none());
    let injection = request.credential_injection.as_ref().unwrap();
    assert_eq!(
        injection.kind,
        HostCredentialInjectionKind::BasicAuthorization
    );
    assert!(injection.header_name.is_none());
    assert_credential(
        injection.username_credential.as_ref().unwrap(),
        "login",
        installation_id,
    );
    assert_credential(
        injection.password_credential.as_ref().unwrap(),
        "password",
        installation_id,
    );
    let tasks = request
        .body_json
        .as_ref()
        .and_then(Value::as_array)
        .unwrap();
    assert_eq!(tasks.len(), 1);
    assert!(tasks[0].is_object());
}

fn assert_credential(
    credential: &fna_apps_wasm::HostCredentialReference,
    kind: &str,
    installation_id: Uuid,
) {
    assert_eq!(credential.app_id, "dataforseo");
    assert_eq!(credential.credential_kind, kind);
    assert_eq!(credential.installation_id, Some(installation_id));
    assert!(credential.user_grant_id.is_none());
    assert!(credential.provider_account_id.is_none());
    assert!(credential.effective_user_id.is_none());
}

fn provider_success(request: &HostHttpRequest) -> HostHttpResponse {
    HostHttpResponse {
        ok: true,
        status: Some(200),
        url: Some(request.url.clone()),
        headers: BTreeMap::from([
            (String::from("x-ratelimit-limit"), String::from("2000")),
            (String::from("x-ratelimit-remaining"), String::from("1999")),
            (String::from("authorization"), String::from("Basic YXBp")),
        ]),
        content_type: Some(String::from("application/json")),
        body_json: Some(json!({
            "status_code": 20000,
            "status_message": "provider request echo api-login:api-password",
            "tasks": [{
                "id": "task-1",
                "status_code": 20000,
                "cost": 0.001,
                "data": {"login": "api-login", "password": "api-password"},
                "result": [{"items": [{"domain": "example.com", "keyword": "rust"}]}]
            }]
        })),
        body_truncated: false,
        error: None,
    }
}
