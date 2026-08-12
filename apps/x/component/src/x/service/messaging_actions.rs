//! Direct Message writes and bookmark-folder creation.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::x::errors::{InvalidInputReason, ToolError};
use crate::x::host::user_request;
use crate::x::response::decode_write_response;
use crate::x::types::common::{AppToolCall, PricedToolSuccess};
use crate::x::types::messaging::{
    CreateBookmarkFolderBody, CreateBookmarkFolderInput, CreateBookmarkFolderOutput,
    CreateGroupDmBody, DmAction, DmAttachmentBody, DmMessageBody, ManageDmInput, ManageDmOutput,
    ProviderBookmarkFolderResponse, ProviderDmActionResponse,
};
use crate::x::types::success::ToolSuccess;

use super::runner::ConfiguredXToolRunner;
use super::usage::reported_cost;
use super::validation::{decode_input, trimmed_bounded, validate_decimal_id, validate_decimal_ids};

const API_URL: &str = "https://api.x.com/2";

impl ConfiguredXToolRunner<'_> {
    pub(super) fn manage_dm(&self, call: AppToolCall) -> Result<PricedToolSuccess, ToolError> {
        let input: ManageDmInput = decode_input(call.input, InvalidInputReason::DmAction)?;
        let request = dm_request(&input)?;
        let response = self.http.send(user_request(
            request.method,
            &request.url,
            &call.installation_id,
            BTreeMap::new(),
            request.body,
        ));
        let provider: ProviderDmActionResponse = decode_write_response(response, "dm.write")?;
        let (conversation_id, event_id) = match input.action {
            DmAction::Delete if provider.data.deleted == Some(true) => (None, input.event_id),
            DmAction::Delete => return Err(ToolError::WriteOutcomeUnknown),
            _ => (
                Some(
                    provider
                        .data
                        .dm_conversation_id
                        .ok_or(ToolError::WriteOutcomeUnknown)?,
                ),
                Some(
                    provider
                        .data
                        .dm_event_id
                        .ok_or(ToolError::WriteOutcomeUnknown)?,
                ),
            ),
        };
        Ok(PricedToolSuccess {
            output: ToolSuccess::ManageDm(ManageDmOutput {
                action: input.action,
                conversation_id,
                event_id,
                applied: true,
            }),
            usage: reported_cost(request.cost),
        })
    }

    pub(super) fn create_bookmark_folder(
        &self,
        call: AppToolCall,
    ) -> Result<PricedToolSuccess, ToolError> {
        let input: CreateBookmarkFolderInput =
            decode_input(call.input, InvalidInputReason::BookmarkFolder)?;
        validate_decimal_id(&input.user_id, InvalidInputReason::BookmarkFolder)?;
        let name = trimmed_bounded(input.name, 25, InvalidInputReason::BookmarkFolder)?;
        let body = encode_body(CreateBookmarkFolderBody { name })?;
        let response = self.http.send(user_request(
            "POST",
            &format!("{API_URL}/users/{}/bookmarks/folders", input.user_id),
            &call.installation_id,
            BTreeMap::new(),
            Some(body),
        ));
        let provider: ProviderBookmarkFolderResponse =
            decode_write_response(response, "bookmark.write")?;
        Ok(PricedToolSuccess {
            output: ToolSuccess::CreateBookmarkFolder(CreateBookmarkFolderOutput {
                folder: provider.data,
            }),
            usage: reported_cost(5_000),
        })
    }
}

struct DmRequest {
    method: &'static str,
    url: String,
    body: Option<serde_json::Value>,
    cost: u64,
}

fn dm_request(input: &ManageDmInput) -> Result<DmRequest, ToolError> {
    match input.action {
        DmAction::SendToParticipant => participant_request(input),
        DmAction::SendToConversation => conversation_request(input),
        DmAction::CreateGroup => group_request(input),
        DmAction::Delete => delete_request(input),
    }
}

