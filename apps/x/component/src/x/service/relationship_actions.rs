//! Follow, mute, and Direct Message blocking actions.

use std::collections::BTreeMap;

use crate::x::errors::{InvalidInputReason, ToolError};
use crate::x::host::user_request;
use crate::x::response::decode_write_response;
use crate::x::types::accounts::{
    ManageRelationshipInput, ManageRelationshipOutput, ProviderRelationshipActionResponse,
    RelationshipAction, TargetUserBody,
};
use crate::x::types::common::{AppToolCall, PricedToolSuccess};
use crate::x::types::success::ToolSuccess;

use super::runner::ConfiguredXToolRunner;
use super::usage::reported_cost;
use super::validation::{decode_input, validate_decimal_id};

const API_URL: &str = "https://api.x.com/2";

impl ConfiguredXToolRunner<'_> {
    pub(super) fn manage_relationship(
        &self,
        call: AppToolCall,
    ) -> Result<PricedToolSuccess, ToolError> {
        let input: ManageRelationshipInput =
            decode_input(call.input, InvalidInputReason::RelationshipAction)?;
        validate_decimal_id(&input.user_id, InvalidInputReason::RelationshipAction)?;
        validate_decimal_id(
            &input.target_user_id,
            InvalidInputReason::RelationshipAction,
        )?;
        let request = relationship_request(&input)?;
        let response = self.http.send(user_request(
            request.method,
            &request.url,
            &call.installation_id,
            BTreeMap::new(),
            request.body,
        ));
        let provider: ProviderRelationshipActionResponse =
            decode_write_response(response, request.scope)?;
        if !confirmed(&provider, input.action) {
            return Err(ToolError::WriteOutcomeUnknown);
        }
        Ok(PricedToolSuccess {
            output: ToolSuccess::ManageRelationship(ManageRelationshipOutput {
                action: input.action,
                target_user_id: input.target_user_id,
                applied: true,
            }),
            usage: reported_cost(request.cost),
        })
    }
}

struct RelationshipRequest {
    method: &'static str,
    url: String,
    body: Option<serde_json::Value>,
    scope: &'static str,
    cost: u64,
}

fn relationship_request(input: &ManageRelationshipInput) -> Result<RelationshipRequest, ToolError> {
    let user = input.user_id.as_str();
    let target = input.target_user_id.as_str();
    let body = || {
        serde_json::to_value(TargetUserBody {
            target_user_id: target.to_owned(),
        })
        .map(Some)
        .or(Err(ToolError::ProviderResponseInvalid))
    };
    let request = match input.action {
        RelationshipAction::Follow => RelationshipRequest {
            method: "POST",
            url: format!("{API_URL}/users/{user}/following"),
            body: body()?,
            scope: "follows.write",
            cost: 15_000,
        },
        RelationshipAction::Unfollow => RelationshipRequest {
            method: "DELETE",
            url: format!("{API_URL}/users/{user}/following/{target}"),
            body: None,
            scope: "follows.write",
            cost: 10_000,
        },
        RelationshipAction::Mute => RelationshipRequest {
            method: "POST",
            url: format!("{API_URL}/users/{user}/muting"),
            body: body()?,
            scope: "mute.write",
            cost: 15_000,
        },
        RelationshipAction::Unmute => RelationshipRequest {
            method: "DELETE",
            url: format!("{API_URL}/users/{user}/muting/{target}"),
            body: None,
            scope: "mute.write",
            cost: 5_000,
        },
        RelationshipAction::DmBlock => RelationshipRequest {
            method: "POST",
            url: format!("{API_URL}/users/{target}/dm/block"),
            body: None,
            scope: "dm.write",
            cost: 10_000,
        },
        RelationshipAction::DmUnblock => RelationshipRequest {
            method: "POST",
            url: format!("{API_URL}/users/{target}/dm/unblock"),
            body: None,
            scope: "dm.write",
            cost: 10_000,
        },
    };
    Ok(request)
}

fn confirmed(response: &ProviderRelationshipActionResponse, action: RelationshipAction) -> bool {
    match action {
        RelationshipAction::Follow => {
            response.data.following == Some(true) || response.data.pending_follow == Some(true)
        }
        RelationshipAction::Unfollow => response.data.following == Some(false),
        RelationshipAction::Mute => response.data.muting == Some(true),
        RelationshipAction::Unmute => response.data.muting == Some(false),
        RelationshipAction::DmBlock => response.data.blocked == Some(true),
        RelationshipAction::DmUnblock => response.data.blocked == Some(false),
    }
}
