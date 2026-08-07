use fna_apps_interface::manifest::{AppSourceKind, AuthOwner, AuthRequirementKind, InstallPolicy};
use serde_json::json;

use crate::manifest;

#[test]
fn dataforseo_manifest_is_explicit_built_in_and_credential_free_at_deploy() {
    let manifest = manifest();

    manifest.validate().unwrap();
    assert_eq!(manifest.id, "dataforseo");
    assert_eq!(manifest.version, "1.0.12");
    assert_eq!(
        manifest.icon.as_ref().unwrap().color_pair.primary,
        "#2563EB"
    );
    assert_eq!(
        manifest.icon.as_ref().unwrap().color_pair.secondary,
        "#7DD3FC"
    );
    assert_eq!(manifest.source.kind, AppSourceKind::BuiltIn);
    assert_eq!(manifest.install.policy, InstallPolicy::Explicit);
    assert!(manifest.icon.is_some());
    assert!(manifest.secrets.is_empty());
    assert_eq!(manifest.tools.len(), 16);
    assert!(
        manifest
            .tools
            .iter()
            .all(|tool| tool.name.starts_with("dataforseo_"))
    );
    assert_eq!(
        manifest
            .tools
            .iter()
            .map(|tool| tool.activity_label.as_str())
            .collect::<Vec<_>>(),
        [
            "Searching Google results",
            "Analyzing keyword metrics",
            "Finding keyword suggestions",
            "Inspecting ranked keywords",
            "Summarizing backlink profile",
            "Finding backlinks",
            "Finding referring domains",
            "Auditing web page",
            "Searching local businesses",
            "Retrieving business information",
            "Searching web citations",
            "Analyzing content sentiment",
            "Detecting domain technologies",
            "Inspecting domain registration",
            "Analyzing AI keyword demand",
            "Analyzing LLM mentions",
        ]
    );
    assert_eq!(manifest.auth_requirements.len(), 1);
    assert_eq!(
        manifest.auth_requirements[0].kind,
        AuthRequirementKind::BasicAuth
    );
    assert_eq!(manifest.auth_requirements[0].owner, AuthOwner::Workspace);
    assert_eq!(manifest.auth_requirements[0].required_for.len(), 16);
    assert!(manifest.ingress.is_empty());
    assert!(manifest.events.is_empty());
    assert_eq!(
        manifest.capabilities.http.unwrap().allowed_hosts,
        vec![String::from("api.dataforseo.com")]
    );
}

#[test]
fn dataforseo_manifest_declares_exact_verifier_and_tool_budget() {
    let manifest = manifest();
    let flow = manifest.credential_flows[0].basic_auth().unwrap();
    let verifier = flow.verification.as_ref().unwrap();

    assert_eq!(
        verifier.url,
        "https://api.dataforseo.com/v3/appendix/status"
    );
    assert_eq!(verifier.json_code_pointer, "/status_code");
    assert_eq!(verifier.json_code_equals, 20000);
    assert_eq!(verifier.max_response_bytes, Some(65_536));
    assert!(manifest.tools.iter().all(|tool| {
        tool.limits.max_response_bytes == Some(1_048_576)
            && tool.limits.max_component_ms == Some(300_000)
    }));
}

#[test]
fn llm_mentions_schema_exposes_chatgpt_selector_limits() {
    let manifest = manifest();
    let schema = manifest
        .tools
        .iter()
        .find(|tool| tool.name == "dataforseo_llm_mentions")
        .and_then(|tool| tool.input_schema.as_ref())
        .unwrap();
    let validator = jsonschema::validator_for(schema).unwrap();

    assert!(validator.is_valid(&json!({
        "platform": "chat_gpt",
        "location_code": 2840,
        "language_code": "en",
        "targets": [{ "domain": "example.com" }]
    })));
    assert!(!validator.is_valid(&json!({
        "platform": "chat_gpt",
        "location_name": "United Kingdom",
        "language_name": "English",
        "targets": [{ "domain": "example.com" }]
    })));
    assert!(validator.is_valid(&json!({
        "platform": "google",
        "location_name": "United Kingdom",
        "language_name": "English",
        "targets": [{ "domain": "example.com" }]
    })));
}
