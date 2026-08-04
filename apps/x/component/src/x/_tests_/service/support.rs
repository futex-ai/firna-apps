use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};
use unimock::{MockFn as _, Unimock, matching};

use crate::x::host::{HostHttpRequest, HostHttpResponse, XHttpClientSendMock};
use crate::x::service::call_tool;

pub(super) fn invoke(http: &Unimock, tool_name: &str, input: Value) -> Value {
    invoke_raw(
        http,
        json!({
            "installation_id": "018f-installation",
            "tool_name": tool_name,
            "operation_id": "durable-operation-id",
            "input": input
        })
        .to_string(),
    )
}

pub(super) fn invoke_raw(http: &Unimock, request: String) -> Value {
    let encoded = call_tool(&request, http);
    serde_json::from_str(&encoded).expect("component output should be JSON")
}

pub(super) fn call_with_response(
    tool_name: &str,
    input: Value,
    response: HostHttpResponse,
) -> Value {
    let http = Unimock::new(
        XHttpClientSendMock
            .next_call(matching!(_))
            .returns(response),
    );
    invoke(&http, tool_name, input)
}

pub(super) fn response(status: u16, body_json: Option<Value>) -> HostHttpResponse {
    response_with_headers(status, BTreeMap::new(), body_json)
}

pub(super) fn response_with_headers(
    status: u16,
    headers: BTreeMap<String, String>,
    body_json: Option<Value>,
) -> HostHttpResponse {
    HostHttpResponse {
        ok: true,
        status: Some(status),
        headers,
        body_json,
        body_truncated: false,
        error: None,
    }
}

pub(super) fn host_error(error: &str) -> HostHttpResponse {
    HostHttpResponse {
        error: Some(error.to_owned()),
        ..HostHttpResponse::default()
    }
}

pub(super) fn capturing_http(
    response: HostHttpResponse,
) -> (Unimock, Arc<Mutex<Vec<HostHttpRequest>>>) {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let captured = requests.clone();
    let http = Unimock::new(
        XHttpClientSendMock
            .next_call(matching!(_))
            .answers_arc(Arc::new(move |_, request| {
                captured
                    .lock()
                    .expect("request capture lock should be available")
                    .push(request);
                response.clone()
            })),
    );
    (http, requests)
}

pub(super) fn assert_error(output: &Value, code: &str) {
    assert_eq!(output["ok"], false);
    assert_eq!(output["error"], code);
    assert!(output.get("output").is_none());
    assert!(output.get("usage").is_none());
}

pub(super) fn success_output(output: &Value) -> &Value {
    output.get("output").expect("priced success output")
}

pub(super) fn assert_read_usage(output: &Value, posts: u64, users: u64) {
    assert_eq!(
        output["usage"],
        json!({
            "kind": "metered",
            "units": [
                {"unit": "post_read", "quantity": posts},
                {"unit": "user_read", "quantity": users}
            ]
        })
    );
}

pub(super) fn assert_create_cost(output: &Value, cost_usd_micros: u64) {
    assert_eq!(
        output["usage"],
        json!({
            "kind": "reported_cost",
            "cost_usd_micros": cost_usd_micros
        })
    );
}
