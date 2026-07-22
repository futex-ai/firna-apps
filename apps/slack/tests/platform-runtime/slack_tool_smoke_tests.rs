use std::collections::BTreeMap;
use std::sync::Arc;

use fna_apps_interface::runtime::{AppRuntime, AppToolCall};
use fna_apps_wasm::{HostHttpRequest, HostHttpResponse, WasmComponentRuntime, WasmHostMock};
use serde_json::{Value, json};
use unimock::{MockFn as _, Unimock, matching};
use uuid::Uuid;

use crate::slack_runtime_support::runtime_with_host;

#[tokio::test]
async fn slack_component_smoke_calls_all_v1_tools() {
    let runtime = runtime_with_host(Arc::new(Unimock::new(
        WasmHostMock::http_request
            .each_call(matching!(_))
            .answers(&|_, request: HostHttpRequest| provider_response(request)),
    )));
    let user_id = Uuid::now_v7();

    let channels = call_tool(
        &runtime,
        "slack_list_channels",
        "slack.list_channels",
        json!({ "limit": 1 }),
        None,
    )
    .await;
    assert_eq!(channels["channels"][0]["id"], "C123");

    let history = call_tool(
        &runtime,
        "slack_read_channel_history",
        "slack.read_channel_history",
        json!({ "channel_id": "C123", "limit": 1 }),
        None,
    )
    .await;
    assert_eq!(history["messages"][0]["text"], "hello");

    let sent = call_tool(
        &runtime,
        "slack_send_message",
        "slack.send_message",
        json!({ "channel_id": "C123", "text": "hello" }),
        None,
    )
    .await;
    assert_eq!(sent["ts"], "1710000000.000300");

    let search = call_tool(
        &runtime,
        "slack_search_messages",
        "slack.search_messages",
        json!({ "query": "hello" }),
        Some(user_id),
    )
    .await;
    assert_eq!(search["messages"][0]["text"], "hello from search");

    let error = runtime
        .call_tool(AppToolCall {
            workspace_id: Uuid::now_v7(),
            installation_id: Uuid::now_v7(),
            tool_name: String::from("slack_search_messages"),
            operation: String::from("slack.search_messages"),
            operation_id: None,
            input: json!({ "query": "hello" }),
            effective_user_id: None,
            output_hints: None,
        })
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        fna_apps_interface::Error::AuthRequired { .. }
    ));
}

async fn call_tool(
    runtime: &WasmComponentRuntime,
    tool_name: &str,
    operation: &str,
    input: Value,
    effective_user_id: Option<Uuid>,
) -> Value {
    runtime
        .call_tool(AppToolCall {
            workspace_id: Uuid::now_v7(),
            installation_id: Uuid::now_v7(),
            tool_name: tool_name.to_owned(),
            operation: operation.to_owned(),
            operation_id: None,
            input,
            effective_user_id,
            output_hints: None,
        })
        .await
        .unwrap()
        .output
}

fn provider_response(request: HostHttpRequest) -> HostHttpResponse {
    let credential = request.credential.as_ref().unwrap();
    if request.url.ends_with("search.messages") {
        assert_eq!(credential.credential_kind, "user_token");
        assert!(credential.effective_user_id.is_some());
    } else {
        assert_eq!(credential.credential_kind, "bot_token");
    }
    assert_request_body_has_no_null_fields(&request);
    HostHttpResponse {
        ok: true,
        status: Some(200),
        url: Some(request.url.clone()),
        headers: BTreeMap::new(),
        content_type: Some(String::from("application/json")),
        body_json: Some(body_json_for_url(&request.url)),
        body_truncated: false,
        error: None,
    }
}

fn assert_request_body_has_no_null_fields(request: &HostHttpRequest) {
    let Some(Value::Object(fields)) = request.body_json.as_ref() else {
        panic!("Slack request must include a JSON object body");
    };
    let null_field = fields.iter().find(|(_, value)| value.is_null());
    assert!(
        null_field.is_none(),
        "Slack request to {} included null field {:?}",
        request.url,
        null_field.map(|(key, _)| key)
    );
}

fn body_json_for_url(url: &str) -> Value {
    match url.rsplit('/').next().unwrap_or_default() {
        "conversations.list" => json!({
            "ok": true,
            "channels": [{ "id": "C123", "name": "general", "is_member": true }]
        }),
        "conversations.history" => json!({
            "ok": true,
            "messages": [{ "ts": "1710000000.000100", "user": "U123", "text": "hello" }]
        }),
        "chat.postMessage" => json!({
            "ok": true,
            "channel": "C123",
            "ts": "1710000000.000300"
        }),
        "search.messages" => json!({
            "ok": true,
            "messages": {
                "matches": [
                    { "ts": "1710000000.000200", "user": "U123", "text": "hello from search" }
                ],
                "pagination": { "next_cursor": "next" }
            }
        }),
        _ => json!({ "ok": false, "error": "unknown_url" }),
    }
}
