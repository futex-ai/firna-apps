use serde_json::{Value, json};

pub(crate) struct ToolCase {
    pub(crate) name: &'static str,
    pub(crate) operation: &'static str,
    pub(crate) input: Value,
    pub(crate) path: &'static str,
}

pub(crate) fn tool_cases() -> Vec<ToolCase> {
    let selectors = json!({"location_code": 2840, "language_code": "en"});
    vec![
        case(
            "dataforseo_google_serp",
            "dataforseo.google_serp",
            merge(selectors.clone(), json!({"keyword": "rust language"})),
            "/v3/serp/google/organic/live/advanced",
        ),
        case(
            "dataforseo_keyword_overview",
            "dataforseo.keyword_overview",
            merge(selectors.clone(), json!({"keywords": ["rust", "wasm"]})),
            "/v3/dataforseo_labs/google/keyword_overview/live",
        ),
        case(
            "dataforseo_keyword_suggestions",
            "dataforseo.keyword_suggestions",
            merge(selectors.clone(), json!({"keyword": "rust", "limit": 3})),
            "/v3/dataforseo_labs/google/keyword_suggestions/live",
        ),
        case(
            "dataforseo_ranked_keywords",
            "dataforseo.ranked_keywords",
            merge(
                selectors.clone(),
                json!({"target": "example.com", "limit": 3}),
            ),
            "/v3/dataforseo_labs/google/ranked_keywords/live",
        ),
        case(
            "dataforseo_backlinks_summary",
            "dataforseo.backlinks_summary",
            json!({"target": "example.com"}),
            "/v3/backlinks/summary/live",
        ),
        case(
            "dataforseo_backlinks",
            "dataforseo.backlinks",
            json!({"target": "example.com", "limit": 3}),
            "/v3/backlinks/backlinks/live",
        ),
        case(
            "dataforseo_referring_domains",
            "dataforseo.referring_domains",
            json!({"target": "example.com", "limit": 3}),
            "/v3/backlinks/referring_domains/live",
        ),
        case(
            "dataforseo_instant_page_audit",
            "dataforseo.instant_page_audit",
            json!({"url": "https://example.com/page"}),
            "/v3/on_page/instant_pages",
        ),
        case(
            "dataforseo_business_search",
            "dataforseo.business_search",
            json!({
                "latitude": 51.5072,
                "longitude": -0.1276,
                "radius_km": 5,
                "query": "coffee",
                "limit": 3
            }),
            "/v3/business_data/business_listings/search/live",
        ),
        case(
            "dataforseo_business_info",
            "dataforseo.business_info",
            merge(
                selectors.clone(),
                json!({"business_name": "Example Coffee"}),
            ),
            "/v3/business_data/google/my_business_info/live",
        ),
        case(
            "dataforseo_content_search",
            "dataforseo.content_search",
            json!({"keyword": "rust", "limit": 3}),
            "/v3/content_analysis/search/live",
        ),
        case(
            "dataforseo_content_sentiment",
            "dataforseo.content_sentiment",
            json!({"keyword": "rust"}),
            "/v3/content_analysis/sentiment_analysis/live",
        ),
        case(
            "dataforseo_domain_technologies",
            "dataforseo.domain_technologies",
            json!({"hostname": "example.com"}),
            "/v3/domain_analytics/technologies/domain_technologies/live",
        ),
        case(
            "dataforseo_domain_whois",
            "dataforseo.domain_whois",
            json!({"hostname": "example.com"}),
            "/v3/domain_analytics/whois/overview/live",
        ),
        case(
            "dataforseo_ai_keyword_volume",
            "dataforseo.ai_keyword_volume",
            merge(selectors.clone(), json!({"keywords": ["rust"]})),
            "/v3/ai_optimization/ai_keyword_data/keywords_search_volume/live",
        ),
        case(
            "dataforseo_llm_mentions",
            "dataforseo.llm_mentions",
            merge(
                selectors,
                json!({
                    "platform": "google",
                    "targets": [{"domain": "example.com"}],
                    "timeout_seconds": 240
                }),
            ),
            "/v3/ai_optimization/llm_mentions/target_metrics/live",
        ),
    ]
}

fn case(name: &'static str, operation: &'static str, input: Value, path: &'static str) -> ToolCase {
    ToolCase {
        name,
        operation,
        input,
        path,
    }
}

fn merge(mut base: Value, additional: Value) -> Value {
    let base = base
        .as_object_mut()
        .expect("base fixture must be an object");
    base.extend(
        additional
            .as_object()
            .expect("additional fixture must be an object")
            .clone(),
    );
    Value::Object(base.clone())
}
