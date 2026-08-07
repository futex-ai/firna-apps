//! Shared exact-field, preview, content, and output-budget rules.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use http::Uri;
use serde_json::{Value, json};

use crate::github::error::{GitHubError, InvalidReason};
use crate::github::models::ProviderUser;

pub(crate) const OUTPUT_BUDGET_BYTES: usize = 768 * 1_024;
pub(crate) const BODY_PREVIEW_BYTES: usize = 65_536;
pub(crate) const COMMENT_PREVIEW_BYTES: usize = 8_192;
pub(crate) const DESCRIPTION_PREVIEW_BYTES: usize = 2_048;
pub(crate) const FRAGMENT_PREVIEW_BYTES: usize = 2_000;
pub(crate) const PATCH_PREVIEW_BYTES: usize = 8_192;

#[derive(Clone, Copy)]
pub(crate) enum ExactKind {
    Identifier,
    PathOrTitle,
    Url,
    EnumOrTimestamp,
}

impl ExactKind {
    fn maximum(self) -> usize {
        match self {
            Self::Identifier => 512,
            Self::PathOrTitle => 1_024,
            Self::Url => 2_048,
            Self::EnumOrTimestamp => 64,
        }
    }
}

pub(crate) fn exact(value: &str, kind: ExactKind) -> Result<String, GitHubError> {
    if value.is_empty()
        || value.len() > kind.maximum()
        || value.chars().any(char::is_control)
        || matches!(kind, ExactKind::Url) && !valid_https_url(value)
    {
        return Err(GitHubError::InvalidProviderResponse);
    }
    Ok(value.to_owned())
}

pub(crate) fn nullable_exact(
    value: Option<&str>,
    kind: ExactKind,
) -> Result<Option<String>, GitHubError> {
    value.map(|value| exact(value, kind)).transpose()
}

pub(crate) fn preview(value: &str, maximum: usize) -> Result<(String, bool), GitHubError> {
    if value
        .chars()
        .any(|character| character.is_control() && !matches!(character, '\t' | '\r' | '\n'))
    {
        return Err(GitHubError::InvalidProviderResponse);
    }
    if value.len() <= maximum {
        return Ok((value.to_owned(), false));
    }
    let mut end = maximum;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    Ok((value[..end].to_owned(), true))
}

pub(crate) fn nullable_preview(
    value: Option<&str>,
    maximum: usize,
) -> Result<(Option<String>, bool), GitHubError> {
    match value {
        Some(value) => {
            let (value, truncated) = preview(value, maximum)?;
            Ok((Some(value), truncated))
        }
        None => Ok((None, false)),
    }
}

pub(crate) fn user(user: &ProviderUser) -> Result<Value, GitHubError> {
    Ok(json!({
        "id": user.id,
        "login": exact(&user.login, ExactKind::Identifier)?,
        "html_url": exact(&user.html_url, ExactKind::Url)?
    }))
}

pub(crate) fn nullable_user(provider_user: Option<&ProviderUser>) -> Result<Value, GitHubError> {
    match provider_user {
        Some(provider_user) => user(provider_user),
        None => Ok(Value::Null),
    }
}

pub(crate) fn decode_file_content(
    encoding: &str,
    content: &str,
    declared_size: u64,
) -> Result<String, GitHubError> {
    if encoding != "base64" {
        return Err(GitHubError::InvalidRequest {
            reason: InvalidReason::UnsupportedContent,
        });
    }
    let compact = content
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect::<Vec<_>>();
    let decoded = match STANDARD.decode(compact) {
        Ok(decoded) => decoded,
        Err(_) => return Err(GitHubError::InvalidProviderResponse),
    };
    if decoded.len() > 256 * 1_024 || declared_size > 256 * 1_024 {
        return Err(GitHubError::InvalidRequest {
            reason: InvalidReason::FileTooLarge,
        });
    }
    if decoded.len() as u64 != declared_size {
        return Err(GitHubError::InvalidProviderResponse);
    }
    let decoded = match String::from_utf8(decoded) {
        Ok(decoded) => decoded,
        Err(_) => {
            return Err(GitHubError::InvalidRequest {
                reason: InvalidReason::UnsupportedContent,
            });
        }
    };
    if decoded
        .chars()
        .any(|character| character.is_control() && !matches!(character, '\t' | '\r' | '\n'))
    {
        return Err(GitHubError::InvalidRequest {
            reason: InvalidReason::UnsupportedContent,
        });
    }
    Ok(decoded)
}

pub(crate) fn ensure_budget(value: &Value) -> Result<(), GitHubError> {
    let size = match serde_json::to_vec(value) {
        Ok(encoded) => encoded.len(),
        Err(_) => return Err(GitHubError::InvalidProviderResponse),
    };
    if size > OUTPUT_BUDGET_BYTES {
        return Err(GitHubError::InvalidProviderResponse);
    }
    Ok(())
}

pub(crate) fn fits_budget(value: &Value) -> bool {
    match serde_json::to_vec(value) {
        Ok(encoded) => encoded.len() <= OUTPUT_BUDGET_BYTES,
        Err(_) => false,
    }
}

fn valid_https_url(value: &str) -> bool {
    let Ok(uri) = value.parse::<Uri>() else {
        return false;
    };
    let Some(authority) = uri.authority() else {
        return false;
    };
    uri.scheme_str() == Some("https")
        && !authority.host().is_empty()
        && !authority.as_str().contains('@')
}
