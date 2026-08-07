//! Manifest contract tests for the GitHub tools and events package.

use std::fs;
use std::path::PathBuf;

use fna_apps_interface::manifest::{
    AppHttpMethod, AppSourceKind, AuthOwner, AuthRequirementKind, CredentialFlow, InstallPolicy,
    ToolSideEffect,
};
use serde_json::Value;

use crate::manifest;

const EXPECTED_TOOLS: [&str; 5] = [
    "github_list_repositories",
    "github_search_code",
    "github_read_file",
    "github_read_pr",
    "github_read_issue",
];
const EXPECTED_EVENTS: [&str; 6] = [
    "push",
    "pull_request",
    "pull_request_review",
    "pull_request_review_comment",
    "issues",
    "issue_comment",
];
const OWNER_PATTERN: &str = "^[A-Za-z0-9]([A-Za-z0-9-]{0,98}[A-Za-z0-9])?$";
const REPOSITORY_PATTERN: &str = "^[A-Za-z0-9._-]+$";
const NON_BLANK_PATTERN: &str = "\\S";
const NO_ASCII_CONTROL_PATTERN: &str = "^[^\\u0000-\\u001F\\u007F]*$";
const FILE_PATH_PATTERN: &str = "^[^/\\u0000-\\u001F\\u007F]+(/[^/\\u0000-\\u001F\\u007F]+)*$";
const DOT_PATH_SEGMENT_PATTERN: &str = "(^|/)\\.{1,2}($|/)";

#[test]
fn github_manifest_preserves_installation_access_and_adds_tools_and_events() {
    let manifest = manifest();

    manifest.validate().unwrap();
    assert_eq!(manifest.id, "github");
    assert_eq!(manifest.version, "2.0.0");
    assert_eq!(manifest.source.kind, AppSourceKind::BuiltIn);
    assert_eq!(manifest.install.policy, InstallPolicy::Explicit);
    assert_eq!(
        manifest
            .secrets
            .iter()
            .map(|secret| secret.name.as_str())
            .collect::<Vec<_>>(),
        ["client_secret", "private_key", "webhook_secret"]
    );
    assert_eq!(
        manifest
            .tools
            .iter()
            .map(|tool| tool.name.as_str())
            .collect::<Vec<_>>(),
        EXPECTED_TOOLS
    );
    assert!(
        manifest
            .tools
            .iter()
            .all(|tool| tool.side_effect == ToolSideEffect::ExternalRead)
    );
    assert_eq!(
        manifest
            .tools
            .iter()
            .map(|tool| tool.activity_label.as_str())
            .collect::<Vec<_>>(),
        [
            "Listing GitHub repositories",
            "Searching GitHub code",
            "Reading GitHub file",
            "Reading GitHub pull request",
            "Reading GitHub issue",
        ]
    );

    let http = manifest.capabilities.http.as_ref().unwrap();
    assert_eq!(http.allowed_hosts, ["api.github.com"]);
    assert_eq!(
        http.allowed_methods,
        Some(vec![AppHttpMethod::Get, AppHttpMethod::Post])
    );
    assert_eq!(http.max_response_bytes, Some(1_048_576));
    assert!(manifest.capabilities.crypto.as_ref().unwrap().hmac);
    assert_eq!(manifest.limits.max_event_payload_bytes, Some(262_144));
    assert_eq!(manifest.limits.max_component_ms, Some(5_000));
}

