//! Search, keyword, and backlink conformance fixtures.

use serde_json::json;

use super::super::error::Error;
use super::support::{call, invalid};

#[test]
fn keyword_suggestions_builds_only_reviewed_filters_and_defaults() {
    let (output, request) = call(
        "dataforseo_keyword_suggestions",
        json!({
            "keyword": "rust wasm",
            "location_name": "United States",
            "language_name": "English",
            "exact_match": true,
            "ignore_synonyms": true,
            "min_search_volume": 50,
            "max_keyword_difficulty": 40,
            "limit": 7
        }),
        Vec::new(),
    );

    assert_eq!(output["result_count"], 0);
    assert_eq!(request.timeout_seconds, 180);
    assert_eq!(request.task["limit"], 7);
    assert_eq!(request.task["exact_match"], true);
    assert_eq!(request.task["ignore_synonyms"], true);
    assert_eq!(
        request.task["filters"],
        json!([
            ["keyword_info.search_volume", ">=", 50],
            "and",
            ["keyword_properties.keyword_difficulty", "<=", 40]
        ])
    );
    assert_eq!(
        request.task["order_by"],
        json!(["keyword_info.search_volume,desc"])
    );
    assert!(request.task.get("offset").is_none());
    assert!(request.task.get("include_clickstream_data").is_none());
}

#[test]
fn keyword_bounds_dedupe_and_selector_conflicts_fail_before_dispatch() {
    for input in [
        json!({
            "keywords": ["Rust", " rust "],
            "location_code": 2840,
            "language_code": "en"
        }),
        json!({
            "keywords": ["one two three four five six seven eight nine ten eleven"],
            "location_code": 2840,
            "language_code": "en"
        }),
        json!({
            "keywords": ["rust"],
            "location_code": 2840,
            "location_name": "United States",
            "language_code": "en"
        }),
    ] {
        assert!(matches!(
            invalid("dataforseo_keyword_overview", input),
            Error::InvalidRequest(_)
        ));
    }
}

#[test]
fn keyword_metrics_use_only_known_enums_and_latest_valid_timestamp() {
    let (output, _) = call(
        "dataforseo_keyword_overview",
        json!({
            "keywords": ["rust", "wasm", "cargo"],
            "location_code": 2840,
            "language_code": "en"
        }),
        vec![json!({"items": [
            {
                "keyword": "rust",
                "keyword_info": {
                    "competition_level": "UNKNOWN",
                    "last_updated_time": "2026-01-01 00:00:00 +00:00",
                    "monthly_searches": [{"year": 2026, "month": 13, "search_volume": 1}]
                },
                "search_intent_info": {
                    "last_updated_time": "2026-02-01 00:00:00 +00:00",
                    "main_intent": {"informational": 0.7, "invented": 0.3}
                }
            },
            {
                "keyword": "wasm",
                "keyword_info": {"last_updated_time": "2026-03-01 00:00:00 +00:00"},
                "search_intent_info": {
                    "last_updated_time": "2026-03-01 00:00:00 +00:00",
                    "main_intent": "commercial",
                    "main_intent_probability": 0.8
                }
            },
            {
                "keyword": "cargo",
                "keyword_info": {"last_updated_time": "not-a-timestamp"},
                "search_intent_info": {
                    "last_updated_time": null,
                    "main_intent": "invented",
                    "main_intent_probability": 0.9
                }
            }
        ]})],
    );

    assert_eq!(output["items"][0]["competition_level"], json!(null));
    assert_eq!(
        output["items"][0]["monthly_searches"][0]["month"],
        json!(null)
    );
    assert_eq!(
        output["items"][0]["last_updated_time"],
        "2026-02-01 00:00:00 +00:00"
    );
    assert_eq!(
        output["items"][0]["intent_probabilities"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        output["items"][1]["last_updated_time"],
        "2026-03-01 00:00:00 +00:00"
    );
    assert_eq!(output["items"][1]["search_intent"], "commercial");
    assert_eq!(
        output["items"][1]["intent_probabilities"],
        json!([{"intent": "commercial", "probability": 0.8}])
    );
    assert_eq!(output["items"][2]["last_updated_time"], json!(null));
    assert_eq!(output["items"][2]["search_intent"], json!(null));
    assert_eq!(output["items"][2]["intent_probabilities"], json!([]));
}

#[test]
fn ranked_keywords_force_organic_filters_and_accept_safe_pages() {
    let (_, request) = call(
        "dataforseo_ranked_keywords",
        json!({
            "target": "https://example.com/page?q=rust",
            "location_code": 2840,
            "language_code": "en",
            "historical_serp_mode": "lost",
            "max_rank": 12,
            "min_search_volume": 25,
            "limit": 5,
            "offset": 10
        }),
        Vec::new(),
    );

    assert_eq!(request.task["item_types"], json!(["organic"]));
    assert_eq!(request.task["historical_serp_mode"], "lost");
    assert_eq!(
        request.task["filters"],
        json!([
            ["ranked_serp_element.serp_item.rank_group", "<=", 12],
            "and",
            ["keyword_data.keyword_info.search_volume", ">=", 25]
        ])
    );
    assert!(matches!(
        invalid(
            "dataforseo_ranked_keywords",
            json!({
                "target": "http://localhost/admin",
                "location_code": 2840,
                "language_code": "en"
            })
        ),
        Error::InvalidRequest(_)
    ));
}

#[test]
fn backlink_filters_defaults_and_count_caps_are_deterministic() {
    let (output, request) = call(
        "dataforseo_backlinks_summary",
        json!({"target": "example.com", "dofollow_only": true}),
        vec![json!({"items": [{
            "target": "example.com",
            "referring_links_tld": {
                "z": 3,
                "b": 5,
                "a": 5,
                "null": null,
                "bad": "secret"
            }
        }]})],
    );

    assert_eq!(request.task["include_subdomains"], true);
    assert_eq!(request.task["backlinks_status_type"], "live");
    assert_eq!(
        request.task["backlinks_filters"],
        json!([["dofollow", "=", true]])
    );
    assert_eq!(request.task["internal_list_limit"], 10);
    assert_eq!(
        output["items"][0]["tlds"],
        json!([
            {"key": "a", "count": 5},
            {"key": "b", "count": 5},
            {"key": "z", "count": 3},
            {"key": "null", "count": null}
        ])
    );
}

#[test]
fn backlink_page_targets_modes_and_pagination_remain_bounded() {
    let (_, request) = call(
        "dataforseo_backlinks",
        json!({
            "target": "https://example.com/page",
            "include_subdomains": false,
            "backlinks_status": "all",
            "mode": "one_per_domain",
            "limit": 50,
            "offset": 1000
        }),
        Vec::new(),
    );
    assert_eq!(request.task["mode"], "one_per_domain");
    assert_eq!(request.task["order_by"], json!(["rank,desc"]));
    assert_eq!(request.task["backlinks_status_type"], "all");
    assert!(matches!(
        invalid(
            "dataforseo_backlinks",
            json!({"target": "www.example.com", "limit": 51})
        ),
        Error::InvalidRequest(_)
    ));
}
