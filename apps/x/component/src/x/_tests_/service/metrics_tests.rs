use serde_json::{Value, json};
use unimock::Unimock;

use super::support::{
    assert_error, assert_post_usage, call_with_response, capturing_http, invoke, response,
    success_output,
};

#[test]
fn metrics_returns_ordered_typed_public_partial_result_and_usage() {
    let (http, requests) = capturing_http(response(
        200,
        Some(json!({
            "data": [
                {
                    "id": "22",
                    "text": "provider-only text",
                    "public_metrics": public_metrics(20)
                },
                {"id": "11", "public_metrics": public_metrics(10)}
            ]
        })),
    ));

    let output = invoke(
        &http,
        "x_get_post_metrics",
        json!({"ids": ["11", "22", "33"]}),
    );

    let result = success_output(&output);
    assert_eq!(result["metrics"][0]["id"], "11");
    assert_eq!(result["metrics"][1]["id"], "22");
    assert_eq!(result["metrics"][0]["public_metrics"]["impressions"], 10);
    assert_eq!(result["metrics"][0]["public_metrics"]["likes"], 11);
    assert_eq!(result["metrics"][0]["public_metrics"]["replies"], 12);
    assert_eq!(result["metrics"][0]["public_metrics"]["reposts"], 13);
    assert_eq!(result["metrics"][0]["public_metrics"]["quotes"], 14);
    assert_eq!(result["metrics"][0]["public_metrics"]["bookmarks"], 15);
    assert!(result["metrics"][0].get("private_metrics").is_none());
    assert!(
        result["metrics"][0]
            .get("unavailable_private_metrics")
            .is_none()
    );
    assert!(result["metrics"][1].get("text").is_none());
    assert_eq!(result["missing_ids"], json!(["33"]));
    assert_eq!(result["result_count"], 2);
    assert_post_usage(&output, 2);

    let requests = requests.lock().expect("request capture lock");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, "GET");
    assert_eq!(requests[0].url, "https://api.x.com/2/tweets");
    assert_eq!(requests[0].query.len(), 2);
    assert_eq!(requests[0].query["ids"], "11,22,33");
    assert_eq!(requests[0].query["tweet.fields"], "public_metrics");
    assert_eq!(requests[0].credential.credential_kind, "access_token");
    assert_eq!(
        requests[0].credential_injection.kind,
        "bearer_authorization"
    );
    assert_eq!(requests[0].response_body_limit_bytes, 262_144);
    assert_eq!(requests[0].timeout_seconds, 30);
}

#[test]
fn metrics_preserves_private_zeroes_and_names_each_omission() {
    let (http, requests) = capturing_http(response(
        200,
        Some(json!({
            "data": [
                {
                    "id": "11",
                    "public_metrics": public_metrics(0),
                    "non_public_metrics": {
                        "engagements": 0,
                        "url_link_clicks": 4,
                        "user_profile_clicks": 0
                    }
                },
                {
                    "id": "22",
                    "public_metrics": public_metrics(20),
                    "non_public_metrics": {"engagements": 7}
                },
                {"id": "33", "public_metrics": public_metrics(30)}
            ]
        })),
    ));

    let output = invoke(
        &http,
        "x_get_post_metrics",
        json!({"ids": ["11", "22", "33"], "include_private_metrics": true}),
    );

    let result = success_output(&output);
    assert_eq!(
        result["metrics"][0]["private_metrics"],
        json!({"engagements": 0, "url_clicks": 4, "profile_clicks": 0})
    );
    assert!(
        result["metrics"][0]
            .get("unavailable_private_metrics")
            .is_none()
    );
    assert_eq!(
        result["metrics"][1]["private_metrics"],
        json!({"engagements": 7})
    );
    assert_eq!(
        result["metrics"][1]["unavailable_private_metrics"],
        json!(["url_clicks", "profile_clicks"])
    );
    assert!(result["metrics"][2].get("private_metrics").is_none());
    assert_eq!(
        result["metrics"][2]["unavailable_private_metrics"],
        json!(["engagements", "url_clicks", "profile_clicks"])
    );
    let encoded = serde_json::to_string(result).expect("serialize result");
    assert!(!encoded.contains("user_profile_clicks"));
    assert!(!encoded.contains("profile_views"));
    assert_post_usage(&output, 3);

    let requests = requests.lock().expect("request capture lock");
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].query["tweet.fields"],
        "public_metrics,non_public_metrics"
    );
}

#[test]
fn metrics_rejects_invalid_ids_without_a_provider_request() {
    let http = Unimock::new(());
    for input in [
        json!({"ids": []}),
        json!({"ids": ["11", "11"]}),
        json!({"ids": [11]}),
        json!({"ids": ["abc"]}),
        json!({"ids": ["11"], "unexpected": true}),
    ] {
        let output = invoke(&http, "x_get_post_metrics", input);
        assert_error(&output, "invalid_request");
        assert_eq!(output["reason"], "invalid_post_ids");
    }
}

#[test]
fn metrics_rejects_malformed_or_untrusted_provider_metrics_without_usage() {
    let malformed = [
        json!({"data": [{"id": "11", "public_metrics": {"like_count": 1}}]}),
        json!({
            "data": [{
                "id": "11",
                "public_metrics": public_metrics(1),
                "non_public_metrics": {"engagements": "many"}
            }]
        }),
        json!({
            "data": [{
                "id": "11",
                "public_metrics": public_metrics(1),
                "non_public_metrics": {"engagements": null}
            }]
        }),
        json!({
            "data": [
                {"id": "11", "public_metrics": public_metrics(1)},
                {"id": "11", "public_metrics": public_metrics(2)}
            ]
        }),
        json!({"data": [{"id": "99", "public_metrics": public_metrics(1)}]}),
    ];

    for body in malformed {
        let output = call_with_response(
            "x_get_post_metrics",
            json!({"ids": ["11"], "include_private_metrics": true}),
            response(200, Some(body)),
        );
        assert_error(&output, "provider_contract_error");
    }
}

#[test]
fn metrics_completely_missing_result_is_uncharged_not_found() {
    let output = call_with_response(
        "x_get_post_metrics",
        json!({"ids": ["11"]}),
        response(200, Some(json!({"data": []}))),
    );

    assert_error(&output, "not_found");
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
