use std::fs;
use std::path::PathBuf;

use fna_apps_interface::manifest::{InstallPolicy, ToolSideEffect};
use serde_json::Value;

use crate::manifest;

const EXPECTED_TOOLS: [&str; 5] = [
    "github_list_repositories",
    "github_search_code",
    "github_read_file",
    "github_read_pr",
    "github_read_issue",
];
const OWNER_PATTERN: &str = "^[A-Za-z0-9]([A-Za-z0-9-]{0,98}[A-Za-z0-9])?$";
const REPOSITORY_PATTERN: &str = "^[A-Za-z0-9._-]+$";
const NON_BLANK_PATTERN: &str = "\\S";
const NO_ASCII_CONTROL_PATTERN: &str = "^[^\\u0000-\\u001F\\u007F]*$";
const FILE_PATH_PATTERN: &str = "^[^/\\u0000-\\u001F\\u007F]+(/[^/\\u0000-\\u001F\\u007F]+)*$";
const DOT_PATH_SEGMENT_PATTERN: &str = "(^|/)\\.{1,2}($|/)";

#[test]
fn github_manifest_declares_read_only_app_installation_package() {
    let manifest = manifest();

    manifest.validate().unwrap();
    assert_eq!(manifest.id, "github");
    assert_eq!(manifest.version, "2.0.0");
    assert_eq!(
        manifest.icon.as_ref().unwrap().color_pair.primary,
        "#24292F"
    );
    assert_eq!(
        manifest.icon.as_ref().unwrap().color_pair.secondary,
        "#2F81F7"
    );
    assert_eq!(manifest.install.policy, InstallPolicy::Explicit);
    assert_eq!(manifest.secrets.len(), 2);
    assert_eq!(manifest.secrets[0].name, "client_secret");
    assert_eq!(manifest.secrets[1].name, "private_key");
    assert_eq!(manifest.tools.len(), EXPECTED_TOOLS.len());
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

    let json = serde_json::to_value(&manifest).unwrap();
    assert_eq!(json["source"]["kind"], "built_in");
    assert_eq!(json["auth_requirements"][0]["id"], "github_installation");
    assert_eq!(json["auth_requirements"][0]["kind"], "app_installation");
    assert_eq!(json["auth_requirements"][0]["owner"], "workspace");
    assert_eq!(
        json["auth_requirements"][0]["credential_kinds"],
        serde_json::json!(["provider_installation_id"])
    );
    assert_eq!(json["credential_flows"][0]["kind"], "installation_token");
    assert_eq!(
        json["credential_flows"][0]["permissions"],
        serde_json::json!({
            "contents": "read",
            "issues": "read",
            "metadata": "read",
            "pull_requests": "read"
        })
    );
    assert_eq!(
        json["capabilities"]["http"]["allowed_hosts"],
        serde_json::json!(["api.github.com"])
    );
    assert_eq!(
        json["capabilities"]["http"]["allowed_methods"],
        serde_json::json!(["GET"])
    );
    assert_eq!(
        json["capabilities"]["http"]["max_response_bytes"],
        1_048_576
    );
}

#[test]
fn registration_identifiers_callbacks_and_secrets_are_explicit() {
    let manifest = serde_json::to_value(manifest()).unwrap();
    let flow = &manifest["credential_flows"][0];
    let management_url = manifest["auth_requirements"][0]["management_url"]
        .as_str()
        .unwrap();

    assert_eq!(
        flow["client_id"],
        "replace-with-registered-github-app-client-id"
    );
    assert_eq!(flow["app_slug"], "firna");
    assert_eq!(
        flow["setup_url"],
        "https://firna.ai/apps/github/install/callback"
    );
    assert_eq!(
        flow["callback_url"],
        "https://firna.ai/apps/github/authorize/callback"
    );
    assert_eq!(flow["client_secret_env"], "client_secret");
    assert_eq!(flow["private_key_env"], "private_key");
    assert_eq!(management_url, "https://github.com/settings/installations");
}

#[test]
fn package_documentation_tracks_registration_contract() {
    let docs = fs::read_to_string(package_readme_path()).unwrap();
    let package = manifest();

    assert_eq!(package.id, "github");
    assert_eq!(package.version, "2.0.0");
    for contract in [
        "slug `firna`",
        "`client_secret`",
        "`private_key`",
        "`github_installation`",
        "https://firna.ai/apps/github/install/callback",
        "https://firna.ai/apps/github/authorize/callback",
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
fn manifest_serialization_contains_no_client_secret_value() {
    let encoded = serde_json::to_string(&manifest()).unwrap();

    assert!(!encoded.contains("github-client-secret-value"));
    assert!(!encoded.contains("authorization: bearer"));
    assert_eq!(
        serde_json::from_str::<Value>(&encoded).unwrap()["secrets"][0]["name"],
        "client_secret"
    );
    assert_eq!(
        serde_json::from_str::<Value>(&encoded).unwrap()["secrets"][1]["name"],
        "private_key"
    );
}

#[test]
fn manifest_string_schemas_match_component_validation_envelope() {
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
