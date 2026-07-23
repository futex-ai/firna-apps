//! Backlink request construction and compact link-profile normalization.

use serde::Serialize;
use serde_json::{Map, Value, json};

use super::{decode_input, joined_filters, provider_items};
use crate::dataforseo::envelope::decode;
use crate::dataforseo::error::Result;
use crate::dataforseo::host::ProviderClient;
use crate::dataforseo::input::{
    BacklinksInput, BacklinksStatus, BacklinksSummaryInput, ReferringDomainsInput,
};
use crate::dataforseo::output::{
    bool_value, count_buckets, number, signed, string, strings, success,
};
use crate::dataforseo::validation::{bounded, target, timeout};

pub(super) fn summary(client: &dyn ProviderClient, input: Value) -> Result<Value> {
    let input: BacklinksSummaryInput = decode_input(input, "invalid_backlinks_summary_input")?;
    let request_timeout = timeout(input.timeout_seconds)?;
    let mut task = base_task(
        input.target,
        input.include_subdomains,
        input.backlinks_status,
    )?;
    if input.dofollow_only.unwrap_or(false) {
        task.insert("backlinks_filters".into(), json!([["dofollow", "=", true]]));
    }
    task.insert("internal_list_limit".into(), json!(10));
    let provider = decode(client.post_task(
        "/v3/backlinks/summary/live",
        Value::Object(task),
        request_timeout,
    )?)?;
    let items = provider_items(&provider.results)
        .first()
        .map(summary_item)
        .into_iter()
        .collect();
    Ok(success("dataforseo.backlinks_summary", provider, items))
}

pub(super) fn backlinks(client: &dyn ProviderClient, input: Value) -> Result<Value> {
    let input: BacklinksInput = decode_input(input, "invalid_backlinks_input")?;
    let limit = bounded(input.limit.unwrap_or(25), 1, 50, "invalid_limit")?;
    let offset = bounded(input.offset.unwrap_or(0), 0, 1_000, "invalid_offset")?;
    let request_timeout = timeout(input.timeout_seconds)?;
    let mut task = base_task(
        input.target,
        input.include_subdomains,
        input.backlinks_status,
    )?;
    task.insert("limit".into(), json!(limit));
    task.insert("offset".into(), json!(offset));
    task.insert("mode".into(), enum_value(input.mode.unwrap_or_default()));
    task.insert("order_by".into(), json!(["rank,desc"]));
    if input.dofollow_only.unwrap_or(false) {
        task.insert("filters".into(), json!([["dofollow", "=", true]]));
    }
    let provider = decode(client.post_task(
        "/v3/backlinks/backlinks/live",
        Value::Object(task),
        request_timeout,
    )?)?;
    let items = provider_items(&provider.results)
        .into_iter()
        .take(limit as usize)
        .map(|item| backlink_item(&item))
        .collect();
    Ok(success("dataforseo.backlinks", provider, items))
}

pub(super) fn referring_domains(client: &dyn ProviderClient, input: Value) -> Result<Value> {
    let input: ReferringDomainsInput = decode_input(input, "invalid_referring_domains_input")?;
    let limit = bounded(input.limit.unwrap_or(25), 1, 50, "invalid_limit")?;
    let offset = bounded(input.offset.unwrap_or(0), 0, 1_000, "invalid_offset")?;
    let request_timeout = timeout(input.timeout_seconds)?;
    let mut task = base_task(
        input.target,
        input.include_subdomains,
        input.backlinks_status,
    )?;
    task.insert("limit".into(), json!(limit));
    task.insert("offset".into(), json!(offset));
    task.insert("order_by".into(), json!(["rank,desc"]));
    task.insert("internal_list_limit".into(), json!(10));
    let mut filters = Vec::new();
    if input.dofollow_only.unwrap_or(false) {
        filters.push(json!(["dofollow", "=", true]));
    }
    if let Some(minimum) = input.min_backlinks {
        filters.push(json!([
            "backlinks",
            ">=",
            bounded(minimum, 0, i32::MAX as u64, "invalid_min_backlinks")?
        ]));
    }
    if let Some(filters) = joined_filters(filters) {
        task.insert("filters".into(), filters);
    }
    let provider = decode(client.post_task(
        "/v3/backlinks/referring_domains/live",
        Value::Object(task),
        request_timeout,
    )?)?;
    let items = provider_items(&provider.results)
        .into_iter()
        .take(limit as usize)
        .map(|item| referring_domain_item(&item))
        .collect();
    Ok(success("dataforseo.referring_domains", provider, items))
}

