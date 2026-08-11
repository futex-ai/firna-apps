//! Full-archive Post search and Post-count tools.

use std::collections::BTreeMap;

use crate::x::errors::{InvalidInputReason, ToolError};
use crate::x::host::app_request;
use crate::x::response::decode_read_response;
use crate::x::types::common::{AppToolCall, CompactPost, PricedToolSuccess, ProviderCollection};
use crate::x::types::posts::{
    CountGranularity, CountRange, GetPostCountsInput, GetPostCountsOutput,
    ProviderPostCountsResponse, SearchAllPostsInput, SearchPostsOutput,
};
use crate::x::types::success::ToolSuccess;

use super::runner::ConfiguredXToolRunner;
use super::usage::{POST_READ, USER_READ, metered, reported_cost};
use super::validation::{
    decode_input, ensure_provider_count, normalized_token, optional_trimmed_bounded,
    trimmed_bounded, validate_page,
};

const ALL_SEARCH_URL: &str = "https://api.x.com/2/tweets/search/all";
const RECENT_COUNTS_URL: &str = "https://api.x.com/2/tweets/counts/recent";
const ALL_COUNTS_URL: &str = "https://api.x.com/2/tweets/counts/all";
const POST_FIELDS: &str = "author_id,created_at,text";
const USER_FIELDS: &str = "id,name,username,description,created_at,location,url,profile_image_url,protected,verified,verified_type,public_metrics";

impl ConfiguredXToolRunner<'_> {
    pub(super) fn search_all_posts(
        &self,
        call: AppToolCall,
    ) -> Result<PricedToolSuccess, ToolError> {
        let input: SearchAllPostsInput = decode_input(call.input, InvalidInputReason::SearchQuery)?;
        let query_text = trimmed_bounded(input.query, 4_096, InvalidInputReason::SearchQuery)?;
        validate_page(input.max_results)?;
        let token = normalized_token(input.pagination_token)?;
        let start_time =
            optional_trimmed_bounded(input.start_time, 64, InvalidInputReason::TimeRange)?;
        let end_time = optional_trimmed_bounded(input.end_time, 64, InvalidInputReason::TimeRange)?;
        let mut query = post_query(input.include_authors);
        query.insert(String::from("query"), query_text);
        query.insert(String::from("max_results"), input.max_results.to_string());
        insert_optional(&mut query, "pagination_token", token);
        insert_optional(&mut query, "start_time", start_time);
        insert_optional(&mut query, "end_time", end_time);
        let response = self
            .http
            .send(app_request("GET", ALL_SEARCH_URL, query, None));
        let provider: ProviderCollection<CompactPost> =
            decode_read_response(response, "tweet.read")?;
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
            output: ToolSuccess::SearchAllPosts(SearchPostsOutput {
                posts: provider.data,
                authors,
                pagination_token: clean_token(provider.meta.next_token),
                result_count,
            }),
            usage,
        })
    }

    pub(super) fn get_post_counts(
        &self,
        call: AppToolCall,
    ) -> Result<PricedToolSuccess, ToolError> {
        let input: GetPostCountsInput = decode_input(call.input, InvalidInputReason::CountQuery)?;
        let maximum = match input.range {
            CountRange::Recent => 512,
            CountRange::All => 4_096,
        };
        let query_text = trimmed_bounded(input.query, maximum, InvalidInputReason::CountQuery)?;
        let token = normalized_token(input.pagination_token)?;
        if matches!(input.range, CountRange::Recent) && token.is_some() {
            return Err(ToolError::InvalidInput(InvalidInputReason::PaginationToken));
        }
        let start_time =
            optional_trimmed_bounded(input.start_time, 64, InvalidInputReason::TimeRange)?;
        let end_time = optional_trimmed_bounded(input.end_time, 64, InvalidInputReason::TimeRange)?;
        let mut query = BTreeMap::from([
            (String::from("query"), query_text),
            (
                String::from("granularity"),
                granularity(input.granularity).to_owned(),
            ),
        ]);
        insert_optional(&mut query, "pagination_token", token);
        insert_optional(&mut query, "start_time", start_time);
        insert_optional(&mut query, "end_time", end_time);
        let (url, cost) = match input.range {
            CountRange::Recent => (RECENT_COUNTS_URL, 5_000),
            CountRange::All => (ALL_COUNTS_URL, 10_000),
        };
        let response = self.http.send(app_request("GET", url, query, None));
        let provider: ProviderPostCountsResponse = decode_read_response(response, "tweet.read")?;
        let total = provider
            .meta
            .total_tweet_count
            .unwrap_or_else(|| provider.data.iter().map(|bucket| bucket.post_count).sum());
        Ok(PricedToolSuccess {
            output: ToolSuccess::GetPostCounts(GetPostCountsOutput {
                buckets: provider.data,
                total_post_count: total,
                pagination_token: clean_token(provider.meta.next_token),
            }),
            usage: reported_cost(cost),
        })
    }
}

fn post_query(include_authors: bool) -> BTreeMap<String, String> {
    let mut query = BTreeMap::from([(String::from("tweet.fields"), String::from(POST_FIELDS))]);
    if include_authors {
        query.insert(String::from("expansions"), String::from("author_id"));
        query.insert(String::from("user.fields"), String::from(USER_FIELDS));
    }
    query
}

fn insert_optional(query: &mut BTreeMap<String, String>, name: &str, value: Option<String>) {
    if let Some(value) = value {
        query.insert(name.to_owned(), value);
    }
}

fn clean_token(token: Option<String>) -> Option<String> {
    token.filter(|value| !value.is_empty())
}

fn granularity(value: CountGranularity) -> &'static str {
    match value {
        CountGranularity::Minute => "minute",
        CountGranularity::Hour => "hour",
        CountGranularity::Day => "day",
    }
}
