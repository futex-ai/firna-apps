//! DataForSEO HTTP and nested status-envelope decoding.

use std::collections::BTreeMap;

use serde_json::Value;

use super::error::{Error, Result};
use super::host::HostHttpResponse;

#[derive(Debug)]
pub(super) struct ProviderResult {
    pub(super) task_id: Option<String>,
    pub(super) cost_usd: Option<f64>,
    pub(super) rate_limit: RateLimit,
    pub(super) results: Vec<Value>,
}

#[derive(Debug)]
pub(super) struct RateLimit {
    pub(super) limit_per_minute: Option<u64>,
    pub(super) remaining: Option<u64>,
}

pub(super) fn decode(response: HostHttpResponse) -> Result<ProviderResult> {
    if !response.ok {
        return Err(Error::ProviderUnavailable(None));
    }
    if response.body_truncated {
        return Err(Error::ProviderResponseTooLarge);
    }
    let status = response.status.ok_or(Error::ProviderUnavailable(None))?;
    let headers = normalized_headers(response.headers);
    classify_http(status, &headers)?;
    let body = response
        .body_json
        .and_then(|value| value.as_object().cloned())
        .ok_or(Error::ProviderUnavailable(None))?;
    let general_code = integer(&body, "status_code").ok_or(Error::ProviderUnavailable(None))?;
    if general_code == 40102 {
        return Ok(empty_result(&body, &headers));
    }
    classify_code(general_code, &headers)?;
    let tasks = body
        .get("tasks")
        .and_then(Value::as_array)
        .filter(|tasks| tasks.len() == 1)
        .ok_or(Error::ProviderUnavailable(Some(general_code)))?;
    let task = tasks[0]
        .as_object()
        .ok_or(Error::ProviderUnavailable(Some(general_code)))?;
    let task_code = integer(task, "status_code").ok_or(Error::ProviderUnavailable(None))?;
    if task_code != 40102 {
        classify_code(task_code, &headers)?;
    }
    let results = if task_code == 40102 {
        Vec::new()
    } else {
        task.get("result")
            .and_then(Value::as_array)
            .cloned()
            .ok_or(Error::ProviderUnavailable(Some(task_code)))?
    };
    Ok(ProviderResult {
        task_id: task.get("id").and_then(Value::as_str).map(str::to_owned),
        cost_usd: task
            .get("cost")
            .and_then(Value::as_f64)
            .or_else(|| body.get("cost").and_then(Value::as_f64)),
        rate_limit: rate_limit(&headers),
        results,
    })
}

fn normalized_headers(headers: BTreeMap<String, String>) -> BTreeMap<String, String> {
    headers
        .into_iter()
        .map(|(mut name, value)| {
            name.make_ascii_lowercase();
            (name, value)
        })
        .collect()
}

fn classify_http(status: u16, headers: &BTreeMap<String, String>) -> Result<()> {
    match status {
        200..=299 => Ok(()),
        400 => Err(Error::InvalidRequest("provider_rejected_request")),
        401 => Err(Error::ProviderAuthenticationFailed(None)),
        402 => Err(Error::ProviderBudgetExhausted(None)),
        403 => Err(Error::ProviderAccessDenied(None)),
        404 => Err(Error::ProviderContract),
        429 => Err(rate_limited(None, headers)),
        _ => Err(Error::ProviderUnavailable(None)),
    }
}

fn classify_code(code: i64, headers: &BTreeMap<String, String>) -> Result<()> {
    match code {
        20000 => Ok(()),
        40100 => Err(Error::ProviderAuthenticationFailed(Some(code))),
        40104 | 40201 | 40204 | 40207 | 40208 => Err(Error::ProviderAccessDenied(Some(code))),
        40200 | 40203 | 40210 => Err(Error::ProviderBudgetExhausted(Some(code))),
        40202 | 40205 | 40206 | 40209 => Err(rate_limited(Some(code), headers)),
        40000..=40006 | 40400..=40408 | 40501..=40506 | 50100 => {
            Err(Error::InvalidRequest("provider_rejected_request"))
        }
        _ => Err(Error::ProviderUnavailable(Some(code))),
    }
}

fn empty_result(
    body: &serde_json::Map<String, Value>,
    headers: &BTreeMap<String, String>,
) -> ProviderResult {
    ProviderResult {
        task_id: None,
        cost_usd: body.get("cost").and_then(Value::as_f64),
        rate_limit: rate_limit(headers),
        results: Vec::new(),
    }
}

fn rate_limited(code: Option<i64>, headers: &BTreeMap<String, String>) -> Error {
    Error::RateLimited {
        provider_code: code,
        retry_after_seconds: header_integer(headers, &["retry-after"]),
    }
}

fn rate_limit(headers: &BTreeMap<String, String>) -> RateLimit {
    RateLimit {
        limit_per_minute: header_integer(
            headers,
            &["ratelimit-limit", "x-ratelimit-limit", "x-rate-limit-limit"],
        ),
        remaining: header_integer(
            headers,
            &[
                "ratelimit-remaining",
                "x-ratelimit-remaining",
                "x-rate-limit-remaining",
            ],
        ),
    }
}

fn header_integer(headers: &BTreeMap<String, String>, names: &[&str]) -> Option<u64> {
    names
        .iter()
        .find_map(|name| headers.get(*name))
        .and_then(|value| value.parse().ok())
}

fn integer(values: &serde_json::Map<String, Value>, key: &str) -> Option<i64> {
    values.get(key).and_then(Value::as_i64)
}
