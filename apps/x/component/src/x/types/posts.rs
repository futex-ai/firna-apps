//! Post, search, feed, count, engagement, and action types.

use serde::{Deserialize, Serialize};

use crate::x::types::common::{CompactPost, CompactUser};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GetPostsInput {
    pub(crate) ids: Vec<String>,
    #[serde(default)]
    pub(crate) include_authors: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SearchRecentPostsInput {
    pub(crate) query: String,
    pub(crate) max_results: u64,
    #[serde(default)]
    pub(crate) next_token: Option<String>,
    #[serde(default)]
    pub(crate) include_authors: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SearchAllPostsInput {
    pub(crate) query: String,
    pub(crate) max_results: u64,
    pub(crate) pagination_token: Option<String>,
    pub(crate) start_time: Option<String>,
    pub(crate) end_time: Option<String>,
    #[serde(default)]
    pub(crate) include_authors: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CountRange {
    Recent,
    All,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CountGranularity {
    Minute,
    Hour,
    Day,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GetPostCountsInput {
    pub(crate) range: CountRange,
    pub(crate) query: String,
    pub(crate) granularity: CountGranularity,
    pub(crate) start_time: Option<String>,
    pub(crate) end_time: Option<String>,
    pub(crate) pagination_token: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum UserFeed {
    Posts,
    Mentions,
    Home,
    Liked,
    Bookmarks,
    BookmarkFolder,
    BookmarkFolders,
    RepostsOfMe,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GetUserFeedInput {
    pub(crate) feed: UserFeed,
    pub(crate) user_id: Option<String>,
    pub(crate) folder_id: Option<String>,
    pub(crate) max_results: u64,
    pub(crate) pagination_token: Option<String>,
    #[serde(default)]
    pub(crate) include_authors: bool,
    #[serde(default)]
    pub(crate) exclude_replies: bool,
    #[serde(default)]
    pub(crate) exclude_reposts: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EngagementView {
    Quotes,
    Reposts,
    LikingUsers,
    RepostingUsers,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GetPostEngagementsInput {
    pub(crate) post_id: String,
    pub(crate) view: EngagementView,
    pub(crate) max_results: u64,
    pub(crate) pagination_token: Option<String>,
    #[serde(default)]
    pub(crate) include_authors: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreatePostInput {
    pub(crate) text: String,
    pub(crate) reply_to_post_id: Option<String>,
    pub(crate) quote_post_id: Option<String>,
    pub(crate) edit_post_id: Option<String>,
    pub(crate) poll_options: Option<Vec<String>>,
    pub(crate) poll_duration_minutes: Option<u64>,
    pub(crate) media_ids: Option<Vec<String>>,
    pub(crate) community_id: Option<String>,
    pub(crate) reply_settings: Option<ReplySetting>,
    pub(crate) made_with_ai: Option<bool>,
    pub(crate) paid_partnership: Option<bool>,
    #[serde(default)]
    pub(crate) allow_link: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReplySetting {
    Following,
    MentionedUsers,
    Subscribers,
    Verified,
}

#[derive(Debug, Serialize)]
pub(crate) struct GetPostsOutput {
    pub(crate) posts: Vec<CompactPost>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) authors: Vec<CompactUser>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) missing_ids: Vec<String>,
    pub(crate) result_count: usize,
}

#[derive(Debug, Serialize)]
pub(crate) struct SearchPostsOutput {
    pub(crate) posts: Vec<CompactPost>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) authors: Vec<CompactUser>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) pagination_token: Option<String>,
    pub(crate) result_count: usize,
}

#[derive(Debug, Serialize)]
pub(crate) struct SearchRecentPostsOutput {
    pub(crate) posts: Vec<CompactPost>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) authors: Vec<CompactUser>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) next_token: Option<String>,
    pub(crate) result_count: usize,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct BookmarkFolder {
    pub(crate) id: String,
    pub(crate) name: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct GetUserFeedOutput {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) posts: Vec<CompactPost>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) authors: Vec<CompactUser>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) bookmark_folders: Vec<BookmarkFolder>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) pagination_token: Option<String>,
    pub(crate) result_count: usize,
}

#[derive(Debug, Serialize)]
pub(crate) struct GetPostEngagementsOutput {
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

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct PostCountBucket {
    pub(crate) start: String,
    pub(crate) end: String,
    #[serde(alias = "tweet_count")]
    pub(crate) post_count: u64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProviderPostCountsResponse {
    #[serde(default)]
    pub(crate) data: Vec<PostCountBucket>,
    #[serde(default)]
    pub(crate) meta: crate::x::types::common::ProviderMeta,
}

#[derive(Debug, Serialize)]
pub(crate) struct GetPostCountsOutput {
    pub(crate) buckets: Vec<PostCountBucket>,
    pub(crate) total_post_count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) pagination_token: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct CreatePostOutput {
    pub(crate) post: CompactPost,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProviderCreatePostResponse {
    pub(crate) data: CompactPost,
}
