use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use chrono::Utc;
use fna_apps_interface::runtime::{AppRuntime, AppToolCall};
use fna_apps_wasm::{
    HostHmacSha256Request, HostHmacSha256Response, HostHttpRequest, HostHttpResponse, WasmHostMock,
};
use serde_json::json;
use unimock::{MockFn as _, Unimock, matching};
use uuid::Uuid;

use crate::slack_runtime_support::runtime_with_host;
use crate::{component_bytes, manifest};

#[path = "slack_component_error_tests.rs"]
mod slack_component_error_tests;

#[test]
fn slack_manifest_declares_v1_tools_ingress_and_events() {
    let manifest = manifest();

    manifest.validate().unwrap();
    assert_eq!(manifest.id, "slack");
    assert_eq!(manifest.version, "1.1.19");
    assert!(manifest.icon.is_some());
    assert_eq!(
        manifest.icon.as_ref().unwrap().color_pair.primary,
        "#36C5F0"
    );
    assert_eq!(
        manifest.icon.as_ref().unwrap().color_pair.secondary,
        "#E01E5A"
    );
    assert_eq!(manifest.credential_flows.len(), 1);
    assert_eq!(manifest.credential_flows[0].kind(), "standard_oauth2");
    assert!(
        manifest
            .auth_requirements
            .iter()
            .all(|requirement| requirement.credential_flow.as_deref() == Some("slack"))
    );
    assert_eq!(manifest.tools.len(), 4);
    assert_eq!(
        manifest
            .tools
            .iter()
            .map(|tool| tool.activity_label.as_str())
            .collect::<Vec<_>>(),
        [
            "Listing Slack channels",
            "Reading Slack channel history",
            "Sending Slack message",
            "Searching Slack messages",
        ]
    );
    assert_eq!(manifest.ingress[0].verify_export, "verify-webhook");
    assert_eq!(
        manifest
            .events
            .iter()
            .map(|event| {
                (
                    event.id.as_str(),
                    event.ingress_id.as_str(),
                    event.provider_type.as_str(),
                    event.description.as_str(),
                    event.contract_version,
                )
            })
            .collect::<Vec<_>>(),
        vec![
            (
                "app_mention",
                "slack_events",
                "app_mention",
                "A Slack message mentions the workspace app bot.",
                1,
            ),
            (
                "message_channels",
                "slack_events",
                "message.channels",
                "A public channel message is visible to the workspace app bot.",
                1,
            ),
            (
                "message_groups",
                "slack_events",
                "message.groups",
                "A private channel message is visible to the workspace app bot.",
                1,
            ),
            (
                "message_im",
                "slack_events",
                "message.im",
                "A direct message is visible to the workspace app bot.",
                1,
            ),
            (
                "message_mpim",
                "slack_events",
                "message.mpim",
                "A group direct message is visible to the workspace app bot.",
                1,
            ),
        ]
    );
}

#[tokio::test]
async fn slack_component_sends_messages_through_host_http() {
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
                    url: Some(String::from("https://slack.com/api/chat.postMessage")),
                    headers: BTreeMap::new(),
                    content_type: Some(String::from("application/json")),
                    body_json: Some(json!({
                        "ok": true,
                        "channel": "C123",
                        "ts": "1710000000.000100"
                    })),
                    body_truncated: false,
                    error: None,
                }
            })),
    )));
    let output = runtime
        .call_tool(AppToolCall {
            workspace_id: Uuid::now_v7(),
            installation_id: Uuid::now_v7(),
            tool_name: String::from("slack_send_message"),
            operation: String::from("slack.send_message"),
            operation_id: Some(String::from("operation-slack-message-1")),
            input: json!({"channel_id": "C123", "text": "hello"}),
            effective_user_id: None,
            output_hints: None,
        })
        .await
        .unwrap()
        .output;

    assert_eq!(output["channel_id"], "C123");
    assert_eq!(output["ts"], "1710000000.000100");
    let requests = requests.lock().unwrap();
    assert_eq!(requests[0].url, "https://slack.com/api/chat.postMessage");
    assert_eq!(
        requests[0].body_json.as_ref().unwrap()["client_msg_id"],
        "operation-slack-message-1"
    );
    assert_eq!(
        requests[0].credential.as_ref().unwrap().credential_kind,
        "bot_token"
    );
}

