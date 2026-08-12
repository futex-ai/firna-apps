//! Bounded compact Post lookup and recent-search tools.

use std::collections::{BTreeMap, HashSet};

use crate::x::errors::{InvalidInputReason, ToolError};
use crate::x::host::user_request;
use crate::x::response::decode_read_response;
use crate::x::types::common::{
    AppToolCall, CompactPost, PricedToolSuccess, ProviderCollection, ToolUsageReport, ToolUsageUnit,
};
use crate::x::types::posts::{
    GetPostsInput, GetPostsOutput, SearchRecentPostsInput, SearchRecentPostsOutput,
};
use crate::x::types::success::ToolSuccess;

use super::runner::ConfiguredXToolRunner;
use super::validation::{decode_input, ensure_provider_count, normalize_search, validate_ids};

const POSTS_URL: &str = "https://api.x.com/2/tweets";
const RECENT_SEARCH_URL: &str = "https://api.x.com/2/tweets/search/recent";
const TWEET_FIELDS: &str = "author_id,created_at,text";
const USER_FIELDS: &str = "id,name,username";
const POST_READ_UNIT: &str = "post_read";
const USER_READ_UNIT: &str = "user_read";

impl ConfiguredXToolRunner<'_> {
    pub(super) fn get_posts(&self, call: AppToolCall) -> Result<PricedToolSuccess, ToolError> {
        let input: GetPostsInput = decode_input(call.input, InvalidInputReason::PostIds)?;
        validate_ids(&input.ids)?;
        let mut query = common_read_query(input.include_authors);
        query.insert(String::from("ids"), input.ids.join(","));
        let response = self.http.send(user_request(
            "GET",
            POSTS_URL,
            &call.installation_id,
            query,
            None,
        ));
        let provider: ProviderCollection<CompactPost> =
            decode_read_response(response, "tweet.read")?;
        ensure_provider_count(provider.data.len(), input.ids.len())?;
        if provider.data.is_empty() {
            return Err(ToolError::NotFound);
        }
        let returned_ids: HashSet<&str> =
            provider.data.iter().map(|post| post.id.as_str()).collect();
        let requested_ids: HashSet<&str> = input.ids.iter().map(String::as_str).collect();
        if returned_ids.len() != provider.data.len()
            || returned_ids.iter().any(|id| !requested_ids.contains(id))
        {
            return Err(ToolError::ProviderResponseInvalid);
        }
        let missing_ids = input
            .ids
            .into_iter()
            .filter(|id| !returned_ids.contains(id.as_str()))
            .collect();
        let authors = include_authors(provider.includes.users, input.include_authors);
        let result_count = provider.data.len();
        ensure_provider_count(authors.len(), result_count)?;
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

    pub(super) fn search_recent_posts(
        &self,
        call: AppToolCall,
    ) -> Result<PricedToolSuccess, ToolError> {
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
        let response = self.http.send(user_request(
            "GET",
            RECENT_SEARCH_URL,
            &call.installation_id,
            query,
            None,
        ));
        let provider: ProviderCollection<CompactPost> =
            decode_read_response(response, "tweet.read")?;
        ensure_provider_count(provider.data.len(), normalized.max_results as usize)?;
        let authors = include_authors(provider.includes.users, normalized.include_authors);
        let result_count = provider.data.len();
        ensure_provider_count(authors.len(), result_count)?;
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