fn participant_request(input: &ManageDmInput) -> Result<DmRequest, ToolError> {
    require_none(input, false, true, true, true)?;
    let id = input
        .participant_id
        .as_deref()
        .ok_or(ToolError::InvalidInput(InvalidInputReason::DmAction))?;
    validate_decimal_id(id, InvalidInputReason::DmAction)?;
    send_request(
        format!("{API_URL}/dm_conversations/with/{id}/messages"),
        message_body(input)?,
    )
}

fn conversation_request(input: &ManageDmInput) -> Result<DmRequest, ToolError> {
    require_none(input, true, false, true, true)?;
    let id = trimmed_bounded(
        input.conversation_id.clone().unwrap_or_default(),
        128,
        InvalidInputReason::DmAction,
    )?;
    send_request(
        format!("{API_URL}/dm_conversations/{id}/messages"),
        message_body(input)?,
    )
}

fn group_request(input: &ManageDmInput) -> Result<DmRequest, ToolError> {
    require_none(input, true, true, false, true)?;
    let ids = input
        .participant_ids
        .as_ref()
        .ok_or(ToolError::InvalidInput(InvalidInputReason::DmAction))?;
    if !(2..=10).contains(&ids.len()) {
        return Err(ToolError::InvalidInput(InvalidInputReason::DmAction));
    }
    validate_decimal_ids(ids, 10, InvalidInputReason::DmAction)?;
    let payload = CreateGroupDmBody {
        conversation_type: "Group",
        participant_ids: ids.clone(),
        message: message_body(input)?,
    };
    send_request(format!("{API_URL}/dm_conversations"), payload)
}

fn delete_request(input: &ManageDmInput) -> Result<DmRequest, ToolError> {
    require_none(input, true, true, true, false)?;
    if input.text.is_some() || input.media_id.is_some() {
        return Err(ToolError::InvalidInput(InvalidInputReason::DmAction));
    }
    let id = input
        .event_id
        .as_deref()
        .ok_or(ToolError::InvalidInput(InvalidInputReason::DmAction))?;
    validate_decimal_id(id, InvalidInputReason::DmAction)?;
    Ok(DmRequest {
        method: "DELETE",
        url: format!("{API_URL}/dm_events/{id}"),
        body: None,
        cost: 10_000,
    })
}

fn message_body(input: &ManageDmInput) -> Result<DmMessageBody, ToolError> {
    let text = match input.text.as_ref() {
        Some(value) if !value.trim().is_empty() && value.chars().count() <= 10_000 => {
            Some(value.clone())
        }
        Some(_) => return Err(ToolError::InvalidInput(InvalidInputReason::DmAction)),
        None => None,
    };
    let attachments = match input.media_id.as_ref() {
        Some(id) => {
            validate_decimal_id(id, InvalidInputReason::DmAction)?;
            vec![DmAttachmentBody {
                media_id: id.clone(),
            }]
        }
        None => Vec::new(),
    };
    if text.is_none() && attachments.is_empty() {
        return Err(ToolError::InvalidInput(InvalidInputReason::DmAction));
    }
    Ok(DmMessageBody { text, attachments })
}

fn require_none(
    input: &ManageDmInput,
    participant: bool,
    conversation: bool,
    participants: bool,
    event: bool,
) -> Result<(), ToolError> {
    let absent = input.participant_id.is_none() == participant
        && input.conversation_id.is_none() == conversation
        && input.participant_ids.is_none() == participants
        && input.event_id.is_none() == event;
    if absent {
        Ok(())
    } else {
        Err(ToolError::InvalidInput(InvalidInputReason::DmAction))
    }
}

fn send_request(url: String, payload: impl Serialize) -> Result<DmRequest, ToolError> {
    Ok(DmRequest {
        method: "POST",
        url,
        body: Some(encode_body(payload)?),
        cost: 15_000,
    })
}

fn encode_body(value: impl Serialize) -> Result<serde_json::Value, ToolError> {
    match serde_json::to_value(value) {
        Ok(value) => Ok(value),
        Err(_) => Err(ToolError::ProviderResponseInvalid),
    }
}
