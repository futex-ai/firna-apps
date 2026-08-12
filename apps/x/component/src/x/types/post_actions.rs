//! Post creation bodies and one-request Post action types.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PostAction {
    Delete,
    Repost,
    Unrepost,
    Like,
    Unlike,
    Bookmark,
    Unbookmark,
    HideReply,
    UnhideReply,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ManagePostInput {
    pub(crate) action: PostAction,
    pub(crate) post_id: String,
    pub(crate) user_id: Option<String>,
    pub(crate) folder_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ManagePostOutput {
    pub(crate) action: PostAction,
    pub(crate) post_id: String,
    pub(crate) applied: bool,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProviderPostActionResponse {
    pub(crate) data: ProviderPostActionData,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProviderPostActionData {
    pub(crate) deleted: Option<bool>,
    pub(crate) retweeted: Option<bool>,
    pub(crate) liked: Option<bool>,
    pub(crate) bookmarked: Option<bool>,
    pub(crate) hidden: Option<bool>,
}

#[derive(Debug, Serialize)]
pub(crate) struct PostIdBody {
    pub(crate) tweet_id: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct BookmarkPostBody {
    pub(crate) tweet_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) folder_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct HiddenPostBody {
    pub(crate) hidden: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct CreatePostBody {
    pub(crate) text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reply: Option<CreateReplyBody>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) quote_tweet_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) edit_options: Option<CreateEditBody>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) poll: Option<CreatePollBody>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) media: Option<CreateMediaBody>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) community_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reply_settings: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) made_with_ai: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) paid_partnership: Option<bool>,
}

#[derive(Debug, Serialize)]
pub(crate) struct CreateReplyBody {
    pub(crate) in_reply_to_tweet_id: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct CreateEditBody {
    pub(crate) previous_post_id: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct CreatePollBody {
    pub(crate) options: Vec<String>,
    pub(crate) duration_minutes: u64,
}

#[derive(Debug, Serialize)]
pub(crate) struct CreateMediaBody {
    pub(crate) media_ids: Vec<String>,
}
