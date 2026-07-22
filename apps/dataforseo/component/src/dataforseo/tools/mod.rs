//! Closed dispatch for the 16 reviewed DataForSEO operations.

mod ai;
mod backlinks;
mod content_domain;
mod page_business;
mod search;

use serde::de::DeserializeOwned;
use serde_json::Value;

use super::error::{Error, Result};
use super::host::ProviderClient;

pub(super) fn call(client: &dyn ProviderClient, tool_name: &str, input: Value) -> Result<Value> {
    match tool_name {
        "dataforseo_google_serp" => search::google_serp(client, input),
        "dataforseo_keyword_overview" => search::keyword_overview(client, input),
        "dataforseo_keyword_suggestions" => search::keyword_suggestions(client, input),
        "dataforseo_ranked_keywords" => search::ranked_keywords(client, input),
        "dataforseo_backlinks_summary" => backlinks::summary(client, input),
        "dataforseo_backlinks" => backlinks::backlinks(client, input),
        "dataforseo_referring_domains" => backlinks::referring_domains(client, input),
        "dataforseo_instant_page_audit" => page_business::instant_page_audit(client, input),
        "dataforseo_business_search" => page_business::business_search(client, input),
        "dataforseo_business_info" => page_business::business_info(client, input),
        "dataforseo_content_search" => content_domain::content_search(client, input),
        "dataforseo_content_sentiment" => content_domain::content_sentiment(client, input),
        "dataforseo_domain_technologies" => content_domain::domain_technologies(client, input),
        "dataforseo_domain_whois" => content_domain::domain_whois(client, input),
        "dataforseo_ai_keyword_volume" => ai::ai_keyword_volume(client, input),
        "dataforseo_llm_mentions" => ai::llm_mentions(client, input),
        _ => Err(Error::InvalidRequest("unknown_tool")),
    }
}

fn decode_input<T: DeserializeOwned>(input: Value, reason: &'static str) -> Result<T> {
    match serde_json::from_value(input) {
        Ok(input) => Ok(input),
        Err(_) => Err(Error::InvalidRequest(reason)),
    }
}

fn provider_items(results: &[Value]) -> Vec<Value> {
    if let [result] = results
        && let Some(items) = result.get("items").and_then(Value::as_array)
    {
        return items.clone();
    }
    results.to_vec()
}

fn joined_filters(filters: Vec<Value>) -> Option<Value> {
    if filters.is_empty() {
        return None;
    }
    let mut joined = Vec::with_capacity(filters.len() * 2 - 1);
    for (index, filter) in filters.into_iter().enumerate() {
        if index > 0 {
            joined.push(Value::String(String::from("and")));
        }
        joined.push(filter);
    }
    Some(Value::Array(joined))
}
