//! Exact file-read smoke tests through the real Wasm runtime.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use fna_apps_interface::{
    Error,
    runtime::{AppRuntime, AppToolCall},
};
use fna_apps_wasm::{HostHttpRequest, HostHttpResponse, WasmHostMock};
use serde_json::json;
use unimock::{MockFn as _, Unimock, matching};
use uuid::Uuid;

use crate::github_runtime_support::runtime_with_host;

const FILE_COMMIT_SHA: &str = "cccccccccccccccccccccccccccccccccccccccc";
const FILE_ROOT_TREE_SHA: &str = "1111111111111111111111111111111111111111";

#[tokio::test]
async fn github_component_rejects_symlinks_before_reading_contents() {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let captured = requests.clone();
    let runtime = runtime_with_host(Arc::new(Unimock::new(
        WasmHostMock::http_request
            .each_call(matching!(_))
            .answers_arc(Arc::new(move |_, request: HostHttpRequest| {
                captured.lock().unwrap().push(request.clone());
                provider_response(&request)
            })),
    )));

    let error = runtime
        .call_tool(AppToolCall {
            workspace_id: Uuid::now_v7(),
            installation_id: Uuid::now_v7(),
            tool_name: String::from("github_read_file"),
            operation: String::from("github.read_file"),
            operation_id: None,
            input: json!({
                "owner": "octo",
                "repository": "repo",
                "path": "README.md"
            }),
            effective_user_id: Some(Uuid::now_v7()),
            output_hints: None,
        })
        .await
        .unwrap_err();

    assert!(
        matches!(error, Error::InvalidRequest { ref reason, .. } if reason == "unsupported_content"),
        "unexpected runtime error: {error:?}"
    );
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    assert!(
        requests
            .iter()
            .all(|request| !request.url.contains("/contents/"))
    );
}

fn provider_response(request: &HostHttpRequest) -> HostHttpResponse {
    let body_json = match request.url.as_str() {
        "https://api.github.com/repos/octo/repo/commits" => json!([{
            "sha": FILE_COMMIT_SHA,
            "commit": { "tree": { "sha": FILE_ROOT_TREE_SHA } }
        }]),
        "https://api.github.com/repos/octo/repo/git/trees/1111111111111111111111111111111111111111" =>
        {
            json!({
                "sha": FILE_ROOT_TREE_SHA,
                "truncated": false,
                "tree": [{
                    "path": "README.md",
                    "mode": "120000",
                    "type": "blob",
                    "sha": "abc"
                }]
            })
        }
        other => panic!("unexpected GitHub provider URL: {other}"),
    };
    HostHttpResponse {
        ok: true,
        status: Some(200),
        url: Some(String::from("https://api.github.com/test")),
        headers: BTreeMap::new(),
        content_type: Some(String::from("application/json")),
        body_json: Some(body_json),
        body_truncated: false,
        error: None,
    }
}
