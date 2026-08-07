//! Shared fixture and signer support for webhook tests.

use std::fs;
use std::path::PathBuf;

use serde_json::{Value, json};
use unimock::{MockFn as _, Unimock, matching};

use crate::github::webhook_host::WebhookSignerDigest;
use crate::github::webhook_types::WebhookHeader;
use crate::github::webhooks::verify_webhook_with;

pub(super) const DIGEST: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
pub(super) const OTHER_DIGEST: &str =
    "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
pub(super) const DELIVERY: &str = "01234567-89ab-cdef-0123-456789abcdef";

pub(super) fn fixture(name: &str) -> String {
    fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../tests/fixtures/webhooks")
            .join(format!("{name}.json")),
    )
    .expect("webhook fixture should exist")
}

pub(super) fn envelope(body: &str, event_type: &str, signature: Option<&str>) -> String {
    let mut headers = vec![
        header("x-github-delivery", DELIVERY),
        header("x-github-event", event_type),
    ];
    if let Some(signature) = signature {
        headers.push(header("x-hub-signature-256", signature));
    }
    envelope_with_headers(body, headers)
}

pub(super) fn envelope_with_headers(body: &str, headers: Vec<WebhookHeader>) -> String {
    json!({
        "app_id": "github",
        "ingress_id": "github_events",
        "headers": headers,
        "body": body.as_bytes(),
        "received_at": "2026-08-03T12:00:00Z"
    })
    .to_string()
}

pub(super) fn header(name: &str, value: &str) -> WebhookHeader {
    WebhookHeader {
        name: name.to_owned(),
        value: value.as_bytes().to_vec(),
    }
}

pub(super) fn verify_with_digest(request: &str, digest: &str) -> Value {
    let signer = Unimock::new(
        WebhookSignerDigest
            .each_call(matching!(_))
            .returns(Ok(digest.to_owned())),
    );
    serde_json::from_str(&verify_webhook_with(request, &signer))
        .expect("verification output should be JSON")
}

pub(super) fn valid_verification(body: &str, event_type: &str) -> (Value, Value) {
    let request = envelope(body, event_type, Some(&format!("sha256={DIGEST}")));
    let verification = verify_with_digest(&request, DIGEST);
    let envelope = serde_json::from_str(&request).expect("envelope should be JSON");
    (envelope, verification)
}

pub(super) fn unused_signer() -> Unimock {
    Unimock::new(())
}
