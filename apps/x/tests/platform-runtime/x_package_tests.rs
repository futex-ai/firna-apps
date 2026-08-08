use std::fs;

use fna_apps_interface::manifest::{
    AppSourceKind, AppToolPricingStructure, AuthOwner, InstallPolicy,
    StandardOAuthClientAuthMethod, StandardOAuthPkceMethod, StandardOAuthPkceMode, ToolSideEffect,
};
use serde_json::json;

use crate::{app_root, component_bytes, manifest};

#[test]
fn x_manifest_declares_exact_oauth_and_host_contract() {
    let manifest = manifest();

    manifest.validate().expect("X manifest should validate");
    assert_eq!(manifest.id, "x");
    assert_eq!(manifest.name, "X");
    assert_eq!(manifest.version, "1.0.11");
    assert_eq!(manifest.source.kind, AppSourceKind::BuiltIn);
    assert_eq!(manifest.source.package, None);
    assert_eq!(manifest.install.policy, InstallPolicy::Explicit);
    assert!(manifest.env.is_empty());
    assert_eq!(
        manifest
            .secrets
            .iter()
            .map(|secret| (secret.name.as_str(), secret.required))
            .collect::<Vec<_>>(),
        [("client_id", true), ("client_secret", true)]
    );
    let http = manifest.capabilities.http.expect("X HTTP capability");
    assert_eq!(http.allowed_hosts, [String::from("api.x.com")]);
    assert!(!http.allow_any_host);
    assert!(http.credential_headers.is_empty());

    assert_eq!(manifest.auth_requirements.len(), 1);
    let requirement = &manifest.auth_requirements[0];
    assert_eq!(requirement.id, "x_workspace");
    assert_eq!(requirement.owner, AuthOwner::Workspace);
    assert_eq!(requirement.credential_flow_id(), Some("x_oauth"));
    assert_eq!(
        requirement.scopes,
        ["tweet.read", "tweet.write", "users.read", "offline.access"]
    );
    assert_eq!(
        requirement.credential_kinds,
        ["access_token", "refresh_token"]
    );

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

    let mapping = &flow.response_mapping.requirements[0];
    assert_eq!(mapping.auth_requirement_id, "x_workspace");
    assert_eq!(
        mapping
            .credentials
            .iter()
            .map(|credential| (
                credential.credential_kind.as_str(),
                credential.value.selectors()
            ))
            .collect::<Vec<_>>(),
        vec![
            ("access_token", [String::from("$.access_token")].as_slice()),
            (
                "refresh_token",
                [String::from("$.refresh_token")].as_slice()
            ),
        ]
    );
    let lifecycle = flow.token_lifecycle.as_ref().expect("token lifecycle");
    assert_eq!(lifecycle.auth_requirement_id, "x_workspace");
    assert_eq!(lifecycle.access_token_credential_kind, "access_token");
    assert_eq!(lifecycle.refresh_token_credential_kind, "refresh_token");
    assert_eq!(lifecycle.expires_in.selectors(), ["$.expires_in"]);
    assert_eq!(lifecycle.refresh_before_seconds, 300);
}

#[test]
fn x_manifest_caps_the_exact_v1_charge_schedule() {
    let manifest = manifest();
    let pricing = manifest.pricing.expect("X pricing declaration");

    assert_eq!(pricing.tools.len(), 4);
    assert_metered_pricing(
        &pricing,
        "x_get_posts",
        &[("post_read", 5_000, 10), ("user_read", 10_000, 10)],
    );
    assert_metered_pricing(&pricing, "x_get_post_metrics", &[("post_read", 5_000, 10)]);
    assert_metered_pricing(
        &pricing,
        "x_search_recent_posts",
        &[("post_read", 5_000, 25), ("user_read", 10_000, 25)],
    );
    let create = pricing
        .tools
        .iter()
        .find(|entry| entry.tool == "x_create_post")
        .expect("create pricing");
    assert_eq!(
        create.structure,
        Some(AppToolPricingStructure::UsageReported {
            max_cost_usd_micros_per_call: 200_000,
        })
    );
}

