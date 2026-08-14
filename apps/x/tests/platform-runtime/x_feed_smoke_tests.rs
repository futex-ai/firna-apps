use std::sync::{Arc, Mutex};

use fna_apps_interface::runtime::{AppToolUsageReport, AppToolUsageUnitReport};
use fna_apps_wasm::{HostHttpRequest, WasmHostMock};
use serde_json::json;
use unimock::{MockFn as _, Unimock, matching};
use uuid::Uuid;

use crate::x_runtime_support::runtime_with_host;
use crate::x_test_support::{call_tool_result, provider_response};

#[tokio::test]
async fn omitted_feed_user_resolves_the_selected_connection_through_wasm() {
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

    let result = call_tool_result(
        &runtime,
        installation_id,
        "x_get_user_feed",
        "x.get_user_feed",
        None,
        json!({"feed": "home", "max_results": 10, "include_authors": true}),
    )
    .await
    .expect("connected-account X feed should complete");

    assert_eq!(result.output["posts"][0]["id"], "11");
    assert_eq!(
        result.usage,
        Some(AppToolUsageReport::Metered {
            units: vec![
                AppToolUsageUnitReport {
                    unit: String::from("post_read"),
                    quantity: 1,
                },
                AppToolUsageUnitReport {
                    unit: String::from("user_read"),
                    quantity: 2,
                },
            ],
        })
    );
    let requests = requests.lock().expect("request capture lock");
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].url, "https://api.x.com/2/users/me");
    assert_eq!(
        requests[1].url,
        "https://api.x.com/2/users/7/timelines/reverse_chronological"
    );
    assert!(requests.iter().all(|request| {
        request
            .credential
            .as_ref()
            .is_some_and(|credential| credential.installation_id == Some(installation_id))
    }));
}

fn response_for(request: &HostHttpRequest) -> fna_apps_wasm::HostHttpResponse {
    let body = match request.url.as_str() {
        "https://api.x.com/2/users/me" => {
            json!({"data": {"id": "7", "name": "Ada", "username": "ada"}})
        }
        "https://api.x.com/2/users/7/timelines/reverse_chronological" => json!({
            "data": [{"id": "11", "text": "Post", "author_id": "7"}],
            "includes": {"users": [{"id": "7", "name": "Ada", "username": "ada"}]}
        }),
        url => panic!("unexpected X feed request: {url}"),
    };
    provider_response(200, Some(body))
}
