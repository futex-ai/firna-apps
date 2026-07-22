//! Content-analysis, technology, and WHOIS requests.

use serde::Serialize;
use serde_json::{Map, Value, json};

use super::{decode_input, joined_filters, provider_items};
use crate::dataforseo::envelope::decode;
use crate::dataforseo::error::{Error, Result};
use crate::dataforseo::host::ProviderClient;
use crate::dataforseo::input::{
    ContentSearchInput, ContentSentimentInput, DomainTechnologiesInput, DomainWhoisInput, PageType,
};
use crate::dataforseo::output::{
    content_item, content_sentiment_item, success, technology_item, whois_item,
};
use crate::dataforseo::validation::{bounded, hostname, text, timeout};

pub(super) fn content_search(client: &dyn ProviderClient, input: Value) -> Result<Value> {
    let input: ContentSearchInput = decode_input(input, "invalid_content_search_input")?;
    let limit = bounded(input.limit.unwrap_or(25), 1, 50, "invalid_limit")?;
    let offset = bounded(input.offset.unwrap_or(0), 0, 1_000, "invalid_offset")?;
    let request_timeout = timeout(input.timeout_seconds)?;
    let (mut task, mut filters) = content_task(
        input.keyword,
        input.page_types,
        input.country_code,
        input.language_code,
        input.min_domain_rank,
    )?;
    task.insert(
        "search_mode".into(),
        enum_value(input.search_mode.unwrap_or_default()),
    );
    if let Some(sentiment) = input.sentiment {
        let name = enum_string(sentiment);
        filters.push(json!([
            format!("content_info.connotation_types.{name}"),
            ">",
            0
        ]));
    }
    if let Some(filters) = joined_filters(filters) {
        task.insert("filters".into(), filters);
    }
    task.insert("limit".into(), json!(limit));
    task.insert("offset".into(), json!(offset));
    task.insert(
        "order_by".into(),
        json!(["content_info.sentiment_connotations.anger,desc"]),
    );
    let provider = decode(client.post_task(
        "/v3/content_analysis/search/live",
        Value::Object(task),
        request_timeout,
    )?)?;
    let items = provider_items(&provider.results)
        .into_iter()
        .take(limit as usize)
        .map(|item| content_item(&item))
        .collect();
    Ok(success("dataforseo.content_search", provider, items))
}

pub(super) fn content_sentiment(client: &dyn ProviderClient, input: Value) -> Result<Value> {
    let input: ContentSentimentInput = decode_input(input, "invalid_content_sentiment_input")?;
    let request_timeout = timeout(input.timeout_seconds)?;
    let (mut task, filters) = content_task(
        input.keyword,
        input.page_types,
        input.country_code,
        input.language_code,
        input.min_domain_rank,
    )?;
    if let Some(filters) = joined_filters(filters) {
        task.insert("initial_dataset_filters".into(), filters);
    }
    task.insert("internal_list_limit".into(), json!(10));
    task.insert("positive_connotation_threshold".into(), json!(0.4));
    task.insert("sentiments_connotation_threshold".into(), json!(0.4));
    let provider = decode(client.post_task(
        "/v3/content_analysis/sentiment_analysis/live",
        Value::Object(task),
        request_timeout,
    )?)?;
    let items = provider_items(&provider.results)
        .first()
        .map(content_sentiment_item)
        .into_iter()
        .collect();
    Ok(success("dataforseo.content_sentiment", provider, items))
}

pub(super) fn domain_technologies(client: &dyn ProviderClient, input: Value) -> Result<Value> {
    let input: DomainTechnologiesInput = decode_input(input, "invalid_domain_technologies_input")?;
    let request_timeout = timeout(input.timeout_seconds)?;
    let task = json!({ "target": hostname(input.hostname, false)? });
    let provider = decode(client.post_task(
        "/v3/domain_analytics/technologies/domain_technologies/live",
        task,
        request_timeout,
    )?)?;
    let items = provider_items(&provider.results)
        .first()
        .map(technology_item)
        .into_iter()
        .collect();
    Ok(success("dataforseo.domain_technologies", provider, items))
}

pub(super) fn domain_whois(client: &dyn ProviderClient, input: Value) -> Result<Value> {
    let input: DomainWhoisInput = decode_input(input, "invalid_domain_whois_input")?;
    let request_timeout = timeout(input.timeout_seconds)?;
    let task = json!({
        "filters": [["domain", "=", hostname(input.hostname, false)?]],
        "limit": 1,
        "offset": 0,
    });
    let provider = decode(client.post_task(
        "/v3/domain_analytics/whois/overview/live",
        task,
        request_timeout,
    )?)?;
    let items = provider_items(&provider.results)
        .first()
        .map(whois_item)
        .into_iter()
        .collect();
    Ok(success("dataforseo.domain_whois", provider, items))
}

fn content_task(
    keyword: String,
    page_types: Option<Vec<PageType>>,
    country_code: Option<String>,
    language_code: Option<String>,
    min_domain_rank: Option<u64>,
) -> Result<(Map<String, Value>, Vec<Value>)> {
    let mut task = Map::new();
    task.insert(
        "keyword".into(),
        json!(text(keyword, 250, "invalid_keyword")?),
    );
    task.insert("rank_scale".into(), json!("one_hundred"));
    if let Some(page_types) = page_types {
        if page_types.is_empty() || page_types.len() > 5 {
            return Err(Error::InvalidRequest("invalid_page_types"));
        }
        let serialized = enum_value(page_types);
        let unique = serialized
            .as_array()
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
            })
            .unwrap_or_default();
        if unique != serialized.as_array().map(Vec::len).unwrap_or_default() {
            return Err(Error::InvalidRequest("duplicate_page_types"));
        }
        task.insert("page_type".into(), serialized);
    }
    let mut filters = Vec::new();
    if let Some(country) = country_code {
        if country.len() != 2 || !country.bytes().all(|byte| byte.is_ascii_uppercase()) {
            return Err(Error::InvalidRequest("invalid_country_code"));
        }
        filters.push(json!(["country", "=", country]));
    }
    if let Some(language) = language_code {
        if language.len() != 2 || !language.bytes().all(|byte| byte.is_ascii_lowercase()) {
            return Err(Error::InvalidRequest("invalid_language_code"));
        }
        filters.push(json!(["language", "=", language]));
    }
    if let Some(rank) = min_domain_rank {
        filters.push(json!([
            "domain_rank",
            ">=",
            bounded(rank, 0, 100, "invalid_min_domain_rank")?
        ]));
    }
    Ok((task, filters))
}

fn enum_value(value: impl Serialize) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}

fn enum_string(value: impl Serialize) -> String {
    enum_value(value).as_str().unwrap_or("").to_owned()
}