#[test]
fn github_installation_and_ingress_contracts_are_exact() {
    let manifest = manifest();
    let [requirement] = manifest.auth_requirements.as_slice() else {
        panic!("expected one GitHub installation requirement");
    };

    assert_eq!(requirement.kind, AuthRequirementKind::AppInstallation);
    assert_eq!(requirement.owner, AuthOwner::Workspace);
    assert_eq!(
        requirement.scopes,
        [
            "contents:write",
            "issues:read",
            "metadata:read",
            "pull_requests:write",
        ]
    );
    assert_eq!(requirement.credential_kinds, ["provider_installation_id"]);
    assert_eq!(
        requirement.required_for,
        EXPECTED_TOOLS.map(|tool| format!("tool:{tool}"))
    );

    let [CredentialFlow::InstallationToken(flow)] = manifest.credential_flows.as_slice() else {
        panic!("expected one installation-token flow");
    };
    assert_eq!(flow.provider, "github");
    assert_eq!(flow.client_id, "Iv23lidBdZ0I2rgwjhXB");
    assert_eq!(flow.app_slug, "firna-ai");
    assert_eq!(
        flow.install_url_template,
        "https://github.com/apps/firna-ai/installations/new?state={state}"
    );
    assert_eq!(flow.setup_url, "https://firna.ai/apps/github/install/setup");
    assert_eq!(
        flow.callback_url,
        "https://firna.ai/apps/github/install/callback"
    );
    assert_eq!(
        serde_json::to_value(flow).unwrap()["permissions"],
        serde_json::json!({
            "contents": "write",
            "issues": "read",
            "metadata": "read",
            "pull_requests": "write"
        })
    );

    let [ingress] = manifest.ingress.as_slice() else {
        panic!("expected one GitHub webhook ingress");
    };
    assert_eq!(ingress.id, "github_events");
    assert_eq!(ingress.auth, ["github_installation"]);
    assert_eq!(
        ingress.allowed_headers,
        ["x-github-delivery", "x-github-event", "x-hub-signature-256",]
    );
    assert_eq!(ingress.credential_kinds, ["webhook_secret"]);
    assert_eq!(ingress.max_payload_bytes, Some(262_144));
    assert_eq!(
        ingress
            .events
            .iter()
            .map(|event| event.provider_type.as_str())
            .collect::<Vec<_>>(),
        EXPECTED_EVENTS
    );
    assert!(
        ingress
            .events
            .iter()
            .all(|event| event.contract_version == 1)
    );
}

#[test]
fn package_documentation_tracks_registration_and_runtime_contracts() {
    let docs = fs::read_to_string(package_readme_path()).unwrap();

    for contract in [
        "App ID: `4504159`",
        "Slug: `firna-ai`",
        "`client_secret`",
        "`private_key`",
        "`webhook_secret`",
        "`github_installation`",
        "16 path segments",
        "https://firna.ai/apps/github/install/setup",
        "https://firna.ai/apps/github/install/callback",
        "/apps/github/webhooks/github_events",
        "`pull_request_review_comment`",
    ] {
        assert!(
            docs.contains(contract),
            "missing protocol contract `{contract}`"
        );
    }
}

fn package_readme_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("README.md")
}

#[test]
fn manifest_serialization_contains_no_secret_or_token_value() {
    let encoded = serde_json::to_string(&manifest()).unwrap();

    for forbidden in [
        "github-client-secret-value",
        "github-private-key-value",
        "github-webhook-secret-value",
        "authorization: bearer",
    ] {
        assert!(!encoded.contains(forbidden));
    }
}

#[test]
fn manifest_string_schemas_match_component_validation() {
    let manifest = serde_json::to_value(manifest()).unwrap();

    for tool in [
        "github_search_code",
        "github_read_file",
        "github_read_pr",
        "github_read_issue",
    ] {
        assert_string_schema(
            tool_property(&manifest, tool, "owner"),
            1,
            100,
            OWNER_PATTERN,
        );
        let repository = tool_property(&manifest, tool, "repository");
        assert_string_schema(repository, 1, 100, REPOSITORY_PATTERN);
        assert_eq!(repository["not"]["enum"], serde_json::json!([".", ".."]));
    }

    assert_string_schema(
        tool_property(&manifest, "github_search_code", "query"),
        1,
        256,
        NON_BLANK_PATTERN,
    );
    assert_string_schema(
        tool_property(&manifest, "github_search_code", "language"),
        1,
        100,
        NON_BLANK_PATTERN,
    );
    assert_string_schema(
        tool_property(&manifest, "github_search_code", "path"),
        1,
        256,
        NO_ASCII_CONTROL_PATTERN,
    );
    let file_path = tool_property(&manifest, "github_read_file", "path");
    assert_string_schema(file_path, 1, 1_024, FILE_PATH_PATTERN);
    assert_eq!(
        file_path["not"]["pattern"],
        serde_json::json!(DOT_PATH_SEGMENT_PATTERN)
    );
    assert_string_schema(
        tool_property(&manifest, "github_read_file", "ref"),
        1,
        255,
        NO_ASCII_CONTROL_PATTERN,
    );
}

fn tool_property<'a>(manifest: &'a Value, tool_name: &str, property: &str) -> &'a Value {
    &manifest["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|tool| tool["name"] == tool_name)
        .unwrap()["input_schema"]["properties"][property]
}

fn assert_string_schema(schema: &Value, minimum: u64, maximum: u64, pattern: &str) {
    assert_eq!(schema["type"], "string");
    assert_eq!(schema["minLength"], minimum);
    assert_eq!(schema["maxLength"], maximum);
    assert_eq!(schema["pattern"], pattern);
}
