use serde_json::{Value, json};

use super::support::{capturing_http, invoke, response, success_output};

#[test]
fn every_account_lookup_routes_to_its_single_provider_endpoint() {
    let cases = [
        (
            json!({"lookup": "me"}),
            json!({"data": {"id": "7", "name": "Ada", "username": "ada"}}),
            "https://api.x.com/2/users/me",
        ),
        (
            json!({"lookup": "ids", "ids": ["7"]}),
            user_collection(),
            "https://api.x.com/2/users",
        ),
        (
            json!({"lookup": "usernames", "usernames": ["ada"]}),
            user_collection(),
            "https://api.x.com/2/users/by",
        ),
    ];

    for (input, body, expected_url) in cases {
        assert_route("x_get_users", input, body, expected_url);
    }
}

#[test]
fn every_relationship_read_routes_to_its_provider_collection() {
    for (relationship, suffix) in [
        ("affiliates", "affiliates"),
        ("followers", "followers"),
        ("following", "following"),
        ("blocked", "blocking"),
        ("muted", "muting"),
    ] {
        assert_route(
            "x_get_relationships",
            json!({"user_id": "7", "relationship": relationship, "max_results": 10}),
            empty_collection(),
            &format!("https://api.x.com/2/users/7/{suffix}"),
        );
    }
}

#[test]
fn every_user_feed_routes_to_its_provider_collection() {
    let cases = [
        feed_case("posts", Some("7"), None, "/users/7/tweets"),
        feed_case("mentions", Some("7"), None, "/users/7/mentions"),
        feed_case(
            "home",
            Some("7"),
            None,
            "/users/7/timelines/reverse_chronological",
        ),
        feed_case("liked", Some("7"), None, "/users/7/liked_tweets"),
        feed_case("bookmarks", Some("7"), None, "/users/7/bookmarks"),
        feed_case(
            "bookmark_folder",
            Some("7"),
            Some("3"),
            "/users/7/bookmarks/folders/3",
        ),
        feed_case(
            "bookmark_folders",
            Some("7"),
            None,
            "/users/7/bookmarks/folders",
        ),
        feed_case("reposts_of_me", None, None, "/users/reposts_of_me"),
    ];

    for (input, path) in cases {
        assert_route(
            "x_get_user_feed",
            input,
            empty_collection(),
            &format!("https://api.x.com/2{path}"),
        );
    }
}

#[test]
fn every_post_engagement_view_routes_to_its_provider_collection() {
    for (view, suffix) in [
        ("quotes", "quote_tweets"),
        ("reposts", "retweets"),
        ("liking_users", "liking_users"),
        ("reposting_users", "retweeted_by"),
    ] {
        assert_route(
            "x_get_post_engagements",
            json!({"post_id": "11", "view": view, "max_results": 10}),
            empty_collection(),
            &format!("https://api.x.com/2/tweets/11/{suffix}"),
        );
    }
}

#[test]
fn every_list_view_routes_to_its_provider_endpoint() {
    assert_route(
        "x_get_lists",
        json!({"view": "list", "list_id": "4"}),
        json!({"data": {"id": "4", "name": "Rust"}}),
        "https://api.x.com/2/lists/4",
    );
    let cases = [
        list_case("owned", "user_id", "/users/7/owned_lists", false),
        list_case("followed", "user_id", "/users/7/followed_lists", false),
        list_case("memberships", "user_id", "/users/7/list_memberships", false),
        list_case("pinned", "user_id", "/users/7/pinned_lists", true),
        list_case("posts", "list_id", "/lists/7/tweets", false),
        list_case("members", "list_id", "/lists/7/members", false),
        list_case("followers", "list_id", "/lists/7/followers", false),
    ];

    for (input, path) in cases {
        assert_route(
            "x_get_lists",
            input,
            empty_collection(),
            &format!("https://api.x.com/2{path}"),
        );
    }
}

#[test]
fn every_space_view_routes_to_its_provider_endpoint() {
    let cases = [
        (json!({"view": "ids", "ids": ["5"]}), "/spaces"),
        (
            json!({"view": "creators", "creator_ids": ["7"]}),
            "/spaces/by/creator_ids",
        ),
        (
            json!({"view": "search", "query": "rust", "max_results": 10}),
            "/spaces/search",
        ),
        (
            json!({"view": "posts", "space_id": "5", "max_results": 10}),
            "/spaces/5/tweets",
        ),
        (
            json!({"view": "buyers", "space_id": "5", "max_results": 10}),
            "/spaces/5/buyers",
        ),
    ];

    for (input, path) in cases {
        assert_route(
            "x_get_spaces",
            input,
            empty_collection(),
            &format!("https://api.x.com/2{path}"),
        );
    }
}

#[test]
fn remaining_collection_modes_route_once() {
    let cases = [
        (
            "x_get_communities",
            json!({"view": "search", "query": "rust", "max_results": 10}),
            "https://api.x.com/2/communities/search",
        ),
        (
            "x_get_dms",
            json!({"view": "all", "max_results": 10}),
            "https://api.x.com/2/dm_events",
        ),
        (
            "x_get_dms",
            json!({"view": "conversation", "conversation_id": "7-8", "max_results": 10}),
            "https://api.x.com/2/dm_conversations/7-8/dm_events",
        ),
        (
            "x_get_dms",
            json!({"view": "participant", "participant_id": "8", "max_results": 10}),
            "https://api.x.com/2/dm_conversations/with/8/dm_events",
        ),
    ];

    for (tool, input, expected_url) in cases {
        assert_route(tool, input, empty_collection(), expected_url);
    }
}

#[test]
fn recent_counts_use_the_app_bearer_and_declared_cost() {
    let (http, requests) = capturing_http(response(
        200,
        Some(json!({"data": [], "meta": {"total_tweet_count": 0}})),
    ));
    let output = invoke(
        &http,
        "x_get_post_counts",
        json!({"range": "recent", "query": "rust", "granularity": "hour"}),
    );

    assert_eq!(success_output(&output)["total_post_count"], 0);
    assert_eq!(output["usage"]["cost_usd_micros"], 5_000);
    let requests = requests.lock().expect("request capture lock");
    assert_eq!(requests[0].url, "https://api.x.com/2/tweets/counts/recent");
    assert_eq!(requests[0].credential.credential_kind, "bearer_token");
}

fn assert_route(tool: &str, input: Value, body: Value, expected_url: &str) {
    let (http, requests) = capturing_http(response(200, Some(body)));
    let output = invoke(&http, tool, input);
    success_output(&output);
    let requests = requests.lock().expect("request capture lock");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].url, expected_url);
}

fn user_collection() -> Value {
    json!({"data": [{"id": "7", "name": "Ada", "username": "ada"}]})
}

fn empty_collection() -> Value {
    json!({"data": []})
}

fn feed_case(
    feed: &str,
    user_id: Option<&str>,
    folder_id: Option<&str>,
    path: &'static str,
) -> (Value, &'static str) {
    let mut input = json!({"feed": feed, "max_results": 10});
    if let Some(user_id) = user_id {
        input["user_id"] = json!(user_id);
    }
    if let Some(folder_id) = folder_id {
        input["folder_id"] = json!(folder_id);
    }
    (input, path)
}

fn list_case(
    view: &str,
    selector: &str,
    path: &'static str,
    pinned: bool,
) -> (Value, &'static str) {
    let mut input = json!({"view": view});
    input[selector] = json!("7");
    if !pinned {
        input["max_results"] = json!(10);
    }
    (input, path)
}
