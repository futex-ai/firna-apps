//! Feed request validation, routing, and query construction.

use std::collections::BTreeMap;

use crate::x::errors::{InvalidInputReason, ToolError};
use crate::x::types::posts::{GetUserFeedInput, UserFeed};

use super::validation::validate_decimal_id;

const API_URL: &str = "https://api.x.com/2";
const POST_FIELDS: &str = "author_id,created_at,text";
const USER_FIELDS: &str = "id,name,username,description,created_at,location,url,profile_image_url,protected,verified,verified_type,public_metrics";

pub(super) fn validate_feed_request(input: &GetUserFeedInput) -> Result<(), ToolError> {
    if !matches!(input.feed, UserFeed::BookmarkFolder) && input.folder_id.is_some() {
        return Err(invalid_feed());
    }
    if matches!(input.feed, UserFeed::RepostsOfMe) {
        if input.user_id.is_some() || input.folder_id.is_some() {
            return Err(invalid_feed());
        }
    } else if let Some(user_id) = input.user_id.as_deref() {
        validate_decimal_id(user_id, InvalidInputReason::UserId)?;
    }
    if matches!(input.feed, UserFeed::BookmarkFolder) {
        let folder = input.folder_id.as_deref().ok_or_else(invalid_feed)?;
        validate_decimal_id(folder, InvalidInputReason::FeedSelector)?;
    }
    if matches!(input.feed, UserFeed::BookmarkFolders) && input.include_authors {
        return Err(invalid_feed());
    }
    if (input.exclude_replies || input.exclude_reposts)
        && !matches!(input.feed, UserFeed::Posts | UserFeed::Mentions)
    {
        return Err(invalid_feed());
    }
    Ok(())
}

pub(super) fn feed_route(
    input: &GetUserFeedInput,
) -> Result<(String, &'static str, bool), ToolError> {
    if matches!(input.feed, UserFeed::RepostsOfMe) {
        return Ok((
            format!("{API_URL}/users/reposts_of_me"),
            "timeline.read",
            false,
        ));
    }
    let user_id = input.user_id.as_deref().ok_or_else(invalid_feed)?;
    let route = match input.feed {
        UserFeed::Posts => (
            format!("{API_URL}/users/{user_id}/tweets"),
            "tweet.read",
            false,
        ),
        UserFeed::Mentions => (
            format!("{API_URL}/users/{user_id}/mentions"),
            "tweet.read",
            false,
        ),
        UserFeed::Home => (
            format!("{API_URL}/users/{user_id}/timelines/reverse_chronological"),
            "timeline.read",
            false,
        ),
        UserFeed::Liked => (
            format!("{API_URL}/users/{user_id}/liked_tweets"),
            "like.read",
            false,
        ),
        UserFeed::Bookmarks => (
            format!("{API_URL}/users/{user_id}/bookmarks"),
            "bookmark.read",
            false,
        ),
        UserFeed::BookmarkFolder => (
            format!(
                "{API_URL}/users/{user_id}/bookmarks/folders/{}",
                input.folder_id.as_deref().ok_or_else(invalid_feed)?
            ),
            "bookmark.read",
            false,
        ),
        UserFeed::BookmarkFolders => (
            format!("{API_URL}/users/{user_id}/bookmarks/folders"),
            "bookmark.read",
            true,
        ),
        UserFeed::RepostsOfMe => return Err(invalid_feed()),
    };
    Ok(route)
}

pub(super) fn add_post_fields(query: &mut BTreeMap<String, String>, include_authors: bool) {
    query.insert(String::from("tweet.fields"), String::from(POST_FIELDS));
    if include_authors {
        query.insert(String::from("expansions"), String::from("author_id"));
        query.insert(String::from("user.fields"), String::from(USER_FIELDS));
    }
}

pub(super) fn add_exclusions(query: &mut BTreeMap<String, String>, input: &GetUserFeedInput) {
    let mut values = Vec::new();
    if input.exclude_replies {
        values.push("replies");
    }
    if input.exclude_reposts {
        values.push("retweets");
    }
    if !values.is_empty() {
        query.insert(String::from("exclude"), values.join(","));
    }
}

fn invalid_feed() -> ToolError {
    ToolError::InvalidInput(InvalidInputReason::FeedSelector)
}
