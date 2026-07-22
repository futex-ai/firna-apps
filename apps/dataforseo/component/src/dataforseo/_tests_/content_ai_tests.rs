//! Content, domain, and AI visibility conformance fixtures.

use serde_json::json;

use super::super::error::Error;
use super::support::{call, invalid};

#[test]
fn content_tools_render_only_typed_filters_and_fixed_ordering() {
    let (_, search) = call(
        "dataforseo_content_search",
        json!({
            "keyword": "rust",
            "page_types": ["news", "blogs"],
            "country_code": "US",
            "language_code": "en",
            "min_domain_rank": 40,
            "sentiment": "positive",
            "search_mode": "one_per_domain",
            "limit": 8,
            "offset": 2
        }),
        Vec::new(),
    );
    assert_eq!(search.task["page_type"], json!(["news", "blogs"]));
    assert_eq!(
        search.task["filters"],
        json!([
            ["country", "=", "US"],
            "and",
            ["language", "=", "en"],
            "and",
            ["domain_rank", ">=", 40],
            "and",
            ["content_info.connotation_types.positive", ">", 0]
        ])
    );
    assert_eq!(search.task["search_mode"], "one_per_domain");
    assert_eq!(
        search.task["order_by"],
        json!(["content_info.sentiment_connotations.anger,desc"])
    );

    let (_, sentiment) = call(
        "dataforseo_content_sentiment",
        json!({"keyword": "rust", "country_code": "US"}),
        Vec::new(),
    );
    assert_eq!(
        sentiment.task["initial_dataset_filters"],
        json!([["country", "=", "US"]])
    );
    assert_eq!(sentiment.task["internal_list_limit"], 10);
    assert!(sentiment.task.get("filters").is_none());
}

#[test]
fn content_validation_and_distributions_are_bounded_and_deterministic() {
    for input in [
        json!({"keyword": "rust", "country_code": "us"}),
        json!({"keyword": "rust", "language_code": "EN"}),
        json!({"keyword": "rust", "page_types": ["news", "news"]}),
    ] {
        assert!(matches!(
            invalid("dataforseo_content_sentiment", input),
            Error::InvalidRequest(_)
        ));
    }

    let (output, _) = call(
        "dataforseo_content_sentiment",
        json!({"keyword": "rust"}),
        vec![json!({"items": [{
            "positive_connotation_distribution": {"positive": {
                "total_count": 10,
                "top_domains": [
                    {"domain": "z.example", "count": 2},
                    {"domain": "b.example", "count": 5},
                    {"domain": "a.example", "count": 5},
                    {"domain": "bad.example", "count": "secret"},
                    {"domain": "null.example", "count": null}
                ]
            }}
        }]})],
    );
    assert_eq!(
        output["items"][0]["top_domains"]["positive"],
        json!([
            {"domain": "a.example", "count": 5},
            {"domain": "b.example", "count": 5},
            {"domain": "z.example", "count": 2},
            {"domain": "null.example", "count": null}
        ])
    );
}

#[test]
fn domain_tools_sort_technologies_and_redact_all_contact_data() {
    let (technologies, request) = call(
        "dataforseo_domain_technologies",
        json!({"hostname": "example.com"}),
        vec![json!({"items": [{
            "domain": "example.com",
            "emails": ["secret@example.com"],
            "phone_numbers": ["555-secret"],
            "technologies": {
                "z-group": {"a-category": ["Zed"]},
                "a-group": {"z-category": [{"name": "Alpha"}]},
                "": {"b-category": ["No Group"]}
            }
        }]})],
    );
    assert_eq!(request.task, json!({"target": "example.com"}));
    assert_eq!(technologies["items"][0]["technologies"][0]["name"], "Alpha");
    assert_eq!(technologies["items"][0]["technologies"][1]["name"], "Zed");
    assert_eq!(
        technologies["items"][0]["technologies"][2]["group"],
        json!(null)
    );
    let encoded = technologies.to_string();
    assert!(!encoded.contains("secret@example.com"));
    assert!(!encoded.contains("555-secret"));

    let (whois, request) = call(
        "dataforseo_domain_whois",
        json!({"hostname": "example.com"}),
        vec![json!({"items": [{
            "domain": "example.com",
            "registrar": "Example Registrar",
            "registrant": {"email": "owner@example.com", "phone": "555-private"},
            "administrative_contacts": [{"name": "Private Person"}]
        }]})],
    );
    assert_eq!(
        request.task,
        json!({"filters": [["domain", "=", "example.com"]], "limit": 1, "offset": 0})
    );
    let encoded = whois.to_string();
    assert!(!encoded.contains("owner@example.com"));
    assert!(!encoded.contains("555-private"));
    assert!(!encoded.contains("Private Person"));
}

