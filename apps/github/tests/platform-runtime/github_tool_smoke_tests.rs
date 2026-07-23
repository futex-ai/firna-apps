use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use fna_apps_interface::{
    Error,
    runtime::{AppRuntime, AppToolCall},
};
use fna_apps_wasm::{
    HostCredentialInjectionKind, HostHttpRequest, HostHttpResponse, WasmComponentRuntime,
    WasmHostMock,
};
use serde_json::{Value, json};
use unimock::{MockFn as _, Unimock, matching};
use uuid::Uuid;

use crate::github_fixtures::{
    code_search, file, issue, issue_comments, pull_request, pull_request_files, repository,
};
use crate::github_runtime_support::runtime_with_host;

#[tokio::test]
async fn github_component_smoke_calls_all_read_tools_with_opaque_grant() {
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

    let repositories = call_tool(
        &runtime,
        "github_list_repositories",
        "github.list_repositories",
        json!({ "per_page": 1 }),
    )
    .await;
    assert_eq!(repositories["repositories"][0]["full_name"], "octo/repo-1");

    let search = call_tool(
        &runtime,
        "github_search_code",
        "github.search_code",
        json!({ "query": "call", "owner": "octo", "repository": "repo" }),
    )
    .await;
    assert_eq!(search["matches"][0]["path"], "src/lib.rs");

    let file = call_tool(
        &runtime,
        "github_read_file",
        "github.read_file",
        json!({ "owner": "octo", "repository": "repo", "path": "README.md" }),
    )
    .await;
    assert_eq!(file["content"], "hello\n");

    let pull_request = call_tool(
        &runtime,
        "github_read_pr",
        "github.read_pr",
        json!({ "owner": "octo", "repository": "repo", "number": 7 }),
    )
    .await;
    assert_eq!(pull_request["files"][0]["filename"], "src/lib.rs");

    let issue = call_tool(
        &runtime,
        "github_read_issue",
        "github.read_issue",
        json!({ "owner": "octo", "repository": "repo", "number": 8 }),
    )
    .await;
    assert_eq!(issue["comments"][0]["body"], "Confirmed");

    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 7);
    let search_request = requests
        .iter()
        .find(|request| request.url == "https://api.github.com/search/code")
        .unwrap();
    assert_eq!(search_request.query["q"], r#""call" repo:"octo/repo""#);
    assert!(!search_request.query["q"].contains("%22"));
    for request in requests.iter() {
        assert_eq!(request.method, "GET");
        assert!(request.url.starts_with("https://api.github.com/"));
        assert_eq!(request.headers.get("authorization"), None);
        assert_eq!(request.headers["x-github-api-version"], "2026-03-10");
        assert_eq!(request.response_body_limit_bytes, Some(1_048_576));
        let credential = request.credential.as_ref().unwrap();
        assert_eq!(credential.app_id, "github");
        assert_eq!(credential.auth_requirement_id, None);
        assert_eq!(credential.credential_kind, "installation_token");
        assert!(credential.installation_id.is_some());
        assert_eq!(credential.effective_user_id, None);
        assert_eq!(credential.user_grant_id, None);
        assert_eq!(
            request.credential_injection.as_ref().unwrap().kind,
            HostCredentialInjectionKind::BearerAuthorization
        );
    }
}

#[tokio::test]
async fn github_component_calls_http_without_effective_user() {
    let runtime = runtime_with_host(Arc::new(Unimock::new(
        WasmHostMock::http_request
            .next_call(matching!(_))
            .returns(response_with(
                json!({ "total_count": 0, "repositories": [] }),
                BTreeMap::new(),
            )),
    )));
    let output = runtime
        .call_tool(tool_call(
            "github_list_repositories",
            "github.list_repositories",
            json!({}),
            None,
        ))
        .await
        .unwrap();

    assert_eq!(output.output["repositories"], json!([]));
}