#[tokio::test]
async fn slack_component_verifies_and_normalizes_webhooks() {
    let runtime = runtime_with_host(Arc::new(Unimock::new(
        WasmHostMock::hmac_sha256.next_call(matching!(_)).answers(
            &|_, request: HostHmacSha256Request| {
                assert_eq!(request.credential.credential_kind, "signing_secret");
                assert_eq!(request.credential.provider_account_id, None);
                HostHmacSha256Response {
                    ok: true,
                    digest: Some(String::from("digest")),
                    error: None,
                }
            },
        ),
    )));
    let now = Utc::now();
    let body = json!({
        "team_id": "T123",
        "event_id": "Ev123",
        "event": {
            "type": "app_mention",
            "user": "U123",
            "channel": "C123",
            "text": "<@UAPP> help",
            "ts": "1710000000.000100"
        }
    })
    .to_string();
    let body_bytes = body.into_bytes();
    let verification = runtime
        .verify_webhook(fna_apps_interface::runtime::WebhookEnvelope {
            app_id: String::from("slack"),
            ingress_id: String::from("slack_events"),
            headers: BTreeMap::from([
                (
                    String::from("x-slack-request-timestamp"),
                    now.timestamp().to_string(),
                ),
                (String::from("x-slack-signature"), String::from("v0=digest")),
            ]),
            query: BTreeMap::new(),
            body: body_bytes.clone(),
            received_at: now,
        })
        .await
        .unwrap();

    assert_eq!(verification.provider_account_id, "T123");
    assert_eq!(verification.provider_event_type, "app_mention");
    let event = runtime
        .normalize_event(fna_apps_interface::runtime::VerifiedProviderEvent {
            workspace_id: Uuid::now_v7(),
            installation_id: Uuid::now_v7(),
            envelope: fna_apps_interface::runtime::WebhookEnvelope {
                app_id: String::from("slack"),
                ingress_id: String::from("slack_events"),
                headers: BTreeMap::new(),
                query: BTreeMap::new(),
                body: body_bytes,
                received_at: now,
            },
            verification,
        })
        .await
        .unwrap();
    assert_eq!(event.provider_event_id, "Ev123");
    assert_eq!(event.source["channel_id"], "C123");
}

#[tokio::test]
async fn slack_component_rejects_bad_or_stale_webhook_signatures() {
    let runtime = runtime_with_host(Arc::new(Unimock::new(
        WasmHostMock::hmac_sha256
            .next_call(matching!(_))
            .returns(HostHmacSha256Response {
                ok: true,
                digest: Some(String::from("digest")),
                error: None,
            }),
    )));
    let now = Utc::now();
    let error = runtime
        .verify_webhook(slack_envelope(now, "v0=wrong", now.timestamp()))
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        fna_apps_interface::Error::RuntimeRejected { .. }
    ));

    let runtime = runtime_with_host(Arc::new(Unimock::new(())));
    let error = runtime
        .verify_webhook(slack_envelope(now, "v0=digest", now.timestamp() - 301))
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        fna_apps_interface::Error::RuntimeRejected { .. }
    ));
}

fn slack_envelope(
    received_at: chrono::DateTime<Utc>,
    signature: &str,
    timestamp: i64,
) -> fna_apps_interface::runtime::WebhookEnvelope {
    fna_apps_interface::runtime::WebhookEnvelope {
        app_id: String::from("slack"),
        ingress_id: String::from("slack_events"),
        headers: BTreeMap::from([
            (
                String::from("x-slack-request-timestamp"),
                timestamp.to_string(),
            ),
            (String::from("x-slack-signature"), String::from(signature)),
        ]),
        query: BTreeMap::new(),
        body: json!({
            "team_id": "T123",
            "event_id": "Ev123",
            "event": { "type": "app_mention" }
        })
        .to_string()
        .into_bytes(),
        received_at,
    }
}

#[test]
fn slack_component_bytes_are_a_component_binary() {
    let bytes = component_bytes();

    assert_eq!(&bytes[0..4], &[0x00, 0x61, 0x73, 0x6d]);
    assert_eq!(&bytes[4..8], &[0x0d, 0x00, 0x01, 0x00]);
}
