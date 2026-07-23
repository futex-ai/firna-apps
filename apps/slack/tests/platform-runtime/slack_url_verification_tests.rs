use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::Utc;
use fna_apps_interface::runtime::{AppRuntime, WebhookResponseRequest};
use fna_apps_wasm::{HostHmacSha256Response, WasmHostMock};
use serde_json::json;
use unimock::{MockFn as _, Unimock, matching};

use crate::slack_runtime_support::runtime_with_host;

#[tokio::test]
async fn slack_component_verifies_url_challenge_without_team_id() {
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
    let body = json!({
        "token": "deprecated-token",
        "challenge": "challenge-token",
        "type": "url_verification"
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

    assert_eq!(verification.provider_account_id, "url_verification");
    assert_eq!(verification.provider_event_id, "challenge-token");
    assert_eq!(verification.provider_event_type, "url_verification");
}

#[tokio::test]
async fn slack_component_returns_url_verification_challenge_response() {
    let runtime = runtime_with_host(Arc::new(Unimock::new(())));
    let envelope = fna_apps_interface::runtime::WebhookEnvelope {
        app_id: String::from("slack"),
        ingress_id: String::from("slack_events"),
        headers: BTreeMap::new(),
        query: BTreeMap::new(),
        body: json!({
            "type": "url_verification",
            "team_id": "T123",
            "challenge": "challenge-token"
        })
        .to_string()
        .into_bytes(),
        received_at: Utc::now(),
    };
    let response = runtime
        .webhook_response(WebhookResponseRequest {
            envelope,
            verification: fna_apps_interface::runtime::WebhookVerificationResult {
                provider_account_id: String::from("T123"),
                provider_event_id: String::from("challenge-token"),
                provider_event_type: String::from("url_verification"),
                provider_user_id: None,
            },
        })
        .await
        .unwrap()
        .unwrap();

    assert_eq!(response.status_code, 200);
    assert_eq!(response.body, "challenge-token");
}
