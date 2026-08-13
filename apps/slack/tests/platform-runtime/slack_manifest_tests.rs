//! Slack manifest authoring and runtime-shape assertions.

use crate::manifest;

#[test]
fn slack_manifest_declares_v1_tools_ingress_and_events() {
    let manifest = manifest();

    manifest.validate().unwrap();
    assert_eq!(manifest.id, "slack");
    assert_eq!(manifest.version, "1.1.26");
    assert!(manifest.icon.is_some());
    assert_eq!(
        manifest.icon.as_ref().unwrap().color_pair.primary,
        "#36C5F0"
    );
    assert_eq!(
        manifest.icon.as_ref().unwrap().color_pair.secondary,
        "#E01E5A"
    );
    assert_eq!(manifest.credential_flows.len(), 1);
    assert_eq!(manifest.credential_flows[0].kind(), "standard_oauth2");
    assert!(
        manifest
            .auth_requirements
            .iter()
            .all(|requirement| requirement.credential_flow.as_deref() == Some("slack"))
    );
    assert_eq!(manifest.tools.len(), 4);
    assert_eq!(
        manifest
            .tools
            .iter()
            .map(|tool| tool.activity_label.as_str())
            .collect::<Vec<_>>(),
        [
            "Listing Slack channels",
            "Reading Slack channel history",
            "Sending Slack message",
            "Searching Slack messages",
        ]
    );
    assert_eq!(
        manifest
            .tools
            .iter()
            .map(|tool| (tool.name.as_str(), tool.icon.is_some()))
            .collect::<Vec<_>>(),
        [
            ("slack_list_channels", false),
            ("slack_read_channel_history", false),
            ("slack_send_message", true),
            ("slack_search_messages", true),
        ]
    );
    let send_message = manifest
        .tools
        .iter()
        .find(|tool| tool.name == "slack_send_message")
        .unwrap();
    assert_eq!(
        send_message.icon.as_ref().unwrap().data_base64,
        include_str!("../../assets/tools/slack_send_message.png.base64").trim()
    );
    let search_messages = manifest
        .tools
        .iter()
        .find(|tool| tool.name == "slack_search_messages")
        .unwrap();
    assert_eq!(
        search_messages.icon.as_ref().unwrap().data_base64,
        include_str!("../../assets/tools/slack_search_messages.png.base64").trim()
    );
    let list_channels = manifest
        .tools
        .iter()
        .find(|tool| tool.name == "slack_list_channels")
        .unwrap();
    let package_image = &manifest.icon.as_ref().unwrap().image;
    let effective_list_image = list_channels.icon.as_ref().unwrap_or(package_image);
    assert_eq!(effective_list_image, package_image);
    assert_eq!(manifest.ingress[0].verify_export, "verify-webhook");
    assert_eq!(
        manifest.ingress[0]
            .allowed_headers
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["x-slack-request-timestamp", "x-slack-signature"]
    );
    assert_eq!(manifest.ingress[0].events.len(), 5);
    assert_eq!(
        manifest
            .events
            .iter()
            .map(|event| {
                (
                    event.id.as_str(),
                    event.ingress_id.as_str(),
                    event.provider_type.as_str(),
                    event.description.as_str(),
                    event.contract_version,
                )
            })
            .collect::<Vec<_>>(),
        vec![
            (
                "app_mention",
                "slack_events",
                "app_mention",
                "A Slack message mentions the workspace app bot.",
                1,
            ),
            (
                "message_channels",
                "slack_events",
                "message.channels",
                "A public channel message is visible to the workspace app bot.",
                1,
            ),
            (
                "message_groups",
                "slack_events",
                "message.groups",
                "A private channel message is visible to the workspace app bot.",
                1,
            ),
            (
                "message_im",
                "slack_events",
                "message.im",
                "A direct message is visible to the workspace app bot.",
                1,
            ),
            (
                "message_mpim",
                "slack_events",
                "message.mpim",
                "A group direct message is visible to the workspace app bot.",
                1,
            ),
        ]
    );
}
