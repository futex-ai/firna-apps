//! Slack component entrypoints.

mod host;
mod tools;
mod types;
mod webhooks;

use serde_json::json;

/// Handles one Slack app tool call.
pub(crate) fn call_tool(request: &str) -> String {
    tools::call_tool(request)
}

/// Verifies one Slack webhook envelope.
pub(crate) fn verify_webhook(request: &str) -> String {
    webhooks::verify_webhook(request)
}

/// Builds a Slack-specific webhook response.
pub(crate) fn webhook_response(request: &str) -> String {
    webhooks::webhook_response(request)
}

/// Normalizes one verified Slack event.
pub(crate) fn normalize_event(request: &str) -> String {
    webhooks::normalize_event(request)
}

pub(crate) fn encode_json(value: serde_json::Value) -> String {
    match serde_json::to_string(&value) {
        Ok(value) => value,
        Err(_) => String::from("{\"ok\":false,\"error\":\"internal\"}"),
    }
}

pub(crate) fn invalid_request(reason: &str) -> serde_json::Value {
    json!({
        "ok": false,
        "error": "invalid_request",
        "reason": reason
    })
}
