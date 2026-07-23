//! Instant-page and detailed business output normalization.

use serde_json::{Value, json};

use super::common::{bounded_signed, signed, string};
use super::normalize::business_listing;

pub(in crate::dataforseo) fn instant_page(item: &Value) -> Value {
    json!({
        "resource_type": string(item, "/resource_type"),
        "status_code": signed(item, "/status_code"),
        "url": item.get("url").and_then(Value::as_str).unwrap_or(""),
        "redirect_url": string(item, "/location"),
        "title": string(item, "/meta/title"),
        "description": string(item, "/meta/description"),
        "canonical": string(item, "/meta/canonical"),
        "size_bytes": signed(item, "/size"),
        "encoded_size_bytes": signed(item, "/encoded_size"),
        "transfer_size_bytes": signed(item, "/total_transfer_size"),
        "word_count": signed(item, "/meta/content/plain_text_word_count"),
        "internal_links_count": signed(item, "/meta/internal_links_count"),
        "external_links_count": signed(item, "/meta/external_links_count"),
        "fetch_timing": {
            "duration_ms": milliseconds(item, "/fetch_timing/duration_time"),
            "fetch_start_ms": milliseconds(item, "/fetch_timing/fetch_start"),
            "fetch_end_ms": milliseconds(item, "/fetch_timing/fetch_end"),
        },
        "failed_checks": failed_checks(item),
    })
}

pub(in crate::dataforseo) fn business_info(item: &Value) -> Value {
    let mut output = match business_listing(item) {
        Value::Object(output) => output,
        _ => serde_json::Map::new(),
    };
    output.insert("description".into(), string(item, "/description"));
    output.insert("attributes".into(), Value::Array(attributes(item)));
    output.insert("popular_times".into(), Value::Array(popular_times(item)));
    output.insert(
        "rating_distribution".into(),
        json!({
            "one": signed(item, "/rating_distribution/1"),
            "two": signed(item, "/rating_distribution/2"),
            "three": signed(item, "/rating_distribution/3"),
            "four": signed(item, "/rating_distribution/4"),
            "five": signed(item, "/rating_distribution/5"),
        }),
    );
    Value::Object(output)
}

fn milliseconds(item: &Value, pointer: &str) -> Value {
    match item.pointer(pointer).and_then(Value::as_f64) {
        Some(value) if value.is_finite() => json!(value * 1_000.0),
        _ => Value::Null,
    }
}

fn failed_checks(item: &Value) -> Vec<Value> {
    const CHECKS: [(&str, &str); 16] = [
        ("broken_links", "/broken_links"),
        ("broken_page", "/checks/is_broken"),
        ("broken_resources", "/broken_resources"),
        ("http_4xx", "/checks/is_4xx_code"),
        ("http_5xx", "/checks/is_5xx_code"),
        ("irrelevant_description", "/checks/irrelevant_description"),
        ("irrelevant_title", "/checks/irrelevant_title"),
        ("large_page", "/checks/large_page"),
        ("low_content", "/checks/low_content"),
        ("missing_doctype", "/checks/no_doctype"),
        ("missing_h1", "/checks/no_h1_tag"),
        ("not_https", "/checks/is_http"),
        ("redirect", "/checks/is_redirect"),
        ("slow_loading", "/checks/high_loading_time"),
        ("title_too_long", "/checks/title_too_long"),
        ("title_too_short", "/checks/title_too_short"),
    ];
    CHECKS
        .into_iter()
        .filter(|(_, pointer)| item.pointer(pointer).and_then(Value::as_bool) == Some(true))
        .map(|(name, _)| Value::String(name.to_owned()))
        .collect()
}

fn attributes(item: &Value) -> Vec<Value> {
    let mut output = Vec::new();
    for (availability, pointer) in [
        ("available", "/attributes/available_attributes"),
        ("unavailable", "/attributes/unavailable_attributes"),
    ] {
        let Some(groups) = item.pointer(pointer).and_then(Value::as_object) else {
            continue;
        };
        for (group, values) in groups {
            for name in values
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
            {
                output.push(json!({ "availability": availability, "group": group, "name": name }));
            }
        }
    }
    output.sort_by_key(Value::to_string);
    output.truncate(20);
    output
}

fn popular_times(item: &Value) -> Vec<Value> {
    const DAYS: [&str; 7] = [
        "monday",
        "tuesday",
        "wednesday",
        "thursday",
        "friday",
        "saturday",
        "sunday",
    ];
    let mut output = Vec::new();
    for (day_index, day) in DAYS.into_iter().enumerate() {
        let pointer = format!("/popular_times/popular_times_by_days/{day}");
        for value in item
            .pointer(&pointer)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let hour = value.get("hour").and_then(Value::as_i64);
            let minute = value.get("minute").and_then(Value::as_i64).unwrap_or(0);
            if let Some(hour @ 0..=23) = hour
                && (0..=59).contains(&minute)
            {
                output.push((day_index, day, hour, minute, value));
            }
        }
    }
    output.sort_by_key(|(day, _, hour, minute, _)| (*day, *hour, *minute));
    output
        .into_iter()
        .take(168)
        .map(|(_, day, hour, minute, value)| {
            json!({
                "day": day,
                "hour": hour,
                "minute": minute,
                "popularity": bounded_signed(value, "/popularity", 0, 100),
            })
        })
        .collect()
}
