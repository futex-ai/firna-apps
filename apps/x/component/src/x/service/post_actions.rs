//! One-request Post deletion and interaction actions.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::x::errors::{InvalidInputReason, ToolError};
use crate::x::host::user_request;
use crate::x::response::decode_write_response;
use crate::x::types::common::{AppToolCall, PricedToolSuccess};
use crate::x::types::post_actions::{
    BookmarkPostBody, HiddenPostBody, ManagePostInput, ManagePostOutput, PostAction, PostIdBody,
    ProviderPostActionResponse,
};
use crate::x::types::success::ToolSuccess;

use super::runner::ConfiguredXToolRunner;
use super::usage::reported_cost;
use super::validation::{decode_input, validate_decimal_id};

const API_URL: &str = "https://api.x.com/2";

impl ConfiguredXToolRunner<'_> {
    pub(super) fn manage_post(&self, call: AppToolCall) -> Result<PricedToolSuccess, ToolError> {
        let input: ManagePostInput = decode_input(call.input, InvalidInputReason::PostAction)?;
        validate_decimal_id(&input.post_id, InvalidInputReason::PostAction)?;
        validate_post_action_shape(&input)?;
        let request = post_action_request(&input)?;
        let response = self.http.send(user_request(
            request.method,
            &request.url,
            &call.installation_id,
            BTreeMap::new(),
            request.body,
        ));
        let provider: ProviderPostActionResponse = decode_write_response(response, request.scope)?;
        if confirmation(&provider, input.action) != Some(expected_state(input.action)) {
            return Err(ToolError::WriteOutcomeUnknown);
        }
        Ok(PricedToolSuccess {
            output: ToolSuccess::ManagePost(ManagePostOutput {
                action: input.action,
                post_id: input.post_id,
                applied: true,
            }),
            usage: reported_cost(request.cost),
        })
    }
}

struct PostActionRequest {
    method: &'static str,
    url: String,
    body: Option<serde_json::Value>,
    scope: &'static str,
    cost: u64,
}

fn validate_post_action_shape(input: &ManagePostInput) -> Result<(), ToolError> {
    let requires_user = matches!(
        input.action,
        PostAction::Repost
            | PostAction::Unrepost
            | PostAction::Like
            | PostAction::Unlike
            | PostAction::Bookmark
            | PostAction::Unbookmark
    );
    if requires_user {
        let user = input
            .user_id
            .as_deref()
            .ok_or(ToolError::InvalidInput(InvalidInputReason::PostAction))?;
        validate_decimal_id(user, InvalidInputReason::PostAction)?;
    } else if input.user_id.is_some() {
        return Err(ToolError::InvalidInput(InvalidInputReason::PostAction));
    }
    if let Some(folder) = input.folder_id.as_deref() {
        if !matches!(input.action, PostAction::Bookmark) {
            return Err(ToolError::InvalidInput(InvalidInputReason::PostAction));
        }
        validate_decimal_id(folder, InvalidInputReason::PostAction)?;
    }
    Ok(())
}

fn post_action_request(input: &ManagePostInput) -> Result<PostActionRequest, ToolError> {
    let post = input.post_id.as_str();
    let user = input.user_id.as_deref().unwrap_or_default();
    let request = match input.action {
        PostAction::Delete => request(
            "DELETE",
            format!("{API_URL}/tweets/{post}"),
            None,
            "tweet.write",
            5_000,
        ),
        PostAction::Repost => request(
            "POST",
            format!("{API_URL}/users/{user}/retweets"),
            body(PostIdBody {
                tweet_id: post.to_owned(),
            })?,
            "tweet.write",
            15_000,
        ),
        PostAction::Unrepost => request(
            "DELETE",
            format!("{API_URL}/users/{user}/retweets/{post}"),
            None,
            "tweet.write",
            10_000,
        ),
        PostAction::Like => request(
            "POST",
            format!("{API_URL}/users/{user}/likes"),
            body(PostIdBody {
                tweet_id: post.to_owned(),
            })?,
            "like.write",
            15_000,
        ),
        PostAction::Unlike => request(
            "DELETE",
            format!("{API_URL}/users/{user}/likes/{post}"),
            None,
            "like.write",
            10_000,
        ),
        PostAction::Bookmark => request(
            "POST",
            format!("{API_URL}/users/{user}/bookmarks"),
            body(BookmarkPostBody {
                tweet_id: post.to_owned(),
                folder_id: input.folder_id.clone(),
            })?,
            "bookmark.write",
            5_000,
        ),
        PostAction::Unbookmark => request(
            "DELETE",
            format!("{API_URL}/users/{user}/bookmarks/{post}"),
            None,
            "bookmark.write",
            5_000,
        ),
        PostAction::HideReply => request(
            "PUT",
            format!("{API_URL}/tweets/{post}/hidden"),
            body(HiddenPostBody { hidden: true })?,
            "tweet.moderate.write",
            10_000,
        ),
        PostAction::UnhideReply => request(
            "PUT",
            format!("{API_URL}/tweets/{post}/hidden"),
            body(HiddenPostBody { hidden: false })?,
            "tweet.moderate.write",
            10_000,
        ),
    };
    Ok(request)
}

fn request(
    method: &'static str,
    url: String,
    body: Option<serde_json::Value>,
    scope: &'static str,
    cost: u64,
) -> PostActionRequest {
    PostActionRequest {
        method,
        url,
        body,
        scope,
        cost,
    }
}

fn body(value: impl Serialize) -> Result<Option<serde_json::Value>, ToolError> {
    match serde_json::to_value(value) {
        Ok(value) => Ok(Some(value)),
        Err(_) => Err(ToolError::ProviderResponseInvalid),
    }
}

fn confirmation(response: &ProviderPostActionResponse, action: PostAction) -> Option<bool> {
    match action {
        PostAction::Delete => response.data.deleted,
        PostAction::Repost | PostAction::Unrepost => response.data.retweeted,
        PostAction::Like | PostAction::Unlike => response.data.liked,
        PostAction::Bookmark | PostAction::Unbookmark => response.data.bookmarked,
        PostAction::HideReply | PostAction::UnhideReply => response.data.hidden,
    }
}

fn expected_state(action: PostAction) -> bool {
    matches!(
        action,
        PostAction::Repost
            | PostAction::Like
            | PostAction::Bookmark
            | PostAction::HideReply
            | PostAction::Delete
    )
}
