use std::collections::BTreeMap;
use std::sync::Arc;

use fna_apps_interface::runtime::{AppRuntime, AppToolCall};
use fna_apps_wasm::{HostHttpResponse, WasmHostMock};
use serde_json::json;
use unimock::{MockFn as _, Unimock, matching};
use uuid::Uuid;

use crate::slack_runtime_support::runtime_with_host;

#[tokio::test]
async fn slack_component_maps_provider_errors() {
    let missing_scope = call_with_provider_response(
        "slack_list_channels",
        "slack.list_channels",
        json!({}),
        200,
        BTreeMap::new(),
        json!({ "ok": false, "error": "missing_scope", "needed": "channels:read" }),
    )
    .await
    .unwrap_err();
    assert!(matches!(
        missing_scope,
        fna_apps_interface::Error::MissingProviderScope { .. }
    ));

    let not_in_channel = call_with_provider_response(
        "slack_read_channel_history",
        "slack.read_channel_history",
        json!({ "channel_id": "C123" }),
        200,
        BTreeMap::new(),
        json!({ "ok": false, "error": "not_in_channel" }),
    )
    .await
    .unwrap_err();
    assert!(matches!(
        not_in_channel,
        fna_apps_interface::Error::InvalidRequest { .. }
    ));

    let rate_limited = call_with_provider_response(
        "slack_send_message",
        "slack.send_message",
        json!({ "channel_id": "C123", "text": "hello" }),
        429,
        BTreeMap::from([(String::from("retry-after"), String::from("30"))]),
        json!({ "ok": false, "error": "ratelimited" }),
    )
    .await
    .unwrap_err();
    assert!(matches!(
        rate_limited,
        fna_apps_interface::Error::RateLimited {
            retry_after_seconds: Some(30),
            ..
        }
    ));

    let revoked = call_with_provider_response(
        "slack_send_message",
        "slack.send_message",
        json!({ "channel_id": "C123", "text": "hello" }),
        200,
        BTreeMap::new(),
        json!({ "ok": false, "error": "token_revoked" }),
    )
    .await
    .unwrap_err();
    assert!(matches!(
        revoked,
        fna_apps_interface::Error::AuthRequired { .. }
    ));

    let outage = call_with_provider_response(
        "slack_send_message",
        "slack.send_message",
        json!({ "channel_id": "C123", "text": "hello" }),
        503,
        BTreeMap::new(),
        json!({ "ok": false }),
    )
    .await
    .unwrap_err();
    assert!(matches!(
        outage,
        fna_apps_interface::Error::ProviderUnavailable { .. }
    ));
}

#[tokio::test]
async fn slack_component_returns_pagination_cursors() {
    let output = call_with_provider_response(
        "slack_list_channels",
        "slack.list_channels",
        json!({ "limit": 1 }),
        200,
        BTreeMap::new(),
        json!({
            "ok": true,
            "channels": [{ "id": "C123", "name": "general", "is_member": true }],
            "response_metadata": { "next_cursor": "next" }
        }),
    )
    .await
    .unwrap()
    .output;

    assert_eq!(output["next_cursor"], "next");
    assert_eq!(output["channels"][0]["id"], "C123");
}

async fn call_with_provider_response(
    tool_name: &str,
    operation: &str,
    input: serde_json::Value,
    status: u16,
    headers: BTreeMap<String, String>,
    body_json: serde_json::Value,
) -> fna_apps_interface::Result<fna_apps_interface::runtime::AppToolResult> {
    let runtime = runtime_with_host(Arc::new(Unimock::new(
        WasmHostMock::http_request
            .next_call(matching!(_))
            .returns(HostHttpResponse {
                ok: true,
                status: Some(status),
                url: Some(String::from("https://slack.com/api/test")),
                headers,
                content_type: Some(String::from("application/json")),
                body_json: Some(body_json),
                body_truncated: false,
                error: None,
            }),
    )));
    runtime
        .call_tool(AppToolCall {
            workspace_id: Uuid::now_v7(),
            installation_id: Uuid::now_v7(),
            tool_name: tool_name.to_owned(),
            operation: operation.to_owned(),
            operation_id: None,
            input,
            effective_user_id: None,
            agent_id: None,
            output_hints: None,
        })
        .await
}
