use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::Utc;
use fna_apps_interface::runtime::AppRuntime;
use fna_apps_wasm::{HostHmacSha256Response, WasmHostMock};
use serde_json::json;
use unimock::{MockFn as _, Unimock, matching};

use crate::slack_runtime_support::runtime_with_host;

#[tokio::test]
async fn slack_component_maps_message_channel_types_to_events_api_names() {
    let runtime = runtime_with_host(Arc::new(Unimock::new(
        WasmHostMock::hmac_sha256
            .each_call(matching!(_))
            .returns(HostHmacSha256Response {
                ok: true,
                digest: Some(String::from("digest")),
                error: None,
            }),
    )));
    let cases = [
        ("channel", "message.channels"),
        ("group", "message.groups"),
        ("im", "message.im"),
        ("mpim", "message.mpim"),
    ];

    for (channel_type, expected_type) in cases {
        let now = Utc::now();
        let body = json!({
            "team_id": "T123",
            "event_id": format!("Ev-{channel_type}"),
            "event": {
                "type": "message",
                "channel_type": channel_type,
                "channel": "C123",
                "user": "U123",
                "text": "hello",
                "ts": "1710000000.000100"
            }
        })
        .to_string();
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
                body: body.into_bytes(),
                received_at: now,
            })
            .await
            .unwrap();

        assert_eq!(verification.provider_event_type, expected_type);
    }
}
