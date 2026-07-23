use std::sync::{Arc, Mutex};

use serde_json::{Value, json};
use unimock::{MockFn as _, Unimock, matching};

use super::super::host::{HostHttpResponse, ProviderClientPostTask};
use super::super::tools;

#[derive(Debug)]
struct CapturedRequest {
    path: String,
    task: Value,
    timeout_seconds: u64,
}

#[test]
fn every_tool_uses_its_reviewed_live_endpoint_and_bounded_task() {
    let captured = Arc::new(Mutex::new(Vec::new()));
    let requests = Arc::clone(&captured);
    let client = Unimock::new(
        ProviderClientPostTask
            .each_call(matching!(_, _, _))
            .answers_arc(Arc::new(move |_, path: &str, task, timeout_seconds| {
                requests.lock().unwrap().push(CapturedRequest {
                    path: path.to_owned(),
                    task,
                    timeout_seconds,
                });
                provider_success()
            })),
    );

    for (tool, input, expected_path) in tool_cases() {
        let output = tools::call(&client, tool, input).unwrap();
        assert_eq!(output["ok"], true, "{tool}");
        assert_eq!(output["provider"], "dataforseo", "{tool}");
        assert_eq!(output["task_id"], "task-redacted", "{tool}");
        assert_eq!(
            captured.lock().unwrap().last().unwrap().path,
            expected_path,
            "{tool}"
        );
    }

    let requests = captured.lock().unwrap();
    assert_eq!(requests.len(), 16);
    assert!(
        requests[..15]
            .iter()
            .all(|request| request.timeout_seconds == 180)
    );
    assert_eq!(requests.last().unwrap().timeout_seconds, 240);
    assert_eq!(requests[1].task["include_clickstream_data"], false);
    assert_eq!(requests[1].task["include_serp_info"], false);
    assert_request_exclusions(&requests);
}

fn tool_cases() -> Vec<(&'static str, Value, &'static str)> {
    let selectors = json!({
        "location_code": 2840,
        "language_code": "en"
    });
    vec![
        (
            "dataforseo_google_serp",
            merge(selectors.clone(), json!({"keyword": "rust language"})),
            "/v3/serp/google/organic/live/advanced",
        ),
        (
            "dataforseo_keyword_overview",
            merge(selectors.clone(), json!({"keywords": ["rust", "wasm"]})),
            "/v3/dataforseo_labs/google/keyword_overview/live",
        ),
        (
            "dataforseo_keyword_suggestions",
            merge(selectors.clone(), json!({"keyword": "rust", "limit": 3})),
            "/v3/dataforseo_labs/google/keyword_suggestions/live",
        ),
        (
            "dataforseo_ranked_keywords",
            merge(
                selectors.clone(),
                json!({"target": "example.com", "limit": 3}),
            ),
            "/v3/dataforseo_labs/google/ranked_keywords/live",
        ),
        (
            "dataforseo_backlinks_summary",
            json!({"target": "example.com"}),
            "/v3/backlinks/summary/live",
        ),
        (
            "dataforseo_backlinks",
            json!({"target": "example.com", "limit": 3}),
            "/v3/backlinks/backlinks/live",
        ),
        (
            "dataforseo_referring_domains",
            json!({"target": "example.com", "limit": 3}),
            "/v3/backlinks/referring_domains/live",
        ),
        (
            "dataforseo_instant_page_audit",
            json!({"url": "https://example.com/page"}),
            "/v3/on_page/instant_pages",
        ),
        (
            "dataforseo_business_search",
            json!({
                "latitude": 51.5072,
                "longitude": -0.1276,
                "radius_km": 5,
                "query": "coffee",
                "limit": 3
            }),
            "/v3/business_data/business_listings/search/live",
        ),
        (
            "dataforseo_business_info",
            merge(
                selectors.clone(),
                json!({"business_name": "Example Coffee"}),
            ),
            "/v3/business_data/google/my_business_info/live",
        ),
        (
            "dataforseo_content_search",
            json!({"keyword": "rust", "limit": 3}),
            "/v3/content_analysis/search/live",
        ),
        (
            "dataforseo_content_sentiment",
            json!({"keyword": "rust"}),
            "/v3/content_analysis/sentiment_analysis/live",
        ),
        (
            "dataforseo_domain_technologies",
            json!({"hostname": "example.com"}),
            "/v3/domain_analytics/technologies/domain_technologies/live",
        ),
        (
            "dataforseo_domain_whois",
            json!({"hostname": "example.com"}),
            "/v3/domain_analytics/whois/overview/live",
        ),
        (
            "dataforseo_ai_keyword_volume",
            merge(selectors.clone(), json!({"keywords": ["rust"]})),
            "/v3/ai_optimization/ai_keyword_data/keywords_search_volume/live",
        ),
        (
            "dataforseo_llm_mentions",
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

fn merge(mut base: Value, additional: Value) -> Value {
    let Some(base) = base.as_object_mut() else {
        return Value::Null;
    };
    if let Some(additional) = additional.as_object() {
        base.extend(additional.clone());
    }
    Value::Object(base.clone())
}

fn provider_success() -> super::super::error::Result<HostHttpResponse> {
    Ok(HostHttpResponse {
        ok: true,
        status: Some(200),
        headers: Default::default(),
        body_json: Some(json!({
            "status_code": 20000,
            "tasks": [{
                "id": "task-redacted",
                "status_code": 20000,
                "cost": 0.001,
                "result": [{"items": [{"domain": "example.com", "keyword": "rust"}]}]
            }]
        })),
        body_truncated: false,
    })
}

fn assert_request_exclusions(requests: &[CapturedRequest]) {
    let tasks = requests
        .iter()
        .map(|request| &request.task)
        .collect::<Vec<_>>();
    let encoded = serde_json::to_string(&tasks).unwrap();
    for excluded in [
        "pingback_url",
        "postback_url",
        "tag",
        "load_async_ai_overview\":true",
        "description",
    ] {
        assert!(
            !encoded.contains(excluded),
            "unexpected provider option {excluded}"
        );
    }
}
