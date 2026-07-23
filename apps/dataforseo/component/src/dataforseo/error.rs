//! Stable redacted errors returned across the component boundary.

use serde_json::{Value, json};

#[derive(Debug, thiserror::Error)]
pub(super) enum Error {
    #[error("[dataforseo/error] invalid tool request: {0}")]
    InvalidRequest(&'static str),
    #[error("[dataforseo/error] provider authentication failed")]
    ProviderAuthenticationFailed(Option<i64>),
    #[error("[dataforseo/error] provider access denied")]
    ProviderAccessDenied(Option<i64>),
    #[error("[dataforseo/error] provider budget exhausted")]
    ProviderBudgetExhausted(Option<i64>),
    #[error("[dataforseo/error] provider rate limited the request")]
    RateLimited {
        provider_code: Option<i64>,
        retry_after_seconds: Option<u64>,
    },
    #[error("[dataforseo/error] provider is unavailable")]
    ProviderUnavailable(Option<i64>),
    #[error("[dataforseo/error] provider response exceeded the read limit")]
    ProviderResponseTooLarge,
    #[error("[dataforseo/error] fixed provider endpoint contract failed")]
    ProviderContract,
}

pub(super) type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub(super) fn into_output(self) -> Value {
        match self {
            Self::InvalidRequest(reason) => {
                json!({ "ok": false, "error": "invalid_request", "reason": reason })
            }
            Self::ProviderAuthenticationFailed(provider_code) => {
                provider_error("provider_authentication_failed", provider_code, None)
            }
            Self::ProviderAccessDenied(provider_code) => {
                provider_error("provider_access_denied", provider_code, None)
            }
            Self::ProviderBudgetExhausted(provider_code) => {
                provider_error("provider_budget_exhausted", provider_code, None)
            }
            Self::RateLimited {
                provider_code,
                retry_after_seconds,
            } => provider_error("rate_limited", provider_code, retry_after_seconds),
            Self::ProviderUnavailable(provider_code) => {
                provider_error("provider_unavailable", provider_code, None)
            }
            Self::ProviderResponseTooLarge => {
                json!({ "ok": false, "error": "provider_response_too_large" })
            }
            Self::ProviderContract => {
                json!({ "ok": false, "error": "provider_contract_error" })
            }
        }
    }
}

fn provider_error(code: &str, provider_code: Option<i64>, retry: Option<u64>) -> Value {
    json!({
        "ok": false,
        "error": code,
        "provider_code": provider_code,
        "retry_after_seconds": retry,
    })
}
