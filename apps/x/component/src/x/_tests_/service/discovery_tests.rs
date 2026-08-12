use serde_json::json;

use super::support::{assert_error, capturing_http, invoke, response, success_output};

#[test]
fn list_post_read_returns_posts_and_expanded_authors() {
    let (http, requests) = capturing_http(response(
        200,
        Some(json!({
            "data": [{"id": "11", "text": "List Post", "author_id": "7"}],
            "includes": {"users": [{"id": "7", "name": "Ada", "username": "ada"}]}
        })),
    ));

    let output = invoke(
        &http,
        "x_get_lists",
        json!({"view": "posts", "list_id": "4", "max_results": 10, "include_authors": true}),
    );

    assert_eq!(success_output(&output)["posts"][0]["id"], "11");
    assert_eq!(success_output(&output)["authors"][0]["id"], "7");
    let requests = requests.lock().expect("request capture lock");
    assert_eq!(requests[0].url, "https://api.x.com/2/lists/4/tweets");
}

#[test]
fn pinned_lists_use_the_provider_endpoint_without_unsupported_paging() {
    let (http, requests) = capturing_http(response(200, Some(json!({"data": []}))));

    let output = invoke(
        &http,
        "x_get_lists",
        json!({"view": "pinned", "user_id": "4"}),
    );

    assert_eq!(success_output(&output)["result_count"], 0);
    let requests = requests.lock().expect("request capture lock");
    assert_eq!(requests[0].url, "https://api.x.com/2/users/4/pinned_lists");
    assert!(!requests[0].query.contains_key("max_results"));
    assert!(!requests[0].query.contains_key("pagination_token"));

    let invalid = invoke(
        &unimock::Unimock::new(()),
        "x_get_lists",
        json!({"view": "pinned", "user_id": "4", "max_results": 10}),
    );
    assert_error(&invalid, "invalid_request");
}

#[test]
fn space_search_returns_compact_space_resources() {
    let (http, requests) = capturing_http(response(
        200,
        Some(json!({"data": [{"id": "5", "state": "live", "title": "Rust"}]})),
    ));

    let output = invoke(
        &http,
        "x_get_spaces",
        json!({"view": "search", "query": "rust", "state": "live", "max_results": 10}),
    );

    assert_eq!(success_output(&output)["spaces"][0]["state"], "live");
    assert_eq!(output["usage"]["units"][0]["unit"], "space_read");
    let requests = requests.lock().expect("request capture lock");
    assert_eq!(requests[0].url, "https://api.x.com/2/spaces/search");
    assert_eq!(requests[0].query["state"], "live");
}

#[test]
fn community_lookup_is_single_and_provider_confirmed() {
    let (http, requests) = capturing_http(response(
        200,
        Some(json!({"data": {"id": "6", "name": "Rustaceans", "access": "Open"}})),
    ));

    let output = invoke(
        &http,
        "x_get_communities",
        json!({"view": "ids", "ids": ["6"]}),
    );

    assert_eq!(
        success_output(&output)["communities"][0]["name"],
        "Rustaceans"
    );
    let requests = requests.lock().expect("request capture lock");
    assert_eq!(requests[0].url, "https://api.x.com/2/communities/6");
}

#[test]
fn personalized_trends_accept_provider_string_counts() {
    let (http, requests) = capturing_http(response(
        200,
        Some(json!({"data": [{"trend_name": "Rust", "post_count": "42"}]})),
    ));

    let output = invoke(&http, "x_get_trends", json!({"view": "personalized"}));

    assert_eq!(success_output(&output)["trends"][0]["post_count"], 42);
    let requests = requests.lock().expect("request capture lock");
    assert_eq!(
        requests[0].url,
        "https://api.x.com/2/users/personalized_trends"
    );
    assert_eq!(requests[0].credential.credential_kind, "access_token");
}

#[test]
fn location_trends_use_the_app_bearer() {
    let (http, requests) = capturing_http(response(
        200,
        Some(json!({"data": [{"trend_name": "Rust", "tweet_count": 9}]})),
    ));

    let output = invoke(
        &http,
        "x_get_trends",
        json!({"view": "location", "woeid": 1, "max_trends": 10}),
    );

    assert_eq!(success_output(&output)["result_count"], 1);
    let requests = requests.lock().expect("request capture lock");
    assert_eq!(requests[0].url, "https://api.x.com/2/trends/by/woeid/1");
    assert_eq!(requests[0].credential.credential_kind, "bearer_token");
    assert_eq!(requests[0].credential.installation_id, None);
}

#[test]
fn media_metadata_is_typed_and_billed_per_returned_media() {
    let (http, requests) = capturing_http(response(
        200,
        Some(json!({"data": [{
            "media_key": "3_12", "type": "video", "width": 1920,
            "public_metrics": {"view_count": 5}
        }]})),
    ));

    let output = invoke(&http, "x_get_media", json!({"media_keys": ["3_12"]}));

    assert_eq!(success_output(&output)["media"][0]["width"], 1920);
    assert_eq!(output["usage"]["units"][0]["unit"], "media_read");
    let requests = requests.lock().expect("request capture lock");
    assert_eq!(requests[0].query["media_keys"], "3_12");
}

#[test]
fn single_dm_event_read_rejects_no_provider_fields() {
    let (http, requests) = capturing_http(response(
        200,
        Some(json!({"data": {
            "id": "15", "event_type": "MessageCreate", "sender_id": "7", "text": "hello"
        }})),
    ));

    let output = invoke(
        &http,
        "x_get_dms",
        json!({"view": "event", "event_id": "15"}),
    );

    assert_eq!(success_output(&output)["events"][0]["text"], "hello");
    assert_eq!(output["usage"]["units"][0]["unit"], "dm_event_read");
    let requests = requests.lock().expect("request capture lock");
    assert_eq!(requests[0].url, "https://api.x.com/2/dm_events/15");
}
