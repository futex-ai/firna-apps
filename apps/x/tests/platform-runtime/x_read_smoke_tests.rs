use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use fna_apps_interface::runtime::{AppToolUsageReport, AppToolUsageUnitReport};
use fna_apps_wasm::{HostCredentialInjectionKind, HostHttpRequest, HostHttpResponse, WasmHostMock};
use serde_json::json;
use unimock::{MockFn as _, Unimock, matching};
use uuid::Uuid;

use crate::x_runtime_support::runtime_with_host;
use crate::x_test_support::{assert_no_null_fields, call_tool_result};

#[tokio::test]
async fn x_component_smokes_bounded_lookup_and_recent_search() {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let captured = Arc::clone(&requests);
    let runtime = runtime_with_host(Arc::new(Unimock::new(
        WasmHostMock::http_request
            .each_call(matching!(_))
            .answers_arc(Arc::new(move |_, request: HostHttpRequest| {
                captured
                    .lock()
                    .expect("request capture lock")
                    .push(request.clone());
                response_for(&request)
            })),
    )));
    let installation_id = Uuid::now_v7();

    let posts = call_tool_result(
        &runtime,
        installation_id,
        "x_get_posts",
        "x.get_posts",
        None,
        json!({"ids": ["11", "22"], "include_authors": true}),
    )
    .await
    .expect("X Post lookup should complete");
    assert_read_usage(posts.usage.as_ref(), 1, 1);
    let posts = posts.output;
    assert!(posts.get("usage").is_none());
    assert_eq!(posts["posts"][0]["id"], "11");
    assert_eq!(posts["authors"][0]["username"], "ada");
    assert_eq!(posts["missing_ids"], json!(["22"]));
    assert!(posts["posts"][0].get("public_metrics").is_none());
    assert_no_null_fields(&posts);

    let search = call_tool_result(
        &runtime,
        installation_id,
        "x_search_recent_posts",
        "x.search_recent_posts",
        None,
        json!({
            "query": "  rust lang:en  ",
            "max_results": 10,
            "next_token": " current-page "
        }),
    )
    .await
    .expect("X recent search should complete");
    assert_read_usage(search.usage.as_ref(), 1, 0);
    let search = search.output;
    assert!(search.get("usage").is_none());
    assert_eq!(search["posts"][0]["id"], "33");
    assert_eq!(search["next_token"], "next-page");
    assert_eq!(search["result_count"], 1);
    assert!(search.get("authors").is_none());
    assert_no_null_fields(&search);

    let requests = requests.lock().expect("request capture lock");
    assert_eq!(requests.len(), 2);
    assert_lookup_request(&requests[0], installation_id);
    assert_search_request(&requests[1], installation_id);
}

#[tokio::test]
async fn x_component_reports_zero_usage_for_an_empty_search_page() {
    let runtime = runtime_with_host(Arc::new(Unimock::new(
        WasmHostMock::http_request
            .next_call(matching!(_))
            .returns(response_for_empty_search()),
    )));

    let result = call_tool_result(
        &runtime,
        Uuid::now_v7(),
        "x_search_recent_posts",
        "x.search_recent_posts",
        None,
        json!({"query": "nothing", "max_results": 10}),
    )
    .await
    .expect("empty X recent search should complete");

    assert_eq!(result.output["result_count"], 0);
    assert_read_usage(result.usage.as_ref(), 0, 0);
}

fn assert_read_usage(usage: Option<&AppToolUsageReport>, posts: u64, users: u64) {
    assert_eq!(
        usage,
        Some(&AppToolUsageReport::Metered {
            units: vec![
                AppToolUsageUnitReport {
                    unit: String::from("post_read"),
                    quantity: posts,
                },
                AppToolUsageUnitReport {
                    unit: String::from("user_read"),
                    quantity: users,
                },
            ],
        })
    );
}

fn response_for_empty_search() -> HostHttpResponse {
    HostHttpResponse {
        ok: true,
        status: Some(200),
        url: Some(String::from("https://api.x.com/2/tweets/search/recent")),
        headers: BTreeMap::new(),
        content_type: Some(String::from("application/json")),
        body_json: Some(json!({"data": []})),
        body_truncated: false,
        error: None,
    }
}

fn response_for(request: &HostHttpRequest) -> HostHttpResponse {
    let body_json = if request.url.ends_with("/search/recent") {
        json!({
            "data": [{"id": "33", "text": "Result"}],
            "meta": {"result_count": 1, "next_token": "next-page"}
        })
    } else {
        json!({
            "data": [{
                "id": "11",
                "text": "First",
                "author_id": "7",
                "created_at": "2026-08-01T09:00:00Z",
                "public_metrics": {"like_count": 9000}
            }],
            "includes": {"users": [{"id": "7", "name": "Ada", "username": "ada"}]}
        })
    };
    HostHttpResponse {
        ok: true,
        status: Some(200),
        url: Some(request.url.clone()),
        headers: BTreeMap::new(),
        content_type: Some(String::from("application/json")),
        body_json: Some(body_json),
        body_truncated: false,
        error: None,
    }
}

fn assert_common_request(request: &HostHttpRequest, installation_id: Uuid) {
    assert_eq!(request.method, "GET");
    assert_eq!(request.timeout_seconds, Some(30));
    assert_eq!(request.response_body_limit_bytes, Some(262_144));
    assert!(request.body_json.is_none());
    assert!(request.headers.is_empty());
    let credential = request.credential.as_ref().expect("opaque credential");
    assert_eq!(credential.app_id, "x");
    assert_eq!(credential.credential_kind, "access_token");
    assert_eq!(credential.installation_id, Some(installation_id));
    assert_eq!(credential.user_grant_id, None);
    assert_eq!(credential.effective_user_id, None);
    assert_eq!(
        request
            .credential_injection
            .as_ref()
            .expect("bearer injection")
            .kind,
        HostCredentialInjectionKind::BearerAuthorization
    );
}

fn assert_lookup_request(request: &HostHttpRequest, installation_id: Uuid) {
    assert_common_request(request, installation_id);
    assert_eq!(request.url, "https://api.x.com/2/tweets");
    assert_eq!(
        request.query,
        BTreeMap::from([
            (String::from("expansions"), String::from("author_id")),
            (String::from("ids"), String::from("11,22")),
            (
                String::from("tweet.fields"),
                String::from("author_id,created_at,text")
            ),
            (
                String::from("user.fields"),
                String::from("id,name,username")
            ),
        ])
    );
}

fn assert_search_request(request: &HostHttpRequest, installation_id: Uuid) {
    assert_common_request(request, installation_id);
    assert_eq!(request.url, "https://api.x.com/2/tweets/search/recent");
    assert_eq!(
        request.query,
        BTreeMap::from([
            (String::from("max_results"), String::from("10")),
            (String::from("next_token"), String::from("current-page")),
            (String::from("query"), String::from("rust lang:en")),
            (
                String::from("tweet.fields"),
                String::from("author_id,created_at,text")
            ),
        ])
    );
}
