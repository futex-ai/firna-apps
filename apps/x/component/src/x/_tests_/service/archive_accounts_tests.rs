use serde_json::json;

use super::support::{capturing_http, invoke, response, success_output};

#[test]
fn full_archive_search_uses_the_app_bearer_and_reports_resources() {
    let (http, requests) = capturing_http(response(
        200,
        Some(json!({
            "data": [{"id": "11", "text": "Archived", "author_id": "7"}],
            "includes": {"users": [{"id": "7", "name": "Ada", "username": "ada"}]},
            "meta": {"next_token": "next"}
        })),
    ));

    let output = invoke(
        &http,
        "x_search_all_posts",
        json!({
            "query": "rust",
            "max_results": 10,
            "start_time": "2026-01-01T00:00:00Z",
            "include_authors": true
        }),
    );

    let result = success_output(&output);
    assert_eq!(result["posts"][0]["id"], "11");
    assert_eq!(result["authors"][0]["username"], "ada");
    assert_eq!(result["pagination_token"], "next");
    assert_eq!(output["usage"]["units"][0]["quantity"], 1);
    assert_eq!(output["usage"]["units"][1]["quantity"], 1);
    let requests = requests.lock().expect("request capture lock");
    assert_eq!(requests[0].url, "https://api.x.com/2/tweets/search/all");
    assert_eq!(requests[0].query["query"], "rust");
    assert_eq!(requests[0].credential.credential_kind, "bearer_token");
    assert_eq!(requests[0].credential.installation_id, None);
}

#[test]
fn archive_search_and_counts_translate_web_engagement_operator_aliases() {
    let search_query = captured_query(
        "x_search_all_posts",
        json!({"query": "AI min_faves:1000", "max_results": 10}),
        json!({"data": [], "meta": {}}),
    );
    assert_eq!(search_query, "AI min_likes:1000");

    let count_query = captured_query(
        "x_get_post_counts",
        json!({
            "range": "recent",
            "query": "AI min_retweets:50",
            "granularity": "hour"
        }),
        json!({"data": [], "meta": {}}),
    );
    assert_eq!(count_query, "AI min_reposts:50");
}

#[test]
fn all_history_counts_use_app_auth_and_exact_request_cost() {
    let (http, requests) = capturing_http(response(
        200,
        Some(json!({
            "data": [{"start": "2026-01-01", "end": "2026-01-02", "tweet_count": 12}],
            "meta": {"total_tweet_count": 12, "next_token": "more"}
        })),
    ));

    let output = invoke(
        &http,
        "x_get_post_counts",
        json!({"range": "all", "query": "rust", "granularity": "day"}),
    );

    let result = success_output(&output);
    assert_eq!(result["buckets"][0]["post_count"], 12);
    assert_eq!(result["total_post_count"], 12);
    assert_eq!(output["usage"]["cost_usd_micros"], 10_000);
    let requests = requests.lock().expect("request capture lock");
    assert_eq!(requests[0].url, "https://api.x.com/2/tweets/counts/all");
    assert_eq!(requests[0].credential.credential_kind, "bearer_token");
}

#[test]
fn user_lookup_returns_requested_missing_values_and_profile_fields() {
    let (http, requests) = capturing_http(response(
        200,
        Some(json!({
            "data": [{
                "id": "7", "name": "Ada", "username": "ada", "verified": true,
                "public_metrics": {"followers_count": 9, "following_count": 2,
                    "tweet_count": 3, "listed_count": 1}
            }]
        })),
    ));

    let output = invoke(
        &http,
        "x_get_users",
        json!({"lookup": "ids", "ids": ["7", "8"]}),
    );

    let result = success_output(&output);
    assert_eq!(result["users"][0]["verified"], true);
    assert_eq!(result["users"][0]["public_metrics"]["post_count"], 3);
    assert_eq!(result["missing_values"], json!(["8"]));
    assert_eq!(output["usage"]["units"][0]["quantity"], 1);
    let requests = requests.lock().expect("request capture lock");
    assert_eq!(requests[0].url, "https://api.x.com/2/users");
    assert_eq!(requests[0].query["ids"], "7,8");
}

#[test]
fn user_search_maps_its_public_token_to_the_provider_next_token() {
    let (http, requests) = capturing_http(response(
        200,
        Some(json!({
            "data": [{"id": "7", "name": "Ada", "username": "ada"}],
            "meta": {"next_token": "next"}
        })),
    ));

    let output = invoke(
        &http,
        "x_search_users",
        json!({"query": "Ada_1", "max_results": 10, "pagination_token": "current"}),
    );

    assert_eq!(success_output(&output)["pagination_token"], "next");
    let requests = requests.lock().expect("request capture lock");
    assert_eq!(requests[0].url, "https://api.x.com/2/users/search");
    assert_eq!(requests[0].query["next_token"], "current");
}

#[test]
fn relationship_read_routes_the_mode_and_scope_bounded_page() {
    let (http, requests) = capturing_http(response(
        200,
        Some(json!({"data": [{"id": "8", "name": "Lin", "username": "lin"}]})),
    ));

    let output = invoke(
        &http,
        "x_get_relationships",
        json!({"user_id": "7", "relationship": "muted", "max_results": 10}),
    );

    assert_eq!(success_output(&output)["users"][0]["id"], "8");
    let requests = requests.lock().expect("request capture lock");
    assert_eq!(requests[0].url, "https://api.x.com/2/users/7/muting");
}

#[test]
fn affiliate_read_routes_through_the_relationship_tool() {
    let (http, requests) = capturing_http(response(
        200,
        Some(json!({"data": [{"id": "8", "name": "Acme", "username": "acme"}]})),
    ));

    let output = invoke(
        &http,
        "x_get_relationships",
        json!({"user_id": "7", "relationship": "affiliates", "max_results": 10}),
    );

    assert_eq!(success_output(&output)["users"][0]["id"], "8");
    let requests = requests.lock().expect("request capture lock");
    assert_eq!(requests[0].url, "https://api.x.com/2/users/7/affiliates");
}

fn captured_query(
    tool: &str,
    input: serde_json::Value,
    provider_body: serde_json::Value,
) -> String {
    let (http, requests) = capturing_http(response(200, Some(provider_body)));
    let output = invoke(&http, tool, input);
    assert!(output.get("output").is_some(), "{output}");
    requests.lock().expect("request capture lock")[0].query["query"].clone()
}