fn assert_metered_pricing(
    pricing: &fna_apps_interface::manifest::AppPricing,
    tool: &str,
    expected: &[(&str, u64, u64)],
) {
    let entry = pricing
        .tools
        .iter()
        .find(|entry| entry.tool == tool)
        .expect("metered pricing");
    let Some(AppToolPricingStructure::Metered { units }) = entry.structure.as_ref() else {
        panic!("{tool} should use metered pricing");
    };
    assert_eq!(
        units
            .iter()
            .map(|unit| {
                (
                    unit.unit.as_str(),
                    unit.price_usd_micros,
                    unit.max_units_per_call,
                )
            })
            .collect::<Vec<_>>(),
        expected.to_vec()
    );
}

#[test]
fn x_manifest_declares_only_the_bounded_v1_tools() {
    let manifest = manifest();

    assert_eq!(manifest.tools.len(), 4);
    assert_eq!(
        manifest
            .tools
            .iter()
            .map(|tool| {
                (
                    tool.name.as_str(),
                    tool.activity_label.as_str(),
                    &tool.side_effect,
                )
            })
            .collect::<Vec<_>>(),
        vec![
            (
                "x_get_posts",
                "Reading X posts",
                &ToolSideEffect::ExternalRead
            ),
            (
                "x_get_post_metrics",
                "Reading X post metrics",
                &ToolSideEffect::ExternalRead
            ),
            (
                "x_search_recent_posts",
                "Searching X posts",
                &ToolSideEffect::ExternalRead
            ),
            (
                "x_create_post",
                "Publishing X post",
                &ToolSideEffect::ExternalWrite
            ),
        ]
    );
    for tool in &manifest.tools {
        assert_eq!(tool.auth, ["x_workspace"]);
        assert_eq!(tool.export, "call-tool");
        assert_eq!(tool.limits.max_response_bytes, Some(262_144));
        assert_eq!(tool.limits.max_component_ms, Some(30_000));
        assert_eq!(
            tool.input_schema.as_ref().expect("inline schema")["additionalProperties"],
            json!(false)
        );
    }
    assert_eq!(manifest.limits.max_tool_response_bytes, Some(262_144));
    assert_eq!(manifest.limits.max_component_ms, Some(30_000));
    assert!(manifest.ingress.is_empty());
    assert!(manifest.events.is_empty());

    let metrics = manifest
        .tools
        .iter()
        .find(|tool| tool.name == "x_get_post_metrics")
        .expect("metrics tool");
    assert_eq!(metrics.operation, "x.get_post_metrics");
    let schema = metrics.input_schema.as_ref().expect("metrics schema");
    assert_eq!(schema["required"], json!(["ids"]));
    assert_eq!(schema["properties"]["ids"]["minItems"], 1);
    assert_eq!(schema["properties"]["ids"]["maxItems"], 10);
    assert_eq!(schema["properties"]["ids"]["uniqueItems"], true);
    assert_eq!(
        schema["properties"]["ids"]["items"]["pattern"],
        "^[0-9]{1,19}$"
    );
    assert_eq!(
        schema["properties"]["include_private_metrics"]["type"],
        "boolean"
    );
}

#[test]
fn x_manifest_embeds_the_repo_owned_png_source() {
    let manifest = manifest();
    let icon = manifest.icon.expect("X icon");
    let base64 =
        fs::read_to_string(app_root().join("assets/icon.png.base64")).expect("read X icon base64");
    let png = fs::read(app_root().join("assets/icon.png")).expect("read X icon PNG");

    assert_eq!(icon.media_type.as_str(), "image/png");
    assert_eq!(icon.data_base64, base64.trim());
    assert_eq!(icon.color_pair.primary, "#000000");
    assert_eq!(icon.color_pair.secondary, "#FFFFFF");
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    assert!(png.len() < 64 * 1_024);
    assert_eq!(
        u32::from_be_bytes(png[16..20].try_into().expect("width")),
        128
    );
    assert_eq!(
        u32::from_be_bytes(png[20..24].try_into().expect("height")),
        128
    );
}

#[test]
fn x_component_bytes_are_a_component_binary() {
    let bytes = component_bytes();

    assert_eq!(&bytes[0..4], &[0x00, 0x61, 0x73, 0x6d]);
    assert_eq!(&bytes[4..8], &[0x0d, 0x00, 0x01, 0x00]);
}