fn base_task(
    target_value: String,
    include_subdomains: Option<bool>,
    status: Option<BacklinksStatus>,
) -> Result<Map<String, Value>> {
    let mut task = Map::new();
    task.insert("target".into(), json!(target(target_value, true)?));
    task.insert(
        "include_subdomains".into(),
        json!(include_subdomains.unwrap_or(true)),
    );
    task.insert(
        "backlinks_status_type".into(),
        enum_value(status.unwrap_or_default()),
    );
    task.insert("include_indirect_links".into(), json!(true));
    task.insert("exclude_internal_backlinks".into(), json!(true));
    task.insert("rank_scale".into(), json!("one_hundred"));
    Ok(task)
}

fn summary_item(item: &Value) -> Value {
    json!({
        "target": item.get("target").and_then(Value::as_str).unwrap_or(""),
        "rank": signed(item, "/rank"),
        "backlinks": signed(item, "/backlinks"),
        "backlinks_spam_score": signed(item, "/backlinks_spam_score"),
        "crawled_pages": signed(item, "/crawled_pages"),
        "broken_backlinks": signed(item, "/broken_backlinks"),
        "broken_pages": signed(item, "/broken_pages"),
        "referring_domains": signed(item, "/referring_domains"),
        "referring_domains_nofollow": signed(item, "/referring_domains_nofollow"),
        "referring_main_domains": signed(item, "/referring_main_domains"),
        "referring_main_domains_nofollow": signed(item, "/referring_main_domains_nofollow"),
        "referring_pages": signed(item, "/referring_pages"),
        "referring_pages_nofollow": signed(item, "/referring_pages_nofollow"),
        "referring_ips": signed(item, "/referring_ips"),
        "referring_subnets": signed(item, "/referring_subnets"),
        "tlds": count_buckets(item, "/referring_links_tld", 10),
        "link_types": count_buckets(item, "/referring_links_types", 10),
        "attributes": count_buckets(item, "/referring_links_attributes", 10),
        "platforms": count_buckets(item, "/referring_links_platform_types", 10),
        "semantic_locations": count_buckets(item, "/referring_links_semantic_locations", 10),
        "countries": count_buckets(item, "/referring_links_countries", 10),
    })
}

fn backlink_item(item: &Value) -> Value {
    json!({
        "domain_from": string(item, "/domain_from"),
        "url_from": string(item, "/url_from"),
        "domain_to": string(item, "/domain_to"),
        "url_to": string(item, "/url_to"),
        "rank": signed(item, "/rank"),
        "domain_from_rank": signed(item, "/domain_from_rank"),
        "page_from_rank": signed(item, "/page_from_rank"),
        "anchor": string(item, "/anchor"),
        "alt": string(item, "/alt"),
        "link_type": string(item, "/type"),
        "attributes": strings(item, "/attributes", 10),
        "dofollow": bool_value(item, "/dofollow"),
        "is_new": bool_value(item, "/is_new"),
        "is_lost": bool_value(item, "/is_lost"),
        "is_broken": bool_value(item, "/is_broken"),
        "is_indirect": bool_value(item, "/is_indirect_link"),
        "first_seen": string(item, "/first_seen"),
        "last_seen": string(item, "/last_seen"),
        "lost_date": string(item, "/lost_date"),
        "spam_score": signed(item, "/backlink_spam_score"),
    })
}

fn referring_domain_item(item: &Value) -> Value {
    json!({
        "domain": item.get("domain").and_then(Value::as_str).unwrap_or(""),
        "rank": signed(item, "/rank"),
        "backlinks": signed(item, "/backlinks"),
        "referring_pages": signed(item, "/referring_pages"),
        "broken_backlinks": signed(item, "/broken_backlinks"),
        "broken_pages": signed(item, "/broken_pages"),
        "average_spam_score": number(item, "/backlinks_spam_score"),
        "first_seen": string(item, "/first_seen"),
        "lost_date": string(item, "/lost_date"),
        "attributes": count_buckets(item, "/referring_links_attributes", 10),
    })
}

fn enum_value(value: impl Serialize) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}
