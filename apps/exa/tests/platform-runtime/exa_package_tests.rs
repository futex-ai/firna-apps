use fna_apps_interface::manifest::{InstallPolicy, ToolSideEffect};

use crate::manifest;

#[test]
fn exa_manifest_declares_default_search_tool() {
    let manifest = manifest();

    manifest.validate().unwrap();
    assert_eq!(manifest.id, "exa");
    assert_eq!(manifest.version, "1.0.14");
    assert_eq!(
        manifest.icon.as_ref().unwrap().color_pair.primary,
        "#111111"
    );
    assert_eq!(
        manifest.icon.as_ref().unwrap().color_pair.secondary,
        "#8B5CF6"
    );
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
    assert_eq!(manifest.tools.len(), 1);
    assert_eq!(manifest.tools[0].name, "exa_web_search");
    assert_eq!(manifest.tools[0].activity_label, "Searching the web");
    assert_eq!(manifest.tools[0].side_effect, ToolSideEffect::ExternalRead);
    assert!(manifest.events.is_empty());
}
