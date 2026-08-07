use fna_apps_interface::manifest::{
    AppSourceKind, AuthOwner, AuthRequirementKind, BasicAuthInputMode, InstallPolicy,
    ToolSideEffect,
};

use crate::manifest;

#[test]
fn exa_manifest_declares_default_search_tool() {
    let manifest = manifest();

    manifest.validate().unwrap();
    assert_eq!(manifest.id, "exa");
    assert_eq!(manifest.version, "1.0.18");
    assert_eq!(
        manifest.icon.as_ref().unwrap().color_pair.primary,
        "#111111"
    );
    assert_eq!(
        manifest.icon.as_ref().unwrap().color_pair.secondary,
        "#8B5CF6"
    );
    assert_eq!(manifest.source.kind, AppSourceKind::BuiltIn);
    assert_eq!(manifest.install.policy, InstallPolicy::WorkspaceDefault);
    assert_eq!(
        manifest.capabilities.http.as_ref().unwrap().allowed_hosts,
        vec![String::from("api.exa.ai")]
    );
    assert_eq!(
        manifest
            .capabilities
            .http
            .as_ref()
            .unwrap()
            .credential_headers,
        vec![String::from("x-api-key")]
    );
    assert_eq!(manifest.secrets[0].name, "api_key");
    assert!(manifest.secrets[0].required);
    let [requirement] = manifest.auth_requirements.as_slice() else {
        panic!("expected one Exa API-key requirement");
    };
    assert_eq!(requirement.id, "exa-api-key");
    assert_eq!(requirement.kind, AuthRequirementKind::ApiKey);
    assert_eq!(requirement.owner, AuthOwner::Workspace);
    assert_eq!(requirement.credential_flow_id(), Some("exa-api-key"));
    assert!(requirement.scopes.is_empty());
    assert_eq!(requirement.credential_kinds, ["api_key"]);
    let [flow] = manifest.credential_flows.as_slice() else {
        panic!("expected one Exa API-key flow");
    };
    let api_key = flow.api_key().expect("Exa API-key flow");
    assert_eq!(api_key.field.credential_kind, "api_key");
    assert_eq!(api_key.field.label, "Exa API key");
    assert_eq!(api_key.field.input_mode, BasicAuthInputMode::Password);
    assert_eq!(api_key.field.max_bytes, 256);
    assert_eq!(api_key.help_url, "https://dashboard.exa.ai/api-keys");
    assert!(api_key.verification.is_none());
    assert_eq!(manifest.tools.len(), 1);
    assert_eq!(manifest.tools[0].name, "exa_web_search");
    assert_eq!(manifest.tools[0].activity_label, "Searching the web");
    assert_eq!(manifest.tools[0].side_effect, ToolSideEffect::ExternalRead);
    assert!(manifest.tools[0].auth.is_empty());
    assert!(manifest.events.is_empty());
}
