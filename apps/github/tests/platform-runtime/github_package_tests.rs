//! Manifest contract tests for the GitHub credential package.

use fna_apps_interface::manifest::{
    AppHttpMethod, AppSourceKind, AuthOwner, AuthRequirementKind, CredentialFlow, InstallPolicy,
};

use crate::manifest;

#[test]
fn github_manifest_declares_the_installation_token_flow() {
    let manifest = manifest();

    manifest
        .validate()
        .expect("GitHub manifest should validate");
    assert_eq!(manifest.id, "github");
    assert_eq!(manifest.version, "1.0.2");
    assert_eq!(manifest.source.kind, AppSourceKind::BuiltIn);
    assert_eq!(manifest.install.policy, InstallPolicy::Explicit);
    assert!(manifest.tools.is_empty());
    assert!(manifest.ingress.is_empty());
    assert!(manifest.events.is_empty());

    let http = manifest.capabilities.http.expect("GitHub HTTP capability");
    assert_eq!(http.allowed_hosts, ["api.github.com"]);
    assert_eq!(
        http.allowed_methods,
        Some(vec![AppHttpMethod::Get, AppHttpMethod::Post])
    );
    assert_eq!(http.max_response_bytes, Some(1_048_576));

    let [requirement] = manifest.auth_requirements.as_slice() else {
        panic!("expected one auth requirement");
    };
    assert_eq!(requirement.kind, AuthRequirementKind::AppInstallation);
    assert_eq!(requirement.owner, AuthOwner::Workspace);
    assert_eq!(
        requirement.scopes,
        ["contents:write", "metadata:read", "pull_requests:write"]
    );
    assert_eq!(requirement.credential_kinds, ["provider_installation_id"]);
    assert!(requirement.required_for.is_empty());

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
}
