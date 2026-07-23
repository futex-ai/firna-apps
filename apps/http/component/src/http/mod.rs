//! HTTP app tool implementation.

mod host;
mod tools;
mod types;

use serde_json::{Value, json};

pub(crate) use tools::call_tool;

fn encode_json(value: Value) -> String {
    match serde_json::to_string(&value) {
        Ok(value) => value,
        Err(_) => String::from(r#"{"ok":false,"error":"provider_unavailable"}"#),
    }
}

fn invalid_request(reason: &str) -> Value {
    json!({
        "ok": false,
        "error": "invalid_request",
        "reason": reason
    })
}
