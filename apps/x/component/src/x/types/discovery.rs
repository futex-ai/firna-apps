//! List, Space, Community, trend, and media types.

use serde::{Deserialize, Deserializer, Serialize};

use crate::x::types::common::{CompactPost, CompactUser};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ListView {
    List,
    Owned,
    Followed,
    Memberships,
    Pinned,
    Posts,
    Members,
    Followers,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GetListsInput {
    pub(crate) view: ListView,
    pub(crate) list_id: Option<String>,
    pub(crate) user_id: Option<String>,
    pub(crate) max_results: Option<u64>,
    pub(crate) pagination_token: Option<String>,
    #[serde(default)]
    pub(crate) include_authors: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ListSummary {
    pub(crate) id: String,
    pub(crate) name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) owner_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) private: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) member_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) follower_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) created_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ListsOutput {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) lists: Vec<ListSummary>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) posts: Vec<CompactPost>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) users: Vec<CompactUser>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) authors: Vec<CompactUser>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) pagination_token: Option<String>,
    pub(crate) result_count: usize,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SpaceView {
    Ids,
    Creators,
    Search,
    Posts,
    Buyers,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GetSpacesInput {
    pub(crate) view: SpaceView,
    pub(crate) ids: Option<Vec<String>>,
    pub(crate) creator_ids: Option<Vec<String>>,
    pub(crate) query: Option<String>,
    pub(crate) state: Option<SpaceState>,
    pub(crate) space_id: Option<String>,
    pub(crate) max_results: Option<u64>,
    pub(crate) pagination_token: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SpaceState {
    Live,
    Scheduled,
    All,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct SpaceSummary {
    pub(crate) id: String,
    pub(crate) state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) creator_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) lang: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) participant_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) is_ticketed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) scheduled_start: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) ended_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct SpacesOutput {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) spaces: Vec<SpaceSummary>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) posts: Vec<CompactPost>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) users: Vec<CompactUser>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) pagination_token: Option<String>,
    pub(crate) result_count: usize,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CommunityView {
    Ids,
    Search,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GetCommunitiesInput {
    pub(crate) view: CommunityView,
    pub(crate) ids: Option<Vec<String>>,
    pub(crate) query: Option<String>,
    pub(crate) max_results: Option<u64>,
    pub(crate) pagination_token: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct CommunitySummary {
    pub(crate) id: String,
    pub(crate) name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) access: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) join_policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) member_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) created_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct CommunitiesOutput {
    pub(crate) communities: Vec<CommunitySummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) pagination_token: Option<String>,
    pub(crate) result_count: usize,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TrendView {
    Personalized,
    Location,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GetTrendsInput {
    pub(crate) view: TrendView,
    pub(crate) woeid: Option<u32>,
    pub(crate) max_trends: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct TrendSummary {
    pub(crate) trend_name: String,
    #[serde(
        alias = "tweet_count",
        default,
        deserialize_with = "deserialize_optional_count",
        skip_serializing_if = "Option::is_none"
    )]
    pub(crate) post_count: Option<u64>,
}

#[derive(Debug, Serialize)]
pub(crate) struct TrendsOutput {
    pub(crate) trends: Vec<TrendSummary>,
    pub(crate) result_count: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GetMediaInput {
    pub(crate) media_keys: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct MediaSummary {
    pub(crate) media_key: String,
    #[serde(rename = "type")]
    pub(crate) media_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) height: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) width: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) preview_image_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) public_metrics: Option<MediaPublicMetrics>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct MediaPublicMetrics {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) view_count: Option<u64>,
}

#[derive(Debug, Serialize)]
pub(crate) struct MediaOutput {
    pub(crate) media: Vec<MediaSummary>,
    pub(crate) result_count: usize,
}

fn deserialize_optional_count<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Count {
        Number(u64),
        String(String),
    }

    match Option::<Count>::deserialize(deserializer)? {
        Some(Count::Number(value)) => Ok(Some(value)),
        Some(Count::String(value)) => match value.parse() {
            Ok(value) => Ok(Some(value)),
            Err(error) => Err(serde::de::Error::custom(error)),
        },
        None => Ok(None),
    }
}
