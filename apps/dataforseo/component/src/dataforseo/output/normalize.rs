//! Shared keyword and business output normalization.

use chrono::NaiveDateTime;
use serde_json::{Value, json};

use super::common::{bool_value, bounded_signed, number, signed, string, strings};

pub(in crate::dataforseo) fn keyword_metric(item: &Value) -> Value {
    let monthly_searches = item
        .pointer("/keyword_info/monthly_searches")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .take(12)
        .map(|entry| {
            json!({
                "year": signed(entry, "/year"),
                "month": bounded_signed(entry, "/month", 1, 12),
                "search_volume": signed(entry, "/search_volume"),
            })
        })
        .collect::<Vec<_>>();
    let intent_probabilities = intent_probabilities(item);
    json!({
        "keyword": item.get("keyword").and_then(Value::as_str).unwrap_or(""),
        "search_volume": signed(item, "/keyword_info/search_volume"),
        "monthly_searches": monthly_searches,
        "cpc": number(item, "/keyword_info/cpc"),
        "competition": number(item, "/keyword_info/competition"),
        "competition_level": competition_level(item),
        "search_intent": search_intent(item),
        "intent_probabilities": intent_probabilities,
        "keyword_difficulty": signed(item, "/keyword_properties/keyword_difficulty"),
        "last_updated_time": latest_keyword_timestamp(item),
    })
}

pub(in crate::dataforseo) fn business_listing(item: &Value) -> Value {
    json!({
        "title": item.get("title").and_then(Value::as_str).unwrap_or(""),
        "categories": strings(item, "/category_ids", 10),
        "cid": string(item, "/cid"),
        "place_id": string(item, "/place_id"),
        "feature_id": string(item, "/feature_id"),
        "address": string(item, "/address"),
        "city": string(item, "/address_info/city"),
        "region": string(item, "/address_info/region"),
        "postal_code": string(item, "/address_info/zip"),
        "country_code": string(item, "/address_info/country_code"),
        "latitude": number(item, "/latitude"),
        "longitude": number(item, "/longitude"),
        "phone": string(item, "/phone"),
        "url": string(item, "/url"),
        "domain": string(item, "/domain"),
        "rating": number(item, "/rating/value"),
        "rating_count": signed(item, "/rating/votes_count"),
        "is_claimed": bool_value(item, "/is_claimed"),
        "hours": business_hours(item),
        "last_updated_time": string(item, "/last_updated_time"),
    })
}

fn intent_probabilities(item: &Value) -> Vec<Value> {
    let mut values = Vec::new();
    let Some(info) = item
        .pointer("/search_intent_info")
        .and_then(Value::as_object)
    else {
        return values;
    };
    for key in ["main_intent", "foreign_intent"] {
        match info.get(key) {
            Some(Value::Object(entries)) => {
                for (intent, probability) in entries {
                    if !known_intent(intent) {
                        continue;
                    }
                    if let Some(probability) = probability.as_f64() {
                        values.push(json!({ "intent": intent, "probability": probability }));
                    }
                }
            }
            Some(Value::String(intent)) if key == "main_intent" && known_intent(intent) => {
                if let Some(probability) =
                    info.get("main_intent_probability").and_then(Value::as_f64)
                {
                    values.push(json!({ "intent": intent, "probability": probability }));
                }
            }
            _ => {}
        }
    }
    values.truncate(4);
    values
}

fn search_intent(item: &Value) -> Value {
    item.pointer("/search_intent_info/main_intent")
        .and_then(Value::as_str)
        .filter(|intent| known_intent(intent))
        .map(|intent| Value::String(intent.to_owned()))
        .unwrap_or(Value::Null)
}

fn known_intent(intent: &str) -> bool {
    matches!(
        intent,
        "informational" | "navigational" | "commercial" | "transactional"
    )
}

fn competition_level(item: &Value) -> Value {
    match item
        .pointer("/keyword_info/competition_level")
        .and_then(Value::as_str)
    {
        Some(level @ ("LOW" | "MEDIUM" | "HIGH")) => Value::String(level.to_owned()),
        _ => Value::Null,
    }
}

fn latest_keyword_timestamp(item: &Value) -> Value {
    let keyword = valid_timestamp(item.pointer("/keyword_info/last_updated_time"));
    let intent = valid_timestamp(item.pointer("/search_intent_info/last_updated_time"));
    match (keyword, intent) {
        (Some((keyword_time, _keyword)), Some((intent_time, intent)))
            if intent_time > keyword_time =>
        {
            Value::String(intent)
        }
        (Some((_, keyword)), _) => Value::String(keyword),
        (None, Some((_, intent))) => Value::String(intent),
        (None, None) => Value::Null,
    }
}

fn valid_timestamp(value: Option<&Value>) -> Option<(NaiveDateTime, String)> {
    let value = value.and_then(Value::as_str)?;
    let timestamp = value.strip_suffix(" +00:00")?;
    let parsed = NaiveDateTime::parse_from_str(timestamp, "%Y-%m-%d %H:%M:%S").ok()?;
    Some((parsed, value.to_owned()))
}

fn business_hours(item: &Value) -> Vec<Value> {
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
    for day in DAYS {
        let pointer = format!("/work_time/work_hours/timetable/{day}");
        for interval in item
            .pointer(&pointer)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let Some(open) = interval.get("open").and_then(Value::as_object) else {
                continue;
            };
            let Some(close) = interval.get("close").and_then(Value::as_object) else {
                continue;
            };
            let Some((open_hour, open_minute)) = valid_time(open) else {
                continue;
            };
            let Some((close_hour, close_minute)) = valid_time(close) else {
                continue;
            };
            output.push(json!({
                "day": day,
                "open_hour": open_hour,
                "open_minute": open_minute,
                "close_hour": close_hour,
                "close_minute": close_minute,
            }));
            if output.len() == 14 {
                return output;
            }
        }
    }
    output
}

fn valid_time(value: &serde_json::Map<String, Value>) -> Option<(i64, i64)> {
    let hour = value.get("hour").and_then(Value::as_i64)?;
    let minute = value.get("minute").and_then(Value::as_i64)?;
    ((0..=23).contains(&hour) && (0..=59).contains(&minute)).then_some((hour, minute))
}
