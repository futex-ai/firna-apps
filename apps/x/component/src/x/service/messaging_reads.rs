//! Bounded Direct Message event reads.

use std::collections::BTreeMap;

use crate::x::errors::{InvalidInputReason, ToolError};
use crate::x::host::user_request;
use crate::x::response::decode_read_response;
use crate::x::types::common::{AppToolCall, PricedToolSuccess, ProviderCollection, ProviderSingle};
use crate::x::types::messaging::{DmEvent, DmView, DmsOutput, GetDmsInput};
use crate::x::types::success::ToolSuccess;

use super::runner::ConfiguredXToolRunner;
use super::usage::{DM_EVENT_READ, metered};
use super::validation::{
    decode_input, ensure_provider_count, normalized_token, trimmed_bounded, validate_decimal_id,
    validate_page,
};

const API_URL: &str = "https://api.x.com/2";
const DM_EVENT_FIELDS: &str =
    "id,event_type,dm_conversation_id,sender_id,participant_ids,text,created_at";

impl ConfiguredXToolRunner<'_> {
    pub(super) fn get_dms(&self, call: AppToolCall) -> Result<PricedToolSuccess, ToolError> {
        let input: GetDmsInput = decode_input(call.input, InvalidInputReason::DmSelector)?;
        if matches!(input.view, DmView::Event) {
            return self.get_dm_event(call.installation_id, input);
        }
        let max_results = input.max_results.unwrap_or(25);
        validate_page(max_results)?;
        let token = normalized_token(input.pagination_token.clone())?;
        let url = dm_collection_url(&input)?;
        let mut query = BTreeMap::from([
            (String::from("max_results"), max_results.to_string()),
            (
                String::from("dm_event.fields"),
                String::from(DM_EVENT_FIELDS),
            ),
        ]);
        if let Some(token) = token {
            query.insert(String::from("pagination_token"), token);
        }
        let response = self.http.send(user_request(
            "GET",
            &url,
            &call.installation_id,
            query,
            None,
        ));
        let provider: ProviderCollection<DmEvent> = decode_read_response(response, "dm.read")?;
        ensure_provider_count(provider.data.len(), max_results as usize)?;
        Ok(dm_success(
            provider.data,
            clean_token(provider.meta.next_token),
        ))
    }

    fn get_dm_event(
        &self,
        installation_id: String,
        input: GetDmsInput,
    ) -> Result<PricedToolSuccess, ToolError> {
        if input.conversation_id.is_some()
            || input.participant_id.is_some()
            || input.max_results.is_some()
            || input.pagination_token.is_some()
        {
            return Err(ToolError::InvalidInput(InvalidInputReason::DmSelector));
        }
        let event_id = input
            .event_id
            .ok_or(ToolError::InvalidInput(InvalidInputReason::DmSelector))?;
        validate_decimal_id(&event_id, InvalidInputReason::DmSelector)?;
        let query = BTreeMap::from([(
            String::from("dm_event.fields"),
            String::from(DM_EVENT_FIELDS),
        )]);
        let response = self.http.send(user_request(
            "GET",
            &format!("{API_URL}/dm_events/{event_id}"),
            &installation_id,
            query,
            None,
        ));
        let provider: ProviderSingle<DmEvent> = decode_read_response(response, "dm.read")?;
        Ok(dm_success(
            vec![provider.data.ok_or(ToolError::NotFound)?],
            None,
        ))
    }
}

fn dm_collection_url(input: &GetDmsInput) -> Result<String, ToolError> {
    let url = match input.view {
        DmView::All => {
            if input.conversation_id.is_some()
                || input.participant_id.is_some()
                || input.event_id.is_some()
            {
                return Err(ToolError::InvalidInput(InvalidInputReason::DmSelector));
            }
            format!("{API_URL}/dm_events")
        }
        DmView::Conversation => {
            if input.participant_id.is_some() || input.event_id.is_some() {
                return Err(ToolError::InvalidInput(InvalidInputReason::DmSelector));
            }
            let id = trimmed_bounded(
                input.conversation_id.clone().unwrap_or_default(),
                128,
                InvalidInputReason::DmSelector,
            )?;
            format!("{API_URL}/dm_conversations/{id}/dm_events")
        }
        DmView::Participant => {
            if input.conversation_id.is_some() || input.event_id.is_some() {
                return Err(ToolError::InvalidInput(InvalidInputReason::DmSelector));
            }
            let id = input
                .participant_id
                .as_deref()
                .ok_or(ToolError::InvalidInput(InvalidInputReason::DmSelector))?;
            validate_decimal_id(id, InvalidInputReason::DmSelector)?;
            format!("{API_URL}/dm_conversations/with/{id}/dm_events")
        }
        DmView::Event => return Err(ToolError::InvalidInput(InvalidInputReason::DmSelector)),
    };
    Ok(url)
}

fn dm_success(events: Vec<DmEvent>, pagination_token: Option<String>) -> PricedToolSuccess {
    let result_count = events.len();
    PricedToolSuccess {
        output: ToolSuccess::GetDms(DmsOutput {
            events,
            pagination_token,
            result_count,
        }),
        usage: metered(&[(DM_EVENT_READ, result_count)]),
    }
}

fn clean_token(token: Option<String>) -> Option<String> {
    token.filter(|value| !value.is_empty())
}
