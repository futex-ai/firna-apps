use serde_json::{Value, json};

use super::support::{capturing_http, invoke, response, success_output};

#[test]
fn list_create_and_every_state_action_route_once() {
    let cases = [
        list_case(
            "create",
            json!({"name": "Rust"}),
            json!({"id": "4", "name": "Rust"}),
            "POST",
            "/lists",
            10_000,
        ),
        list_case(
            "update",
            json!({"list_id": "4", "description": "Updated"}),
            json!({"updated": true}),
            "PUT",
            "/lists/4",
            5_000,
        ),
        list_case(
            "delete",
            json!({"list_id": "4"}),
            json!({"deleted": true}),
            "DELETE",
            "/lists/4",
            5_000,
        ),
        list_case(
            "add_member",
            json!({"list_id": "4", "target_user_id": "8"}),
            json!({"is_member": true}),
            "POST",
            "/lists/4/members",
            5_000,
        ),
        list_case(
            "remove_member",
            json!({"list_id": "4", "target_user_id": "8"}),
            json!({"is_member": false}),
            "DELETE",
            "/lists/4/members/8",
            5_000,
        ),
        list_case(
            "follow",
            json!({"list_id": "4", "user_id": "7"}),
            json!({"following": true}),
            "POST",
            "/users/7/followed_lists",
            5_000,
        ),
        list_case(
            "unfollow",
            json!({"list_id": "4", "user_id": "7"}),
            json!({"following": false}),
            "DELETE",
            "/users/7/followed_lists/4",
            5_000,
        ),
        list_case(
            "pin",
            json!({"list_id": "4", "user_id": "7"}),
            json!({"pinned": true}),
            "POST",
            "/users/7/pinned_lists",
            5_000,
        ),
        list_case(
            "unpin",
            json!({"list_id": "4", "user_id": "7"}),
            json!({"pinned": false}),
            "DELETE",
            "/users/7/pinned_lists/4",
            5_000,
        ),
    ];

    for case in cases {
        let (http, requests) = capturing_http(response(200, Some(json!({"data": case.data}))));
        let mut input = case.input;
        input["action"] = json!(case.action);
        let output = invoke(&http, "x_manage_list", input);
        assert_eq!(success_output(&output)["applied"], true, "{}", case.action);
        assert_eq!(output["usage"]["cost_usd_micros"], case.cost);
        let requests = requests.lock().expect("request capture lock");
        assert_eq!(requests[0].method, case.method);
        assert_eq!(requests[0].url, format!("https://api.x.com/2{}", case.path));
        assert_eq!(requests.len(), 1);
    }
}

#[test]
fn list_privacy_update_uses_the_higher_declared_cost() {
    let (http, _) = capturing_http(response(200, Some(json!({"data": {"updated": true}}))));
    let output = invoke(
        &http,
        "x_manage_list",
        json!({"action": "update", "list_id": "4", "private": true}),
    );
    assert_eq!(output["usage"]["cost_usd_micros"], 10_000);
}

struct ListCase {
    action: &'static str,
    input: Value,
    data: Value,
    method: &'static str,
    path: &'static str,
    cost: u64,
}

fn list_case(
    action: &'static str,
    input: Value,
    data: Value,
    method: &'static str,
    path: &'static str,
    cost: u64,
) -> ListCase {
    ListCase {
        action,
        input,
        data,
        method,
        path,
        cost,
    }
}
