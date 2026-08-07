use fna_apps_interface::manifest::{AppSourceKind, InstallPolicy, ToolSideEffect};

use crate::manifest;

#[test]
fn http_manifest_declares_workspace_default_request_tool() {
    let manifest = manifest();

    manifest.validate().unwrap();
    assert_eq!(manifest.id, "http");
    assert_eq!(manifest.version, "1.0.14");
    assert_eq!(
        manifest.icon.as_ref().unwrap().color_pair.primary,
        "#3266B8"
    );
    assert_eq!(
        manifest.icon.as_ref().unwrap().color_pair.secondary,
        "#58B5E8"
    );
    assert_eq!(manifest.source.kind, AppSourceKind::BuiltIn);
    assert_eq!(manifest.install.policy, InstallPolicy::WorkspaceDefault);
    assert!(manifest.capabilities.http.as_ref().unwrap().allow_any_host);
    assert_eq!(manifest.tools.len(), 1);
    assert_eq!(manifest.tools[0].name, "http_request");
    assert_eq!(manifest.tools[0].activity_label, "Sending HTTP request");
    assert_eq!(manifest.tools[0].side_effect, ToolSideEffect::ExternalWrite);
    assert!(manifest.events.is_empty());
}
