//! Shared fake-provider helpers for component conformance tests.

use std::sync::{Arc, Mutex};

use serde_json::{Value, json};
use unimock::{MockFn as _, Unimock, matching};

use super::super::host::{HostHttpResponse, ProviderClientPostTask};
use super::super::tools;

#[derive(Clone, Debug)]
pub(super) struct CapturedRequest {
    pub(super) task: Value,
    pub(super) timeout_seconds: u64,
}

pub(super) fn call(
    tool: &str,
    input: Value,
    provider_results: Vec<Value>,
) -> (Value, CapturedRequest) {
    let captured = Arc::new(Mutex::new(None));
    let request = Arc::clone(&captured);
    let response = provider_response(provider_results);
    let client = Unimock::new(
        ProviderClientPostTask
            .next_call(matching!(_, _, _))
            .answers_arc(Arc::new(move |_, _path: &str, task, timeout_seconds| {
                *request.lock().unwrap() = Some(CapturedRequest {
                    task,
                    timeout_seconds,
                });
                Ok(response.clone())
            })),
    );

    let output = tools::call(&client, tool, input).unwrap();
    let request = captured.lock().unwrap().clone().unwrap();
    (output, request)
}

pub(super) fn invalid(tool: &str, input: Value) -> super::super::error::Error {
    tools::call(&Unimock::new(()), tool, input).unwrap_err()
}

fn provider_response(results: Vec<Value>) -> HostHttpResponse {
    HostHttpResponse {
        ok: true,
        status: Some(200),
        headers: Default::default(),
        body_json: Some(json!({
            "status_code": 20000,
            "tasks": [{
                "id": "task-1",
                "status_code": 20000,
                "cost": 0.001,
                "result": results,
            }]
        })),
        body_truncated: false,
    }
}
