//! Stable GitHub component error contract.

use serde::Serialize;
use serde_json::{Value, json};
use thiserror::Error;

#[derive(Clone, Copy, Debug, Error, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum InvalidReason {
    #[error("[github/error] invalid tool call")]
    InvalidToolCall,
    #[error("[github/error] unknown tool")]
    UnknownTool,
    #[error("[github/error] invalid owner")]
    InvalidOwner,
    #[error("[github/error] invalid repository")]
    InvalidRepository,
    #[error("[github/error] invalid path")]
    InvalidPath,
    #[error("[github/error] invalid ref")]
    InvalidRef,
    #[error("[github/error] invalid query")]
    InvalidQuery,
    #[error("[github/error] invalid language")]
    InvalidLanguage,
    #[error("[github/error] repository qualifier requires owner")]
    RepositoryQualifierRequiresOwner,
    #[error("[github/error] owner qualifier requires repository")]
    OwnerQualifierRequiresRepository,
    #[error("[github/error] invalid number")]
    InvalidNumber,
    #[error("[github/error] invalid page")]
    InvalidPage,
    #[error("[github/error] invalid page size")]
    InvalidPageSize,
    #[error("[github/error] result window exceeded")]
    ResultWindowExceeded,
    #[error("[github/error] provider rejected the request")]
    ProviderRejected,
    #[error("[github/error] unsupported content")]
    UnsupportedContent,
    #[error("[github/error] file too large")]
    FileTooLarge,
}

#[derive(Debug, Error)]
pub(crate) enum GitHubError {
    #[error("[github/error] authentication required")]
    AuthRequired,
    #[error("[github/error] invalid request")]
    InvalidRequest { reason: InvalidReason },
    #[error("[github/error] provider rate limited the request")]
    RateLimited { retry_after_seconds: Option<u64> },
    #[error("[github/error] access denied")]
    AccessDenied,
    #[error("[github/error] resource not found or inaccessible")]
    NotFoundOrNotAccessible,
    #[error("[github/error] provider response exceeded its limit")]
    ProviderResponseTooLarge { retry_input: Option<Value> },
    #[error("[github/error] provider response was invalid")]
    InvalidProviderResponse,
    #[error("[github/error] provider is unavailable")]
    ProviderUnavailable,
    #[error("[github/error] issue number belongs to a pull request")]
    UseGitHubReadPr,
}

impl GitHubError {
    pub(crate) fn into_value(self) -> Value {
        match self {
            Self::AuthRequired => json!({
                "ok": false,
                "error": "auth_required",
                "auth_ids": ["github_installation"]
            }),
            Self::InvalidRequest { reason } => json!({
                "ok": false,
                "error": "invalid_request",
                "reason": reason
            }),
            Self::RateLimited {
                retry_after_seconds,
            } => json!({
                "ok": false,
                "error": "rate_limited",
                "retry_after_seconds": retry_after_seconds
            }),
            Self::AccessDenied => json!({ "ok": false, "error": "access_denied" }),
            Self::NotFoundOrNotAccessible => {
                json!({ "ok": false, "error": "not_found_or_not_accessible" })
            }
            Self::ProviderResponseTooLarge { retry_input } => json!({
                "ok": false,
                "error": "provider_response_too_large",
                "retry_input": retry_input
            }),
            Self::InvalidProviderResponse => {
                json!({ "ok": false, "error": "invalid_provider_response" })
            }
            Self::ProviderUnavailable => {
                json!({ "ok": false, "error": "provider_unavailable" })
            }
            Self::UseGitHubReadPr => {
                json!({ "ok": false, "error": "use_github_read_pr" })
            }
        }
    }
}
