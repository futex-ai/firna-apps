//! X tool validation, request construction, and response mapping.

use std::collections::{BTreeMap, HashSet};

use serde::Serialize;
use serde_json::Value;

use crate::x::errors::{ErrorEnvelope, InvalidInputReason, ToolError};
use crate::x::host::{XHttpClient, request};
use crate::x::response::{decode_create_response, decode_read_response};
use crate::x::types::{
    AppToolCall, CreatePostBody, CreatePostInput, CreatePostOutput, CreateReplyBody, GetPostsInput,
    GetPostsOutput, PricedToolSuccess, ProviderReadResponse, SearchRecentPostsInput,
    SearchRecentPostsOutput, ToolSuccess, ToolUsageReport, ToolUsageUnit,
};

const POSTS_URL: &str = "https://api.x.com/2/tweets";
const RECENT_SEARCH_URL: &str = "https://api.x.com/2/tweets/search/recent";
const TWEET_FIELDS: &str = "author_id,created_at,text";
const USER_FIELDS: &str = "id,name,username";
const POST_READ_UNIT: &str = "post_read";
const USER_READ_UNIT: &str = "user_read";
const TEXT_CREATE_COST_USD_MICROS: u64 = 15_000;
const LINK_CREATE_COST_USD_MICROS: u64 = 200_000;

pub(crate) fn call_tool(request_json: &str, http: &dyn XHttpClient) -> String {
    let output = match serde_json::from_str::<AppToolCall>(request_json) {
        Ok(call) => ConfiguredXToolRunner { http }.run(call),
        Err(_) => Err(ToolError::InvalidInput(
            InvalidInputReason::MalformedToolCall,
        )),
    };
    encode_output(output)
}

struct ConfiguredXToolRunner<'a> {
    http: &'a dyn XHttpClient,
}

