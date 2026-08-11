//! List lookup and List-associated collection reads.

use std::collections::BTreeMap;

use crate::x::errors::{InvalidInputReason, ToolError};
use crate::x::host::user_request;
use crate::x::response::decode_read_response;
use crate::x::types::common::{
    AppToolCall, CompactPost, CompactUser, PricedToolSuccess, ProviderCollection, ProviderSingle,
};
use crate::x::types::discovery::{GetListsInput, ListSummary, ListView, ListsOutput};
use crate::x::types::success::ToolSuccess;

use super::runner::ConfiguredXToolRunner;
use super::usage::{LIST_READ, POST_READ, USER_READ, metered};
use super::validation::{
    decode_input, ensure_provider_count, normalized_token, validate_decimal_id, validate_page,
};

const API_URL: &str = "https://api.x.com/2";
const LIST_FIELDS: &str =
    "id,name,description,owner_id,private,member_count,follower_count,created_at";
const POST_FIELDS: &str = "author_id,created_at,text";
const USER_FIELDS: &str = "id,name,username,description,created_at,location,url,profile_image_url,protected,verified,verified_type,public_metrics";

impl ConfiguredXToolRunner<'_> {
    pub(super) fn get_lists(&self, call: AppToolCall) -> Result<PricedToolSuccess, ToolError> {
        let input: GetListsInput = decode_input(call.input, InvalidInputReason::ListSelector)?;
        if matches!(input.view, ListView::List) {
            return self.get_single_list(call.installation_id, input);
        }
        let pinned = matches!(input.view, ListView::Pinned);
        if pinned && (input.max_results.is_some() || input.pagination_token.is_some()) {
            return Err(ToolError::InvalidInput(InvalidInputReason::ListSelector));
        }
        let max_results = input.max_results.unwrap_or(25);
        validate_page(max_results)?;
        let token = normalized_token(input.pagination_token.clone())?;
        let (url, kind) = list_route(&input)?;
        let mut query = BTreeMap::new();
        if !pinned {
            query.insert(String::from("max_results"), max_results.to_string());
            if let Some(token) = token {
                query.insert(String::from("pagination_token"), token);
            }
        }
        match kind {
            ListResultKind::Lists => {
                query.insert(String::from("list.fields"), String::from(LIST_FIELDS));
            }
            ListResultKind::Posts => add_post_fields(&mut query, input.include_authors),
            ListResultKind::Users => {
                query.insert(String::from("user.fields"), String::from(USER_FIELDS));
            }
        }
        let response = self.http.send(user_request(
            "GET",
            &url,
            &call.installation_id,
            query,
            None,
        ));
        match kind {
            ListResultKind::Lists => list_collection(response, max_results as usize),
            ListResultKind::Posts => {
                post_collection(response, input.include_authors, max_results as usize)
            }
            ListResultKind::Users => user_collection(response, max_results as usize),
        }
    }

    fn get_single_list(
        &self,
        installation_id: String,
        input: GetListsInput,
    ) -> Result<PricedToolSuccess, ToolError> {
        if input.user_id.is_some()
            || input.max_results.is_some()
            || input.pagination_token.is_some()
            || input.include_authors
        {
            return Err(ToolError::InvalidInput(InvalidInputReason::ListSelector));
        }
        let list_id = input
            .list_id
            .ok_or(ToolError::InvalidInput(InvalidInputReason::ListSelector))?;
        validate_decimal_id(&list_id, InvalidInputReason::ListSelector)?;
        let query = BTreeMap::from([(String::from("list.fields"), String::from(LIST_FIELDS))]);
        let url = format!("{API_URL}/lists/{list_id}");
        let response = self
            .http
            .send(user_request("GET", &url, &installation_id, query, None));
        let provider: ProviderSingle<ListSummary> = decode_read_response(response, "list.read")?;
        let list = provider.data.ok_or(ToolError::NotFound)?;
        Ok(PricedToolSuccess {
            output: ToolSuccess::GetLists(ListsOutput {
                lists: vec![list],
                posts: Vec::new(),
                users: Vec::new(),
                authors: Vec::new(),
                pagination_token: None,
                result_count: 1,
            }),
            usage: metered(&[(LIST_READ, 1)]),
        })
    }
}

#[derive(Clone, Copy)]
enum ListResultKind {
    Lists,
    Posts,
    Users,
}

