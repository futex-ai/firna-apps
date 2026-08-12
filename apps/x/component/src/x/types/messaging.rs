//! Direct Message and bookmark-folder types.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DmView {
    All,
    Conversation,
    Participant,
    Event,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GetDmsInput {
    pub(crate) view: DmView,
    pub(crate) conversation_id: Option<String>,
    pub(crate) participant_id: Option<String>,
    pub(crate) event_id: Option<String>,
    pub(crate) max_results: Option<u64>,
    pub(crate) pagination_token: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct DmEvent {
    pub(crate) id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) event_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) dm_conversation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) sender_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) participant_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) created_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct DmsOutput {
    pub(crate) events: Vec<DmEvent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) pagination_token: Option<String>,
    pub(crate) result_count: usize,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DmAction {
    SendToParticipant,
    SendToConversation,
    CreateGroup,
    Delete,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ManageDmInput {
    pub(crate) action: DmAction,
    pub(crate) participant_id: Option<String>,
    pub(crate) conversation_id: Option<String>,
    pub(crate) participant_ids: Option<Vec<String>>,
    pub(crate) event_id: Option<String>,
    pub(crate) text: Option<String>,
    pub(crate) media_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ManageDmOutput {
    pub(crate) action: DmAction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) conversation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) event_id: Option<String>,
    pub(crate) applied: bool,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProviderDmActionResponse {
    pub(crate) data: ProviderDmActionData,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProviderDmActionData {
    pub(crate) dm_conversation_id: Option<String>,
    pub(crate) dm_event_id: Option<String>,
    pub(crate) deleted: Option<bool>,
}

#[derive(Debug, Serialize)]
pub(crate) struct DmMessageBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) text: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) attachments: Vec<DmAttachmentBody>,
}

#[derive(Debug, Serialize)]
pub(crate) struct DmAttachmentBody {
    pub(crate) media_id: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct CreateGroupDmBody {
    pub(crate) conversation_type: &'static str,
    pub(crate) participant_ids: Vec<String>,
    pub(crate) message: DmMessageBody,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CreateBookmarkFolderInput {
    pub(crate) user_id: String,
    pub(crate) name: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct CreateBookmarkFolderBody {
    pub(crate) name: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct CreateBookmarkFolderOutput {
    pub(crate) folder: crate::x::types::posts::BookmarkFolder,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProviderBookmarkFolderResponse {
    pub(crate) data: crate::x::types::posts::BookmarkFolder,
}
