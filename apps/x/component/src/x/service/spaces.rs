//! Space lookup, search, Post, and buyer reads.

use std::collections::BTreeMap;

use crate::x::errors::{InvalidInputReason, ToolError};
use crate::x::host::user_request;
use crate::x::response::decode_read_response;
use crate::x::types::common::{
    AppToolCall, CompactPost, CompactUser, PricedToolSuccess, ProviderCollection,
};
use crate::x::types::discovery::{
    GetSpacesInput, SpaceState, SpaceSummary, SpaceView, SpacesOutput,
};
use crate::x::types::success::ToolSuccess;

use super::runner::ConfiguredXToolRunner;
use super::usage::{POST_READ, SPACE_READ, USER_READ, metered};
use super::validation::{
    decode_input, ensure_provider_count, normalized_token, trimmed_bounded, validate_decimal_id,
    validate_decimal_ids, validate_page,
};

const API_URL: &str = "https://api.x.com/2";
const SPACE_FIELDS: &str = "id,state,title,creator_id,lang,participant_count,is_ticketed,scheduled_start,started_at,ended_at";
const POST_FIELDS: &str = "author_id,created_at,text";
const USER_FIELDS: &str = "id,name,username,description,created_at,location,url,profile_image_url,protected,verified,verified_type,public_metrics";

impl ConfiguredXToolRunner<'_> {
    pub(super) fn get_spaces(&self, call: AppToolCall) -> Result<PricedToolSuccess, ToolError> {
        let input: GetSpacesInput = decode_input(call.input, InvalidInputReason::SpaceSelector)?;
        let (url, result_kind, query, maximum) = space_request(&input)?;
        let response = self.http.send(user_request(
            "GET",
            &url,
            &call.installation_id,
            query,
            None,
        ));
        match result_kind {
            SpaceResultKind::Spaces => space_output(response, maximum),
            SpaceResultKind::Posts => post_output(response, maximum),
            SpaceResultKind::Users => user_output(response, maximum),
        }
    }
}

#[derive(Clone, Copy)]
enum SpaceResultKind {
    Spaces,
    Posts,
    Users,
}

fn space_request(
    input: &GetSpacesInput,
) -> Result<(String, SpaceResultKind, BTreeMap<String, String>, usize), ToolError> {
    match input.view {
        SpaceView::Ids => space_ids_request(input),
        SpaceView::Creators => creator_request(input),
        SpaceView::Search => search_request(input),
        SpaceView::Posts | SpaceView::Buyers => space_collection_request(input),
    }
}

fn space_ids_request(
    input: &GetSpacesInput,
) -> Result<(String, SpaceResultKind, BTreeMap<String, String>, usize), ToolError> {
    require_only(input, true, false, false, false, false)?;
    let ids = input
        .ids
        .as_ref()
        .ok_or(ToolError::InvalidInput(InvalidInputReason::SpaceSelector))?;
    validate_decimal_ids(ids, 10, InvalidInputReason::SpaceSelector)?;
    let query = BTreeMap::from([
        (String::from("ids"), ids.join(",")),
        (String::from("space.fields"), String::from(SPACE_FIELDS)),
    ]);
    Ok((
        format!("{API_URL}/spaces"),
        SpaceResultKind::Spaces,
        query,
        ids.len(),
    ))
}

fn creator_request(
    input: &GetSpacesInput,
) -> Result<(String, SpaceResultKind, BTreeMap<String, String>, usize), ToolError> {
    require_only(input, false, true, false, false, false)?;
    let ids = input
        .creator_ids
        .as_ref()
        .ok_or(ToolError::InvalidInput(InvalidInputReason::SpaceSelector))?;
    validate_decimal_ids(ids, 10, InvalidInputReason::SpaceSelector)?;
    let query = BTreeMap::from([
        (String::from("user_ids"), ids.join(",")),
        (String::from("space.fields"), String::from(SPACE_FIELDS)),
    ]);
    Ok((
        format!("{API_URL}/spaces/by/creator_ids"),
        SpaceResultKind::Spaces,
        query,
        10,
    ))
}

fn search_request(
    input: &GetSpacesInput,
) -> Result<(String, SpaceResultKind, BTreeMap<String, String>, usize), ToolError> {
    require_only(input, false, false, true, false, true)?;
    let query_text = trimmed_bounded(
        input.query.clone().unwrap_or_default(),
        2_048,
        InvalidInputReason::SpaceSelector,
    )?;
    let max_results = input.max_results.unwrap_or(25);
    validate_page(max_results)?;
    let mut query = BTreeMap::from([
        (String::from("query"), query_text),
        (String::from("max_results"), max_results.to_string()),
        (String::from("space.fields"), String::from(SPACE_FIELDS)),
    ]);
    if let Some(state) = input.state {
        let value = match state {
            SpaceState::Live => "live",
            SpaceState::Scheduled => "scheduled",
            SpaceState::All => "all",
        };
        query.insert(String::from("state"), value.to_owned());
    }
    Ok((
        format!("{API_URL}/spaces/search"),
        SpaceResultKind::Spaces,
        query,
        max_results as usize,
    ))
}

