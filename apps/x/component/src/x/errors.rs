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
    #[error("invalid Post options")]
    PostOptions,
    #[error("invalid poll")]
    Poll,
    #[error("invalid reply target")]
    ReplyTarget,
    #[error("invalid time range")]
    TimeRange,
    #[error("invalid count query")]
    CountQuery,
    #[error("invalid user selector")]
    UserSelector,
    #[error("invalid username")]
    Username,
    #[error("invalid user query")]
    UserQuery,
    #[error("invalid user id")]
    UserId,
    #[error("invalid feed selector")]
    FeedSelector,
    #[error("invalid engagement selector")]
    EngagementSelector,
    #[error("invalid List selector")]
    ListSelector,
    #[error("invalid Space selector")]
    SpaceSelector,
    #[error("invalid Community selector")]
    CommunitySelector,
    #[error("invalid trend selector")]
    TrendSelector,
    #[error("invalid media keys")]
    MediaKeys,
    #[error("invalid media action")]
    MediaAction,
    #[error("invalid Direct Message selector")]
    DmSelector,
    #[error("invalid Post action")]
    PostAction,
    #[error("invalid relationship action")]
    RelationshipAction,
    #[error("invalid List action")]
    ListAction,
    #[error("invalid Direct Message action")]
    DmAction,
    #[error("invalid bookmark folder")]
    BookmarkFolder,
    #[error("link acknowledgement required")]
    LinkAcknowledgementRequired,
    #[error("provider rejected request")]
    ProviderRejectedRequest,
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
            Self::PostOptions => "invalid_post_options",
            Self::Poll => "invalid_poll",
            Self::ReplyTarget => "invalid_reply_target",
            Self::TimeRange => "invalid_time_range",
            Self::CountQuery => "invalid_count_query",
            Self::UserSelector => "invalid_user_selector",
            Self::Username => "invalid_username",
            Self::UserQuery => "invalid_user_query",
            Self::UserId => "invalid_user_id",
            Self::FeedSelector => "invalid_feed_selector",
            Self::EngagementSelector => "invalid_engagement_selector",
            Self::ListSelector => "invalid_list_selector",
            Self::SpaceSelector => "invalid_space_selector",
            Self::CommunitySelector => "invalid_community_selector",
            Self::TrendSelector => "invalid_trend_selector",
            Self::MediaKeys => "invalid_media_keys",
            Self::MediaAction => "invalid_media_action",
            Self::DmSelector => "invalid_dm_selector",
            Self::PostAction => "invalid_post_action",
            Self::RelationshipAction => "invalid_relationship_action",
            Self::ListAction => "invalid_list_action",
            Self::DmAction => "invalid_dm_action",
            Self::BookmarkFolder => "invalid_bookmark_folder",
            Self::LinkAcknowledgementRequired => "link_acknowledgement_required",
            Self::ProviderRejectedRequest => "provider_rejected_request",
        }
    }
}
