//! User feed and Post-engagement collection tools.

use std::collections::BTreeMap;

use crate::x::errors::{InvalidInputReason, ToolError};
use crate::x::host::user_request;
use crate::x::response::decode_read_response;
use crate::x::types::common::{
    AppToolCall, CompactPost, CompactUser, PricedToolSuccess, ProviderCollection,
};
use crate::x::types::posts::{
    BookmarkFolder, EngagementView, GetPostEngagementsInput, GetPostEngagementsOutput,
    GetUserFeedInput, GetUserFeedOutput, UserFeed,
};
use crate::x::types::success::ToolSuccess;

use super::runner::ConfiguredXToolRunner;
use super::usage::{POST_READ, USER_READ, metered};
use super::validation::{
    decode_input, ensure_provider_count, normalized_token, validate_decimal_id, validate_page,
};

const API_URL: &str = "https://api.x.com/2";
const POST_FIELDS: &str = "author_id,created_at,text";
const USER_FIELDS: &str = "id,name,username,description,created_at,location,url,profile_image_url,protected,verified,verified_type,public_metrics";

impl ConfiguredXToolRunner<'_> {
    pub(super) fn get_user_feed(&self, call: AppToolCall) -> Result<PricedToolSuccess, ToolError> {
        let input: GetUserFeedInput = decode_input(call.input, InvalidInputReason::FeedSelector)?;
        validate_page(input.max_results)?;
        let (url, scope, folders) = feed_route(&input)?;
        let token = normalized_token(input.pagination_token.clone())?;
        let mut query = BTreeMap::new();
        query.insert(String::from("max_results"), input.max_results.to_string());
        if let Some(token) = token {
            query.insert(String::from("pagination_token"), token);
        }
        if !folders {
            add_post_fields(&mut query, input.include_authors);
        }
        add_exclusions(&mut query, &input)?;
        let response = self.http.send(user_request(
            "GET",
            &url,
            &call.installation_id,
            query,
            None,
        ));
        if folders {
            let provider: ProviderCollection<BookmarkFolder> =
                decode_read_response(response, scope)?;
            ensure_provider_count(provider.data.len(), input.max_results as usize)?;
            let result_count = provider.data.len();
            return Ok(PricedToolSuccess {
                output: ToolSuccess::GetUserFeed(GetUserFeedOutput {
                    posts: Vec::new(),
                    authors: Vec::new(),
                    bookmark_folders: provider.data,
                    pagination_token: clean_token(provider.meta.next_token),
                    result_count,
                }),
                usage: metered(&[]),
            });
        }
        let provider: ProviderCollection<CompactPost> = decode_read_response(response, scope)?;
        ensure_provider_count(provider.data.len(), input.max_results as usize)?;
        let authors = if input.include_authors {
            provider.includes.users
        } else {
            Vec::new()
        };
        let result_count = provider.data.len();
        ensure_provider_count(authors.len(), result_count)?;
        let usage = metered(&[(POST_READ, result_count), (USER_READ, authors.len())]);
        Ok(PricedToolSuccess {
            output: ToolSuccess::GetUserFeed(GetUserFeedOutput {
                posts: provider.data,
                authors,
                bookmark_folders: Vec::new(),
                pagination_token: clean_token(provider.meta.next_token),
                result_count,
            }),
            usage,
        })
    }

    pub(super) fn get_post_engagements(
        &self,
        call: AppToolCall,
    ) -> Result<PricedToolSuccess, ToolError> {
        let input: GetPostEngagementsInput =
            decode_input(call.input, InvalidInputReason::EngagementSelector)?;
        validate_decimal_id(&input.post_id, InvalidInputReason::PostIds)?;
        validate_page(input.max_results)?;
        let token = normalized_token(input.pagination_token)?;
        let (suffix, scope, post_result) = match input.view {
            EngagementView::Quotes => ("quote_tweets", "tweet.read", true),
            EngagementView::Reposts => ("retweets", "tweet.read", true),
            EngagementView::LikingUsers => ("liking_users", "like.read", false),
            EngagementView::RepostingUsers => ("retweeted_by", "tweet.read", false),
        };
        if input.include_authors && !post_result {
            return Err(ToolError::InvalidInput(
                InvalidInputReason::EngagementSelector,
            ));
        }
        let url = format!("{API_URL}/tweets/{}/{suffix}", input.post_id);
        let mut query =
            BTreeMap::from([(String::from("max_results"), input.max_results.to_string())]);
        if let Some(token) = token {
            query.insert(String::from("pagination_token"), token);
        }
        if post_result {
            add_post_fields(&mut query, input.include_authors);
        } else {
            query.insert(String::from("user.fields"), String::from(USER_FIELDS));
        }
        let response = self.http.send(user_request(
            "GET",
            &url,
            &call.installation_id,
            query,
            None,
        ));
        if post_result {
            return post_engagement_output(
                response,
                scope,
                input.include_authors,
                input.max_results as usize,
            );
        }
        user_engagement_output(response, scope, input.max_results as usize)
    }
}