#[tokio::test]
async fn github_component_rejects_host_truncated_provider_body() {
    let runtime = runtime_with_host(Arc::new(Unimock::new(
        WasmHostMock::http_request
            .next_call(matching!(_))
            .returns(HostHttpResponse {
                ok: true,
                status: Some(200),
                url: Some(String::from(
                    "https://api.github.com/installation/repositories",
                )),
                headers: BTreeMap::new(),
                content_type: Some(String::from("application/json")),
                body_json: Some(json!([])),
                body_truncated: true,
                error: None,
            }),
    )));
    let error = runtime
        .call_tool(tool_call(
            "github_list_repositories",
            "github.list_repositories",
            json!({}),
            Some(Uuid::now_v7()),
        ))
        .await
        .unwrap_err();

    assert!(
        matches!(
            error,
            Error::Provider(fna_apps_interface::provider_error::ProviderError::ResponseTooLarge {
                ref app_id,
            }) if app_id == "github"
        ),
        "unexpected runtime error: {error:?}"
    );
}

#[tokio::test]
async fn github_large_projection_keeps_numeric_provider_pagination() {
    let repositories = (1..=50)
        .map(|id| repository(id, Some(&"é".repeat(160))))
        .collect::<Vec<_>>();
    let body = json!({ "total_count": 50, "repositories": repositories });
    let runtime = runtime_with_host(Arc::new(Unimock::new(
        WasmHostMock::http_request
            .next_call(matching!(_))
            .returns(response_with(
                body,
                BTreeMap::from([(
                    String::from("link"),
                    String::from(
                        "<https://api.github.com/installation/repositories?page=2>; rel=\"next\"",
                    ),
                )]),
            )),
    )));
    let output = call_tool(
        &runtime,
        "github_list_repositories",
        "github.list_repositories",
        json!({ "per_page": 50 }),
    )
    .await;

    assert_eq!(output["repositories"].as_array().unwrap().len(), 50);
    assert_eq!(output["next_page"], 2);
    assert!(serde_json::to_vec(&output).unwrap().len() > 20_000);
}

async fn call_tool(
    runtime: &WasmComponentRuntime,
    tool_name: &str,
    operation: &str,
    input: Value,
) -> Value {
    runtime
        .call_tool(tool_call(tool_name, operation, input, Some(Uuid::now_v7())))
        .await
        .unwrap()
        .output
}

fn tool_call(
    tool_name: &str,
    operation: &str,
    input: Value,
    effective_user_id: Option<Uuid>,
) -> AppToolCall {
    AppToolCall {
        workspace_id: Uuid::now_v7(),
        installation_id: Uuid::now_v7(),
        tool_name: tool_name.to_owned(),
        operation: operation.to_owned(),
        operation_id: None,
        input,
        effective_user_id,
        output_hints: None,
    }
}

fn provider_response(request: &HostHttpRequest) -> HostHttpResponse {
    let body = match request.url.as_str() {
        "https://api.github.com/installation/repositories" => {
            json!({ "total_count": 1, "repositories": [repository(1, None)] })
        }
        "https://api.github.com/search/code" => code_search(),
        "https://api.github.com/repos/octo/repo/contents/README%2Emd" => file(),
        "https://api.github.com/repos/octo/repo/pulls/7" => pull_request(),
        "https://api.github.com/repos/octo/repo/pulls/7/files" => pull_request_files(),
        "https://api.github.com/repos/octo/repo/issues/8" => issue(),
        "https://api.github.com/repos/octo/repo/issues/8/comments" => issue_comments(),
        other => panic!("unexpected GitHub provider URL: {other}"),
    };
    response_with(body, BTreeMap::new())
}

fn response_with(body_json: Value, headers: BTreeMap<String, String>) -> HostHttpResponse {
    HostHttpResponse {
        ok: true,
        status: Some(200),
        url: Some(String::from("https://api.github.com/test")),
        headers,
        content_type: Some(String::from("application/json")),
        body_json: Some(body_json),
        body_truncated: false,
        error: None,
    }
}
