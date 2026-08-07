use std::collections::VecDeque;
use std::path::Path;
use std::sync::{Arc, Mutex};

use fna_apps::credentials::{
    AppCredentialVault, PutInstallationCredential, PutInstallationCredentialPair,
    PutUserGrantCredential,
};
use fna_apps::host::{
    AppHostInvocation, CredentialScopedWasmHost, ProviderArtifactHttpResult, ProviderHttpClient,
};
use fna_apps::oauth_tokens::{
    AppOAuthAccessToken, AppOAuthTokenMode, AppOAuthTokenRequest, AppOAuthTokenResult,
    AppOAuthTokenService,
};
use fna_apps_interface::Result;
use fna_apps_interface::runtime::{AppRuntime, AppToolCall, AppToolUsageReport};
use fna_apps_store_interface::{
    AppCredentialRecord, AppCredentialRefreshInvalidation, AppCredentialRefreshRelease,
};
use fna_apps_wasm::{HostCredentialReference, HostHttpRequest, HostHttpResponse};
use serde_json::json;
use uuid::Uuid;

use crate::manifest;
use crate::x_runtime_support::runtime_with_host;

#[tokio::test]
async fn x_host_refreshes_and_retries_once_after_unauthorized() {
    let installation_id = Uuid::now_v7();
    let stale_id = Uuid::now_v7();
    let fresh_id = Uuid::now_v7();
    let provider = Arc::new(SequencedProvider::new(vec![
        response(401, json!({"title": "Unauthorized"})),
        response(201, json!({"data": {"id": "45", "text": "Hello X"}})),
    ]));
    let oauth = Arc::new(SequencedOAuth::new(vec![
        AppOAuthAccessToken {
            value: String::from("stale-access-token"),
            credential_id: stale_id,
        },
        AppOAuthAccessToken {
            value: String::from("rotated-access-token"),
            credential_id: fresh_id,
        },
    ]));
    let invocation = AppHostInvocation::from_manifest(
        &manifest(),
        String::from("x"),
        Some(installation_id),
        None,
        None,
    );
    let host = CredentialScopedWasmHost::new(
        Arc::new(UnusedCredentialVault),
        provider.clone(),
        invocation,
        oauth.clone(),
    );
    let runtime = runtime_with_host(Arc::new(host));

    let result = runtime
        .call_tool(AppToolCall {
            workspace_id: Uuid::now_v7(),
            installation_id,
            tool_name: String::from("x_create_post"),
            operation: String::from("x.create_post"),
            operation_id: Some(String::from("oauth-retry-operation")),
            input: json!({"text": "Hello X"}),
            effective_user_id: None,
            agent_id: None,
            output_hints: None,
        })
        .await
        .expect("X write should succeed after host refresh");

    assert_eq!(result.output["post"]["id"], "45");
    assert_eq!(
        result.usage,
        Some(AppToolUsageReport::ReportedCost {
            cost_usd_micros: 15_000,
        })
    );
    let requests = provider.requests.lock().expect("provider request lock");
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].body_json, requests[1].body_json);
    assert_eq!(
        requests[0].headers.get("Authorization").map(String::as_str),
        Some("Bearer stale-access-token")
    );
    assert_eq!(
        requests[1].headers.get("Authorization").map(String::as_str),
        Some("Bearer rotated-access-token")
    );
    let modes = oauth.modes.lock().expect("OAuth mode lock");
    assert_eq!(
        modes.as_slice(),
        [
            AppOAuthTokenMode::Proactive,
            AppOAuthTokenMode::AfterUnauthorized {
                credential_id: stale_id
            }
        ]
    );
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
            .expect("provider response should exist")
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
        panic!("X OAuth lifecycle test does not exercise redirect-sensitive requests")
    }

    fn execute_artifact_upload(
        &self,
        _request: &HostHttpRequest,
        _source: &Path,
        _media_type: &str,
        _response_limit_bytes: u64,
    ) -> ProviderArtifactHttpResult {
        panic!("X OAuth lifecycle test does not exercise artifact uploads")
    }

    fn execute_artifact_download(
        &self,
        _request: &HostHttpRequest,
        _destination: &Path,
        _response_limit_bytes: u64,
    ) -> ProviderArtifactHttpResult {
        panic!("X OAuth lifecycle test does not exercise artifact downloads")
    }
}