#[test]
fn ai_keyword_volume_enforces_dedupe_and_month_bounds() {
    assert!(matches!(
        invalid(
            "dataforseo_ai_keyword_volume",
            json!({
                "keywords": ["Rust", " rust "],
                "location_code": 2840,
                "language_code": "en"
            })
        ),
        Error::InvalidRequest(_)
    ));
    let (output, _) = call(
        "dataforseo_ai_keyword_volume",
        json!({
            "keywords": ["rust"],
            "location_code": 2840,
            "language_code": "en"
        }),
        vec![json!({"items": [{
            "keyword": "rust",
            "ai_search_volume": 100,
            "ai_monthly_searches": [{"year": 2026, "month": 0, "ai_search_volume": 20}]
        }]})],
    );
    assert_eq!(
        output["items"][0]["monthly_searches"][0]["month"],
        json!(null)
    );
}

#[test]
fn llm_mentions_enforces_platform_scopes_inclusion_and_timeout() {
    let (output, request) = call(
        "dataforseo_llm_mentions",
        json!({
            "platform": "chat_gpt",
            "location_name": "United States",
            "language_name": "English",
            "targets": [
                {"domain": "example.com", "search_scope": ["sources"], "include_subdomains": true},
                {"keyword": "example", "search_scope": ["question", "answer"], "match_type": "partial_match"}
            ],
            "timeout_seconds": 300
        }),
        vec![json!({"aggregated_metrics": {
            "total": {"mentions": 2, "ai_search_volume": 10},
            "platform": [
                {"key": "chat_gpt", "mentions": 2, "ai_search_volume": 10},
                {"key": "extra", "mentions": 1, "ai_search_volume": 1},
                {"key": "discarded", "mentions": 1, "ai_search_volume": 1}
            ]
        }})],
    );
    assert_eq!(request.timeout_seconds, 300);
    assert_eq!(request.task["internal_list_limit"], 5);
    assert_eq!(request.task["target"][0]["include_subdomains"], true);
    assert_eq!(request.task["target"][1]["match_type"], "partial_match");
    assert!(request.task.get("initial_dataset_filters").is_none());
    assert_eq!(
        output["items"][0]["by_platform"].as_array().unwrap().len(),
        2
    );

    for input in [
        json!({
            "platform": "chat_gpt",
            "location_code": 2826,
            "language_code": "en",
            "targets": [{"domain": "example.com"}]
        }),
        json!({
            "platform": "google",
            "location_code": 2840,
            "language_code": "en",
            "targets": [{"domain": "example.com", "search_scope": ["any", "sources"]}]
        }),
        json!({
            "platform": "google",
            "location_code": 2840,
            "language_code": "en",
            "targets": [{"domain": "example.com", "search_scope": ["any", "sources", "search_results", "sources"]}]
        }),
        json!({
            "platform": "google",
            "location_code": 2840,
            "language_code": "en",
            "targets": [{"keyword": "excluded", "search_filter": "exclude"}]
        }),
        json!({
            "platform": "google",
            "location_code": 2840,
            "language_code": "en",
            "targets": [{"domain": "example.com"}],
            "timeout_seconds": 301
        }),
    ] {
        assert!(matches!(
            invalid("dataforseo_llm_mentions", input),
            Error::InvalidRequest(_)
        ));
    }
}
