use serde_json::{Value, json};

use super::support::{capturing_http, invoke, response, success_output};

#[test]
fn every_direct_message_action_routes_once_and_reports_exact_cost() {
    let cases = [
        dm_case(
            "send_to_participant",
            json!({"participant_id": "8", "text": "hello"}),
            json!({"dm_conversation_id": "7-8", "dm_event_id": "20"}),
            "POST",
            "/dm_conversations/with/8/messages",
            15_000,
        ),
        dm_case(
            "send_to_conversation",
            json!({"conversation_id": "7-8", "media_id": "31"}),
            json!({"dm_conversation_id": "7-8", "dm_event_id": "21"}),
            "POST",
            "/dm_conversations/7-8/messages",
            15_000,
        ),
        dm_case(
            "create_group",
            json!({"participant_ids": ["8", "9"], "text": "group"}),
            json!({"dm_conversation_id": "group-1", "dm_event_id": "22"}),
            "POST",
            "/dm_conversations",
            15_000,
        ),
        dm_case(
            "delete",
            json!({"event_id": "20"}),
            json!({"deleted": true}),
            "DELETE",
            "/dm_events/20",
            10_000,
        ),
    ];

    for case in cases {
        let (http, requests) = capturing_http(response(200, Some(json!({"data": case.data}))));
        let mut input = case.input;
        input["action"] = json!(case.action);
        let output = invoke(&http, "x_manage_dm", input);
        assert_eq!(success_output(&output)["applied"], true, "{}", case.action);
        assert_eq!(output["usage"]["cost_usd_micros"], case.cost);
        let requests = requests.lock().expect("request capture lock");
        assert_eq!(requests[0].method, case.method);
        assert_eq!(requests[0].url, format!("https://api.x.com/2{}", case.path));
        assert_eq!(requests.len(), 1);
    }
}

#[test]
fn direct_message_group_uses_the_typed_group_shape() {
    let (http, requests) = capturing_http(response(
        201,
        Some(json!({"data": {"dm_conversation_id": "group-1", "dm_event_id": "22"}})),
    ));

    let output = invoke(
        &http,
        "x_manage_dm",
        json!({
            "action": "create_group", "participant_ids": ["8", "9"],
            "text": "hello", "media_id": "31"
        }),
    );

    assert_eq!(success_output(&output)["conversation_id"], "group-1");
    let requests = requests.lock().expect("request capture lock");
    assert_eq!(
        requests[0].body_json,
        Some(json!({
            "conversation_type": "Group",
            "participant_ids": ["8", "9"],
            "message": {"text": "hello", "attachments": [{"media_id": "31"}]}
        }))
    );
}

#[test]
fn direct_message_text_preserves_user_whitespace() {
    let (http, requests) = capturing_http(response(
        201,
        Some(json!({"data": {"dm_conversation_id": "7-8", "dm_event_id": "22"}})),
    ));

    let output = invoke(
        &http,
        "x_manage_dm",
        json!({
            "action": "send_to_participant", "participant_id": "8", "text": "  hello  "
        }),
    );

    assert_eq!(success_output(&output)["applied"], true);
    let requests = requests.lock().expect("request capture lock");
    assert_eq!(requests[0].body_json, Some(json!({"text": "  hello  "})));
}

#[test]
fn bookmark_folder_creation_returns_provider_confirmed_folder() {
    let (http, requests) = capturing_http(response(
        201,
        Some(json!({"data": {"id": "3", "name": "Research"}})),
    ));

    let output = invoke(
        &http,
        "x_create_bookmark_folder",
        json!({"user_id": "7", "name": " Research "}),
    );

    assert_eq!(success_output(&output)["folder"]["id"], "3");
    assert_eq!(output["usage"]["cost_usd_micros"], 5_000);
    let requests = requests.lock().expect("request capture lock");
    assert_eq!(
        requests[0].url,
        "https://api.x.com/2/users/7/bookmarks/folders"
    );
    assert_eq!(requests[0].body_json, Some(json!({"name": "Research"})));
}

struct DmCase {
    action: &'static str,
    input: Value,
    data: Value,
    method: &'static str,
    path: &'static str,
    cost: u64,
}

fn dm_case(
    action: &'static str,
    input: Value,
    data: Value,
    method: &'static str,
    path: &'static str,
    cost: u64,
) -> DmCase {
    DmCase {
        action,
        input,
        data,
        method,
        path,
        cost,
    }
}
