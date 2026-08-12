//! One-request List creation, mutation, membership, follow, and pin actions.

use std::collections::BTreeMap;

use crate::x::errors::{InvalidInputReason, ToolError};
use crate::x::host::user_request;
use crate::x::response::decode_write_response;
use crate::x::types::common::{AppToolCall, PricedToolSuccess};
use crate::x::types::discovery::ListSummary;
use crate::x::types::list_actions::{
    ListAction, ManageListInput, ManageListOutput, ProviderListActionResponse,
};
use crate::x::types::success::ToolSuccess;

use super::list_action_request::list_action_request;
use super::runner::ConfiguredXToolRunner;
use super::usage::reported_cost;
use super::validation::decode_input;

impl ConfiguredXToolRunner<'_> {
    pub(super) fn manage_list(&self, call: AppToolCall) -> Result<PricedToolSuccess, ToolError> {
        let input: ManageListInput = decode_input(call.input, InvalidInputReason::ListAction)?;
        let request = list_action_request(&input)?;
        let response = self.http.send(user_request(
            request.method,
            &request.url,
            &call.installation_id,
            BTreeMap::new(),
            request.body,
        ));
        let provider: ProviderListActionResponse = decode_write_response(response, "list.write")?;
        if !confirmed(&provider, input.action) {
            return Err(ToolError::WriteOutcomeUnknown);
        }
        let list = if matches!(input.action, ListAction::Create) {
            Some(created_list(&provider)?)
        } else {
            None
        };
        let list_id = list
            .as_ref()
            .map(|value| value.id.clone())
            .or(input.list_id);
        Ok(PricedToolSuccess {
            output: ToolSuccess::ManageList(ManageListOutput {
                action: input.action,
                list,
                list_id,
                target_user_id: input.target_user_id,
                applied: true,
            }),
            usage: reported_cost(request.cost),
        })
    }
}

fn confirmed(response: &ProviderListActionResponse, action: ListAction) -> bool {
    match action {
        ListAction::Create => response.data.id.is_some() && response.data.name.is_some(),
        ListAction::Update => response.data.updated == Some(true),
        ListAction::Delete => response.data.deleted == Some(true),
        ListAction::AddMember => response.data.is_member == Some(true),
        ListAction::RemoveMember => response.data.is_member == Some(false),
        ListAction::Follow => response.data.following == Some(true),
        ListAction::Unfollow => response.data.following == Some(false),
        ListAction::Pin => response.data.pinned == Some(true),
        ListAction::Unpin => response.data.pinned == Some(false),
    }
}

fn created_list(response: &ProviderListActionResponse) -> Result<ListSummary, ToolError> {
    Ok(ListSummary {
        id: response
            .data
            .id
            .clone()
            .ok_or(ToolError::WriteOutcomeUnknown)?,
        name: response
            .data
            .name
            .clone()
            .ok_or(ToolError::WriteOutcomeUnknown)?,
        description: None,
        owner_id: None,
        private: None,
        member_count: None,
        follower_count: None,
        created_at: None,
    })
}
