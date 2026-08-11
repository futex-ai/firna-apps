use serde_json::{Value, json};

use super::support::{assert_error, capturing_http, invoke, response, success_output};

#[test]
fn expanded_create_post_maps_quote_media_and_policy_fields() {
    let (http, requests) = capturing_http(response(
        201,
        Some(json!({"data": {"id": "44", "text": "Quote"}})),
    ));

    let output = invoke(
        &http,
        "x_create_post",
        json!({
            "text": "Quote",
            "quote_post_id": "11",
            "media_ids": ["21", "22"],
            "community_id": "9",
            "reply_settings": "mentioned_users",
            "made_with_ai": true,
            "paid_partnership": false
        }),
    );

    assert_eq!(success_output(&output)["post"]["id"], "44");
    let requests = requests.lock().expect("request capture lock");
    assert_eq!(
        requests[0].body_json,
        Some(json!({
            "text": "Quote",
            "quote_tweet_id": "11",
            "media": {"media_ids": ["21", "22"]},
            "community_id": "9",
            "reply_settings": "mentionedUsers",
            "made_with_ai": true,
            "paid_partnership": false
        }))
    );
}

#[test]
fn create_post_maps_edit_options_to_the_previous_post() {
    let (http, requests) = capturing_http(response(
        201,
        Some(json!({"data": {"id": "45", "text": "Edited"}})),
    ));

    let output = invoke(
        &http,
        "x_create_post",
        json!({"text": "Edited", "edit_post_id": "44"}),
    );

    assert_eq!(success_output(&output)["post"]["id"], "45");
    let requests = requests.lock().expect("request capture lock");
    assert_eq!(
        requests[0].body_json,
        Some(json!({
            "text": "Edited",
            "edit_options": {"previous_post_id": "44"}
        }))
    );
}

#[test]
fn create_poll_maps_valid_poll_and_rejects_incompatible_media() {
    let (http, requests) = capturing_http(response(
        201,
        Some(json!({"data": {"id": "44", "text": "Choose"}})),
    ));
    let output = invoke(
        &http,
        "x_create_post",
        json!({
            "text": "Choose", "poll_options": ["A", "B"],
            "poll_duration_minutes": 60
        }),
    );
    assert_eq!(success_output(&output)["post"]["id"], "44");
    assert_eq!(
        requests.lock().expect("request capture lock")[0].body_json,
        Some(json!({
            "text": "Choose",
            "poll": {"options": ["A", "B"], "duration_minutes": 60}
        }))
    );

    let no_http = unimock::Unimock::new(());
    let rejected = invoke(
        &no_http,
        "x_create_post",
        json!({
            "text": "Choose", "poll_options": ["A", "B"],
            "poll_duration_minutes": 60, "media_ids": ["21"]
        }),
    );
    assert_error(&rejected, "invalid_request");
    assert_eq!(rejected["reason"], "invalid_poll");
}

#[test]
fn every_post_action_routes_once_and_requires_matching_confirmation() {
    let cases = [
        post_case(
            "delete",
            json!({"deleted": true}),
            "DELETE",
            "/tweets/11",
            5_000,
            false,
        ),
        post_case(
            "repost",
            json!({"retweeted": true}),
            "POST",
            "/users/7/retweets",
            15_000,
            true,
        ),
        post_case(
            "unrepost",
            json!({"retweeted": false}),
            "DELETE",
            "/users/7/retweets/11",
            10_000,
            true,
        ),
        post_case(
            "like",
            json!({"liked": true}),
            "POST",
            "/users/7/likes",
            15_000,
            true,
        ),
        post_case(
            "unlike",
            json!({"liked": false}),
            "DELETE",
            "/users/7/likes/11",
            10_000,
            true,
        ),
        post_case(
            "bookmark",
            json!({"bookmarked": true}),
            "POST",
            "/users/7/bookmarks",
            5_000,
            true,
        ),
        post_case(
            "unbookmark",
            json!({"bookmarked": false}),
            "DELETE",
            "/users/7/bookmarks/11",
            5_000,
            true,
        ),
        post_case(
            "hide_reply",
            json!({"hidden": true}),
            "PUT",
            "/tweets/11/hidden",
            10_000,
            false,
        ),
        post_case(
            "unhide_reply",
            json!({"hidden": false}),
            "PUT",
            "/tweets/11/hidden",
            10_000,
            false,
        ),
    ];

    for case in cases {
        let (http, requests) = capturing_http(response(200, Some(json!({"data": case.data}))));
        let mut input = json!({"action": case.action, "post_id": "11"});
        if case.needs_user {
            input["user_id"] = json!("7");
        }
        let output = invoke(&http, "x_manage_post", input);
        assert_eq!(success_output(&output)["applied"], true, "{}", case.action);
        assert_eq!(output["usage"]["cost_usd_micros"], case.cost);
        let requests = requests.lock().expect("request capture lock");
        assert_eq!(requests[0].method, case.method);
        assert_eq!(requests[0].url, format!("https://api.x.com/2{}", case.path));
        assert_eq!(requests.len(), 1);
    }
}

#[test]
fn every_relationship_action_routes_and_confirms_state() {
    let cases = [
        (
            "follow",
            json!({"following": true, "pending_follow": false}),
            "POST",
            "/users/7/following",
            15_000,
        ),
        (
            "unfollow",
            json!({"following": false}),
            "DELETE",
            "/users/7/following/8",
            10_000,
        ),
        (
            "mute",
            json!({"muting": true}),
            "POST",
            "/users/7/muting",
            15_000,
        ),
        (
            "unmute",
            json!({"muting": false}),
            "DELETE",
            "/users/7/muting/8",
            5_000,
        ),
        (
            "dm_block",
            json!({"blocked": true}),
            "POST",
            "/users/8/dm/block",
            10_000,
        ),
        (
            "dm_unblock",
            json!({"blocked": false}),
            "POST",
            "/users/8/dm/unblock",
            10_000,
        ),
    ];

    for (action, data, method, path, cost) in cases {
        let (http, requests) = capturing_http(response(200, Some(json!({"data": data}))));
        let output = invoke(
            &http,
            "x_manage_relationship",
            json!({"action": action, "user_id": "7", "target_user_id": "8"}),
        );
        assert_eq!(success_output(&output)["applied"], true, "{action}");
        assert_eq!(output["usage"]["cost_usd_micros"], cost);
        let requests = requests.lock().expect("request capture lock");
        assert_eq!(requests[0].method, method);
        assert_eq!(requests[0].url, format!("https://api.x.com/2{path}"));
    }
}

struct PostCase {
    action: &'static str,
    data: Value,
    method: &'static str,
    path: &'static str,
    cost: u64,
    needs_user: bool,
}

fn post_case(
    action: &'static str,
    data: Value,
    method: &'static str,
    path: &'static str,
    cost: u64,
    needs_user: bool,
) -> PostCase {
    PostCase {
        action,
        data,
        method,
        path,
        cost,
        needs_user,
    }
}
