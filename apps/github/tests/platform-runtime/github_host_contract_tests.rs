use std::collections::BTreeMap;
use std::sync::Arc;

use fna_apps_interface::{
    Error,
    runtime::{AppRuntime, AppToolCall},
};
use fna_apps_wasm::{HostHttpResponse, WasmHostMock};
use serde_json::json;
use unimock::{MockFn as _, Unimock, matching};
use uuid::Uuid;

use crate::github_runtime_support::runtime_with_host;

#[tokio::test]
async fn github_component_maps_host_transport_failure_without_exposing_details() {
    let runtime = runtime_with_host(Arc::new(Unimock::new(
        WasmHostMock::http_request
            .next_call(matching!(_))
            .returns(HostHttpResponse {
                ok: false,
                status: None,
                url: None,
                headers: BTreeMap::new(),
                content_type: None,
                body_json: Some(json!({ "private": "provider body" })),
                body_truncated: false,
                error: Some(String::from("secret-network-detail")),
            }),
    )));
    let error = runtime
        .call_tool(AppToolCall {
            workspace_id: Uuid::now_v7(),
            installation_id: Uuid::now_v7(),
            tool_name: String::from("github_list_repositories"),
            operation: String::from("github.list_repositories"),
            operation_id: None,
            input: json!({}),
            effective_user_id: Some(Uuid::now_v7()),
            output_hints: None,
        })
        .await
        .unwrap_err();

    assert!(
        matches!(
            error,
            Error::ProviderUnavailable(ref app_id) if app_id == "github"
        ),
        "unexpected runtime error: {error:?}"
    );
    let debug = format!("{error:?}");
    assert!(!debug.contains("secret-network-detail"));
    assert!(!debug.contains("provider body"));
}

#[tokio::test]
async fn github_component_maps_missing_credential_to_auth_required() {
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
                error: Some(String::from("credential_not_found")),
            }),
    )));
    let error = runtime
        .call_tool(AppToolCall {
            workspace_id: Uuid::now_v7(),
            installation_id: Uuid::now_v7(),
            tool_name: String::from("github_list_repositories"),
            operation: String::from("github.list_repositories"),
            operation_id: None,
            input: json!({}),
            effective_user_id: Some(Uuid::now_v7()),
            output_hints: None,
        })
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        Error::AuthRequired {
            ref app_id,
            ref auth_ids,
        } if app_id == "github" && auth_ids == &[String::from("github_installation")]
    ));
}

#[tokio::test]
async fn github_component_keeps_credential_vault_failure_unavailable() {
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
                error: Some(String::from("credential_unavailable")),
            }),
    )));
    let error = runtime
        .call_tool(AppToolCall {
            workspace_id: Uuid::now_v7(),
            installation_id: Uuid::now_v7(),
            tool_name: String::from("github_list_repositories"),
            operation: String::from("github.list_repositories"),
            operation_id: None,
            input: json!({}),
            effective_user_id: Some(Uuid::now_v7()),
            output_hints: None,
        })
        .await
        .unwrap_err();

    assert!(matches!(
        error,
        Error::ProviderUnavailable(ref app_id) if app_id == "github"
    ));
}
