//! One-request List action inputs, bodies, and normalized outputs.

use serde::{Deserialize, Serialize};

use crate::x::types::discovery::ListSummary;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ListAction {
    Create,
    Update,
    Delete,
    AddMember,
    RemoveMember,
    Follow,
    Unfollow,
    Pin,
    Unpin,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ManageListInput {
    pub(crate) action: ListAction,
    pub(crate) list_id: Option<String>,
    pub(crate) user_id: Option<String>,
    pub(crate) target_user_id: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) private: Option<bool>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ManageListOutput {
    pub(crate) action: ListAction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) list: Option<ListSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) list_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) target_user_id: Option<String>,
    pub(crate) applied: bool,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProviderListActionResponse {
    pub(crate) data: ProviderListActionData,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProviderListActionData {
    pub(crate) id: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) updated: Option<bool>,
    pub(crate) deleted: Option<bool>,
    pub(crate) is_member: Option<bool>,
    pub(crate) following: Option<bool>,
    pub(crate) pinned: Option<bool>,
}

#[derive(Debug, Serialize)]
pub(crate) struct CreateListBody {
    pub(crate) name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) private: Option<bool>,
}

#[derive(Debug, Serialize)]
pub(crate) struct UpdateListBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) private: Option<bool>,
}

#[derive(Debug, Serialize)]
pub(crate) struct UserIdBody {
    pub(crate) user_id: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct ListIdBody {
    pub(crate) list_id: String,
}