struct SequencedOAuth {
    tokens: Mutex<VecDeque<AppOAuthAccessToken>>,
    modes: Mutex<Vec<AppOAuthTokenMode>>,
}

impl SequencedOAuth {
    fn new(tokens: Vec<AppOAuthAccessToken>) -> Self {
        Self {
            tokens: Mutex::new(tokens.into()),
            modes: Mutex::new(Vec::new()),
        }
    }
}

impl AppOAuthTokenService for SequencedOAuth {
    fn access_token(
        &self,
        request: &AppOAuthTokenRequest,
    ) -> AppOAuthTokenResult<AppOAuthAccessToken> {
        assert_eq!(request.app_id, "x");
        assert_eq!(request.flow.id, "x_oauth");
        assert!(request.flow.token_lifecycle.is_some());
        self.modes
            .lock()
            .expect("OAuth mode lock")
            .push(request.mode.clone());
        Ok(self
            .tokens
            .lock()
            .expect("OAuth token lock")
            .pop_front()
            .expect("OAuth access token should exist"))
    }
}

struct UnusedCredentialVault;

impl AppCredentialVault for UnusedCredentialVault {
    fn put_installation_credential(
        &self,
        _input: PutInstallationCredential,
    ) -> Result<AppCredentialRecord> {
        panic!("OAuth lifecycle should own installation credentials")
    }

    fn remove_installation_credential(
        &self,
        _workspace_id: Uuid,
        _installation_id: Uuid,
        _credential_kind: &str,
    ) -> Result<bool> {
        panic!("OAuth lifecycle should own installation credentials")
    }

    fn put_installation_credential_pair(
        &self,
        _input: PutInstallationCredentialPair,
    ) -> Result<Vec<AppCredentialRecord>> {
        panic!("OAuth lifecycle should own installation credentials")
    }

    fn put_claimed_installation_credential_pair(
        &self,
        _input: PutInstallationCredentialPair,
        _fence: &AppCredentialRefreshRelease,
    ) -> Result<Option<Vec<AppCredentialRecord>>> {
        panic!("OAuth lifecycle should own installation credentials")
    }

    fn invalidate_claimed_installation_credential_pair(
        &self,
        _workspace_id: Uuid,
        _app_id: &str,
        _invalidation: &AppCredentialRefreshInvalidation,
    ) -> Result<bool> {
        panic!("OAuth lifecycle test does not exercise terminal invalidation")
    }

    fn put_user_grant_credential(
        &self,
        _input: PutUserGrantCredential,
    ) -> Result<AppCredentialRecord> {
        panic!("X V1 has no user-owned credential grants")
    }

    fn resolve_for_host(
        &self,
        _reference: &HostCredentialReference,
        _app_secret_storage_id: &str,
    ) -> Result<Option<String>> {
        panic!("lifecycle-managed X access tokens must bypass basic vault resolution")
    }

    fn resolve_record(&self, _credential: &AppCredentialRecord) -> Result<Option<String>> {
        panic!("lifecycle-managed X access tokens resolve through the token service")
    }
}

fn response(status: u16, body_json: serde_json::Value) -> HostHttpResponse {
    HostHttpResponse {
        ok: true,
        status: Some(status),
        url: Some(String::from("https://api.x.com/2/tweets")),
        headers: Default::default(),
        content_type: Some(String::from("application/json")),
        body_json: Some(body_json),
        body_truncated: false,
        error: None,
    }
}
