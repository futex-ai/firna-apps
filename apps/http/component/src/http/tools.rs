//! HTTP tool handlers.

use serde_json::{Value, json};

use crate::http::host::{host_error, send_http};
use crate::http::types::{AppToolCall, HttpMethod, HttpRequestInput, NormalizedHttpRequest};
use crate::http::{encode_json, invalid_request};

const DEFAULT_TIMEOUT_SECONDS: u64 = 60;
const MAX_TIMEOUT_SECONDS: u64 = 300;

pub(crate) fn call_tool(request: &str) -> String {
    let Ok(call) = serde_json::from_str::<AppToolCall>(request) else {
        return encode_json(invalid_request("invalid_tool_call"));
    };
    let result = match call.tool_name.as_str() {
        "http_request" => http_request(&call),
        _ => invalid_request("unknown_tool"),
    };
    encode_json(result)
}

fn http_request(call: &AppToolCall) -> Value {
    let Ok(input) = serde_json::from_value::<HttpRequestInput>(call.input.clone()) else {
        return invalid_request("invalid_http_request_input");
    };
    let (request, response_url) = match normalize_input(input) {
        Ok(request) => request,
        Err(reason) => return invalid_request(reason),
    };
    let response = match send_http(&request) {
        Ok(response) => response,
        Err(error) => return error,
    };
    if !response.ok {
        return host_error(response.error.as_deref().unwrap_or("host_http_failed"));
    }
    normalize_response(response, &response_url)
}

fn normalize_input(
    input: HttpRequestInput,
) -> Result<(NormalizedHttpRequest, String), &'static str> {
    if input.body_json.is_some() && input.body_text.is_some() {
        return Err("multiple_body_fields");
    }
    let method =
        HttpMethod::parse(input.method.as_deref().unwrap_or("GET")).ok_or("invalid_method")?;
    let timeout_seconds = match input.timeout_seconds.unwrap_or(DEFAULT_TIMEOUT_SECONDS) {
        value @ 1..=MAX_TIMEOUT_SECONDS => Some(value),
        _ => return Err("invalid_timeout_seconds"),
    };
    let response_url = input.url.clone();
    Ok((
        NormalizedHttpRequest {
            method: method.as_str().to_owned(),
            url: input.url,
            query: input.query,
            headers: input.headers,
            body_json: input.body_json,
            body_text: input.body_text,
            timeout_seconds,
            credential: None,
            credential_injection: None,
        },
        response_url,
    ))
}

fn normalize_response(response: crate::http::host::HostHttpResponse, fallback_url: &str) -> Value {
    let status = response.status.unwrap_or(0);
    json!({
        "status": status,
        "ok": (200..300).contains(&status),
        "url": response.url.unwrap_or_else(|| fallback_url.to_owned()),
        "headers": response.headers,
        "content_type": response.content_type,
        "body": response.body_json.unwrap_or(Value::Null),
        "body_truncated": response.body_truncated
    })
}

#[cfg(test)]
#[path = "_tests_/tools_tests.rs"]
mod tools_tests;
