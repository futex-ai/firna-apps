//! Exa app component implementation.

mod host;
mod tools;
mod types;

use serde_json::{Value, json};

pub(crate) fn call_tool(request: &str) -> String {
    tools::call_tool(request)
}

pub(crate) fn encode_json(value: Value) -> String {
    value.to_string()
}

pub(crate) fn invalid_request(reason: &str) -> Value {
    json!({
        "ok": false,
        "error": "invalid_request",
        "reason": reason
    })
}
