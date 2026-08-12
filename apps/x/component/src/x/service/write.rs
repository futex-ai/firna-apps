//! Single-dispatch create-Post tool.

use std::collections::BTreeMap;

use crate::x::errors::{InvalidInputReason, ToolError};
use crate::x::host::user_request;
use crate::x::response::decode_write_response;
use crate::x::types::common::{AppToolCall, PricedToolSuccess, ToolUsageReport};
use crate::x::types::post_actions::{
    CreateEditBody, CreateMediaBody, CreatePollBody, CreatePostBody, CreateReplyBody,
};
use crate::x::types::posts::{
    CreatePostInput, CreatePostOutput, ProviderCreatePostResponse, ReplySetting,
};
use crate::x::types::success::ToolSuccess;

use super::runner::ConfiguredXToolRunner;
use super::validation::{
    contains_link, decode_input, valid_post_id, validate_decimal_ids, validate_post_text,
};

const POSTS_URL: &str = "https://api.x.com/2/tweets";
const TEXT_CREATE_COST_USD_MICROS: u64 = 15_000;
const LINK_CREATE_COST_USD_MICROS: u64 = 200_000;

impl ConfiguredXToolRunner<'_> {
    pub(super) fn create_post(&self, call: AppToolCall) -> Result<PricedToolSuccess, ToolError> {
        let input: CreatePostInput = decode_input(call.input, InvalidInputReason::PostText)?;
        validate_post_text(&input)?;
        let create_cost_usd_micros = if contains_link(&input.text) {
            LINK_CREATE_COST_USD_MICROS
        } else {
            TEXT_CREATE_COST_USD_MICROS
        };
        let body = create_body(input)?;
        let body_json = match serde_json::to_value(body) {
            Ok(body) => body,
            Err(_) => return Err(ToolError::ProviderResponseInvalid),
        };
        let response = self.http.send(user_request(
            "POST",
            POSTS_URL,
            &call.installation_id,
            BTreeMap::new(),
            Some(body_json),
        ));
        let provider: ProviderCreatePostResponse = decode_write_response(response, "tweet.write")?;
        Ok(PricedToolSuccess {
            output: ToolSuccess::CreatePost(CreatePostOutput {
                post: provider.data,
            }),
            usage: ToolUsageReport::ReportedCost {
                cost_usd_micros: create_cost_usd_micros,
            },
        })
    }
}

fn create_body(input: CreatePostInput) -> Result<CreatePostBody, ToolError> {
    let referenced = [
        input.reply_to_post_id.as_ref(),
        input.quote_post_id.as_ref(),
        input.edit_post_id.as_ref(),
    ];
    if referenced.iter().flatten().count() > 1 {
        return Err(ToolError::InvalidInput(InvalidInputReason::PostOptions));
    }
    if referenced.iter().flatten().any(|id| !valid_post_id(id)) {
        return Err(ToolError::InvalidInput(InvalidInputReason::ReplyTarget));
    }
    if input
        .community_id
        .as_ref()
        .is_some_and(|id| !valid_post_id(id))
    {
        return Err(ToolError::InvalidInput(InvalidInputReason::PostOptions));
    }
    let poll = poll_body(&input)?;
    let media = media_body(&input)?;
    if poll.is_some() && (media.is_some() || referenced.iter().flatten().next().is_some()) {
        return Err(ToolError::InvalidInput(InvalidInputReason::Poll));
    }
    let reply = input.reply_to_post_id.map(|post_id| CreateReplyBody {
        in_reply_to_tweet_id: post_id,
    });
    let edit_options = input
        .edit_post_id
        .map(|previous_post_id| CreateEditBody { previous_post_id });
    Ok(CreatePostBody {
        text: input.text,
        reply,
        quote_tweet_id: input.quote_post_id,
        edit_options,
        poll,
        media,
        community_id: input.community_id,
        reply_settings: input.reply_settings.map(reply_setting),
        made_with_ai: input.made_with_ai,
        paid_partnership: input.paid_partnership,
    })
}

fn poll_body(input: &CreatePostInput) -> Result<Option<CreatePollBody>, ToolError> {
    match (&input.poll_options, input.poll_duration_minutes) {
        (None, None) => Ok(None),
        (Some(options), Some(duration))
            if (2..=4).contains(&options.len())
                && (5..=10_080).contains(&duration)
                && options.iter().all(|option| {
                    let count = option.trim().chars().count();
                    (1..=25).contains(&count)
                }) =>
        {
            Ok(Some(CreatePollBody {
                options: options
                    .iter()
                    .map(|option| option.trim().to_owned())
                    .collect(),
                duration_minutes: duration,
            }))
        }
        _ => Err(ToolError::InvalidInput(InvalidInputReason::Poll)),
    }
}

fn media_body(input: &CreatePostInput) -> Result<Option<CreateMediaBody>, ToolError> {
    let Some(ids) = input.media_ids.as_ref() else {
        return Ok(None);
    };
    validate_decimal_ids(ids, 4, InvalidInputReason::PostOptions)?;
    Ok(Some(CreateMediaBody {
        media_ids: ids.clone(),
    }))
}

fn reply_setting(value: ReplySetting) -> &'static str {
    match value {
        ReplySetting::Following => "following",
        ReplySetting::MentionedUsers => "mentionedUsers",
        ReplySetting::Subscribers => "subscribers",
        ReplySetting::Verified => "verified",
    }
}
