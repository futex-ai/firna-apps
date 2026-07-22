//! Safe typed extraction from untrusted provider JSON.

use serde_json::Value;

pub(in crate::dataforseo) fn string(value: &Value, pointer: &str) -> Value {
    nullable(
        value
            .pointer(pointer)
            .and_then(Value::as_str)
            .map(Value::from),
    )
}

pub(in crate::dataforseo) fn signed(value: &Value, pointer: &str) -> Value {
    nullable(
        value
            .pointer(pointer)
            .and_then(Value::as_i64)
            .map(Value::from),
    )
}

pub(in crate::dataforseo) fn bounded_signed(
    value: &Value,
    pointer: &str,
    minimum: i64,
    maximum: i64,
) -> Value {
    nullable(
        value
            .pointer(pointer)
            .and_then(Value::as_i64)
            .filter(|value| (minimum..=maximum).contains(value))
            .map(Value::from),
    )
}

pub(in crate::dataforseo) fn number(value: &Value, pointer: &str) -> Value {
    nullable(
        value
            .pointer(pointer)
            .and_then(Value::as_f64)
            .map(Value::from),
    )
}

pub(in crate::dataforseo) fn bool_value(value: &Value, pointer: &str) -> Value {
    nullable(
        value
            .pointer(pointer)
            .and_then(Value::as_bool)
            .map(Value::from),
    )
}

pub(in crate::dataforseo) fn strings(value: &Value, pointer: &str, limit: usize) -> Vec<Value> {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .take(limit)
        .map(Value::from)
        .collect()
}

fn nullable(value: Option<Value>) -> Value {
    value.unwrap_or(Value::Null)
}
