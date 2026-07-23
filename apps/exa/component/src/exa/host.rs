//! Host import helpers for Exa provider calls.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::exa::types::ExaProviderRequest;

const EXA_SEARCH_URL: &str = "https://api.exa.ai/search";

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
pub(crate) struct HostCredentialInjection {
    pub(crate) kind: String,
    pub(crate) header_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct HostHttpRequest {
    pub(crate) method: String,
    pub(crate) url: String,
    pub(crate) headers: BTreeMap<String, String>,
    pub(crate) body_json: Option<Value>,
    pub(crate) timeout_seconds: Option<u64>,
    pub(crate) credential: Option<HostCredentialReference>,
    pub(crate) credential_injection: Option<HostCredentialInjection>,
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

pub(crate) fn exa_search(body: ExaProviderRequest, timeout_seconds: u64) -> Result<Value, Value> {
    let request = HostHttpRequest {
        method: String::from("POST"),
        url: String::from(EXA_SEARCH_URL),
        headers: BTreeMap::new(),
        body_json: Some(body.into_value()),
        timeout_seconds: Some(timeout_seconds),
        credential: Some(api_key_credential()),
        credential_injection: Some(HostCredentialInjection {
            kind: String::from("header"),
            header_name: Some(String::from("x-api-key")),
        }),
    };
    let response = host_http(&request)?;
    provider_body(response)
}

fn api_key_credential() -> HostCredentialReference {
    HostCredentialReference {
        app_id: String::from("exa"),
        auth_requirement_id: None,
        credential_kind: String::from("api_key"),
        installation_id: None,
        user_grant_id: None,
        provider_account_id: None,
        effective_user_id: None,
    }
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
    let status = response.status.unwrap_or(500);
    if status == 429 {
        return Err(rate_limited(&response));
    }
    if status == 401 || status == 403 || status >= 500 {
        return Err(json!({ "ok": false, "error": "provider_unavailable" }));
    }
    if !(200..300).contains(&status) {
        return Err(json!({
            "ok": false,
            "error": "provider_error",
            "status": status
        }));
    }
    if !response.ok {
        return Err(host_error(
            response.error.as_deref().unwrap_or("host_http_failed"),
        ));
    }
    let mut body = match response.body_json {
        Some(Value::Object(body)) => body,
        Some(body) => serde_json::Map::from_iter([(String::from("body"), body)]),
        None => return Err(host_error("missing_provider_body")),
    };
    body.insert(String::from("provider"), json!("exa"));
    body.insert(String::from("status"), json!(response.status));
    body.insert(String::from("ok"), json!(true));
    Ok(Value::Object(body))
}

fn host_error(code: &str) -> Value {
    match code {
        "credential_not_found" | "credential_unavailable" => {
            json!({ "ok": false, "error": "provider_unavailable" })
        }
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
