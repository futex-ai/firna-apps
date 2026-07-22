//! Deterministic bounded provider count and aggregate groups.

use std::cmp::Ordering;

use serde_json::{Value, json};

pub(in crate::dataforseo) fn count_buckets(
    value: &Value,
    pointer: &str,
    limit: usize,
) -> Vec<Value> {
    let Some(values) = value.pointer(pointer).and_then(Value::as_object) else {
        return Vec::new();
    };
    let mut buckets = values
        .iter()
        .filter(|(key, _)| !key.is_empty())
        .filter_map(|(key, count)| match count {
            Value::Null => Some((key.clone(), None)),
            Value::Number(count) => count.as_i64().map(|count| (key.clone(), Some(count))),
            _ => None,
        })
        .collect::<Vec<_>>();
    buckets.sort_by(compare_count_key);
    buckets
        .into_iter()
        .take(limit)
        .map(|(key, count)| json!({ "key": key, "count": count }))
        .collect()
}

pub(super) fn domain_counts(value: &Value, pointer: &str, limit: usize) -> Vec<Value> {
    let mut domains = value
        .pointer(pointer)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let domain = entry.get("domain").and_then(Value::as_str)?.trim();
            if domain.is_empty() {
                return None;
            }
            let count = match entry.get("count") {
                None | Some(Value::Null) => None,
                Some(value) => Some(value.as_i64()?),
            };
            Some((domain.to_owned(), count))
        })
        .collect::<Vec<_>>();
    domains.sort_by(compare_count_key);
    domains
        .into_iter()
        .take(limit)
        .map(|(domain, count)| json!({ "domain": domain, "count": count }))
        .collect()
}

pub(in crate::dataforseo) fn keyed_aggregates(
    value: &Value,
    pointer: &str,
    limit: usize,
) -> Vec<Value> {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let key = entry.get("key").and_then(|key| match key {
                Value::String(key) => Some(key.clone()),
                Value::Number(key) => Some(key.to_string()),
                _ => None,
            })?;
            Some(json!({
                "key": key,
                "mentions": entry.get("mentions").and_then(Value::as_i64),
                "ai_search_volume": entry.get("ai_search_volume").and_then(Value::as_i64),
            }))
        })
        .take(limit)
        .collect()
}

fn compare_count_key(left: &(String, Option<i64>), right: &(String, Option<i64>)) -> Ordering {
    match (left.1, right.1) {
        (Some(left_count), Some(right_count)) => right_count
            .cmp(&left_count)
            .then_with(|| left.0.cmp(&right.0)),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => left.0.cmp(&right.0),
    }
}
