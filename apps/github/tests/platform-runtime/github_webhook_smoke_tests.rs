//! Signed webhook smoke tests through the real Wasm runtime.

use std::collections::BTreeMap;
use std::sync::Arc;

use fna_apps_interface::runtime::{
    AppRuntime, ProviderInstallationLifecycle, VerifiedProviderEvent, WebhookEnvelope,
    WebhookHeader, WebhookResponseRequest,
};
use fna_apps_wasm::{HostHmacSha256Request, HostHmacSha256Response, WasmHostMock};
use serde_json::Value;
use unimock::{MockFn as _, Unimock, matching};
use uuid::Uuid;

use crate::github_runtime_support::runtime_with_host;

const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const DELIVERY: &str = "123e4567-e89b-12d3-a456-426614174000";

#[tokio::test]
async fn signed_push_verifies_and_normalizes_through_real_wasm() {
    let body = include_str!("../fixtures/webhooks/push.json")
        .as_bytes()
        .to_vec();
    let expected_body = body.clone();
    let runtime = runtime_with_host(Arc::new(Unimock::new(
        WasmHostMock::hmac_sha256
            .next_call(matching!(_))
            .answers_arc(Arc::new(move |_, request: HostHmacSha256Request| {
                assert_eq!(request.credential.app_id, "github");
                assert_eq!(request.credential.credential_kind, "webhook_secret");
                assert_eq!(request.credential.installation_id, None);
                assert_eq!(request.credential.provider_account_id, None);
                assert_eq!(request.output_encoding, "hex");
                assert_eq!(request.message.as_bytes(), expected_body);
                HostHmacSha256Response {
                    ok: true,
                    digest: Some(String::from(DIGEST)),
                    error: None,
                }
            })),
    )));
    let envelope = envelope("push", body);
    let verification = runtime.verify_webhook(envelope.clone()).await.unwrap();

    assert_eq!(verification.provider_account_id, "2001");
    assert_eq!(
        verification
            .provider_installation_id
            .as_ref()
            .unwrap()
            .as_str(),
        "1001"
    );
    assert_eq!(verification.provider_event_id, DELIVERY);
    assert_eq!(verification.provider_event_type, "push");
    let normalized = runtime
        .normalize_event(VerifiedProviderEvent {
            workspace_id: Uuid::now_v7(),
            installation_id: Uuid::now_v7(),
            envelope,
            verification,
        })
        .await
        .unwrap();
    assert_eq!(normalized.provider_event_id, DELIVERY);
    assert_eq!(normalized.provider_event_type, "push");
    assert_eq!(normalized.provider_account_id, "2001");
    assert_eq!(normalized.source["repository_id"], "3001");
    assert_eq!(normalized.payload["event"]["kind"], "push");
    assert!(normalized.payload.get("signature").is_none());
    assert!(normalized.payload.get("token").is_none());
}

#[tokio::test]
async fn remaining_declared_events_normalize_through_real_wasm() {
    let cases = [
        (
            "pull_request",
            include_str!("../fixtures/webhooks/pull_request.json"),
        ),
        (
            "pull_request_review",
            include_str!("../fixtures/webhooks/pull_request_review.json"),
        ),
        (
            "pull_request_review_comment",
            include_str!("../fixtures/webhooks/pull_request_review_comment.json"),
        ),
        ("issues", include_str!("../fixtures/webhooks/issues.json")),
        (
            "issue_comment",
            include_str!("../fixtures/webhooks/issue_comment.json"),
        ),
    ];

    for (event_type, body) in cases {
        let runtime = runtime_with_digest(DIGEST);
        let envelope = envelope(event_type, body.as_bytes().to_vec());
        let verification = runtime.verify_webhook(envelope.clone()).await.unwrap();
        assert_eq!(verification.provider_event_type, event_type);

        let normalized = runtime
            .normalize_event(VerifiedProviderEvent {
                workspace_id: Uuid::now_v7(),
                installation_id: Uuid::now_v7(),
                envelope,
                verification,
            })
            .await
            .unwrap();
        assert_eq!(normalized.payload["event"]["kind"], event_type);
    }
}

#[tokio::test]
async fn duplicate_signature_headers_fail_before_hmac() {
    let runtime = runtime_with_host(Arc::new(Unimock::new(())));
    let body = include_str!("../fixtures/webhooks/push.json")
        .as_bytes()
        .to_vec();
    let mut envelope = envelope("push", body);
    envelope.headers.push(webhook_header(
        "x-hub-signature-256",
        &format!("sha256={DIGEST}"),
    ));

    let error = runtime.verify_webhook(envelope).await.unwrap_err();
    assert!(matches!(
        error,
        fna_apps_interface::Error::RuntimeRejected { .. }
    ));
}

#[tokio::test]
async fn signed_ping_returns_preinstall_acknowledgement() {
    let body = include_str!("../fixtures/webhooks/ping.json")
        .as_bytes()
        .to_vec();
    let runtime = runtime_with_digest(DIGEST);
    let envelope = envelope("ping", body);
    let verification = runtime.verify_webhook(envelope.clone()).await.unwrap();

    assert_eq!(verification.provider_event_type, "ping");
    assert_eq!(verification.provider_installation_id, None);
    let response = runtime
        .webhook_response(WebhookResponseRequest {
            envelope,
            verification,
        })
        .await
        .unwrap()
        .unwrap();
    assert_eq!(response.status_code, 200);
    assert_eq!(response.content_type, "application/json; charset=utf-8");
    assert_eq!(
        serde_json::from_str::<Value>(&response.body).unwrap()["ok"],
        true
    );
}

#[tokio::test]
async fn lifecycle_and_invalid_signature_stay_out_of_content_delivery() {
    let body = include_str!("../fixtures/webhooks/installation_repositories.json")
        .as_bytes()
        .to_vec();
    let runtime = runtime_with_digest(DIGEST);
    let verification = runtime
        .verify_webhook(envelope("installation_repositories", body.clone()))
        .await
        .unwrap();
    assert_eq!(
        verification.installation_lifecycle,
        Some(ProviderInstallationLifecycle::Reconcile)
    );

    let invalid_runtime =
        runtime_with_digest("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    let error = invalid_runtime
        .verify_webhook(envelope("installation_repositories", body))
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        fna_apps_interface::Error::RuntimeRejected { .. }
    ));
}

fn runtime_with_digest(digest: &'static str) -> fna_apps_wasm::WasmComponentRuntime {
    runtime_with_host(Arc::new(Unimock::new(
        WasmHostMock::hmac_sha256
            .next_call(matching!(_))
            .returns(HostHmacSha256Response {
                ok: true,
                digest: Some(String::from(digest)),
                error: None,
            }),
    )))
}

fn envelope(event_type: &str, body: Vec<u8>) -> WebhookEnvelope {
    WebhookEnvelope {
        app_id: String::from("github"),
        ingress_id: String::from("github_events"),
        headers: vec![
            webhook_header("x-hub-signature-256", &format!("sha256={DIGEST}")),
            webhook_header("x-github-delivery", DELIVERY),
            webhook_header("x-github-event", event_type),
        ],
        query: BTreeMap::new(),
        body,
        received_at: "2026-08-03T12:00:00Z".parse().unwrap(),
    }
}

fn webhook_header(name: &str, value: &str) -> WebhookHeader {
    WebhookHeader {
        name: name.to_owned(),
        value: value.as_bytes().to_vec(),
    }
}
