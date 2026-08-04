//! Host HTTP boundary for X provider calls.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub(crate) const RESPONSE_BODY_LIMIT_BYTES: u64 = 262_144;
pub(crate) const TIMEOUT_SECONDS: u64 = 30;

#[cfg_attr(test, unimock::unimock(api = [XHttpClientSendMock]))]
pub(crate) trait XHttpClient {
    fn send(&self, request: HostHttpRequest) -> HostHttpResponse;
}

pub(crate) struct ImportedXHttpClient;

impl XHttpClient for ImportedXHttpClient {
    fn send(&self, request: HostHttpRequest) -> HostHttpResponse {
        let Ok(encoded) = serde_json::to_string(&request) else {
            return HostHttpResponse::host_error("invalid_host_http_request");
        };
        let response = crate::host_http_request(&encoded);
        match serde_json::from_str(&response) {
            Ok(response) => response,
            Err(_) => HostHttpResponse::host_error("invalid_host_http_response"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct HostCredentialReference {
    pub(crate) app_id: String,
    pub(crate) credential_kind: String,
    pub(crate) installation_id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct HostCredentialInjection {
    pub(crate) kind: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub(crate) struct HostHttpRequest {
    pub(crate) method: String,
    pub(crate) url: String,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) query: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub(crate) headers: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) body_json: Option<Value>,
    pub(crate) timeout_seconds: u64,
    pub(crate) response_body_limit_bytes: u64,
    pub(crate) credential: HostCredentialReference,
    pub(crate) credential_injection: HostCredentialInjection,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub(crate) struct HostHttpResponse {
    pub(crate) ok: bool,
    pub(crate) status: Option<u16>,
    #[serde(default)]
    pub(crate) headers: BTreeMap<String, String>,
    pub(crate) body_json: Option<Value>,
    #[serde(default)]
    pub(crate) body_truncated: bool,
    pub(crate) error: Option<String>,
}

impl HostHttpResponse {
    fn host_error(error: &str) -> Self {
        Self {
            error: Some(error.to_owned()),
            ..Self::default()
        }
    }
}

pub(crate) fn request(
    method: &str,
    url: &str,
    installation_id: &str,
    query: BTreeMap<String, String>,
    body_json: Option<Value>,
) -> HostHttpRequest {
    let headers = if body_json.is_some() {
        BTreeMap::from([(
            String::from("content-type"),
            String::from("application/json"),
        )])
    } else {
        BTreeMap::new()
    };
    HostHttpRequest {
        method: method.to_owned(),
        url: url.to_owned(),
        query,
        headers,
        body_json,
        timeout_seconds: TIMEOUT_SECONDS,
        response_body_limit_bytes: RESPONSE_BODY_LIMIT_BYTES,
        credential: HostCredentialReference {
            app_id: String::from("x"),
            credential_kind: String::from("access_token"),
            installation_id: installation_id.to_owned(),
        },
        credential_injection: HostCredentialInjection {
            kind: String::from("bearer_authorization"),
        },
    }
}