fn list_route(input: &GetListsInput) -> Result<(String, ListResultKind), ToolError> {
    let (id, uses_list) = match input.view {
        ListView::Owned | ListView::Followed | ListView::Memberships | ListView::Pinned => {
            (input.user_id.as_deref(), false)
        }
        ListView::Posts | ListView::Members | ListView::Followers => {
            (input.list_id.as_deref(), true)
        }
        ListView::List => return Err(ToolError::InvalidInput(InvalidInputReason::ListSelector)),
    };
    if uses_list && input.user_id.is_some() || !uses_list && input.list_id.is_some() {
        return Err(ToolError::InvalidInput(InvalidInputReason::ListSelector));
    }
    let id = id.ok_or(ToolError::InvalidInput(InvalidInputReason::ListSelector))?;
    validate_decimal_id(id, InvalidInputReason::ListSelector)?;
    let route = match input.view {
        ListView::Owned => (
            format!("{API_URL}/users/{id}/owned_lists"),
            ListResultKind::Lists,
        ),
        ListView::Followed => (
            format!("{API_URL}/users/{id}/followed_lists"),
            ListResultKind::Lists,
        ),
        ListView::Memberships => (
            format!("{API_URL}/users/{id}/list_memberships"),
            ListResultKind::Lists,
        ),
        ListView::Pinned => (
            format!("{API_URL}/users/{id}/pinned_lists"),
            ListResultKind::Lists,
        ),
        ListView::Posts => (
            format!("{API_URL}/lists/{id}/tweets"),
            ListResultKind::Posts,
        ),
        ListView::Members => (
            format!("{API_URL}/lists/{id}/members"),
            ListResultKind::Users,
        ),
        ListView::Followers => (
            format!("{API_URL}/lists/{id}/followers"),
            ListResultKind::Users,
        ),
        ListView::List => return Err(ToolError::InvalidInput(InvalidInputReason::ListSelector)),
    };
    if input.include_authors && !matches!(input.view, ListView::Posts) {
        return Err(ToolError::InvalidInput(InvalidInputReason::ListSelector));
    }
    Ok(route)
}

fn list_collection(
    response: crate::x::host::HostHttpResponse,
    maximum: usize,
) -> Result<PricedToolSuccess, ToolError> {
    let provider: ProviderCollection<ListSummary> = decode_read_response(response, "list.read")?;
    ensure_provider_count(provider.data.len(), maximum)?;
    let result_count = provider.data.len();
    Ok(success(
        ListsOutput {
            lists: provider.data,
            posts: Vec::new(),
            users: Vec::new(),
            authors: Vec::new(),
            pagination_token: clean_token(provider.meta.next_token),
            result_count,
        },
        LIST_READ,
        result_count,
    ))
}

fn post_collection(
    response: crate::x::host::HostHttpResponse,
    include_authors: bool,
    maximum: usize,
) -> Result<PricedToolSuccess, ToolError> {
    let provider: ProviderCollection<CompactPost> = decode_read_response(response, "list.read")?;
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
        output: ToolSuccess::GetLists(ListsOutput {
            lists: Vec::new(),
            posts: provider.data,
            users: Vec::new(),
            authors,
            pagination_token: clean_token(provider.meta.next_token),
            result_count,
        }),
        usage,
    })
}

fn user_collection(
    response: crate::x::host::HostHttpResponse,
    maximum: usize,
) -> Result<PricedToolSuccess, ToolError> {
    let provider: ProviderCollection<CompactUser> = decode_read_response(response, "list.read")?;
    ensure_provider_count(provider.data.len(), maximum)?;
    let result_count = provider.data.len();
    Ok(success(
        ListsOutput {
            lists: Vec::new(),
            posts: Vec::new(),
            users: provider.data,
            authors: Vec::new(),
            pagination_token: clean_token(provider.meta.next_token),
            result_count,
        },
        USER_READ,
        result_count,
    ))
}

fn success(output: ListsOutput, unit: &'static str, count: usize) -> PricedToolSuccess {
    PricedToolSuccess {
        output: ToolSuccess::GetLists(output),
        usage: metered(&[(unit, count)]),
    }
}

fn add_post_fields(query: &mut BTreeMap<String, String>, include_authors: bool) {
    query.insert(String::from("tweet.fields"), String::from(POST_FIELDS));
    if include_authors {
        query.insert(String::from("expansions"), String::from("author_id"));
        query.insert(String::from("user.fields"), String::from(USER_FIELDS));
    }
}

fn clean_token(token: Option<String>) -> Option<String> {
    token.filter(|value| !value.is_empty())
}
