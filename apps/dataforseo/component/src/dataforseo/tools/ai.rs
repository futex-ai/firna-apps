//! AI keyword-demand and LLM mention metric requests.

use std::collections::BTreeSet;

use serde::Serialize;
use serde_json::{Value, json};

use super::{decode_input, provider_items};
use crate::dataforseo::envelope::decode;
use crate::dataforseo::error::{Error, Result};
use crate::dataforseo::host::ProviderClient;
use crate::dataforseo::input::{
    AiKeywordVolumeInput, DomainScope, KeywordScope, LlmMentionsInput, LlmPlatform, MatchType,
    SearchFilter, TargetEntity,
};
use crate::dataforseo::output::{bounded_signed, keyed_aggregates, signed, success};
use crate::dataforseo::validation::{hostname, location_language, text, timeout, unique_texts};

pub(super) fn ai_keyword_volume(client: &dyn ProviderClient, input: Value) -> Result<Value> {
    let input: AiKeywordVolumeInput = decode_input(input, "invalid_ai_keyword_volume_input")?;
    let keywords = unique_texts(input.keywords, 1, 100, 250, None, "invalid_keywords")?;
    let request_timeout = timeout(input.timeout_seconds)?;
    let mut task = location_language(
        input.location_name,
        input.location_code,
        input.language_name,
        input.language_code,
    )?;
    task.insert("keywords".into(), json!(keywords));
    let provider = decode(client.post_task(
        "/v3/ai_optimization/ai_keyword_data/keywords_search_volume/live",
        Value::Object(task),
        request_timeout,
    )?)?;
    let items = provider_items(&provider.results)
        .into_iter()
        .take(100)
        .map(|item| ai_keyword_item(&item))
        .collect();
    Ok(success("dataforseo.ai_keyword_volume", provider, items))
}

pub(super) fn llm_mentions(client: &dyn ProviderClient, input: Value) -> Result<Value> {
    let input: LlmMentionsInput = decode_input(input, "invalid_llm_mentions_input")?;
    if input.targets.is_empty() || input.targets.len() > 10 {
        return Err(Error::InvalidRequest("invalid_llm_targets"));
    }
    if input.platform == LlmPlatform::ChatGpt
        && !chat_gpt_selectors(
            input.location_name.as_deref(),
            input.location_code,
            input.language_name.as_deref(),
            input.language_code.as_deref(),
        )
    {
        return Err(Error::InvalidRequest("chat_gpt_us_english_required"));
    }
    let mut task = location_language(
        input.location_name,
        input.location_code,
        input.language_name,
        input.language_code,
    )?;
    let targets = normalize_targets(input.targets)?;
    if !targets
        .iter()
        .any(|target| target.get("search_filter") == Some(&json!("include")))
    {
        return Err(Error::InvalidRequest("included_llm_target_required"));
    }
    task.insert("platform".into(), enum_value(input.platform));
    task.insert("target".into(), Value::Array(targets));
    task.insert("internal_list_limit".into(), json!(5));
    let request_timeout = timeout(input.timeout_seconds)?;
    let provider = decode(client.post_task(
        "/v3/ai_optimization/llm_mentions/target_metrics/live",
        Value::Object(task),
        request_timeout,
    )?)?;
    let items = provider
        .results
        .first()
        .map(llm_mentions_item)
        .into_iter()
        .collect();
    Ok(success("dataforseo.llm_mentions", provider, items))
}

fn ai_keyword_item(item: &Value) -> Value {
    let monthly_searches = item
        .get("ai_monthly_searches")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(12)
        .map(|month| {
            json!({
                "year": signed(month, "/year"),
                "month": bounded_signed(month, "/month", 1, 12),
                "ai_search_volume": signed(month, "/ai_search_volume"),
            })
        })
        .collect::<Vec<_>>();
    json!({
        "keyword": item.get("keyword").and_then(Value::as_str).unwrap_or(""),
        "ai_search_volume": signed(item, "/ai_search_volume"),
        "monthly_searches": monthly_searches,
    })
}

fn normalize_targets(targets: Vec<TargetEntity>) -> Result<Vec<Value>> {
    targets
        .into_iter()
        .map(|target| match target {
            TargetEntity::Domain(target) => {
                let domain = hostname(target.domain, true)?;
                if domain.len() > 63 {
                    return Err(Error::InvalidRequest("llm_domain_too_long"));
                }
                let scopes = target
                    .search_scope
                    .unwrap_or_else(|| vec![DomainScope::Any]);
                validate_scopes(&scopes, DomainScope::Any, 3)?;
                Ok(json!({
                    "domain": domain,
                    "search_filter": target.search_filter.unwrap_or_default(),
                    "search_scope": scopes,
                    "include_subdomains": target.include_subdomains.unwrap_or(false),
                }))
            }
            TargetEntity::Keyword(target) => {
                let scopes = target
                    .search_scope
                    .unwrap_or_else(|| vec![KeywordScope::Any]);
                validate_scopes(&scopes, KeywordScope::Any, 5)?;
                Ok(json!({
                    "keyword": text(target.keyword, 250, "invalid_llm_keyword")?,
                    "search_filter": target.search_filter.unwrap_or(SearchFilter::Include),
                    "search_scope": scopes,
                    "match_type": target.match_type.unwrap_or(MatchType::WordMatch),
                }))
            }
        })
        .collect()
}

fn validate_scopes<T>(scopes: &[T], any: T, maximum: usize) -> Result<()>
where
    T: Copy + Ord,
{
    let unique = scopes.iter().copied().collect::<BTreeSet<_>>();
    if scopes.is_empty() || unique.len() != scopes.len() || scopes.len() > maximum {
        return Err(Error::InvalidRequest("invalid_llm_scope"));
    }
    if scopes.len() > 1 && unique.contains(&any) {
        return Err(Error::InvalidRequest("llm_any_scope_conflict"));
    }
    Ok(())
}

fn llm_mentions_item(item: &Value) -> Value {
    json!({
        "total": {
            "mentions": signed(item, "/aggregated_metrics/total/mentions"),
            "ai_search_volume": signed(item, "/aggregated_metrics/total/ai_search_volume"),
        },
        "by_location": keyed_aggregates(item, "/aggregated_metrics/location", 5),
        "by_language": keyed_aggregates(item, "/aggregated_metrics/language", 5),
        "by_platform": keyed_aggregates(item, "/aggregated_metrics/platform", 2),
        "top_source_domains": keyed_aggregates(item, "/aggregated_metrics/sources_domain", 5),
        "top_search_result_domains": keyed_aggregates(item, "/aggregated_metrics/search_results_domain", 5),
    })
}

fn chat_gpt_selectors(
    location_name: Option<&str>,
    location_code: Option<i64>,
    language_name: Option<&str>,
    language_code: Option<&str>,
) -> bool {
    matches!(
        (location_name, location_code),
        (Some("United States"), None) | (None, Some(2840))
    ) && matches!(
        (language_name, language_code),
        (Some("English"), None) | (None, Some("en"))
    )
}

fn enum_value(value: impl Serialize) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}
