//! Host import helpers for generic HTTP calls.

use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::{Value, json};

use crate::http::types::NormalizedHttpRequest;

#[derive(Debug, Deserialize)]
pub(crate) struct HostHttpResponse {
    pub(crate) ok: bool,
    pub(crate) status: Option<u16>,
    #[serde(default)]
    pub(crate) url: Option<String>,
    #[serde(default)]
    pub(crate) headers: BTreeMap<String, String>,
    #[serde(default)]
    pub(crate) content_type: Option<String>,
    #[serde(default)]
    pub(crate) body_json: Option<Value>,
    #[serde(default)]
    pub(crate) body_truncated: bool,
    #[serde(default)]
    pub(crate) error: Option<String>,
}

pub(crate) fn send_http(request: &NormalizedHttpRequest) -> Result<HostHttpResponse, Value> {
    let Ok(encoded) = serde_json::to_string(request) else {
        return Err(host_error("invalid_host_http_request"));
    };
    let response = crate::host_http_request(&encoded);
    match serde_json::from_str::<HostHttpResponse>(&response) {
        Ok(response) => Ok(response),
        Err(_) => Err(host_error("invalid_host_http_response")),
    }
}

pub(crate) fn host_error(code: &str) -> Value {
    if is_invalid_request_host_error(code) {
        return json!({ "ok": false, "error": "invalid_request", "reason": code });
    }
    json!({ "ok": false, "error": "provider_unavailable" })
}

fn is_invalid_request_host_error(code: &str) -> bool {
    matches!(
        code,
        "invalid_host_http_request"
            | "invalid_url"
            | "host_http_scheme_denied"
            | "host_http_https_required"
            | "host_http_host_denied"
            | "host_http_capability_denied"
            | "host_http_credentials_denied"
            | "multiple_body_fields"
            | "invalid_method"
            | "invalid_timeout_seconds"
            | "credential_scope_mismatch"
            | "credential_required"
            | "credential_header_reserved"
            | "credential_header_required"
            | "invalid_credential_header"
            | "credential_header_denied"
            | "credential_header_conflict"
    )
}
