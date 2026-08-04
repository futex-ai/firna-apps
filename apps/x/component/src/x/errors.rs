//! Stable errors returned by X tools.

use serde::Serialize;
use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub(crate) enum ToolError {
    #[error("[fna_app_x_component/errors] invalid X tool input")]
    InvalidInput(InvalidInputReason),
    #[error("[fna_app_x_component/errors] X authorization is required")]
    AuthRequired,
    #[error("[fna_app_x_component/errors] X authorization lacks a required scope")]
    InsufficientScope { scope: &'static str },
    #[error("[fna_app_x_component/errors] requested X posts were not found")]
    NotFound,
    #[error("[fna_app_x_component/errors] X rate limited the request")]
    RateLimited { retry_after_seconds: Option<u64> },
    #[error("[fna_app_x_component/errors] X credit or usage limit is exhausted")]
    CreditExhausted,
    #[error("[fna_app_x_component/errors] X provider is unavailable")]
    ProviderUnavailable,
    #[error("[fna_app_x_component/errors] X returned an invalid response")]
    ProviderResponseInvalid,
    #[error("[fna_app_x_component/errors] X write outcome is unknown")]
    WriteOutcomeUnknown,
}

#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub(crate) enum InvalidInputReason {
    #[error("malformed tool call")]
    MalformedToolCall,
    #[error("unknown tool")]
    UnknownTool,
    #[error("invalid post ids")]
    PostIds,
    #[error("invalid search query")]
    SearchQuery,
    #[error("invalid search page size")]
    SearchPageSize,
    #[error("invalid search pagination token")]
    PaginationToken,
    #[error("invalid post text")]
    PostText,
    #[error("invalid reply target")]
    ReplyTarget,
    #[error("link acknowledgement required")]
    LinkAcknowledgementRequired,
}

#[derive(Serialize)]
pub(crate) struct ErrorEnvelope {
    ok: bool,
    #[serde(rename = "error")]
    error: ErrorCode,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    auth_ids: Vec<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scope: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    retry_after_seconds: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum ErrorCode {
    InvalidRequest,
    AuthRequired,
    MissingScope,
    NotFound,
    RateLimited,
    ProviderBudgetExhausted,
    ProviderUnavailable,
    ProviderContractError,
    WriteOutcomeUnknown,
}

impl ToolError {
    pub(crate) fn envelope(self) -> ErrorEnvelope {
        match self {
            Self::InvalidInput(reason) => ErrorEnvelope {
                reason: Some(reason.code()),
                ..ErrorEnvelope::new(ErrorCode::InvalidRequest)
            },
            Self::AuthRequired => ErrorEnvelope {
                auth_ids: vec!["x_workspace"],
                ..ErrorEnvelope::new(ErrorCode::AuthRequired)
            },
            Self::InsufficientScope { scope } => ErrorEnvelope {
                scope: Some(scope),
                ..ErrorEnvelope::new(ErrorCode::MissingScope)
            },
            Self::NotFound => ErrorEnvelope::new(ErrorCode::NotFound),
            Self::RateLimited {
                retry_after_seconds,
            } => ErrorEnvelope {
                retry_after_seconds,
                ..ErrorEnvelope::new(ErrorCode::RateLimited)
            },
            Self::CreditExhausted => ErrorEnvelope::new(ErrorCode::ProviderBudgetExhausted),
            Self::ProviderUnavailable => ErrorEnvelope::new(ErrorCode::ProviderUnavailable),
            Self::ProviderResponseInvalid => ErrorEnvelope::new(ErrorCode::ProviderContractError),
            Self::WriteOutcomeUnknown => ErrorEnvelope::new(ErrorCode::WriteOutcomeUnknown),
        }
    }
}

impl ErrorEnvelope {
    fn new(error: ErrorCode) -> Self {
        Self {
            ok: false,
            error,
            auth_ids: Vec::new(),
            reason: None,
            scope: None,
            retry_after_seconds: None,
        }
    }
}

impl InvalidInputReason {
    fn code(self) -> &'static str {
        match self {
            Self::MalformedToolCall => "malformed_tool_call",
            Self::UnknownTool => "unknown_tool",
            Self::PostIds => "invalid_post_ids",
            Self::SearchQuery => "invalid_search_query",
            Self::SearchPageSize => "invalid_search_page_size",
            Self::PaginationToken => "invalid_pagination_token",
            Self::PostText => "invalid_post_text",
            Self::ReplyTarget => "invalid_reply_target",
            Self::LinkAcknowledgementRequired => "link_acknowledgement_required",
        }
    }
}
