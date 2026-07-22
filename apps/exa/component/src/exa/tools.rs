//! Exa tool handlers.

use serde_json::Value;

use crate::exa::host::exa_search as host_exa_search;
use crate::exa::types::{
    AppToolCall, ExaContentsRequest, ExaMaxCharacters, ExaProviderRequest, ExaSearchInput,
    ExaTextRequest, SearchType,
};
use crate::exa::{encode_json, invalid_request};

const DEFAULT_NUM_RESULTS: u64 = 10;
const DEFAULT_HIGHLIGHTS_MAX_CHARACTERS: u64 = 4_000;
const DEFAULT_TIMEOUT_SECONDS: u64 = 60;
const MAX_RESULTS: u64 = 100;
const MAX_CHARACTERS: u64 = 20_000;
const MAX_TIMEOUT_SECONDS: u64 = 300;

pub(crate) fn call_tool(request: &str) -> String {
    let Ok(call) = serde_json::from_str::<AppToolCall>(request) else {
        return encode_json(invalid_request("invalid_tool_call"));
    };
    let result = match call.tool_name.as_str() {
        "exa_web_search" => exa_search(&call),
        _ => invalid_request("unknown_tool"),
    };
    encode_json(result)
}

fn exa_search(call: &AppToolCall) -> Value {
    let Ok(input) = serde_json::from_value::<ExaSearchInput>(call.input.clone()) else {
        return invalid_request("invalid_search_input");
    };
    let (provider_request, timeout_seconds) = match normalize_input(input) {
        Ok(request) => request,
        Err(reason) => return invalid_request(reason),
    };
    match host_exa_search(provider_request, timeout_seconds) {
        Ok(response) => response,
        Err(error) => error,
    }
}

fn normalize_input(input: ExaSearchInput) -> Result<(ExaProviderRequest, u64), &'static str> {
    let query = input.query.trim().to_owned();
    if query.is_empty() {
        return Err("empty_query");
    }
    let num_results = bounded_value(input.num_results, DEFAULT_NUM_RESULTS, MAX_RESULTS)
        .ok_or("invalid_num_results")?;
    let highlights_max_characters = bounded_value(
        input.highlights_max_characters,
        DEFAULT_HIGHLIGHTS_MAX_CHARACTERS,
        MAX_CHARACTERS,
    )
    .ok_or("invalid_highlights_max_characters")?;
    let text = match input.text_max_characters {
        Some(max_characters) if !(1..=MAX_CHARACTERS).contains(&max_characters) => {
            return Err("invalid_text_max_characters");
        }
        Some(max_characters) => Some(ExaTextRequest {
            max_characters,
            verbosity: String::from("compact"),
        }),
        None => None,
    };
    let timeout_seconds = bounded_value(
        input.timeout_seconds,
        DEFAULT_TIMEOUT_SECONDS,
        MAX_TIMEOUT_SECONDS,
    )
    .ok_or("invalid_timeout_seconds")?;
    let provider_request = ExaProviderRequest {
        query,
        search_type: input.search_type.unwrap_or(SearchType::Auto),
        num_results,
        category: input.category,
        include_domains: non_empty_strings(input.include_domains),
        exclude_domains: non_empty_strings(input.exclude_domains),
        start_published_date: trimmed_optional(input.start_published_date),
        end_published_date: trimmed_optional(input.end_published_date),
        contents: ExaContentsRequest {
            highlights: ExaMaxCharacters {
                max_characters: highlights_max_characters,
            },
            text,
            max_age_hours: input.livecrawl.unwrap_or(false).then_some(0),
        },
    };
    Ok((provider_request, timeout_seconds))
}

fn bounded_value(value: Option<u64>, default: u64, max: u64) -> Option<u64> {
    let value = value.unwrap_or(default);
    (1..=max).contains(&value).then_some(value)
}

fn non_empty_strings(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .collect()
}

fn trimmed_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
#[path = "_tests_/tools_tests.rs"]
mod tools_tests;
