//! Typed host HTTP boundary for one DataForSEO Live request.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::error::{Error, Result};

const API_ORIGIN: &str = "https://api.dataforseo.com";
const RESPONSE_LIMIT_BYTES: u64 = 1_048_576;

#[derive(Debug, Serialize)]
struct HostCredentialReference {
    app_id: &'static str,
    auth_requirement_id: Option<&'static str>,
    credential_kind: &'static str,
    installation_id: Option<String>,
    user_grant_id: Option<String>,
    provider_account_id: Option<String>,
    effective_user_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct HostCredentialInjection {
    kind: &'static str,
    header_name: Option<String>,
    username_credential: HostCredentialReference,
    password_credential: HostCredentialReference,
}

#[derive(Debug, Serialize)]
struct HostHttpRequest {
    method: &'static str,
    url: String,
    headers: BTreeMap<String, String>,
    body_json: Value,
    timeout_seconds: Option<u64>,
    response_body_limit_bytes: Option<u64>,
    credential: Option<HostCredentialReference>,
    credential_injection: HostCredentialInjection,
}

#[derive(Clone, Debug, Deserialize)]
pub(super) struct HostHttpResponse {
    pub(super) ok: bool,
    pub(super) status: Option<u16>,
    #[serde(default)]
    pub(super) headers: BTreeMap<String, String>,
    pub(super) body_json: Option<Value>,
    #[serde(default)]
    pub(super) body_truncated: bool,
}

#[cfg_attr(test, unimock::unimock(api = [ProviderClientPostTask]))]
pub(super) trait ProviderClient {
    fn post_task(&self, path: &str, task: Value, timeout_seconds: u64) -> Result<HostHttpResponse>;
}

pub(super) struct WasmProviderClient<'a> {
    installation_id: &'a str,
}

impl<'a> WasmProviderClient<'a> {
    pub(super) fn new(installation_id: &'a str) -> Self {
        Self { installation_id }
    }
}

impl ProviderClient for WasmProviderClient<'_> {
    fn post_task(&self, path: &str, task: Value, timeout_seconds: u64) -> Result<HostHttpResponse> {
        let request = HostHttpRequest {
            method: "POST",
            url: format!("{API_ORIGIN}{path}"),
            headers: BTreeMap::new(),
            body_json: Value::Array(vec![task]),
            timeout_seconds: Some(timeout_seconds),
            response_body_limit_bytes: Some(RESPONSE_LIMIT_BYTES),
            credential: None,
            credential_injection: HostCredentialInjection {
                kind: "basic_authorization",
                header_name: None,
                username_credential: credential(self.installation_id, "login"),
                password_credential: credential(self.installation_id, "password"),
            },
        };
        let encoded = match serde_json::to_string(&request) {
            Ok(encoded) => encoded,
            Err(_) => return Err(Error::InvalidRequest("host_request_serialization_failed")),
        };
        let response = crate::host_http_request(&encoded);
        match serde_json::from_str(&response) {
            Ok(response) => Ok(response),
            Err(_) => Err(Error::ProviderUnavailable(None)),
        }
    }
}

fn credential(installation_id: &str, credential_kind: &'static str) -> HostCredentialReference {
    HostCredentialReference {
        app_id: "dataforseo",
        auth_requirement_id: None,
        credential_kind,
        installation_id: Some(installation_id.to_owned()),
        user_grant_id: None,
        provider_account_id: None,
        effective_user_id: None,
    }
}
