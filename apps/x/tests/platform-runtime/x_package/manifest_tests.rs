use fna_apps_interface::manifest::{
    AppConnectionMode, AppSourceKind, AuthOwner, InstallPolicy, StandardOAuthClientAuthMethod,
    StandardOAuthPkceMethod, StandardOAuthPkceMode,
};

use crate::manifest;

#[test]
fn x_manifest_declares_comprehensive_oauth_and_host_contract() {
    let manifest = manifest();

    manifest.validate().expect("X manifest should validate");
    assert_eq!(manifest.id, "x");
    assert_eq!(manifest.name, "X");
    assert_eq!(manifest.version, "2.0.2");
    assert_eq!(manifest.source.kind, AppSourceKind::BuiltIn);
    assert_eq!(manifest.source.package, None);
    assert_eq!(manifest.install.policy, InstallPolicy::Explicit);
    assert_eq!(
        manifest.install.connection_mode,
        AppConnectionMode::Multiple
    );
    assert!(manifest.env.is_empty());
    assert_eq!(
        manifest
            .secrets
            .iter()
            .map(|secret| (secret.name.as_str(), secret.required))
            .collect::<Vec<_>>(),
        [
            ("client_id", true),
            ("client_secret", true),
            ("bearer_token", true)
        ]
    );
    let http = manifest.capabilities.http.expect("X HTTP capability");
    assert_eq!(http.allowed_hosts, [String::from("api.x.com")]);
    assert!(!http.allow_any_host);
    assert!(http.credential_headers.is_empty());
    assert_eq!(http.max_response_bytes, Some(262_144));

    assert_eq!(manifest.auth_requirements.len(), 1);
    let requirement = &manifest.auth_requirements[0];
    assert_eq!(requirement.id, "x_workspace");
    assert_eq!(requirement.owner, AuthOwner::Workspace);
    assert_eq!(requirement.credential_flow_id(), Some("x_oauth"));
    assert_eq!(
        requirement.scopes,
        [
            "tweet.read",
            "tweet.write",
            "users.read",
            "follows.read",
            "follows.write",
            "like.read",
            "like.write",
            "list.read",
            "list.write",
            "block.read",
            "mute.read",
            "mute.write",
            "bookmark.read",
            "bookmark.write",
            "dm.read",
            "dm.write",
            "space.read",
            "timeline.read",
            "tweet.moderate.write",
            "media.write",
            "offline.access",
        ]
    );
    assert_eq!(
        requirement.credential_kinds,
        ["access_token", "refresh_token"]
    );
}

#[test]
fn x_oauth_flow_keeps_identity_and_refresh_contract() {
    let manifest = manifest();
    let flow = manifest.credential_flows[0]
        .standard_oauth2()
        .expect("standard OAuth flow");

    assert_eq!(flow.authorization_url, "https://x.com/i/oauth2/authorize");
    assert_eq!(flow.token_url, "https://api.x.com/2/oauth2/token");
    assert_eq!(flow.client.client_id, None);
    assert_eq!(flow.client.client_id_env.as_deref(), Some("client_id"));
    assert_eq!(
        flow.client.client_secret_env.as_deref(),
        Some("client_secret")
    );
    assert_eq!(
        flow.client.auth_method,
        StandardOAuthClientAuthMethod::ClientSecretBasic
    );
    let pkce = flow.pkce.as_ref().expect("required PKCE");
    assert_eq!(pkce.mode, StandardOAuthPkceMode::Required);
    assert_eq!(pkce.method, StandardOAuthPkceMethod::S256);

    let identity = flow.identity.as_ref().expect("X OAuth identity request");
    assert_eq!(identity.url, "https://api.x.com/2/users/me");
    assert_eq!(identity.access_token_credential_kind, "access_token");
    assert_eq!(
        identity
            .response_mapping
            .provider_account_id
            .as_ref()
            .expect("provider account id mapping")
            .selectors(),
        ["$.data.id"]
    );
    assert_eq!(
        identity
            .response_mapping
            .provider_account_label
            .as_ref()
            .expect("provider account label mapping")
            .selectors(),
        ["$.data.username"]
    );

    let mapping = &flow.response_mapping.requirements[0];
    assert_eq!(mapping.auth_requirement_id, "x_workspace");
    assert_eq!(
        mapping
            .credentials
            .iter()
            .map(|credential| credential.credential_kind.as_str())
            .collect::<Vec<_>>(),
        ["access_token", "refresh_token"]
    );
    let lifecycle = flow.token_lifecycle.as_ref().expect("token lifecycle");
    assert_eq!(lifecycle.auth_requirement_id, "x_workspace");
    assert_eq!(lifecycle.access_token_credential_kind, "access_token");
    assert_eq!(lifecycle.refresh_token_credential_kind, "refresh_token");
    assert_eq!(lifecycle.expires_in.selectors(), ["$.expires_in"]);
    assert_eq!(lifecycle.refresh_before_seconds, 300);
}
