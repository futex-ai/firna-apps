//! Account lookup, relationship read, and relationship action types.

use serde::{Deserialize, Serialize};

use crate::x::types::common::CompactUser;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum UserLookup {
    Me,
    Ids,
    Usernames,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GetUsersInput {
    pub(crate) lookup: UserLookup,
    pub(crate) ids: Option<Vec<String>>,
    pub(crate) usernames: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SearchUsersInput {
    pub(crate) query: String,
    pub(crate) max_results: u64,
    pub(crate) pagination_token: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Relationship {
    Affiliates,
    Followers,
    Following,
    Blocked,
    Muted,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GetRelationshipsInput {
    pub(crate) user_id: String,
    pub(crate) relationship: Relationship,
    pub(crate) max_results: u64,
    pub(crate) pagination_token: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RelationshipAction {
    Follow,
    Unfollow,
    Mute,
    Unmute,
    DmBlock,
    DmUnblock,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ManageRelationshipInput {
    pub(crate) action: RelationshipAction,
    pub(crate) user_id: String,
    pub(crate) target_user_id: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct UsersOutput {
    pub(crate) users: Vec<CompactUser>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) missing_values: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) pagination_token: Option<String>,
    pub(crate) result_count: usize,
}

#[derive(Debug, Serialize)]
pub(crate) struct ManageRelationshipOutput {
    pub(crate) action: RelationshipAction,
    pub(crate) target_user_id: String,
    pub(crate) applied: bool,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProviderRelationshipActionResponse {
    pub(crate) data: ProviderRelationshipActionData,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProviderRelationshipActionData {
    pub(crate) following: Option<bool>,
    pub(crate) pending_follow: Option<bool>,
    pub(crate) muting: Option<bool>,
    pub(crate) blocked: Option<bool>,
}

#[derive(Debug, Serialize)]
pub(crate) struct TargetUserBody {
    pub(crate) target_user_id: String,
}