fn space_collection_request(
    input: &GetSpacesInput,
) -> Result<(String, SpaceResultKind, BTreeMap<String, String>, usize), ToolError> {
    require_only(input, false, false, false, true, true)?;
    let id = input
        .space_id
        .as_deref()
        .ok_or(ToolError::InvalidInput(InvalidInputReason::SpaceSelector))?;
    validate_decimal_id(id, InvalidInputReason::SpaceSelector)?;
    let max_results = input.max_results.unwrap_or(25);
    validate_page(max_results)?;
    let token = normalized_token(input.pagination_token.clone())?;
    let (suffix, kind) = match input.view {
        SpaceView::Posts => ("tweets", SpaceResultKind::Posts),
        SpaceView::Buyers => ("buyers", SpaceResultKind::Users),
        _ => return Err(ToolError::InvalidInput(InvalidInputReason::SpaceSelector)),
    };
    let mut query = BTreeMap::from([(String::from("max_results"), max_results.to_string())]);
    if let Some(token) = token {
        query.insert(String::from("pagination_token"), token);
    }
    match kind {
        SpaceResultKind::Posts => {
            query.insert(String::from("tweet.fields"), String::from(POST_FIELDS));
        }
        SpaceResultKind::Users => {
            query.insert(String::from("user.fields"), String::from(USER_FIELDS));
        }
        SpaceResultKind::Spaces => {}
    }
    Ok((
        format!("{API_URL}/spaces/{id}/{suffix}"),
        kind,
        query,
        max_results as usize,
    ))
}

fn require_only(
    input: &GetSpacesInput,
    ids: bool,
    creators: bool,
    query: bool,
    space: bool,
    allow_max: bool,
) -> Result<(), ToolError> {
    let valid = input.ids.is_some() == ids
        && input.creator_ids.is_some() == creators
        && input.query.is_some() == query
        && input.space_id.is_some() == space
        && (allow_max || input.max_results.is_none())
        && (space || input.pagination_token.is_none())
        && (query || input.state.is_none());
    if valid {
        Ok(())
    } else {
        Err(ToolError::InvalidInput(InvalidInputReason::SpaceSelector))
    }
}

fn space_output(
    response: crate::x::host::HostHttpResponse,
    maximum: usize,
) -> Result<PricedToolSuccess, ToolError> {
    let provider: ProviderCollection<SpaceSummary> = decode_read_response(response, "space.read")?;
    ensure_provider_count(provider.data.len(), maximum)?;
    let count = provider.data.len();
    Ok(success(
        SpacesOutput {
            spaces: provider.data,
            posts: Vec::new(),
            users: Vec::new(),
            pagination_token: clean_token(provider.meta.next_token),
            result_count: count,
        },
        SPACE_READ,
        count,
    ))
}

fn post_output(
    response: crate::x::host::HostHttpResponse,
    maximum: usize,
) -> Result<PricedToolSuccess, ToolError> {
    let provider: ProviderCollection<CompactPost> = decode_read_response(response, "space.read")?;
    ensure_provider_count(provider.data.len(), maximum)?;
    let count = provider.data.len();
    Ok(success(
        SpacesOutput {
            spaces: Vec::new(),
            posts: provider.data,
            users: Vec::new(),
            pagination_token: clean_token(provider.meta.next_token),
            result_count: count,
        },
        POST_READ,
        count,
    ))
}

fn user_output(
    response: crate::x::host::HostHttpResponse,
    maximum: usize,
) -> Result<PricedToolSuccess, ToolError> {
    let provider: ProviderCollection<CompactUser> = decode_read_response(response, "space.read")?;
    ensure_provider_count(provider.data.len(), maximum)?;
    let count = provider.data.len();
    Ok(success(
        SpacesOutput {
            spaces: Vec::new(),
            posts: Vec::new(),
            users: provider.data,
            pagination_token: clean_token(provider.meta.next_token),
            result_count: count,
        },
        USER_READ,
        count,
    ))
}

fn success(output: SpacesOutput, unit: &'static str, count: usize) -> PricedToolSuccess {
    PricedToolSuccess {
        output: ToolSuccess::GetSpaces(output),
        usage: metered(&[(unit, count)]),
    }
}

fn clean_token(token: Option<String>) -> Option<String> {
    token.filter(|value| !value.is_empty())
}
