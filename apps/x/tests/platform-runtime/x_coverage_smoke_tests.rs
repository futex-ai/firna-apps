use std::sync::{Arc, Mutex};

use fna_apps_wasm::{HostHttpRequest, HostHttpResponse, WasmHostMock};
use serde_json::{Value, json};
use unimock::{MockFn as _, Unimock, matching};
use uuid::Uuid;

use crate::x_runtime_support::runtime_with_host;
use crate::x_test_support::{call_tool_result, provider_response};

#[tokio::test]
async fn expanded_x_surface_executes_through_the_real_wasm_boundary() {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let captured = Arc::clone(&requests);
    let runtime = runtime_with_host(Arc::new(Unimock::new(
        WasmHostMock::http_request
            .each_call(matching!(_))
            .answers_arc(Arc::new(move |_, request: HostHttpRequest| {
                let response = response_for(&request);
                captured.lock().expect("request capture lock").push(request);
                response
            })),
    )));
    let installation_id = Uuid::now_v7();
    let calls = coverage_calls();

    for (tool, operation, input) in &calls {
        let result = call_tool_result(
            &runtime,
            installation_id,
            tool,
            operation,
            None,
            input.clone(),
        )
        .await
        .unwrap_or_else(|error| panic!("{tool} should complete: {error}"));
        assert!(result.output.is_object(), "{tool} output");
        assert!(result.usage.is_some(), "{tool} usage");
    }

    let requests = requests.lock().expect("request capture lock");
    assert_eq!(requests.len(), calls.len());
    let app_requests = requests
        .iter()
        .filter(|request| {
            request
                .credential
                .as_ref()
                .is_some_and(|credential| credential.credential_kind == "bearer_token")
        })
        .collect::<Vec<_>>();
    assert_eq!(app_requests.len(), 3);
    assert!(app_requests.iter().all(|request| {
        request
            .credential
            .as_ref()
            .is_some_and(|credential| credential.installation_id.is_none())
    }));
    assert_eq!(
        requests
            .iter()
            .filter(|request| request.method != "GET")
            .count(),
        6
    );
}

fn coverage_calls() -> Vec<(&'static str, &'static str, Value)> {
    vec![
        (
            "x_search_all_posts",
            "x.search_all_posts",
            json!({"query": "rust", "max_results": 10}),
        ),
        (
            "x_get_post_counts",
            "x.get_post_counts",
            json!({"range": "all", "query": "rust", "granularity": "day"}),
        ),
        (
            "x_get_users",
            "x.get_users",
            json!({"lookup": "ids", "ids": ["7"]}),
        ),
        (
            "x_search_users",
            "x.search_users",
            json!({"query": "Ada", "max_results": 10}),
        ),
        (
            "x_get_user_feed",
            "x.get_user_feed",
            json!({"feed": "posts", "user_id": "7", "max_results": 10}),
        ),
        (
            "x_get_post_engagements",
            "x.get_post_engagements",
            json!({"post_id": "11", "view": "liking_users", "max_results": 10}),
        ),
        (
            "x_get_relationships",
            "x.get_relationships",
            json!({"user_id": "7", "relationship": "followers", "max_results": 10}),
        ),
        (
            "x_get_lists",
            "x.get_lists",
            json!({"view": "list", "list_id": "4"}),
        ),
        (
            "x_get_spaces",
            "x.get_spaces",
            json!({"view": "ids", "ids": ["5"]}),
        ),
        (
            "x_get_communities",
            "x.get_communities",
            json!({"view": "ids", "ids": ["6"]}),
        ),
        (
            "x_get_trends",
            "x.get_trends",
            json!({"view": "location", "woeid": 1, "max_trends": 10}),
        ),
        (
            "x_get_dms",
            "x.get_dms",
            json!({"view": "event", "event_id": "15"}),
        ),
        (
            "x_get_media",
            "x.get_media",
            json!({"media_keys": ["3_12"]}),
        ),
        (
            "x_manage_post",
            "x.manage_post",
            json!({"action": "delete", "post_id": "11"}),
        ),
        (
            "x_manage_relationship",
            "x.manage_relationship",
            json!({"action": "mute", "user_id": "7", "target_user_id": "8"}),
        ),
        (
            "x_manage_list",
            "x.manage_list",
            json!({"action": "create", "name": "Rust"}),
        ),
        (
            "x_manage_dm",
            "x.manage_dm",
            json!({"action": "delete", "event_id": "15"}),
        ),
        (
            "x_manage_media",
            "x.manage_media",
            json!({"action": "set_alt_text", "media_id": "31", "alt_text": "Diagram"}),
        ),
        (
            "x_create_bookmark_folder",
            "x.create_bookmark_folder",
            json!({"user_id": "7", "name": "Research"}),
        ),
    ]
}

fn response_for(request: &HostHttpRequest) -> HostHttpResponse {
    let body = match (request.method.as_str(), request.url.as_str()) {
        ("GET", url) if url.ends_with("/tweets/search/all") => {
            json!({"data": [{"id": "11", "text": "Archive"}]})
        }
        ("GET", url) if url.ends_with("/tweets/counts/all") => json!({
            "data": [{"start": "2026-01-01", "end": "2026-01-02", "tweet_count": 1}],
            "meta": {"total_tweet_count": 1}
        }),
        ("GET", "https://api.x.com/2/users")
        | ("GET", "https://api.x.com/2/users/search")
        | ("GET", "https://api.x.com/2/tweets/11/liking_users")
        | ("GET", "https://api.x.com/2/users/7/followers") => {
            json!({"data": [{"id": "7", "name": "Ada", "username": "ada"}]})
        }
        ("GET", "https://api.x.com/2/users/7/tweets") => {
            json!({"data": [{"id": "11", "text": "Feed"}]})
        }
        ("GET", "https://api.x.com/2/lists/4") => {
            json!({"data": {"id": "4", "name": "Rust"}})
        }
        ("GET", "https://api.x.com/2/spaces") => {
            json!({"data": [{"id": "5", "state": "live"}]})
        }
        ("GET", "https://api.x.com/2/communities/6") => {
            json!({"data": {"id": "6", "name": "Rustaceans"}})
        }
        ("GET", "https://api.x.com/2/trends/by/woeid/1") => {
            json!({"data": [{"trend_name": "Rust", "tweet_count": 1}]})
        }
        ("GET", "https://api.x.com/2/dm_events/15") => {
            json!({"data": {"id": "15", "text": "hello"}})
        }
        ("GET", "https://api.x.com/2/media") => {
            json!({"data": [{"media_key": "3_12", "type": "photo"}]})
        }
        ("DELETE", "https://api.x.com/2/tweets/11") => json!({"data": {"deleted": true}}),
        ("POST", "https://api.x.com/2/users/7/muting") => json!({"data": {"muting": true}}),
        ("POST", "https://api.x.com/2/lists") => json!({"data": {"id": "4", "name": "Rust"}}),
        ("DELETE", "https://api.x.com/2/dm_events/15") => json!({"data": {"deleted": true}}),
        ("POST", "https://api.x.com/2/media/metadata") => json!({"data": {"id": "31"}}),
        ("POST", "https://api.x.com/2/users/7/bookmarks/folders") => {
            json!({"data": {"id": "3", "name": "Research"}})
        }
        _ => panic!(
            "unexpected X coverage request: {} {}",
            request.method, request.url
        ),
    };
    provider_response(200, Some(body))
}
