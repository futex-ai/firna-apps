//! Pure validation and normalization for X tool inputs.

use std::collections::HashSet;

use serde_json::Value;

use crate::x::errors::{InvalidInputReason, ToolError};
use crate::x::types::{CreatePostInput, SearchRecentPostsInput};

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
    let query = input.query.trim().to_owned();
    if query.is_empty() || query.chars().count() > 512 {
        return Err(ToolError::InvalidInput(InvalidInputReason::SearchQuery));
    }
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

pub(super) fn validate_ids(ids: &[String]) -> Result<(), ToolError> {
    let unique: HashSet<&str> = ids.iter().map(String::as_str).collect();
    if !(1..=10).contains(&ids.len())
        || unique.len() != ids.len()
        || ids.iter().any(|id| !valid_post_id(id))
    {
        return Err(ToolError::InvalidInput(InvalidInputReason::PostIds));
    }
    Ok(())
}

pub(super) fn valid_post_id(id: &str) -> bool {
    (1..=19).contains(&id.len()) && id.bytes().all(|byte| byte.is_ascii_digit())
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
