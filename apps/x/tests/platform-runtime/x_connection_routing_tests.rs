use std::collections::{BTreeMap, VecDeque};
use std::path::Path;
use std::sync::{Arc, Mutex};

use fna_apps::host::{
    AppHostInvocation, CredentialScopedWasmHost, ProviderArtifactHttpResult, ProviderHttpClient,
};
use fna_apps::oauth_tokens::{
    AppOAuthAccessToken, AppOAuthTokenMode, AppOAuthTokenRequest, AppOAuthTokenResult,
    AppOAuthTokenService,
};
use fna_apps_wasm::{HostHttpRequest, HostHttpResponse, WasmComponentRuntime};
use serde_json::json;
use uuid::Uuid;

use crate::manifest;
use crate::x_runtime_support::runtime_with_host;
use crate::x_test_support::{UnusedCredentialVault, call_tool_result, provider_response};

#[tokio::test]
async fn x_connections_keep_bearers_and_refresh_isolated() {
    let first = Uuid::now_v7();
    let second = Uuid::now_v7();
    let stale_credential = Uuid::now_v7();
    let provider = Arc::new(SequencedProvider::new(vec![
        provider_response(401, Some(json!({"title": "Unauthorized"}))),
        provider_response(200, Some(json!({"data": [{"id": "1", "text": "first"}]}))),
        provider_response(200, Some(json!({"data": [{"id": "2", "text": "second"}]}))),
    ]));
    let oauth = Arc::new(ConnectionOAuth::new(vec![
        (
            first,
            vec![
                token("first-stale", stale_credential),
                token("first-rotated", Uuid::now_v7()),
            ],
        ),
        (second, vec![token("second-current", Uuid::now_v7())]),
    ]));
    let first_runtime = connection_runtime(first, provider.clone(), oauth.clone());
    let second_runtime = connection_runtime(second, provider.clone(), oauth.clone());

    let first_result = call_tool_result(
        &first_runtime,
        first,
        "x_get_posts",
        "x.get_posts",
        None,
        json!({"ids": ["1"]}),
    )
    .await
    .expect("first X connection should refresh and read");
    let second_result = call_tool_result(
        &second_runtime,
        second,
        "x_get_posts",
        "x.get_posts",
        None,
        json!({"ids": ["2"]}),
    )
    .await
    .expect("second X connection should read independently");

    assert_eq!(first_result.output["posts"][0]["id"], "1");
    assert_eq!(second_result.output["posts"][0]["id"], "2");
    let requests = provider.requests.lock().expect("provider request lock");
    assert_eq!(requests.len(), 3);
    assert_bearer(&requests[0], "first-stale");
    assert_bearer(&requests[1], "first-rotated");
    assert_bearer(&requests[2], "second-current");
    let calls = oauth.calls.lock().expect("OAuth request lock");
    assert_eq!(
        calls.as_slice(),
        [
            (first, AppOAuthTokenMode::Proactive),
            (
                first,
                AppOAuthTokenMode::AfterUnauthorized {
                    credential_id: stale_credential,
                },
            ),
            (second, AppOAuthTokenMode::Proactive),
        ]
    );
}

fn connection_runtime(
    installation_id: Uuid,
    provider: Arc<SequencedProvider>,
    oauth: Arc<ConnectionOAuth>,
) -> WasmComponentRuntime {
    let invocation = AppHostInvocation::from_manifest(
        &manifest(),
        String::from("x"),
        Some(installation_id),
        None,
        None,
    );
    let host =
        CredentialScopedWasmHost::new(Arc::new(UnusedCredentialVault), provider, invocation, oauth);
    runtime_with_host(Arc::new(host))
}

fn assert_bearer(request: &HostHttpRequest, expected: &str) {
    let expected = format!("Bearer {expected}");
    assert_eq!(
        request.headers.get("Authorization").map(String::as_str),
        Some(expected.as_str())
    );
}

fn token(value: &str, credential_id: Uuid) -> AppOAuthAccessToken {
    AppOAuthAccessToken {
        value: value.to_owned(),
        credential_id,
    }
}

struct ConnectionOAuth {
    tokens: Mutex<BTreeMap<Uuid, VecDeque<AppOAuthAccessToken>>>,
    calls: Mutex<Vec<(Uuid, AppOAuthTokenMode)>>,
}

impl ConnectionOAuth {
    fn new(entries: Vec<(Uuid, Vec<AppOAuthAccessToken>)>) -> Self {
        Self {
            tokens: Mutex::new(
                entries
                    .into_iter()
                    .map(|(id, tokens)| (id, tokens.into()))
                    .collect(),
            ),
            calls: Mutex::new(Vec::new()),
        }
    }
}

impl AppOAuthTokenService for ConnectionOAuth {
    fn access_token(
        &self,
        request: &AppOAuthTokenRequest,
    ) -> AppOAuthTokenResult<AppOAuthAccessToken> {
        self.calls
            .lock()
            .expect("OAuth request lock")
            .push((request.installation_id, request.mode.clone()));
        Ok(self
            .tokens
            .lock()
            .expect("OAuth token lock")
            .get_mut(&request.installation_id)
            .expect("connection token queue")
            .pop_front()
            .expect("connection access token"))
    }
}

struct SequencedProvider {
    responses: Mutex<VecDeque<HostHttpResponse>>,
    requests: Mutex<Vec<HostHttpRequest>>,
}

impl SequencedProvider {
    fn new(responses: Vec<HostHttpResponse>) -> Self {
        Self {
            responses: Mutex::new(responses.into()),
            requests: Mutex::new(Vec::new()),
        }
    }
}

impl ProviderHttpClient for SequencedProvider {
    fn execute(&self, request: &HostHttpRequest) -> HostHttpResponse {
        self.requests
            .lock()
            .expect("provider request lock")
            .push(request.clone());
        self.responses
            .lock()
            .expect("provider response lock")
            .pop_front()
            .expect("provider response")
    }

    fn execute_basic(
        &self,
        _request: &HostHttpRequest,
        _username: &str,
        _password: &str,
    ) -> HostHttpResponse {
        panic!("X provider requests must not use Basic authorization")
    }

    fn execute_without_redirects(&self, _request: &HostHttpRequest) -> HostHttpResponse {
        panic!("X tool requests do not use identity transport")
    }

    fn execute_artifact_upload(
        &self,
        _request: &HostHttpRequest,
        _source: &Path,
        _media_type: &str,
        _response_limit_bytes: u64,
    ) -> ProviderArtifactHttpResult {
        panic!("X does not upload artifacts")
    }

    fn execute_artifact_download(
        &self,
        _request: &HostHttpRequest,
        _destination: &Path,
        _response_limit_bytes: u64,
    ) -> ProviderArtifactHttpResult {
        panic!("X does not download artifacts")
    }
}
