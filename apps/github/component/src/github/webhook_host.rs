//! Opaque host-backed GitHub webhook signing operations.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::host_hmac_sha256;

#[derive(Clone, Copy, Debug, Error, Eq, PartialEq)]
pub(crate) enum WebhookError {
    #[error("[github/webhook] invalid webhook envelope")]
    InvalidEnvelope,
    #[error("[github/webhook] missing sha256 signature")]
    MissingSignature,
    #[error("[github/webhook] malformed sha256 signature")]
    MalformedSignature,
    #[error("[github/webhook] invalid sha256 signature")]
    InvalidSignature,
    #[error("[github/webhook] webhook HMAC unavailable")]
    HmacUnavailable,
    #[error("[github/webhook] invalid webhook body encoding")]
    InvalidBodyEncoding,
    #[error("[github/webhook] invalid webhook JSON")]
    InvalidJson,
    #[error("[github/webhook] webhook payload exceeded its limit")]
    PayloadTooLarge,
    #[error("[github/webhook] missing delivery identifier")]
    MissingDelivery,
    #[error("[github/webhook] malformed delivery identifier")]
    MalformedDelivery,
    #[error("[github/webhook] missing event type")]
    MissingEventType,
    #[error("[github/webhook] malformed event type")]
    MalformedEventType,
    #[error("[github/webhook] conflicting trusted header")]
    ConflictingHeader,
    #[error("[github/webhook] event header disagrees with payload")]
    EventTypeDisagreement,
    #[error("[github/webhook] missing provider installation")]
    MissingInstallation,
    #[error("[github/webhook] missing provider account")]
    MissingAccount,
    #[error("[github/webhook] missing repository")]
    MissingRepository,
    #[error("[github/webhook] unsupported normalized event")]
    UnsupportedEvent,
}

impl WebhookError {
    pub(crate) fn reason(self) -> &'static str {
        match self {
            Self::InvalidEnvelope => "invalid_webhook_envelope",
            Self::MissingSignature => "missing_github_signature",
            Self::MalformedSignature => "malformed_github_signature",
            Self::InvalidSignature => "invalid_github_signature",
            Self::HmacUnavailable => "github_hmac_unavailable",
            Self::InvalidBodyEncoding => "invalid_webhook_body",
            Self::InvalidJson => "invalid_webhook_json",
            Self::PayloadTooLarge => "webhook_payload_too_large",
            Self::MissingDelivery => "missing_github_delivery",
            Self::MalformedDelivery => "malformed_github_delivery",
            Self::MissingEventType => "missing_github_event",
            Self::MalformedEventType => "malformed_github_event",
            Self::ConflictingHeader => "conflicting_github_header",
            Self::EventTypeDisagreement => "github_event_type_disagreement",
            Self::MissingInstallation => "missing_github_installation",
            Self::MissingAccount => "missing_github_account",
            Self::MissingRepository => "missing_github_repository",
            Self::UnsupportedEvent => "unsupported_github_event",
        }
    }
}

#[derive(Debug, Serialize)]
struct HostCredentialReference {
    app_id: String,
    credential_kind: String,
    installation_id: Option<String>,
    user_grant_id: Option<String>,
    provider_account_id: Option<String>,
    effective_user_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct HostHmacSha256Request {
    credential: HostCredentialReference,
    message: String,
    output_encoding: String,
}

#[derive(Debug, Deserialize)]
struct HostHmacSha256Response {
    ok: bool,
    digest: Option<String>,
}

#[cfg_attr(test, unimock::unimock(api = [WebhookSignerDigest]))]
pub(crate) trait WebhookSigner {
    fn digest(&self, message: &str) -> Result<String, WebhookError>;
}

pub(crate) struct HostWebhookSigner;

impl WebhookSigner for HostWebhookSigner {
    fn digest(&self, message: &str) -> Result<String, WebhookError> {
        let request = HostHmacSha256Request {
            credential: HostCredentialReference {
                app_id: String::from("github"),
                credential_kind: String::from("webhook_secret"),
                installation_id: None,
                user_grant_id: None,
                provider_account_id: None,
                effective_user_id: None,
            },
            message: message.to_owned(),
            output_encoding: String::from("hex"),
        };
        let encoded = serde_json::to_string(&request).or(Err(WebhookError::HmacUnavailable))?;
        let raw = host_hmac_sha256(&encoded);
        let response = serde_json::from_str::<HostHmacSha256Response>(&raw)
            .or(Err(WebhookError::HmacUnavailable))?;
        match (response.ok, response.digest) {
            (true, Some(digest)) => Ok(digest),
            _ => Err(WebhookError::HmacUnavailable),
        }
    }
}
