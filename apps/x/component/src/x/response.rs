//! Typed X provider response validation and stable error mapping.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::x::errors::ToolError;
use crate::x::host::HostHttpResponse;
use crate::x::types::{ProviderCreateResponse, ProviderErrorResponse, ProviderReadResponse};

pub(super) fn decode_read_response(
    response: HostHttpResponse,
) -> Result<ProviderReadResponse, ToolError> {
    let body = validated_body(response, false)?;
    match serde_json::from_value(body) {
        Ok(response) => Ok(response),
        Err(_) => Err(ToolError::ProviderResponseInvalid),
    }
}

pub(super) fn decode_create_response(
    response: HostHttpResponse,
) -> Result<ProviderCreateResponse, ToolError> {
    let body = validated_body(response, true)?;
    match serde_json::from_value(body) {
        Ok(response) => Ok(response),
        Err(_) => Err(ToolError::WriteOutcomeUnknown),
    }
}

fn validated_body(response: HostHttpResponse, is_write: bool) -> Result<Value, ToolError> {
    if response.body_truncated {
        return Err(response_contract_failure(is_write));
    }
    if !response.ok {
        return Err(host_failure(response.error.as_deref(), is_write));
    }
    let Some(status) = response.status else {
        return Err(response_contract_failure(is_write));
    };
    if !(200..300).contains(&status) {
        return Err(provider_failure(
            status,
            response.headers,
            response.body_json,
            is_write,
        ));
    }
    response
        .body_json
        .ok_or(response_contract_failure(is_write))
}

fn response_contract_failure(is_write: bool) -> ToolError {
    if is_write {
        ToolError::WriteOutcomeUnknown
    } else {
        ToolError::ProviderResponseInvalid
    }
}

fn host_failure(error: Option<&str>, is_write: bool) -> ToolError {
    match error {
        Some("credential_not_found" | "credential_unavailable" | "auth_required") => {
            ToolError::AuthRequired
        }
        _ if is_write => ToolError::WriteOutcomeUnknown,
        _ => ToolError::ProviderUnavailable,
    }
}

fn provider_failure(
    status: u16,
    headers: BTreeMap<String, String>,
    body: Option<Value>,
    is_write: bool,
) -> ToolError {
    let provider_error = body
        .and_then(|body| serde_json::from_value::<ProviderErrorResponse>(body).ok())
        .unwrap_or_default();
    if provider_error.is_usage_capped() {
        return ToolError::CreditExhausted;
    }
    match status {
        401 => ToolError::AuthRequired,
        403 => ToolError::InsufficientScope {
            scope: if is_write {
                "tweet.write"
            } else {
                "tweet.read"
            },
        },
        404 => ToolError::NotFound,
        429 => ToolError::RateLimited {
            retry_after_seconds: retry_after_seconds(&headers),
        },
        500..=599 if is_write => ToolError::WriteOutcomeUnknown,
        _ => ToolError::ProviderUnavailable,
    }
}

fn retry_after_seconds(headers: &BTreeMap<String, String>) -> Option<u64> {
    headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("retry-after"))
        .and_then(|(_, value)| value.parse().ok())
}
