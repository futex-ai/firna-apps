//! Signed GitHub webhook verification and lifecycle tests.

use serde_json::{Value, json};

use crate::github::webhooks::{verify_webhook_with, webhook_response};

use super::webhook_support::{
    DELIVERY, DIGEST, OTHER_DIGEST, envelope, envelope_with_headers, fixture, header,
    unused_signer, valid_verification, verify_with_digest,
};

#[test]
fn verifies_raw_sha256_signature_and_routes_numeric_identity() {
    let body = fixture("push");
    let result = verify_with_digest(
        &envelope(&body, "push", Some(&format!("sha256={DIGEST}"))),
        DIGEST,
    );

    assert_eq!(result["provider_account_id"], "2001");
    assert_eq!(result["provider_installation_id"], "1001");
    assert_eq!(result["provider_user_id"], "4001");
    assert_eq!(result["provider_event_id"], DELIVERY);
    assert_eq!(result["provider_event_type"], "push");
}

#[test]
fn rejects_unsigned_sha1_empty_malformed_and_altered_deliveries() {
    let body = fixture("push");
    let cases = [
        (envelope(&body, "push", None), "missing_github_signature"),
        (
            envelope(&body, "push", Some("sha1=0123456789abcdef")),
            "malformed_github_signature",
        ),
        (
            envelope(&body, "push", Some("")),
            "missing_github_signature",
        ),
        (
            envelope(&body, "push", Some("sha256=abc")),
            "malformed_github_signature",
        ),
    ];
    for (request, reason) in cases {
        let result: Value = serde_json::from_str(&verify_webhook_with(&request, &unused_signer()))
            .expect("error should be JSON");
        assert_eq!(result["reason"], reason);
    }

    let altered = format!("{} ", body.trim_end());
    let result = verify_with_digest(
        &envelope(&altered, "push", Some(&format!("sha256={DIGEST}"))),
        OTHER_DIGEST,
    );
    assert_eq!(result["reason"], "invalid_github_signature");
}

#[test]
fn rejects_missing_conflicting_or_malformed_trusted_headers() {
    let body = fixture("push");
    let signature = format!("sha256={DIGEST}");
    let cases = [
        (
            vec![
                header("x-hub-signature-256", &signature),
                header("x-github-event", "push"),
            ],
            "missing_github_delivery",
        ),
        (
            vec![
                header("x-hub-signature-256", &signature),
                header("x-github-delivery", "not-a-guid"),
                header("x-github-event", "push"),
            ],
            "malformed_github_delivery",
        ),
    ];
    for (headers, reason) in cases {
        let result = verify_with_digest(&envelope_with_headers(&body, headers), DIGEST);
        assert_eq!(result["reason"], reason);
    }

    let conflicting = vec![
        header("x-hub-signature-256", &signature),
        header("x-hub-signature-256", &signature),
        header("x-github-delivery", DELIVERY),
        header("x-github-event", "push"),
    ];
    let result: Value = serde_json::from_str(&verify_webhook_with(
        &envelope_with_headers(&body, conflicting),
        &unused_signer(),
    ))
    .expect("error should be JSON");
    assert_eq!(result["reason"], "conflicting_github_header");
}

#[test]
fn rejects_duplicate_signature_values_from_lossless_headers() {
    let body = fixture("push");
    let signature = format!("sha256={DIGEST}");
    let request = envelope_with_headers(
        &body,
        vec![
            header("x-hub-signature-256", &signature),
            header("x-hub-signature-256", &signature),
            header("x-github-delivery", DELIVERY),
            header("x-github-event", "push"),
        ],
    );

    let result: Value = serde_json::from_str(&verify_webhook_with(&request, &unused_signer()))
        .expect("error should be JSON");

    assert_eq!(result["reason"], "conflicting_github_header");
}

#[test]
fn rejects_signed_malformed_json_event_disagreement_and_oversized_body() {
    let invalid_json = "{not-json";
    let result = verify_with_digest(
        &envelope(invalid_json, "push", Some(&format!("sha256={DIGEST}"))),
        DIGEST,
    );
    assert_eq!(result["reason"], "invalid_webhook_json");

    let issue = fixture("issues");
    let result = verify_with_digest(
        &envelope(&issue, "pull_request", Some(&format!("sha256={DIGEST}"))),
        DIGEST,
    );
    assert_eq!(result["reason"], "github_event_type_disagreement");

    let oversized = "x".repeat(262_145);
    let result: Value = serde_json::from_str(&verify_webhook_with(
        &envelope(&oversized, "push", Some(&format!("sha256={DIGEST}"))),
        &unused_signer(),
    ))
    .expect("error should be JSON");
    assert_eq!(result["reason"], "webhook_payload_too_large");
}

#[test]
fn acknowledges_authenticated_ping_before_installation_routing() {
    let body = fixture("ping");
    let (envelope, verification) = valid_verification(&body, "ping");

    assert_eq!(verification["provider_installation_id"], Value::Null);
    let response: Value = serde_json::from_str(&webhook_response(
        &json!({"envelope": envelope, "verification": verification}).to_string(),
    ))
    .expect("ping response should be JSON");
    assert_eq!(response["status_code"], 200);
    assert_eq!(response["body"], r#"{"ok":true}"#);
}

#[test]
fn classifies_lifecycle_events_and_rejects_unsupported_events() {
    let body = fixture("installation");
    for (action, lifecycle) in [
        ("created", "reconcile"),
        ("deleted", "revoke"),
        ("suspend", "revoke"),
        ("unsuspend", "reconcile"),
        ("new_permissions_accepted", "reconcile"),
    ] {
        let mut payload: Value = serde_json::from_str(&body).expect("fixture should be JSON");
        payload["action"] = json!(action);
        let payload = payload.to_string();
        let result = verify_with_digest(
            &envelope(&payload, "installation", Some(&format!("sha256={DIGEST}"))),
            DIGEST,
        );
        assert_eq!(result["installation_lifecycle"], lifecycle);
    }

    let repositories = fixture("installation_repositories");
    let result = verify_with_digest(
        &envelope(
            &repositories,
            "installation_repositories",
            Some(&format!("sha256={DIGEST}")),
        ),
        DIGEST,
    );
    assert_eq!(result["installation_lifecycle"], "reconcile");

    let mut unsupported: Value = serde_json::from_str(&body).expect("fixture should be JSON");
    unsupported["action"] = json!("created");
    let result = verify_with_digest(
        &envelope(
            &unsupported.to_string(),
            "membership",
            Some(&format!("sha256={DIGEST}")),
        ),
        DIGEST,
    );
    assert_eq!(result["error"], "invalid_request");
    assert_eq!(result["reason"], "unsupported_github_event");
}
