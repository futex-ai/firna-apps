use std::collections::BTreeMap;

use fna_apps::credentials::{
    AppCredentialVault, PutInstallationCredential, PutInstallationCredentialPair,
    PutOAuthConnectionCredentials, PutUserGrantCredential,
};
use fna_apps_interface::runtime::{AppRuntime, AppToolCall, AppToolResult};
use fna_apps_interface::{Error, Result};
use fna_apps_store_interface::{
    AppCredentialRecord, AppCredentialRefreshInvalidation, AppCredentialRefreshRelease,
    AppInstallationRecord,
};
use fna_apps_wasm::{HostCredentialReference, HostHttpResponse, WasmComponentRuntime};
use serde_json::Value;
use uuid::Uuid;

pub(crate) async fn call_tool_result(
    runtime: &WasmComponentRuntime,
    installation_id: Uuid,
    tool_name: &str,
    operation: &str,
    operation_id: Option<&str>,
    input: Value,
) -> Result<AppToolResult> {
    runtime
        .call_tool(AppToolCall {
            workspace_id: Uuid::now_v7(),
            installation_id,
            tool_name: tool_name.to_owned(),
            operation: operation.to_owned(),
            operation_id: operation_id.map(str::to_owned),
            input,
            effective_user_id: None,
            agent_id: None,
            output_hints: None,
        })
        .await
}

pub(crate) async fn call_tool_error(
    runtime: &WasmComponentRuntime,
    installation_id: Uuid,
    tool_name: &str,
    operation: &str,
    operation_id: Option<&str>,
    input: Value,
) -> Error {
    call_tool_result(
        runtime,
        installation_id,
        tool_name,
        operation,
        operation_id,
        input,
    )
    .await
    .expect_err("X tool call should return a typed error")
}

pub(crate) fn provider_response(status: u16, body_json: Option<Value>) -> HostHttpResponse {
    provider_response_with_headers(status, BTreeMap::new(), body_json)
}

pub(crate) fn provider_response_with_headers(
    status: u16,
    headers: BTreeMap<String, String>,
    body_json: Option<Value>,
) -> HostHttpResponse {
    HostHttpResponse {
        ok: true,
        status: Some(status),
        url: Some(String::from("https://api.x.com/2/tweets")),
        headers,
        content_type: Some(String::from("application/json")),
        body_json,
        body_truncated: false,
        error: None,
    }
}

pub(crate) fn host_error(error: &str) -> HostHttpResponse {
    HostHttpResponse {
        ok: false,
        status: None,
        url: None,
        headers: BTreeMap::new(),
        content_type: None,
        body_json: None,
        body_truncated: false,
        error: Some(error.to_owned()),
    }
}

pub(crate) fn assert_no_null_fields(value: &Value) {
    match value {
        Value::Array(values) => values.iter().for_each(assert_no_null_fields),
        Value::Object(fields) => {
            for value in fields.values() {
                assert!(!value.is_null(), "output included an undeclared null field");
                assert_no_null_fields(value);
            }
        }
        _ => {}
    }
}

pub(crate) struct UnusedCredentialVault;

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

    fn publish_oauth_connection_credentials(
        &self,
        _input: PutOAuthConnectionCredentials,
    ) -> Result<AppInstallationRecord> {
        panic!("OAuth lifecycle should own connection publication")
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
