//! Pure validation and normalization for X tool inputs.

use std::collections::HashSet;

use serde_json::Value;

use crate::x::errors::{InvalidInputReason, ToolError};
use crate::x::types::posts::{CreatePostInput, SearchRecentPostsInput};

pub(super) struct NormalizedSearch {
    pub(super) query: String,
    pub(super) max_results: u64,
    pub(super) next_token: Option<String>,
    pub(super) include_authors: bool,
}

pub(super) fn decode_input<T>(input: Value, reason: InvalidInputReason) -> Result<T, ToolError>
where
    T: serde::de::DeserializeOwned,
{
    match serde_json::from_value(input) {
        Ok(input) => Ok(input),
        Err(_) => Err(ToolError::InvalidInput(reason)),
    }
}

pub(super) fn normalize_search(
    input: SearchRecentPostsInput,
) -> Result<NormalizedSearch, ToolError> {
    let query = normalized_search_query(input.query, 512, InvalidInputReason::SearchQuery)?;
    if !(10..=25).contains(&input.max_results) {
        return Err(ToolError::InvalidInput(InvalidInputReason::SearchPageSize));
    }
    let had_next_token = input.next_token.is_some();
    let next_token = input
        .next_token
        .map(|token| token.trim().to_owned())
        .filter(|token| !token.is_empty());
    if next_token.as_ref().is_some_and(|token| token.len() > 1_024)
        || had_next_token && next_token.is_none()
    {
        return Err(ToolError::InvalidInput(InvalidInputReason::PaginationToken));
    }
    Ok(NormalizedSearch {
        query,
        max_results: input.max_results,
        next_token,
        include_authors: input.include_authors,
    })
}

pub(super) fn normalized_search_query(
    value: String,
    maximum: usize,
    reason: InvalidInputReason,
) -> Result<String, ToolError> {
    let query = trimmed_bounded(value, maximum, reason)?;
    Ok(translate_engagement_aliases(&query))
}

pub(super) fn validate_ids(ids: &[String]) -> Result<(), ToolError> {
    validate_decimal_ids(ids, 10, InvalidInputReason::PostIds)
}

pub(super) fn valid_post_id(id: &str) -> bool {
    (1..=19).contains(&id.len()) && id.bytes().all(|byte| byte.is_ascii_digit())
}

pub(super) fn validate_decimal_id(id: &str, reason: InvalidInputReason) -> Result<(), ToolError> {
    if valid_post_id(id) {
        Ok(())
    } else {
        Err(ToolError::InvalidInput(reason))
    }
}

pub(super) fn validate_decimal_ids(
    ids: &[String],
    maximum: usize,
    reason: InvalidInputReason,
) -> Result<(), ToolError> {
    let unique: HashSet<&str> = ids.iter().map(String::as_str).collect();
    if !(1..=maximum).contains(&ids.len())
        || unique.len() != ids.len()
        || ids.iter().any(|id| !valid_post_id(id))
    {
        return Err(ToolError::InvalidInput(reason));
    }
    Ok(())
}

pub(super) fn validate_page(max_results: u64) -> Result<(), ToolError> {
    if (10..=25).contains(&max_results) {
        Ok(())
    } else {
        Err(ToolError::InvalidInput(InvalidInputReason::SearchPageSize))
    }
}

pub(super) fn normalized_token(token: Option<String>) -> Result<Option<String>, ToolError> {
    let supplied = token.is_some();
    let normalized = token
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    if supplied && normalized.is_none()
        || normalized.as_ref().is_some_and(|value| value.len() > 1_024)
    {
        return Err(ToolError::InvalidInput(InvalidInputReason::PaginationToken));
    }
    Ok(normalized)
}

pub(super) fn trimmed_bounded(
    value: String,
    maximum: usize,
    reason: InvalidInputReason,
) -> Result<String, ToolError> {
    let value = value.trim().to_owned();
    if value.is_empty() || value.chars().count() > maximum {
        Err(ToolError::InvalidInput(reason))
    } else {
        Ok(value)
    }
}

pub(super) fn optional_trimmed_bounded(
    value: Option<String>,
    maximum: usize,
    reason: InvalidInputReason,
) -> Result<Option<String>, ToolError> {
    match value {
        Some(value) => Ok(Some(trimmed_bounded(value, maximum, reason)?)),
        None => Ok(None),
    }
}

fn translate_engagement_aliases(query: &str) -> String {
    let mut normalized = String::with_capacity(query.len());
    let mut quoted = false;
    let mut escaped = false;
    let mut index = 0;
    while index < query.len() {
        let remaining = &query[index..];
        if !quoted
            && at_operator_boundary(query, index)
            && starts_with_ascii_case_insensitive(remaining, "min_faves:")
        {
            normalized.push_str("min_likes:");
            index += "min_faves:".len();
            continue;
        }
        if !quoted
            && at_operator_boundary(query, index)
            && starts_with_ascii_case_insensitive(remaining, "min_retweets:")
        {
            normalized.push_str("min_reposts:");
            index += "min_retweets:".len();
            continue;
        }
        let Some(character) = remaining.chars().next() else {
            break;
        };
        normalized.push(character);
        index += character.len_utf8();
        if character == '"' && !escaped {
            quoted = !quoted;
        }
        escaped = character == '\\' && !escaped;
    }
    normalized
}

fn starts_with_ascii_case_insensitive(value: &str, prefix: &str) -> bool {
    value
        .as_bytes()
        .get(..prefix.len())
        .is_some_and(|candidate| candidate.eq_ignore_ascii_case(prefix.as_bytes()))
}

fn at_operator_boundary(query: &str, index: usize) -> bool {
    index == 0
        || query[..index]
            .chars()
            .next_back()
            .is_some_and(|character| character.is_whitespace() || character == '(')
}

pub(super) fn valid_username(username: &str) -> bool {
    (1..=50).contains(&username.len())
        && username
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

pub(super) fn valid_media_key(media_key: &str) -> bool {
    let Some((prefix, suffix)) = media_key.split_once('_') else {
        return false;
    };
    !prefix.is_empty()
        && !suffix.is_empty()
        && !suffix.contains('_')
        && prefix.bytes().all(|byte| byte.is_ascii_digit())
        && suffix.bytes().all(|byte| byte.is_ascii_digit())
}

pub(super) fn ensure_provider_count(actual: usize, maximum: usize) -> Result<(), ToolError> {
    if actual <= maximum {
        Ok(())
    } else {
        Err(ToolError::ProviderResponseInvalid)
    }
}

pub(super) fn validate_post_text(input: &CreatePostInput) -> Result<(), ToolError> {
    if input.text.trim().is_empty() || input.text.chars().count() > 280 {
        return Err(ToolError::InvalidInput(InvalidInputReason::PostText));
    }
    if !input.allow_link && contains_link(&input.text) {
        return Err(ToolError::InvalidInput(
            InvalidInputReason::LinkAcknowledgementRequired,
        ));
    }
    Ok(())
}

pub(super) fn contains_link(text: &str) -> bool {
    let lowercase = text.to_ascii_lowercase();
    lowercase.contains("http://") || lowercase.contains("https://")
}
