//! Validated provider request construction for List actions.

use serde::Serialize;

use crate::x::errors::{InvalidInputReason, ToolError};
use crate::x::types::list_actions::{
    CreateListBody, ListAction, ListIdBody, ManageListInput, UpdateListBody, UserIdBody,
};

use super::validation::validate_decimal_id;

const API_URL: &str = "https://api.x.com/2";

pub(super) struct ListActionRequest {
    pub(super) method: &'static str,
    pub(super) url: String,
    pub(super) body: Option<serde_json::Value>,
    pub(super) cost: u64,
}

pub(super) fn list_action_request(input: &ManageListInput) -> Result<ListActionRequest, ToolError> {
    match input.action {
        ListAction::Create => create_request(input),
        ListAction::Update => update_request(input),
        ListAction::Delete => simple_list_request(input, "DELETE"),
        ListAction::AddMember => member_request(input, true),
        ListAction::RemoveMember => member_request(input, false),
        ListAction::Follow => user_list_request(input, "followed_lists", "POST"),
        ListAction::Unfollow => user_list_request(input, "followed_lists", "DELETE"),
        ListAction::Pin => user_list_request(input, "pinned_lists", "POST"),
        ListAction::Unpin => user_list_request(input, "pinned_lists", "DELETE"),
    }
}

fn create_request(input: &ManageListInput) -> Result<ListActionRequest, ToolError> {
    require_absent(input, true, true, true)?;
    let name = valid_name(input.name.as_deref())?;
    let description = valid_description(input.description.as_deref())?;
    Ok(ListActionRequest {
        method: "POST",
        url: format!("{API_URL}/lists"),
        body: body(CreateListBody {
            name,
            description,
            private: input.private,
        })?,
        cost: 10_000,
    })
}

fn update_request(input: &ManageListInput) -> Result<ListActionRequest, ToolError> {
    require_ids(input, true, false, false)?;
    if input.name.is_none() && input.description.is_none() && input.private.is_none() {
        return Err(ToolError::InvalidInput(InvalidInputReason::ListAction));
    }
    let name = match input.name.as_deref() {
        Some(_) => Some(valid_name(input.name.as_deref())?),
        None => None,
    };
    let description = valid_description(input.description.as_deref())?;
    Ok(ListActionRequest {
        method: "PUT",
        url: format!(
            "{API_URL}/lists/{}",
            input.list_id.as_deref().unwrap_or_default()
        ),
        body: body(UpdateListBody {
            name,
            description,
            private: input.private,
        })?,
        cost: if input.private.is_some() {
            10_000
        } else {
            5_000
        },
    })
}

fn simple_list_request(
    input: &ManageListInput,
    method: &'static str,
) -> Result<ListActionRequest, ToolError> {
    require_ids(input, true, false, false)?;
    require_mutable_absent(input)?;
    Ok(ListActionRequest {
        method,
        url: format!(
            "{API_URL}/lists/{}",
            input.list_id.as_deref().unwrap_or_default()
        ),
        body: None,
        cost: 5_000,
    })
}

fn member_request(input: &ManageListInput, add: bool) -> Result<ListActionRequest, ToolError> {
    require_ids(input, true, false, true)?;
    require_mutable_absent(input)?;
    let list = input.list_id.as_deref().unwrap_or_default();
    let target = input.target_user_id.as_deref().unwrap_or_default();
    if add {
        Ok(ListActionRequest {
            method: "POST",
            url: format!("{API_URL}/lists/{list}/members"),
            body: body(UserIdBody {
                user_id: target.to_owned(),
            })?,
            cost: 5_000,
        })
    } else {
        Ok(ListActionRequest {
            method: "DELETE",
            url: format!("{API_URL}/lists/{list}/members/{target}"),
            body: None,
            cost: 5_000,
        })
    }
}

fn user_list_request(
    input: &ManageListInput,
    route: &str,
    method: &'static str,
) -> Result<ListActionRequest, ToolError> {
    require_ids(input, true, true, false)?;
    require_mutable_absent(input)?;
    let user = input.user_id.as_deref().unwrap_or_default();
    let list = input.list_id.as_deref().unwrap_or_default();
    let (url, payload) = if method == "POST" {
        (
            format!("{API_URL}/users/{user}/{route}"),
            body(ListIdBody {
                list_id: list.to_owned(),
            })?,
        )
    } else {
        (format!("{API_URL}/users/{user}/{route}/{list}"), None)
    };
    Ok(ListActionRequest {
        method,
        url,
        body: payload,
        cost: 5_000,
    })
}

fn require_ids(
    input: &ManageListInput,
    list: bool,
    user: bool,
    target: bool,
) -> Result<(), ToolError> {
    let valid = input.list_id.is_some() == list
        && input.user_id.is_some() == user
        && input.target_user_id.is_some() == target;
    if !valid {
        return Err(ToolError::InvalidInput(InvalidInputReason::ListAction));
    }
    for id in [
        input.list_id.as_deref(),
        input.user_id.as_deref(),
        input.target_user_id.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        validate_decimal_id(id, InvalidInputReason::ListAction)?;
    }
    Ok(())
}

fn require_absent(
    input: &ManageListInput,
    list: bool,
    user: bool,
    target: bool,
) -> Result<(), ToolError> {
    if list && input.list_id.is_some()
        || user && input.user_id.is_some()
        || target && input.target_user_id.is_some()
    {
        Err(ToolError::InvalidInput(InvalidInputReason::ListAction))
    } else {
        Ok(())
    }
}

fn require_mutable_absent(input: &ManageListInput) -> Result<(), ToolError> {
    if input.name.is_some() || input.description.is_some() || input.private.is_some() {
        Err(ToolError::InvalidInput(InvalidInputReason::ListAction))
    } else {
        Ok(())
    }
}

fn valid_name(name: Option<&str>) -> Result<String, ToolError> {
    let name = name.unwrap_or_default().trim();
    if (1..=25).contains(&name.chars().count()) {
        Ok(name.to_owned())
    } else {
        Err(ToolError::InvalidInput(InvalidInputReason::ListAction))
    }
}

fn valid_description(description: Option<&str>) -> Result<Option<String>, ToolError> {
    match description {
        Some(value) if value.chars().count() <= 100 => Ok(Some(value.to_owned())),
        Some(_) => Err(ToolError::InvalidInput(InvalidInputReason::ListAction)),
        None => Ok(None),
    }
}

fn body(value: impl Serialize) -> Result<Option<serde_json::Value>, ToolError> {
    match serde_json::to_value(value) {
        Ok(value) => Ok(Some(value)),
        Err(_) => Err(ToolError::ProviderResponseInvalid),
    }
}
