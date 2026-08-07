//! GitHub webhook component exports.

use serde::Serialize;
use serde_json::json;

use crate::github::webhook_host::{HostWebhookSigner, WebhookError, WebhookSigner};
use crate::github::webhook_projection;
use crate::github::webhook_types::{
    GitHubWebhookPayload, VerifiedProviderEvent, WebhookResponseRequest,
};
use crate::github::webhook_validation;

pub(crate) fn verify_webhook(request: &str) -> String {
    verify_webhook_with(request, &HostWebhookSigner)
}

pub(crate) fn webhook_response(request: &str) -> String {
    let Ok(request) = serde_json::from_str::<WebhookResponseRequest>(request) else {
        return encode_error(WebhookError::InvalidEnvelope);
    };
    if request.verification.provider_event_type != "ping" {
        return String::from("null");
    }
    encode(&json!({
        "status_code": 200,
        "content_type": "application/json; charset=utf-8",
        "body": "{\"ok\":true}"
    }))
}

pub(crate) fn normalize_event(request: &str) -> String {
    let verified = match serde_json::from_str::<VerifiedProviderEvent>(request) {
        Ok(verified) => verified,
        Err(_) => return encode_error(WebhookError::InvalidEnvelope),
    };
    if !webhook_validation::is_supported_content_event(&verified.verification.provider_event_type) {
        return encode_error(WebhookError::UnsupportedEvent);
    }
    let body = match serde_json::from_slice::<GitHubWebhookPayload>(&verified.envelope.body) {
        Ok(body) => body,
        Err(_) => return encode_error(WebhookError::InvalidJson),
    };
    match webhook_projection::normalize(verified, body) {
        Ok(event) => encode(&event),
        Err(error) => encode_error(error),
    }
}

pub(super) fn verify_webhook_with(request: &str, signer: &dyn WebhookSigner) -> String {
    match webhook_validation::verify(request, signer) {
        Ok(verification) => encode(&verification),
        Err(error) => encode_error(error),
    }
}

fn encode<T: Serialize>(value: &T) -> String {
    match serde_json::to_string(value) {
        Ok(encoded) => encoded,
        Err(_) => String::from(r#"{"ok":false,"error":"provider_contract_error"}"#),
    }
}

fn encode_error(error: WebhookError) -> String {
    encode(&json!({
        "ok": false,
        "error": "invalid_request",
        "reason": error.reason()
    }))
}
