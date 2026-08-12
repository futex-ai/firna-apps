//! Typed X provider response validation and stable error mapping.

use std::collections::BTreeMap;

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::x::errors::{InvalidInputReason, ToolError};
use crate::x::host::HostHttpResponse;
use crate::x::metrics_types::ProviderPostMetricsResponse;
use crate::x::types::common::ProviderErrorResponse;

pub(super) fn decode_read_response<T>(
    response: HostHttpResponse,
    required_scope: &'static str,
) -> Result<T, ToolError>
where
    T: DeserializeOwned,
{
    let body = validated_body(response, false, required_scope)?;
    match serde_json::from_value(body) {
        Ok(response) => Ok(response),
        Err(_) => Err(ToolError::ProviderResponseInvalid),
    }
}

pub(super) fn decode_write_response<T>(
    response: HostHttpResponse,
    required_scope: &'static str,
) -> Result<T, ToolError>
where
    T: DeserializeOwned,
{
    let body = validated_body(response, true, required_scope)?;
    match serde_json::from_value(body) {
        Ok(response) => Ok(response),
        Err(_) => Err(ToolError::WriteOutcomeUnknown),
    }
}

pub(super) fn decode_metrics_response(
    response: HostHttpResponse,
) -> Result<ProviderPostMetricsResponse, ToolError> {
    let body = validated_body(response, false, "tweet.read")?;
    match serde_json::from_value(body) {
        Ok(response) => Ok(response),
        Err(_) => Err(ToolError::ProviderResponseInvalid),
    }
}

fn validated_body(
    response: HostHttpResponse,
    is_write: bool,
    required_scope: &'static str,
) -> Result<Value, ToolError> {
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
            required_scope,
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
    required_scope: &'static str,
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
            scope: required_scope,
        },
        404 => ToolError::NotFound,
        429 => ToolError::RateLimited {
            retry_after_seconds: retry_after_seconds(&headers),
        },
        400..=499 => ToolError::InvalidInput(InvalidInputReason::ProviderRejectedRequest),
        500..=599 if is_write => ToolError::WriteOutcomeUnknown,
        500..=599 => ToolError::ProviderUnavailable,
        _ => response_contract_failure(is_write),
    }
}

fn retry_after_seconds(headers: &BTreeMap<String, String>) -> Option<u64> {
    headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("retry-after"))
        .and_then(|(_, value)| value.parse().ok())
}
