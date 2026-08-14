use serde_json::json;
use unimock::Unimock;

use super::support::{
    assert_error, assert_read_usage, capturing_http, capturing_http_responses, invoke, response,
    success_output,
};

#[test]
fn omitted_feed_user_resolves_the_connected_account() {
    let (http, requests) = capturing_http_responses(vec![
        response(
            200,
            Some(json!({"data": {"id": "7", "name": "Ada", "username": "ada"}})),
        ),
        response(
            200,
            Some(json!({
                "data": [{"id": "11", "text": "Post", "author_id": "7"}],
                "includes": {"users": [{"id": "7", "name": "Ada", "username": "ada"}]}
            })),
        ),
    ]);

    let output = invoke(
        &http,
        "x_get_user_feed",
        json!({"feed": "home", "max_results": 10, "include_authors": true}),
    );

    assert_eq!(success_output(&output)["posts"][0]["id"], "11");
    assert_read_usage(&output, 1, 2);
    let requests = requests.lock().expect("request capture lock");
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].url, "https://api.x.com/2/users/me");
    assert_eq!(
        requests[1].url,
        "https://api.x.com/2/users/7/timelines/reverse_chronological"
    );
}

#[test]
fn omitted_feed_user_rejects_an_invalid_connected_account_id() {
    let (http, requests) = capturing_http(response(200, Some(json!({"data": {"id": "invalid"}}))));

    let output = invoke(
        &http,
        "x_get_user_feed",
        json!({"feed": "home", "max_results": 10}),
    );

    assert_error(&output, "provider_contract_error");
    let requests = requests.lock().expect("request capture lock");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].url, "https://api.x.com/2/users/me");
}

#[test]
fn connected_account_bookmark_folders_report_the_identity_read() {
    let (http, requests) = capturing_http_responses(vec![
        response(200, Some(json!({"data": {"id": "7"}}))),
        response(
            200,
            Some(json!({"data": [{"id": "3", "name": "Research"}]})),
        ),
    ]);

    let output = invoke(
        &http,
        "x_get_user_feed",
        json!({"feed": "bookmark_folders", "max_results": 10}),
    );

    assert_eq!(success_output(&output)["bookmark_folders"][0]["id"], "3");
    assert_eq!(
        output["usage"],
        json!({
            "kind": "metered",
            "units": [{"unit": "user_read", "quantity": 1}]
        })
    );
    let requests = requests.lock().expect("request capture lock");
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].url, "https://api.x.com/2/users/me");
    assert_eq!(
        requests[1].url,
        "https://api.x.com/2/users/7/bookmarks/folders"
    );
}

#[test]
fn invalid_feed_options_fail_before_connected_account_resolution() {
    let http = Unimock::new(());

    let output = invoke(
        &http,
        "x_get_user_feed",
        json!({"feed": "home", "max_results": 10, "exclude_replies": true}),
    );

    assert_error(&output, "invalid_request");
    assert_eq!(output["reason"], "invalid_feed_selector");
}

#[test]
fn user_feed_maps_exclusions_authors_and_pagination() {
    let (http, requests) = capturing_http(response(
        200,
        Some(json!({
            "data": [{"id": "11", "text": "Post", "author_id": "7"}],
            "includes": {"users": [{"id": "7", "name": "Ada", "username": "ada"}]},
            "meta": {"next_token": "next"}
        })),
    ));

    let output = invoke(
        &http,
        "x_get_user_feed",
        json!({
            "feed": "posts", "user_id": "7", "max_results": 10,
            "include_authors": true, "exclude_replies": true, "exclude_reposts": true
        }),
    );

    let result = success_output(&output);
    assert_eq!(result["posts"][0]["id"], "11");
    assert_eq!(result["authors"][0]["id"], "7");
    assert_eq!(result["pagination_token"], "next");
    let requests = requests.lock().expect("request capture lock");
    assert_eq!(requests[0].url, "https://api.x.com/2/users/7/tweets");
    assert_eq!(requests[0].query["exclude"], "replies,retweets");
}

#[test]
fn bookmark_folder_feed_returns_folders_without_post_usage() {
    let (http, requests) = capturing_http(response(
        200,
        Some(json!({"data": [{"id": "3", "name": "Research"}]})),
    ));

    let output = invoke(
        &http,
        "x_get_user_feed",
        json!({"feed": "bookmark_folders", "user_id": "7", "max_results": 10}),
    );

    assert_eq!(
        success_output(&output)["bookmark_folders"][0]["name"],
        "Research"
    );
    assert_eq!(output["usage"], json!({"kind": "metered", "units": []}));
    let requests = requests.lock().expect("request capture lock");
    assert_eq!(
        requests[0].url,
        "https://api.x.com/2/users/7/bookmarks/folders"
    );
}

#[test]
fn post_engagement_mode_returns_users_and_user_usage() {
    let (http, requests) = capturing_http(response(
        200,
        Some(json!({"data": [{"id": "8", "name": "Lin", "username": "lin"}]})),
    ));

    let output = invoke(
        &http,
        "x_get_post_engagements",
        json!({"post_id": "11", "view": "liking_users", "max_results": 10}),
    );

    assert_eq!(success_output(&output)["users"][0]["username"], "lin");
    assert_eq!(output["usage"]["units"][0]["unit"], "user_read");
    let requests = requests.lock().expect("request capture lock");
    assert_eq!(
        requests[0].url,
        "https://api.x.com/2/tweets/11/liking_users"
    );
}
