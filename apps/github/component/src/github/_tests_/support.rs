use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};
use unimock::{MockFn as _, Unimock, matching};

use crate::github::call_tool_with;
use crate::github::provider::{Clock, GitHubProviderGet, ProviderRequest, ProviderResponse};

pub(super) struct FixedClock(pub(super) u64);

impl Clock for FixedClock {
    fn now_unix_seconds(&self) -> u64 {
        self.0
    }
}

pub(super) fn call(
    tool_name: &str,
    input: Value,
    responses: Vec<ProviderResponse>,
) -> (Value, Vec<ProviderRequest>) {
    call_with_clock(tool_name, input, responses, &FixedClock(1_000))
}

pub(super) fn call_with_clock(
    tool_name: &str,
    input: Value,
    responses: Vec<ProviderResponse>,
    clock: &dyn Clock,
) -> (Value, Vec<ProviderRequest>) {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let captured = requests.clone();
    let responses = Arc::new(Mutex::new(VecDeque::from(responses)));
    let queued = responses.clone();
    let provider = if responses
        .lock()
        .expect("response queue should lock")
        .is_empty()
    {
        Unimock::new(())
    } else {
        Unimock::new(
            GitHubProviderGet
                .each_call(matching!(_))
                .answers_arc(Arc::new(move |_, request: ProviderRequest| {
                    captured
                        .lock()
                        .expect("request capture should lock")
                        .push(request);
                    Ok(queued
                        .lock()
                        .expect("response queue should lock")
                        .pop_front()
                        .expect("fixture response should exist"))
                })),
        )
    };
    let raw = call_tool_with(
        &json!({
            "installation_id": "018f-install",
            "tool_name": tool_name,
            "input": input,
            "effective_user_id": "018f-user"
        })
        .to_string(),
        &provider,
        clock,
    );
    let value = serde_json::from_str(&raw).expect("component output should be JSON");
    let requests = requests
        .lock()
        .expect("request capture should lock")
        .clone();
    (value, requests)
}

pub(super) fn response(status: u16, body: Value) -> ProviderResponse {
    ProviderResponse {
        status,
        headers: BTreeMap::new(),
        body,
        body_truncated: false,
    }
}

pub(super) fn response_with_headers(
    status: u16,
    headers: BTreeMap<String, String>,
    body: Value,
) -> ProviderResponse {
    ProviderResponse {
        status,
        headers,
        body,
        body_truncated: false,
    }
}

pub(super) fn user(login: &str) -> Value {
    json!({
        "id": 1,
        "login": login,
        "html_url": format!("https://github.com/{login}")
    })
}

pub(super) fn repository() -> Value {
    json!({
        "id": 1,
        "full_name": "octo/repo",
        "description": null,
        "visibility": "private",
        "archived": false,
        "fork": false,
        "default_branch": "main",
        "language": null,
        "pushed_at": null,
        "html_url": "https://github.com/octo/repo"
    })
}

pub(super) fn installation_repositories(repositories: Vec<Value>) -> Value {
    json!({
        "total_count": repositories.len(),
        "repositories": repositories
    })
}
