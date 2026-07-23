//! Host import helpers for Slack provider calls.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, Serialize)]
pub(crate) struct HostCredentialReference {
    pub(crate) app_id: String,
    pub(crate) auth_requirement_id: Option<String>,
    pub(crate) credential_kind: String,
    pub(crate) installation_id: Option<String>,
    pub(crate) user_grant_id: Option<String>,
    pub(crate) provider_account_id: Option<String>,
    pub(crate) effective_user_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct HostHttpRequest {
    pub(crate) method: String,
    pub(crate) url: String,
    pub(crate) headers: BTreeMap<String, String>,
    pub(crate) body_json: Option<Value>,
    pub(crate) credential: Option<HostCredentialReference>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct HostHttpResponse {
    pub(crate) ok: bool,
    pub(crate) status: Option<u16>,
    #[serde(default)]
    pub(crate) headers: BTreeMap<String, String>,
    pub(crate) body_json: Option<Value>,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct HostHmacSha256Request {
    pub(crate) credential: HostCredentialReference,
    pub(crate) message: String,
    pub(crate) output_encoding: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct HostHmacSha256Response {
    pub(crate) ok: bool,
    pub(crate) digest: Option<String>,
    pub(crate) error: Option<String>,
}

pub(crate) fn bot_credential(
    installation_id: &str,
    credential_kind: &str,
) -> HostCredentialReference {
    HostCredentialReference {
        app_id: String::from("slack"),
        auth_requirement_id: None,
        credential_kind: credential_kind.to_owned(),
        installation_id: Some(installation_id.to_owned()),
        user_grant_id: None,
        provider_account_id: None,
        effective_user_id: None,
    }
}

pub(crate) fn signing_credential() -> HostCredentialReference {
    HostCredentialReference {
        app_id: String::from("slack"),
        auth_requirement_id: None,
        credential_kind: String::from("signing_secret"),
        installation_id: None,
        user_grant_id: None,
        provider_account_id: None,
        effective_user_id: None,
    }
}

pub(crate) fn user_credential(
    installation_id: &str,
    effective_user_id: &str,
) -> HostCredentialReference {
    HostCredentialReference {
        app_id: String::from("slack"),
        auth_requirement_id: Some(String::from("slack_user_search")),
        credential_kind: String::from("user_token"),
        installation_id: Some(installation_id.to_owned()),
        user_grant_id: None,
        provider_account_id: None,
        effective_user_id: Some(effective_user_id.to_owned()),
    }
}

pub(crate) fn slack_post(
    endpoint: &str,
    credential: HostCredentialReference,
    body_json: Value,
) -> Result<Value, Value> {
    let request = HostHttpRequest {
        method: String::from("POST"),
        url: format!("https://slack.com/api/{endpoint}"),
        headers: BTreeMap::new(),
        body_json: Some(body_json),
        credential: Some(credential),
    };
    let response = host_http(&request)?;
    provider_body(response)
}

pub(crate) fn hmac_sha256(
    credential: HostCredentialReference,
    message: String,
) -> Result<String, Value> {
    let request = HostHmacSha256Request {
        credential,
        message,
        output_encoding: String::from("hex"),
    };
    let Ok(encoded) = serde_json::to_string(&request) else {
        return Err(host_error("invalid_hmac_request"));
    };
    let response = crate::host_hmac_sha256(&encoded);
    let Ok(response) = serde_json::from_str::<HostHmacSha256Response>(&response) else {
        return Err(host_error("invalid_hmac_response"));
    };
    if response.ok
        && let Some(digest) = response.digest
    {
        return Ok(digest);
    }
    Err(host_error(
        response.error.as_deref().unwrap_or("hmac_unavailable"),
    ))
}

fn host_http(request: &HostHttpRequest) -> Result<HostHttpResponse, Value> {
    let Ok(encoded) = serde_json::to_string(request) else {
        return Err(host_error("invalid_host_http_request"));
    };
    let response = crate::host_http_request(&encoded);
    match serde_json::from_str::<HostHttpResponse>(&response) {
        Ok(response) => Ok(response),
        Err(_) => Err(host_error("invalid_host_http_response")),
    }
}

fn provider_body(response: HostHttpResponse) -> Result<Value, Value> {
    if !response.ok {
        return Err(host_error(
            response.error.as_deref().unwrap_or("host_http_failed"),
        ));
    }
    if response.status == Some(429) {
        return Err(rate_limited(&response));
    }
    if response.status.unwrap_or(500) >= 500 {
        return Err(json!({ "ok": false, "error": "provider_unavailable" }));
    }
    let Some(body) = response.body_json.as_ref() else {
        return Err(host_error("missing_provider_body"));
    };
    if body.get("ok").and_then(Value::as_bool) == Some(false) {
        return Err(slack_provider_error(body, &response));
    }
    Ok(body.clone())
}

fn slack_provider_error(body: &Value, response: &HostHttpResponse) -> Value {
    let code = body
        .get("error")
        .and_then(Value::as_str)
        .unwrap_or("provider_error");
    match code {
        "missing_scope" => json!({
            "ok": false,
            "error": "missing_scope",
            "scope": body.get("needed").and_then(Value::as_str).unwrap_or("unknown")
        }),
        "ratelimited" | "rate_limited" => rate_limited(response),
        "not_authed" | "invalid_auth" | "account_inactive" | "token_revoked" => json!({
            "ok": false,
            "error": "auth_required",
            "auth_ids": ["slack_bot", "slack_user_search"]
        }),
        "channel_not_found" | "not_in_channel" | "is_archived" | "msg_too_long" | "no_text"
        | "invalid_arguments" => json!({
            "ok": false,
            "error": "invalid_request",
            "reason": code
        }),
        _ => json!({
            "ok": false,
            "error": "provider_unavailable"
        }),
    }
}

fn host_error(code: &str) -> Value {
    match code {
        "credential_not_found" => json!({
            "ok": false,
            "error": "auth_required",
            "auth_ids": ["slack_bot", "slack_user_search"]
        }),
        _ => json!({ "ok": false, "error": "provider_unavailable" }),
    }
}

fn rate_limited(response: &HostHttpResponse) -> Value {
    json!({
        "ok": false,
        "error": "rate_limited",
        "retry_after_seconds": response
            .headers
            .get("retry-after")
            .and_then(|value| value.parse::<u64>().ok())
    })
}
