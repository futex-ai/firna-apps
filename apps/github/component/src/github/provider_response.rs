//! Provider status, structured-error, rate-limit, and JSON mapping.

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::github::error::{GitHubError, InvalidReason};
use crate::github::pagination::header;
use crate::github::provider::{Clock, ProviderResponse};

const MAX_RETRY_SECONDS: u64 = 86_400;

pub(crate) fn decode<T: DeserializeOwned>(
    response: ProviderResponse,
    clock: &dyn Clock,
    retry_input: Option<Value>,
) -> Result<(T, std::collections::BTreeMap<String, String>), GitHubError> {
    validate_status(&response, clock, retry_input)?;
    let value = response.body;
    let decoded = match serde_json::from_value::<T>(value) {
        Ok(decoded) => decoded,
        Err(_) => return Err(GitHubError::InvalidProviderResponse),
    };
    Ok((decoded, response.headers))
}

fn validate_status(
    response: &ProviderResponse,
    clock: &dyn Clock,
    retry_input: Option<Value>,
) -> Result<(), GitHubError> {
    if response.body_truncated {
        return Err(GitHubError::ProviderResponseTooLarge { retry_input });
    }
    match response.status {
        200..=299 => Ok(()),
        401 => Err(GitHubError::AuthRequired),
        403 => map_forbidden(response, clock),
        404 => Err(GitHubError::NotFoundOrNotAccessible),
        409 | 422 => Err(GitHubError::InvalidRequest {
            reason: InvalidReason::ProviderRejected,
        }),
        429 => Err(GitHubError::RateLimited {
            retry_after_seconds: retry_seconds(response, clock, false),
        }),
        500..=599 => Err(GitHubError::ProviderUnavailable),
        _ => Err(GitHubError::ProviderUnavailable),
    }
}

fn map_forbidden(response: &ProviderResponse, clock: &dyn Clock) -> Result<(), GitHubError> {
    let primary_exhausted = header(&response.headers, "x-ratelimit-remaining") == Some("0");
    let secondary = structured_error_kind(&response.body) == ProviderErrorKind::SecondaryRateLimit;
    let has_retry_after = valid_decimal_header(response, "retry-after").is_some();
    if primary_exhausted || secondary || has_retry_after {
        return Err(GitHubError::RateLimited {
            retry_after_seconds: retry_seconds(response, clock, secondary),
        });
    }
    Err(GitHubError::AccessDenied)
}

fn retry_seconds(response: &ProviderResponse, clock: &dyn Clock, secondary: bool) -> Option<u64> {
    if let Some(seconds) = valid_decimal_header(response, "retry-after") {
        return Some(seconds.min(MAX_RETRY_SECONDS));
    }
    if header(&response.headers, "x-ratelimit-remaining") == Some("0")
        && let Some(reset) = valid_decimal_header(response, "x-ratelimit-reset")
    {
        let now = clock.now_unix_seconds();
        if reset <= now {
            return secondary.then_some(60);
        }
        return Some(reset.saturating_sub(now).min(MAX_RETRY_SECONDS));
    }
    secondary.then_some(60)
}

fn valid_decimal_header(response: &ProviderResponse, name: &str) -> Option<u64> {
    let value = header(&response.headers, name)?;
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    value.parse().ok()
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ProviderErrorKind {
    SecondaryRateLimit,
    Other,
}

fn structured_error_kind(body: &Value) -> ProviderErrorKind {
    let Some(message) = body.get("message").and_then(Value::as_str) else {
        return ProviderErrorKind::Other;
    };
    let message = message.to_ascii_lowercase();
    if message.starts_with("you have exceeded a secondary rate limit")
        || message.starts_with("you have triggered an abuse detection mechanism")
    {
        ProviderErrorKind::SecondaryRateLimit
    } else {
        ProviderErrorKind::Other
    }
}
