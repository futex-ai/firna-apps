//! Centralized GitHub input validation and encoding.

use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};

use crate::github::error::{GitHubError, InvalidReason};

const MAX_I32: i64 = 2_147_483_647;
const MAX_FILE_PATH_SEGMENTS: usize = 16;

pub(crate) fn owner(value: &str) -> Result<String, GitHubError> {
    if value.is_empty()
        || value.len() > 100
        || value.starts_with('-')
        || value.ends_with('-')
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return invalid(InvalidReason::InvalidOwner);
    }
    Ok(value.to_owned())
}

pub(crate) fn repository(value: &str) -> Result<String, GitHubError> {
    if value.is_empty()
        || value.len() > 100
        || matches!(value, "." | "..")
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return invalid(InvalidReason::InvalidRepository);
    }
    Ok(value.to_owned())
}

pub(crate) fn path(value: &str) -> Result<String, GitHubError> {
    if value.is_empty()
        || value.len() > 1_024
        || value.starts_with('/')
        || has_control(value)
        || value.split('/').count() > MAX_FILE_PATH_SEGMENTS
        || value
            .split('/')
            .any(|segment| segment.is_empty() || matches!(segment, "." | ".."))
    {
        return invalid(InvalidReason::InvalidPath);
    }
    Ok(value.to_owned())
}

pub(crate) fn git_ref(value: Option<String>) -> Result<Option<String>, GitHubError> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_empty() || value.len() > 255 || has_control(&value) {
        return invalid(InvalidReason::InvalidRef);
    }
    Ok(Some(value))
}

pub(crate) fn search_term(value: &str) -> Result<String, GitHubError> {
    bounded_trimmed_scalars(value, 256, InvalidReason::InvalidQuery)
}

pub(crate) fn language(value: Option<String>) -> Result<Option<String>, GitHubError> {
    match value {
        Some(value) => {
            let value = bounded_trimmed_scalars(&value, 100, InvalidReason::InvalidLanguage)?;
            Ok(Some(value))
        }
        None => Ok(None),
    }
}

pub(crate) fn search_path(value: Option<String>) -> Result<Option<String>, GitHubError> {
    match value {
        Some(value) if value.is_empty() || value.len() > 256 || has_control(&value) => {
            invalid(InvalidReason::InvalidPath)
        }
        value => Ok(value),
    }
}

pub(crate) fn positive_number(value: i64) -> Result<u32, GitHubError> {
    if !(1..=MAX_I32).contains(&value) {
        return invalid(InvalidReason::InvalidNumber);
    }
    Ok(value as u32)
}

pub(crate) fn page(value: Option<i64>) -> Result<u32, GitHubError> {
    let value = value.unwrap_or(1);
    if !(1..=MAX_I32).contains(&value) {
        return invalid(InvalidReason::InvalidPage);
    }
    Ok(value as u32)
}

pub(crate) fn page_size(
    value: Option<i64>,
    default: u32,
    maximum: u32,
) -> Result<u32, GitHubError> {
    let value = value.unwrap_or(i64::from(default));
    if !(1..=i64::from(maximum)).contains(&value) {
        return invalid(InvalidReason::InvalidPageSize);
    }
    Ok(value as u32)
}

pub(crate) fn result_offset(page: u32, per_page: u32) -> Result<u64, GitHubError> {
    u64::from(page - 1)
        .checked_mul(u64::from(per_page))
        .ok_or(GitHubError::InvalidRequest {
            reason: InvalidReason::ResultWindowExceeded,
        })
}

pub(crate) fn encoded_segment(value: &str) -> String {
    utf8_percent_encode(value, NON_ALPHANUMERIC).to_string()
}

pub(crate) fn encoded_path(value: &str) -> String {
    value
        .split('/')
        .map(encoded_segment)
        .collect::<Vec<_>>()
        .join("/")
}

pub(crate) fn quoted_search_literal(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');
    for character in value.chars() {
        if matches!(character, '\\' | '"') {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped.push('"');
    escaped
}

fn bounded_trimmed_scalars(
    value: &str,
    maximum: usize,
    reason: InvalidReason,
) -> Result<String, GitHubError> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > maximum || has_control(value) {
        return invalid(reason);
    }
    Ok(value.to_owned())
}

fn has_control(value: &str) -> bool {
    value.chars().any(char::is_control)
}

fn invalid<T>(reason: InvalidReason) -> Result<T, GitHubError> {
    Err(GitHubError::InvalidRequest { reason })
}
