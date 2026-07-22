//! Content-analysis, technology, and WHOIS output normalization.

use serde_json::{Value, json};

use super::common::{bool_value, number, signed, string, strings};
use super::groups::domain_counts;

pub(in crate::dataforseo) fn content_item(item: &Value) -> Value {
    let snippet = item
        .pointer("/content_info/snippet")
        .and_then(Value::as_str)
        .map(|value| value.chars().take(1_000).collect::<String>());
    json!({
        "url": item.get("url").and_then(Value::as_str).unwrap_or(""),
        "domain": string(item, "/domain"),
        "domain_rank": signed(item, "/domain_rank"),
        "url_rank": signed(item, "/url_rank"),
        "spam_score": signed(item, "/spam_score"),
        "country_code": string(item, "/country"),
        "language_code": string(item, "/language"),
        "page_types": strings(item, "/page_types", 5),
        "title": string(item, "/content_info/title"),
        "author": string(item, "/content_info/author"),
        "snippet": snippet,
        "publication_time": string(item, "/content_info/date_published"),
        "fetch_time": string(item, "/fetch_time"),
        "content_quality_score": number(item, "/content_info/content_quality_score"),
        "sentiment": sentiment_scores(item, "/content_info/connotation_types"),
        "connotations": connotation_scores(item, "/content_info/sentiment_connotations"),
    })
}

pub(in crate::dataforseo) fn content_sentiment_item(item: &Value) -> Value {
    json!({
        "sentiment_counts": {
            "positive": signed(item, "/positive_connotation_distribution/positive/total_count"),
            "negative": signed(item, "/positive_connotation_distribution/negative/total_count"),
            "neutral": signed(item, "/positive_connotation_distribution/neutral/total_count"),
        },
        "connotation_counts": {
            "anger": signed(item, "/sentiment_connotation_distribution/anger/total_count"),
            "happiness": signed(item, "/sentiment_connotation_distribution/happiness/total_count"),
            "love": signed(item, "/sentiment_connotation_distribution/love/total_count"),
            "sadness": signed(item, "/sentiment_connotation_distribution/sadness/total_count"),
            "share": signed(item, "/sentiment_connotation_distribution/share/total_count"),
            "fun": signed(item, "/sentiment_connotation_distribution/fun/total_count"),
        },
        "top_domains": {
            "positive": domain_counts(item, "/positive_connotation_distribution/positive/top_domains", 10),
            "negative": domain_counts(item, "/positive_connotation_distribution/negative/top_domains", 10),
            "neutral": domain_counts(item, "/positive_connotation_distribution/neutral/top_domains", 10),
        },
    })
}

pub(in crate::dataforseo) fn technology_item(item: &Value) -> Value {
    json!({
        "domain": item.get("domain").and_then(Value::as_str).unwrap_or(""),
        "title": string(item, "/title"),
        "description": string(item, "/description"),
        "domain_rank": signed(item, "/domain_rank"),
        "last_visited_time": string(item, "/last_visited"),
        "country_code": string(item, "/country_iso_code"),
        "language_code": string(item, "/language_code"),
        "technologies": technologies(item),
    })
}

pub(in crate::dataforseo) fn whois_item(item: &Value) -> Value {
    json!({
        "domain": item.get("domain").and_then(Value::as_str).unwrap_or(""),
        "created_time": string(item, "/created_datetime"),
        "changed_time": string(item, "/changed_datetime"),
        "expiration_time": string(item, "/expiration_datetime"),
        "updated_time": string(item, "/updated_datetime"),
        "first_seen": string(item, "/first_seen"),
        "registered": bool_value(item, "/registered"),
        "epp_status_codes": strings(item, "/epp_status_codes", 20),
        "tld": string(item, "/tld"),
        "registrar": string(item, "/registrar"),
        "organic": traffic(item, "/metrics/organic"),
        "paid": traffic(item, "/metrics/paid"),
        "backlinks": {
            "backlinks": signed(item, "/backlinks_info/backlinks"),
            "dofollow": signed(item, "/backlinks_info/dofollow"),
            "referring_domains": signed(item, "/backlinks_info/referring_domains"),
            "referring_main_domains": signed(item, "/backlinks_info/referring_main_domains"),
            "referring_pages": signed(item, "/backlinks_info/referring_pages"),
            "updated_time": string(item, "/backlinks_info/time_update"),
        },
    })
}

fn sentiment_scores(item: &Value, pointer: &str) -> Value {
    json!({
        "positive": number(item, &format!("{pointer}/positive")),
        "negative": number(item, &format!("{pointer}/negative")),
        "neutral": number(item, &format!("{pointer}/neutral")),
    })
}

fn connotation_scores(item: &Value, pointer: &str) -> Value {
    json!({
        "anger": number(item, &format!("{pointer}/anger")),
        "happiness": number(item, &format!("{pointer}/happiness")),
        "love": number(item, &format!("{pointer}/love")),
        "sadness": number(item, &format!("{pointer}/sadness")),
        "share": number(item, &format!("{pointer}/share")),
        "fun": number(item, &format!("{pointer}/fun")),
    })
}

fn traffic(item: &Value, pointer: &str) -> Value {
    json!({
        "keywords": signed(item, &format!("{pointer}/count")),
        "estimated_traffic": number(item, &format!("{pointer}/etv")),
        "estimated_traffic_cost": number(item, &format!("{pointer}/estimated_paid_traffic_cost")),
    })
}

fn technologies(item: &Value) -> Vec<Value> {
    let Some(groups) = item.get("technologies").and_then(Value::as_object) else {
        return Vec::new();
    };
    let mut output = Vec::new();
    for (group, categories) in groups {
        let Some(categories) = categories.as_object() else {
            continue;
        };
        for (category, values) in categories {
            for value in values.as_array().into_iter().flatten() {
                let name = value
                    .as_str()
                    .or_else(|| value.get("name").and_then(Value::as_str));
                if let Some(name) = name.filter(|name| !name.trim().is_empty()) {
                    output.push((non_empty(group), non_empty(category), name.to_owned()));
                }
            }
        }
    }
    output.sort_by(|left, right| {
        optional_text(&left.0, &right.0)
            .then_with(|| optional_text(&left.1, &right.1))
            .then_with(|| left.2.cmp(&right.2))
    });
    output
        .into_iter()
        .take(50)
        .map(
            |(group, category, name)| json!({ "group": group, "category": category, "name": name }),
        )
        .collect()
}

fn non_empty(value: &str) -> Option<String> {
    (!value.trim().is_empty()).then(|| value.to_owned())
}

fn optional_text(left: &Option<String>, right: &Option<String>) -> std::cmp::Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left.cmp(right),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}
