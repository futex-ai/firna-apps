use std::collections::HashSet;

use fna_apps_interface::manifest::ToolSideEffect;
use serde_json::json;

use crate::manifest;

#[test]
fn x_manifest_declares_the_exact_comprehensive_tool_surface() {
    let manifest = manifest();
    let expected = [
        "x_get_posts",
        "x_get_post_metrics",
        "x_search_recent_posts",
        "x_create_post",
        "x_search_all_posts",
        "x_get_post_counts",
        "x_get_users",
        "x_search_users",
        "x_get_user_feed",
        "x_get_post_engagements",
        "x_get_relationships",
        "x_get_lists",
        "x_get_spaces",
        "x_get_communities",
        "x_get_trends",
        "x_get_dms",
        "x_get_media",
        "x_manage_post",
        "x_manage_relationship",
        "x_manage_list",
        "x_manage_dm",
        "x_manage_media",
        "x_create_bookmark_folder",
    ];
    let actual = manifest
        .tools
        .iter()
        .map(|tool| tool.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(actual, expected);
    assert_eq!(actual.iter().copied().collect::<HashSet<_>>().len(), 23);
    for tool in &manifest.tools {
        assert_eq!(tool.auth, ["x_workspace"]);
        assert_eq!(tool.export, "call-tool");
        assert_eq!(tool.limits.max_response_bytes, Some(262_144));
        assert_eq!(tool.limits.max_component_ms, Some(30_000));
        let schema = tool.input_schema.as_ref().expect("inline schema");
        assert_eq!(schema["additionalProperties"], json!(false));
        assert!(schema["properties"].get("connection_id").is_none());
        if tool.name.starts_with("x_manage_")
            || tool.name == "x_create_post"
            || tool.name == "x_create_bookmark_folder"
        {
            assert_eq!(tool.side_effect, ToolSideEffect::ExternalWrite);
        } else {
            assert_eq!(tool.side_effect, ToolSideEffect::ExternalRead);
        }
    }
    assert_eq!(manifest.limits.max_tool_response_bytes, Some(262_144));
    assert_eq!(manifest.limits.max_component_ms, Some(30_000));
    assert!(manifest.ingress.is_empty());
    assert!(manifest.events.is_empty());
}

#[test]
fn x_manifest_exposes_expanded_create_and_mode_selectors() {
    let manifest = manifest();
    let create = manifest
        .tools
        .iter()
        .find(|tool| tool.name == "x_create_post")
        .expect("create tool");
    let properties = &create.input_schema.as_ref().expect("schema")["properties"];
    for field in [
        "reply_to_post_id",
        "quote_post_id",
        "edit_post_id",
        "poll_options",
        "poll_duration_minutes",
        "media_ids",
        "community_id",
        "reply_settings",
        "made_with_ai",
        "paid_partnership",
        "allow_link",
    ] {
        assert!(
            properties.get(field).is_some(),
            "missing create field {field}"
        );
    }

    for (tool_name, selector) in [
        ("x_get_users", "lookup"),
        ("x_get_user_feed", "feed"),
        ("x_get_post_engagements", "view"),
        ("x_get_relationships", "relationship"),
        ("x_get_lists", "view"),
        ("x_get_spaces", "view"),
        ("x_get_communities", "view"),
        ("x_get_trends", "view"),
        ("x_get_dms", "view"),
        ("x_manage_post", "action"),
        ("x_manage_relationship", "action"),
        ("x_manage_list", "action"),
        ("x_manage_dm", "action"),
        ("x_manage_media", "action"),
    ] {
        let tool = manifest
            .tools
            .iter()
            .find(|tool| tool.name == tool_name)
            .expect("mode tool");
        assert!(
            tool.input_schema.as_ref().expect("schema")["properties"][selector]["enum"]
                .as_array()
                .is_some_and(|values| !values.is_empty())
        );
    }
}
