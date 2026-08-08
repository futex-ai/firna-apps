use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use fna_apps_interface::runtime::{AppToolUsageReport, AppToolUsageUnitReport};
use fna_apps_wasm::{HostCredentialInjectionKind, HostHttpRequest, WasmHostMock};
use serde_json::{Value, json};
use unimock::{MockFn as _, Unimock, matching};
use uuid::Uuid;

use crate::x_runtime_support::runtime_with_host;
use crate::x_test_support::{assert_no_null_fields, call_tool_result, provider_response};

#[tokio::test]
async fn x_component_smokes_public_and_private_post_metrics() {
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

    let public = call_tool_result(
        &runtime,
        installation_id,
        "x_get_post_metrics",
        "x.get_post_metrics",
        None,
        json!({"ids": ["11", "22", "33"]}),
    )
    .await
    .expect("public X metrics should complete");
    assert_post_usage(public.usage.as_ref(), 2);
    let public = public.output;
    assert!(public.get("usage").is_none());
    assert_eq!(public["metrics"][0]["id"], "11");
    assert_eq!(public["metrics"][1]["id"], "22");
    assert_eq!(public["metrics"][0]["public_metrics"]["impressions"], 10);
    assert_eq!(public["missing_ids"], json!(["33"]));
    assert!(public["metrics"][0].get("text").is_none());
    assert!(public["metrics"][0].get("private_metrics").is_none());
    assert_no_null_fields(&public);

    let private = call_tool_result(
        &runtime,
        installation_id,
        "x_get_post_metrics",
        "x.get_post_metrics",
        None,
        json!({"ids": ["44", "55", "66"], "include_private_metrics": true}),
    )
    .await
    .expect("private X metrics should complete");
    assert_post_usage(private.usage.as_ref(), 2);
    let private = private.output;
    assert_eq!(
        private["metrics"][0]["private_metrics"],
        json!({"engagements": 0, "url_clicks": 3, "profile_clicks": 0})
    );
    assert_eq!(
        private["metrics"][1]["unavailable_private_metrics"],
        json!(["engagements", "url_clicks", "profile_clicks"])
    );
    assert!(private["metrics"][1].get("private_metrics").is_none());
    assert_eq!(private["missing_ids"], json!(["66"]));
    let encoded = serde_json::to_string(&private).expect("serialize private output");
    assert!(!encoded.contains("user_profile_clicks"));
    assert!(!encoded.contains("profile_views"));
    assert_no_null_fields(&private);

    let requests = requests.lock().expect("request capture lock");
    assert_eq!(requests.len(), 2);
    assert_request(&requests[0], installation_id, "11,22,33", "public_metrics");
    assert_request(
        &requests[1],
        installation_id,
        "44,55,66",
        "public_metrics,non_public_metrics",
    );
}

fn response_for(request: &HostHttpRequest) -> fna_apps_wasm::HostHttpResponse {
    let body = if request.query["tweet.fields"].contains("non_public_metrics") {
        json!({
            "data": [
                {
                    "id": "55",
                    "public_metrics": public_metrics(50)
                },
                {
                    "id": "44",
                    "public_metrics": public_metrics(40),
                    "non_public_metrics": {
                        "engagements": 0,
                        "url_link_clicks": 3,
                        "user_profile_clicks": 0
                    }
                }
            ]
        })
    } else {
        json!({
            "data": [
                {"id": "22", "public_metrics": public_metrics(20)},
                {
                    "id": "11",
                    "text": "must stay private to the provider boundary",
                    "public_metrics": public_metrics(10)
                }
            ]
        })
    };
    provider_response(200, Some(body))
}

fn public_metrics(base: u64) -> Value {
    json!({
        "impression_count": base,
        "like_count": base + 1,
        "reply_count": base + 2,
        "retweet_count": base + 3,
        "quote_count": base + 4,
        "bookmark_count": base + 5
    })
}

fn assert_post_usage(usage: Option<&AppToolUsageReport>, posts: u64) {
    assert_eq!(
        usage,
        Some(&AppToolUsageReport::Metered {
            units: vec![AppToolUsageUnitReport {
                unit: String::from("post_read"),
                quantity: posts,
            }],
        })
    );
}

fn assert_request(request: &HostHttpRequest, installation_id: Uuid, ids: &str, fields: &str) {
    assert_eq!(request.method, "GET");
    assert_eq!(request.url, "https://api.x.com/2/tweets");
    assert_eq!(
        request.query,
        BTreeMap::from([
            (String::from("ids"), String::from(ids)),
            (String::from("tweet.fields"), String::from(fields)),
        ])
    );
    assert_eq!(request.timeout_seconds, Some(30));
    assert_eq!(request.response_body_limit_bytes, Some(262_144));
    assert!(request.body_json.is_none());
    assert!(request.headers.is_empty());
    let credential = request.credential.as_ref().expect("opaque credential");
    assert_eq!(credential.app_id, "x");
    assert_eq!(credential.credential_kind, "access_token");
    assert_eq!(credential.installation_id, Some(installation_id));
    assert_eq!(
        request
            .credential_injection
            .as_ref()
            .expect("bearer injection")
            .kind,
        HostCredentialInjectionKind::BearerAuthorization
    );
}
