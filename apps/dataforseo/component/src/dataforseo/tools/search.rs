//! Search and keyword request construction and output normalization.

use serde_json::{Map, Value, json};

use super::{decode_input, joined_filters, provider_items};
use crate::dataforseo::envelope::decode;
use crate::dataforseo::error::{Error, Result};
use crate::dataforseo::host::ProviderClient;
use crate::dataforseo::input::{
    GoogleSerpInput, KeywordOverviewInput, KeywordSuggestionsInput, RankedKeywordsInput,
};
use crate::dataforseo::output::{bool_value, keyword_metric, number, signed, string, success};
use crate::dataforseo::validation::{
    bounded, location_language, premium_serp_operator, target, text, timeout, unique_texts,
};

pub(super) fn google_serp(client: &dyn ProviderClient, input: Value) -> Result<Value> {
    let input: GoogleSerpInput = decode_input(input, "invalid_google_serp_input")?;
    let keyword = text(input.keyword, 700, "invalid_keyword")?;
    if premium_serp_operator(&keyword) {
        return Err(Error::InvalidRequest("premium_serp_operator_denied"));
    }
    let depth = bounded(input.depth.unwrap_or(10), 1, 10, "invalid_depth")?;
    let request_timeout = timeout(input.timeout_seconds)?;
    let mut task = location_language(
        input.location_name,
        input.location_code,
        input.language_name,
        input.language_code,
    )?;
    task.insert("keyword".into(), json!(keyword));
    task.insert("device".into(), json!(input.device.unwrap_or_default()));
    task.insert("depth".into(), json!(depth));
    task.insert("load_async_ai_overview".into(), json!(false));
    let provider = decode(client.post_task(
        "/v3/serp/google/organic/live/advanced",
        Value::Object(task),
        request_timeout,
    )?)?;
    let items = provider_items(&provider.results)
        .into_iter()
        .take(depth as usize)
        .map(|item| {
            json!({
                "type": item.get("type").and_then(Value::as_str).unwrap_or("unknown"),
                "rank_group": signed(&item, "/rank_group"),
                "rank_absolute": signed(&item, "/rank_absolute"),
                "page": signed(&item, "/page"),
                "title": string(&item, "/title"),
                "question": string(&item, "/question"),
                "url": string(&item, "/url"),
                "domain": string(&item, "/domain"),
                "description": string(&item, "/description"),
                "text": string(&item, "/text"),
            })
        })
        .collect();
    Ok(success("dataforseo.google_serp", provider, items))
}

pub(super) fn keyword_overview(client: &dyn ProviderClient, input: Value) -> Result<Value> {
    let input: KeywordOverviewInput = decode_input(input, "invalid_keyword_overview_input")?;
    let keywords = unique_texts(input.keywords, 1, 100, 80, Some(10), "invalid_keywords")?;
    let request_timeout = timeout(input.timeout_seconds)?;
    let mut task = location_language(
        input.location_name,
        input.location_code,
        input.language_name,
        input.language_code,
    )?;
    task.insert("keywords".into(), json!(keywords));
    task.insert("include_serp_info".into(), json!(false));
    task.insert("include_clickstream_data".into(), json!(false));
    let provider = decode(client.post_task(
        "/v3/dataforseo_labs/google/keyword_overview/live",
        Value::Object(task),
        request_timeout,
    )?)?;
    let items = provider_items(&provider.results)
        .into_iter()
        .take(100)
        .map(|item| keyword_metric(&item))
        .collect();
    Ok(success("dataforseo.keyword_overview", provider, items))
}

pub(super) fn keyword_suggestions(client: &dyn ProviderClient, input: Value) -> Result<Value> {
    let input: KeywordSuggestionsInput = decode_input(input, "invalid_keyword_suggestions_input")?;
    let keyword = text(input.keyword, 80, "invalid_keyword")?;
    if keyword.split_whitespace().count() > 10 {
        return Err(Error::InvalidRequest("invalid_keyword"));
    }
    let limit = bounded(input.limit.unwrap_or(25), 1, 50, "invalid_limit")?;
    let request_timeout = timeout(input.timeout_seconds)?;
    let mut task = location_language(
        input.location_name,
        input.location_code,
        input.language_name,
        input.language_code,
    )?;
    task.insert("keyword".into(), json!(keyword));
    task.insert("limit".into(), json!(limit));
    task.insert(
        "exact_match".into(),
        json!(input.exact_match.unwrap_or(false)),
    );
    task.insert(
        "ignore_synonyms".into(),
        json!(input.ignore_synonyms.unwrap_or(false)),
    );
    let filters = suggestion_filters(input.min_search_volume, input.max_keyword_difficulty)?;
    if let Some(filters) = joined_filters(filters) {
        task.insert("filters".into(), filters);
    }
    task.insert(
        "order_by".into(),
        json!(["keyword_info.search_volume,desc"]),
    );
    keyword_provider_result(
        client,
        "/v3/dataforseo_labs/google/keyword_suggestions/live",
        "dataforseo.keyword_suggestions",
        task,
        request_timeout,
        limit,
    )
}

