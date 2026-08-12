//! Community, trend, and media-metadata read tools.

use std::collections::{BTreeMap, HashSet};

use crate::x::errors::{InvalidInputReason, ToolError};
use crate::x::host::{app_request, user_request};
use crate::x::response::decode_read_response;
use crate::x::types::common::{AppToolCall, PricedToolSuccess, ProviderCollection, ProviderSingle};
use crate::x::types::discovery::{
    CommunitiesOutput, CommunitySummary, CommunityView, GetCommunitiesInput, GetMediaInput,
    GetTrendsInput, MediaOutput, MediaSummary, TrendSummary, TrendView, TrendsOutput,
};
use crate::x::types::success::ToolSuccess;

use super::runner::ConfiguredXToolRunner;
use super::usage::{COMMUNITY_READ, MEDIA_READ, TREND_READ, metered};
use super::validation::{
    decode_input, ensure_provider_count, normalized_token, trimmed_bounded, valid_media_key,
    validate_decimal_id, validate_page,
};

const API_URL: &str = "https://api.x.com/2";
const COMMUNITY_FIELDS: &str = "id,name,description,access,join_policy,member_count,created_at";
const MEDIA_FIELDS: &str =
    "media_key,type,duration_ms,height,width,preview_image_url,public_metrics";

impl ConfiguredXToolRunner<'_> {
    pub(super) fn get_communities(
        &self,
        call: AppToolCall,
    ) -> Result<PricedToolSuccess, ToolError> {
        let input: GetCommunitiesInput =
            decode_input(call.input, InvalidInputReason::CommunitySelector)?;
        match input.view {
            CommunityView::Ids => self.get_community_by_id(call.installation_id, input),
            CommunityView::Search => self.search_communities(call.installation_id, input),
        }
    }

    pub(super) fn get_trends(&self, call: AppToolCall) -> Result<PricedToolSuccess, ToolError> {
        let input: GetTrendsInput = decode_input(call.input, InvalidInputReason::TrendSelector)?;
        let (response, maximum) = match input.view {
            TrendView::Personalized => {
                if input.woeid.is_some() || input.max_trends.is_some() {
                    return Err(ToolError::InvalidInput(InvalidInputReason::TrendSelector));
                }
                (
                    self.http.send(user_request(
                        "GET",
                        &format!("{API_URL}/users/personalized_trends"),
                        &call.installation_id,
                        BTreeMap::new(),
                        None,
                    )),
                    50,
                )
            }
            TrendView::Location => {
                let woeid = input
                    .woeid
                    .filter(|value| *value > 0)
                    .ok_or(ToolError::InvalidInput(InvalidInputReason::TrendSelector))?;
                let max_trends = input.max_trends.unwrap_or(25);
                if !(1..=25).contains(&max_trends) {
                    return Err(ToolError::InvalidInput(InvalidInputReason::TrendSelector));
                }
                let query = BTreeMap::from([(String::from("max_trends"), max_trends.to_string())]);
                (
                    self.http.send(app_request(
                        "GET",
                        &format!("{API_URL}/trends/by/woeid/{woeid}"),
                        query,
                        None,
                    )),
                    max_trends as usize,
                )
            }
        };
        let provider: ProviderCollection<TrendSummary> =
            decode_read_response(response, "users.read")?;
        ensure_provider_count(provider.data.len(), maximum)?;
        let result_count = provider.data.len();
        Ok(PricedToolSuccess {
            output: ToolSuccess::GetTrends(TrendsOutput {
                trends: provider.data,
                result_count,
            }),
            usage: metered(&[(TREND_READ, result_count)]),
        })
    }

    pub(super) fn get_media(&self, call: AppToolCall) -> Result<PricedToolSuccess, ToolError> {
        let input: GetMediaInput = decode_input(call.input, InvalidInputReason::MediaKeys)?;
        let unique: HashSet<&str> = input.media_keys.iter().map(String::as_str).collect();
        if !(1..=10).contains(&input.media_keys.len())
            || unique.len() != input.media_keys.len()
            || input.media_keys.iter().any(|key| !valid_media_key(key))
        {
            return Err(ToolError::InvalidInput(InvalidInputReason::MediaKeys));
        }
        let query = BTreeMap::from([
            (String::from("media_keys"), input.media_keys.join(",")),
            (String::from("media.fields"), String::from(MEDIA_FIELDS)),
        ]);
        let response = self.http.send(user_request(
            "GET",
            &format!("{API_URL}/media"),
            &call.installation_id,
            query,
            None,
        ));
        let provider: ProviderCollection<MediaSummary> =
            decode_read_response(response, "tweet.read")?;
        ensure_provider_count(provider.data.len(), input.media_keys.len())?;
        let returned: HashSet<&str> = provider
            .data
            .iter()
            .map(|media| media.media_key.as_str())
            .collect();
        if returned.len() != provider.data.len() || returned.iter().any(|key| !unique.contains(key))
        {
            return Err(ToolError::ProviderResponseInvalid);
        }
        let result_count = provider.data.len();
        Ok(PricedToolSuccess {
            output: ToolSuccess::GetMedia(MediaOutput {
                media: provider.data,
                result_count,
            }),
            usage: metered(&[(MEDIA_READ, result_count)]),
        })
    }

    fn get_community_by_id(
        &self,
        installation_id: String,
        input: GetCommunitiesInput,
    ) -> Result<PricedToolSuccess, ToolError> {
        if input.query.is_some()
            || input.max_results.is_some()
            || input.pagination_token.is_some()
            || input.ids.as_ref().is_none_or(|ids| ids.len() != 1)
        {
            return Err(ToolError::InvalidInput(
                InvalidInputReason::CommunitySelector,
            ));
        }
        let id =
            input
                .ids
                .and_then(|ids| ids.into_iter().next())
                .ok_or(ToolError::InvalidInput(
                    InvalidInputReason::CommunitySelector,
                ))?;
        validate_decimal_id(&id, InvalidInputReason::CommunitySelector)?;
        let query = BTreeMap::from([(
            String::from("community.fields"),
            String::from(COMMUNITY_FIELDS),
        )]);
        let response = self.http.send(user_request(
            "GET",
            &format!("{API_URL}/communities/{id}"),
            &installation_id,
            query,
            None,
        ));
        let provider: ProviderSingle<CommunitySummary> =
            decode_read_response(response, "list.read")?;
        let community = provider.data.ok_or(ToolError::NotFound)?;
        Ok(community_success(vec![community], None))
    }

    fn search_communities(
        &self,
        installation_id: String,
        input: GetCommunitiesInput,
    ) -> Result<PricedToolSuccess, ToolError> {
        if input.ids.is_some() {
            return Err(ToolError::InvalidInput(
                InvalidInputReason::CommunitySelector,
            ));
        }
        let query_text = trimmed_bounded(
            input.query.unwrap_or_default(),
            4_096,
            InvalidInputReason::CommunitySelector,
        )?;
        let max_results = input.max_results.unwrap_or(25);
        validate_page(max_results)?;
        let token = normalized_token(input.pagination_token)?;
        let mut query = BTreeMap::from([
            (String::from("query"), query_text),
            (String::from("max_results"), max_results.to_string()),
            (
                String::from("community.fields"),
                String::from(COMMUNITY_FIELDS),
            ),
        ]);
        if let Some(token) = token {
            query.insert(String::from("pagination_token"), token);
        }
        let response = self.http.send(user_request(
            "GET",
            &format!("{API_URL}/communities/search"),
            &installation_id,
            query,
            None,
        ));
        let provider: ProviderCollection<CommunitySummary> =
            decode_read_response(response, "users.read")?;
        ensure_provider_count(provider.data.len(), max_results as usize)?;
        Ok(community_success(
            provider.data,
            clean_token(provider.meta.next_token),
        ))
    }
}

fn community_success(
    communities: Vec<CommunitySummary>,
    pagination_token: Option<String>,
) -> PricedToolSuccess {
    let result_count = communities.len();
    PricedToolSuccess {
        output: ToolSuccess::GetCommunities(CommunitiesOutput {
            communities,
            pagination_token,
            result_count,
        }),
        usage: metered(&[(COMMUNITY_READ, result_count)]),
    }
}

fn clean_token(token: Option<String>) -> Option<String> {
    token.filter(|value| !value.is_empty())
}