impl ConfiguredXToolRunner<'_> {
    fn run(&self, call: AppToolCall) -> Result<PricedToolSuccess, ToolError> {
        match call.tool_name.as_str() {
            "x_get_posts" => self.get_posts(call),
            "x_search_recent_posts" => self.search_recent_posts(call),
            "x_create_post" => self.create_post(call),
            _ => Err(ToolError::InvalidInput(InvalidInputReason::UnknownTool)),
        }
    }

    fn get_posts(&self, call: AppToolCall) -> Result<PricedToolSuccess, ToolError> {
        let input: GetPostsInput = decode_input(call.input, InvalidInputReason::PostIds)?;
        validate_ids(&input.ids)?;
        let mut query = common_read_query(input.include_authors);
        query.insert(String::from("ids"), input.ids.join(","));
        let response = self.http.send(request(
            "GET",
            POSTS_URL,
            &call.installation_id,
            query,
            None,
        ));
        let provider: ProviderReadResponse = decode_read_response(response)?;
        if provider.data.is_empty() {
            return Err(ToolError::NotFound);
        }
        let returned_ids: HashSet<&str> =
            provider.data.iter().map(|post| post.id.as_str()).collect();
        let missing_ids = input
            .ids
            .into_iter()
            .filter(|id| !returned_ids.contains(id.as_str()))
            .collect();
        let authors = include_authors(provider.includes.users, input.include_authors);
        let result_count = provider.data.len();
        let usage = read_usage(result_count, authors.len());
        Ok(PricedToolSuccess {
            output: ToolSuccess::GetPosts(GetPostsOutput {
                posts: provider.data,
                authors,
                missing_ids,
                result_count,
            }),
            usage,
        })
    }

    fn search_recent_posts(&self, call: AppToolCall) -> Result<PricedToolSuccess, ToolError> {
        let input: SearchRecentPostsInput =
            decode_input(call.input, InvalidInputReason::SearchQuery)?;
        let normalized = normalize_search(input)?;
        let mut query = common_read_query(normalized.include_authors);
        query.insert(String::from("query"), normalized.query);
        query.insert(
            String::from("max_results"),
            normalized.max_results.to_string(),
        );
        if let Some(next_token) = normalized.next_token {
            query.insert(String::from("next_token"), next_token);
        }
        let response = self.http.send(request(
            "GET",
            RECENT_SEARCH_URL,
            &call.installation_id,
            query,
            None,
        ));
        let provider: ProviderReadResponse = decode_read_response(response)?;
        let authors = include_authors(provider.includes.users, normalized.include_authors);
        let result_count = provider.data.len();
        let usage = read_usage(result_count, authors.len());
        Ok(PricedToolSuccess {
            output: ToolSuccess::SearchRecentPosts(SearchRecentPostsOutput {
                posts: provider.data,
                authors,
                next_token: provider.meta.next_token.filter(|token| !token.is_empty()),
                result_count,
            }),
            usage,
        })
    }

    fn create_post(&self, call: AppToolCall) -> Result<PricedToolSuccess, ToolError> {
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

struct NormalizedSearch {
    query: String,
    max_results: u64,
    next_token: Option<String>,
    include_authors: bool,
}

fn normalize_search(input: SearchRecentPostsInput) -> Result<NormalizedSearch, ToolError> {
    let query = input.query.trim().to_owned();
    if query.is_empty() || query.chars().count() > 512 {
        return Err(ToolError::InvalidInput(InvalidInputReason::SearchQuery));
    }
    if !(10..=25).contains(&input.max_results) {
        return Err(ToolError::InvalidInput(InvalidInputReason::SearchPageSize));
    }
    let had_next_token = input.next_token.is_some();
    let next_token = input
        .next_token
        .map(|token| token.trim().to_owned())
        .filter(|token| !token.is_empty());
    if next_token.as_ref().is_some_and(|token| token.len() > 1_024)
        || had_next_token && next_token.is_none()
    {
        return Err(ToolError::InvalidInput(InvalidInputReason::PaginationToken));
    }
    Ok(NormalizedSearch {
        query,
        max_results: input.max_results,
        next_token,
        include_authors: input.include_authors,
    })
}

fn validate_ids(ids: &[String]) -> Result<(), ToolError> {
    let unique: HashSet<&str> = ids.iter().map(String::as_str).collect();
    if !(1..=10).contains(&ids.len())
        || unique.len() != ids.len()
        || ids.iter().any(|id| !valid_post_id(id))
    {
        return Err(ToolError::InvalidInput(InvalidInputReason::PostIds));
    }
    Ok(())
}

fn valid_post_id(id: &str) -> bool {
    (1..=19).contains(&id.len()) && id.bytes().all(|byte| byte.is_ascii_digit())
}

fn validate_post_text(input: &CreatePostInput) -> Result<(), ToolError> {
    if input.text.trim().is_empty() || input.text.chars().count() > 280 {
        return Err(ToolError::InvalidInput(InvalidInputReason::PostText));
    }
    if !input.allow_link && contains_link(&input.text) {
        return Err(ToolError::InvalidInput(
            InvalidInputReason::LinkAcknowledgementRequired,
        ));
    }
    Ok(())
}

fn contains_link(text: &str) -> bool {
    let lowercase = text.to_ascii_lowercase();
    lowercase.contains("http://") || lowercase.contains("https://")
}

fn read_usage(post_count: usize, author_count: usize) -> ToolUsageReport {
    ToolUsageReport::Metered {
        units: vec![
            ToolUsageUnit {
                unit: POST_READ_UNIT,
                quantity: post_count as u64,
            },
            ToolUsageUnit {
                unit: USER_READ_UNIT,
                quantity: author_count as u64,
            },
        ],
    }
}

fn common_read_query(include_authors: bool) -> BTreeMap<String, String> {
    let mut query = BTreeMap::from([(String::from("tweet.fields"), String::from(TWEET_FIELDS))]);
    if include_authors {
        query.insert(String::from("expansions"), String::from("author_id"));
        query.insert(String::from("user.fields"), String::from(USER_FIELDS));
    }
    query
}

fn include_authors<T>(authors: Vec<T>, requested: bool) -> Vec<T> {
    if requested { authors } else { Vec::new() }
}

fn decode_input<T>(input: Value, reason: InvalidInputReason) -> Result<T, ToolError>
where
    T: serde::de::DeserializeOwned,
{
    match serde_json::from_value(input) {
        Ok(input) => Ok(input),
        Err(_) => Err(ToolError::InvalidInput(reason)),
    }
}

#[derive(Serialize)]
#[serde(untagged)]
enum ComponentOutput {
    Success(PricedToolSuccess),
    Error(ErrorEnvelope),
}

fn encode_output(output: Result<PricedToolSuccess, ToolError>) -> String {
    let output = match output {
        Ok(success) => ComponentOutput::Success(success),
        Err(error) => ComponentOutput::Error(error.envelope()),
    };
    serde_json::to_string(&output)
        .unwrap_or_else(|_| String::from("{\"ok\":false,\"error\":\"provider_contract_error\"}"))
}

#[cfg(test)]
#[path = "_tests_/service/mod.rs"]
mod service_tests;