pub(super) fn ranked_keywords(client: &dyn ProviderClient, input: Value) -> Result<Value> {
    let input: RankedKeywordsInput = decode_input(input, "invalid_ranked_keywords_input")?;
    let limit = bounded(input.limit.unwrap_or(25), 1, 50, "invalid_limit")?;
    let offset = bounded(input.offset.unwrap_or(0), 0, 1_000, "invalid_offset")?;
    let max_rank = bounded(input.max_rank.unwrap_or(100), 1, 100, "invalid_max_rank")?;
    let min_volume = bounded(
        input.min_search_volume.unwrap_or(0),
        0,
        i32::MAX as u64,
        "invalid_min_search_volume",
    )?;
    let request_timeout = timeout(input.timeout_seconds)?;
    let mut task = location_language(
        input.location_name,
        input.location_code,
        input.language_name,
        input.language_code,
    )?;
    task.insert("target".into(), json!(target(input.target, false)?));
    task.insert("limit".into(), json!(limit));
    task.insert("offset".into(), json!(offset));
    task.insert(
        "historical_serp_mode".into(),
        json!(input.historical_serp_mode.unwrap_or_default()),
    );
    task.insert("item_types".into(), json!(["organic"]));
    task.insert(
        "filters".into(),
        joined_filters(vec![
            json!(["ranked_serp_element.serp_item.rank_group", "<=", max_rank]),
            json!(["keyword_data.keyword_info.search_volume", ">=", min_volume]),
        ])
        .unwrap_or(Value::Array(Vec::new())),
    );
    task.insert(
        "order_by".into(),
        json!(["ranked_serp_element.serp_item.rank_group,asc"]),
    );
    let provider = decode(client.post_task(
        "/v3/dataforseo_labs/google/ranked_keywords/live",
        Value::Object(task),
        request_timeout,
    )?)?;
    let items = provider_items(&provider.results)
        .into_iter()
        .take(limit as usize)
        .map(|item| ranked_keyword(&item))
        .collect();
    Ok(success("dataforseo.ranked_keywords", provider, items))
}

fn suggestion_filters(min_volume: Option<u64>, max_difficulty: Option<u64>) -> Result<Vec<Value>> {
    let mut filters = Vec::new();
    if let Some(value) = min_volume {
        filters.push(json!([
            "keyword_info.search_volume",
            ">=",
            bounded(value, 0, i32::MAX as u64, "invalid_min_search_volume")?
        ]));
    }
    if let Some(value) = max_difficulty {
        filters.push(json!([
            "keyword_properties.keyword_difficulty",
            "<=",
            bounded(value, 0, 100, "invalid_max_keyword_difficulty")?
        ]));
    }
    Ok(filters)
}

fn keyword_provider_result(
    client: &dyn ProviderClient,
    path: &str,
    operation: &str,
    task: Map<String, Value>,
    request_timeout: u64,
    limit: u64,
) -> Result<Value> {
    let provider = decode(client.post_task(path, Value::Object(task), request_timeout)?)?;
    let items = provider_items(&provider.results)
        .into_iter()
        .take(limit as usize)
        .map(|item| keyword_metric(&item))
        .collect();
    Ok(success(operation, provider, items))
}

fn ranked_keyword(item: &Value) -> Value {
    json!({
        "keyword_metrics": keyword_metric(item.get("keyword_data").unwrap_or(&Value::Null)),
        "rank_group": signed(item, "/ranked_serp_element/serp_item/rank_group"),
        "rank_absolute": signed(item, "/ranked_serp_element/serp_item/rank_absolute"),
        "url": string(item, "/ranked_serp_element/serp_item/url"),
        "title": string(item, "/ranked_serp_element/serp_item/title"),
        "estimated_traffic": number(item, "/ranked_serp_element/serp_item/etv"),
        "is_new": bool_value(item, "/ranked_serp_element/is_new"),
        "is_up": bool_value(item, "/ranked_serp_element/is_up"),
        "is_down": bool_value(item, "/ranked_serp_element/is_down"),
        "is_lost": bool_value(item, "/ranked_serp_element/is_lost"),
    })
}
