//! Single-dispatch create-Post tool.

use std::collections::BTreeMap;

use crate::x::errors::{InvalidInputReason, ToolError};
use crate::x::host::request;
use crate::x::response::decode_create_response;
use crate::x::types::{
    AppToolCall, CreatePostBody, CreatePostInput, CreatePostOutput, CreateReplyBody,
    PricedToolSuccess, ToolSuccess, ToolUsageReport,
};

use super::runner::ConfiguredXToolRunner;
use super::validation::{contains_link, decode_input, valid_post_id, validate_post_text};

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
        let reply = match input.reply_to_post_id {
            Some(post_id) if valid_post_id(&post_id) => Some(CreateReplyBody {
                in_reply_to_tweet_id: post_id,
            }),
            Some(_) => return Err(ToolError::InvalidInput(InvalidInputReason::ReplyTarget)),
            None => None,
        };
        let body = CreatePostBody {
            text: input.text,
            reply,
        };
        let body_json = match serde_json::to_value(body) {
            Ok(body) => body,
            Err(_) => return Err(ToolError::ProviderResponseInvalid),
        };
        let response = self.http.send(request(
            "POST",
            POSTS_URL,
            &call.installation_id,
            BTreeMap::new(),
            Some(body_json),
        ));
        let provider = decode_create_response(response)?;
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
