//! Trusted-host request adapter for GitHub REST calls.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::github::error::GitHubError;
use crate::github::provider::{ProviderMediaType, ProviderRequest, ProviderResponse};

const API_ORIGIN: &str = "https://api.github.com";
const PROVIDER_RESPONSE_LIMIT_BYTES: usize = 1_048_576;

#[derive(Debug, Serialize)]
struct HostCredentialReference {
    app_id: String,
    auth_requirement_id: Option<String>,
    credential_kind: String,
    installation_id: Option<String>,
    user_grant_id: Option<String>,
    provider_account_id: Option<String>,
    effective_user_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct HostCredentialInjection {
    kind: String,
    header_name: Option<String>,
}

#[derive(Debug, Serialize)]
struct HostHttpRequest {
    method: String,
    url: String,
    query: BTreeMap<String, String>,
    headers: BTreeMap<String, String>,
    body_json: Option<Value>,
    body_text: Option<String>,
    timeout_seconds: Option<u64>,
    response_body_limit_bytes: Option<usize>,
    credential: Option<HostCredentialReference>,
    credential_injection: Option<HostCredentialInjection>,
}

#[derive(Debug, Deserialize)]
struct HostHttpResponse {
    ok: bool,
    status: Option<u16>,
    #[serde(default)]
    headers: BTreeMap<String, String>,
    body_json: Option<Value>,
    #[serde(default)]
    body_truncated: bool,
    #[serde(default)]
    error: Option<String>,
}

pub(crate) fn get(request: ProviderRequest) -> Result<ProviderResponse, GitHubError> {
    let accept = match request.media_type {
        ProviderMediaType::Json => "application/vnd.github+json",
        ProviderMediaType::TextMatch => "application/vnd.github.text-match+json",
    };
    let host_request = HostHttpRequest {
        method: String::from("GET"),
        url: format!("{API_ORIGIN}{}", request.path),
        query: request.query,
        headers: BTreeMap::from([
            (String::from("accept"), String::from(accept)),
            (
                String::from("x-github-api-version"),
                String::from("2026-03-10"),
            ),
            (
                String::from("user-agent"),
                String::from("Firna-GitHub-App/2.0"),
            ),
        ]),
        body_json: None,
        body_text: None,
        timeout_seconds: Some(60),
        response_body_limit_bytes: Some(PROVIDER_RESPONSE_LIMIT_BYTES),
        credential: Some(HostCredentialReference {
            app_id: String::from("github"),
            auth_requirement_id: None,
            credential_kind: String::from("installation_token"),
            installation_id: Some(request.installation_id),
            user_grant_id: None,
            provider_account_id: None,
            effective_user_id: None,
        }),
        credential_injection: Some(HostCredentialInjection {
            kind: String::from("bearer_authorization"),
            header_name: None,
        }),
    };
    let encoded = match serde_json::to_string(&host_request) {
        Ok(encoded) => encoded,
        Err(_) => return Err(GitHubError::ProviderUnavailable),
    };
    let raw = crate::host_http_request(&encoded);
    let response = match serde_json::from_str::<HostHttpResponse>(&raw) {
        Ok(response) => response,
        Err(_) => return Err(GitHubError::ProviderUnavailable),
    };
    if !response.ok {
        return match response.error.as_deref() {
            Some("credential_not_found") => Err(GitHubError::AuthRequired),
            _ => Err(GitHubError::ProviderUnavailable),
        };
    }
    let status = match response.status {
        Some(status) => status,
        None => return Err(GitHubError::ProviderUnavailable),
    };
    let body = match response.body_json {
        Some(body) => body,
        None => Value::Null,
    };
    Ok(ProviderResponse {
        status,
        headers: response.headers,
        body,
        body_truncated: response.body_truncated,
    })
}