fn feed_route(input: &GetUserFeedInput) -> Result<(String, &'static str, bool), ToolError> {
    if !matches!(input.feed, UserFeed::BookmarkFolder) && input.folder_id.is_some() {
        return Err(ToolError::InvalidInput(InvalidInputReason::FeedSelector));
    }
    if matches!(input.feed, UserFeed::RepostsOfMe) {
        if input.user_id.is_some() || input.folder_id.is_some() {
            return Err(ToolError::InvalidInput(InvalidInputReason::FeedSelector));
        }
        return Ok((
            format!("{API_URL}/users/reposts_of_me"),
            "timeline.read",
            false,
        ));
    }
    let user_id = input
        .user_id
        .as_deref()
        .ok_or(ToolError::InvalidInput(InvalidInputReason::FeedSelector))?;
    validate_decimal_id(user_id, InvalidInputReason::UserId)?;
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
        UserFeed::BookmarkFolder => {
            let folder = input
                .folder_id
                .as_deref()
                .ok_or(ToolError::InvalidInput(InvalidInputReason::FeedSelector))?;
            validate_decimal_id(folder, InvalidInputReason::FeedSelector)?;
            (
                format!("{API_URL}/users/{user_id}/bookmarks/folders/{folder}"),
                "bookmark.read",
                false,
            )
        }
        UserFeed::BookmarkFolders => (
            format!("{API_URL}/users/{user_id}/bookmarks/folders"),
            "bookmark.read",
            true,
        ),
        UserFeed::RepostsOfMe => {
            return Err(ToolError::InvalidInput(InvalidInputReason::FeedSelector));
        }
    };
    if route.2 && input.include_authors {
        return Err(ToolError::InvalidInput(InvalidInputReason::FeedSelector));
    }
    Ok(route)
}

fn add_post_fields(query: &mut BTreeMap<String, String>, include_authors: bool) {
    query.insert(String::from("tweet.fields"), String::from(POST_FIELDS));
    if include_authors {
        query.insert(String::from("expansions"), String::from("author_id"));
        query.insert(String::from("user.fields"), String::from(USER_FIELDS));
    }
}

fn add_exclusions(
    query: &mut BTreeMap<String, String>,
    input: &GetUserFeedInput,
) -> Result<(), ToolError> {
    if (input.exclude_replies || input.exclude_reposts)
        && !matches!(input.feed, UserFeed::Posts | UserFeed::Mentions)
    {
        return Err(ToolError::InvalidInput(InvalidInputReason::FeedSelector));
    }
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
    Ok(())
}

fn post_engagement_output(
    response: crate::x::host::HostHttpResponse,
    scope: &'static str,
    include_authors: bool,
    maximum: usize,
) -> Result<PricedToolSuccess, ToolError> {
    let provider: ProviderCollection<CompactPost> = decode_read_response(response, scope)?;
    ensure_provider_count(provider.data.len(), maximum)?;
    let authors = if include_authors {
        provider.includes.users
    } else {
        Vec::new()
    };
    let result_count = provider.data.len();
    ensure_provider_count(authors.len(), result_count)?;
    let usage = metered(&[(POST_READ, result_count), (USER_READ, authors.len())]);
    Ok(PricedToolSuccess {
        output: ToolSuccess::GetPostEngagements(GetPostEngagementsOutput {
            posts: provider.data,
            users: Vec::new(),
            authors,
            pagination_token: clean_token(provider.meta.next_token),
            result_count,
        }),
        usage,
    })
}

fn user_engagement_output(
    response: crate::x::host::HostHttpResponse,
    scope: &'static str,
    maximum: usize,
) -> Result<PricedToolSuccess, ToolError> {
    let provider: ProviderCollection<CompactUser> = decode_read_response(response, scope)?;
    ensure_provider_count(provider.data.len(), maximum)?;
    let result_count = provider.data.len();
    Ok(PricedToolSuccess {
        output: ToolSuccess::GetPostEngagements(GetPostEngagementsOutput {
            posts: Vec::new(),
            users: provider.data,
            authors: Vec::new(),
            pagination_token: clean_token(provider.meta.next_token),
            result_count,
        }),
        usage: metered(&[(USER_READ, result_count)]),
    })
}

fn clean_token(token: Option<String>) -> Option<String> {
    token.filter(|value| !value.is_empty())
}
