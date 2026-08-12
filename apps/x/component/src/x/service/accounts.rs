//! Account profile, search, and relationship read tools.

use std::collections::{BTreeMap, HashSet};

use crate::x::errors::{InvalidInputReason, ToolError};
use crate::x::host::user_request;
use crate::x::response::decode_read_response;
use crate::x::types::accounts::{
    GetRelationshipsInput, GetUsersInput, Relationship, SearchUsersInput, UserLookup, UsersOutput,
};
use crate::x::types::common::{
    AppToolCall, CompactUser, PricedToolSuccess, ProviderCollection, ProviderSingle,
};
use crate::x::types::success::ToolSuccess;

use super::runner::ConfiguredXToolRunner;
use super::usage::{USER_READ, metered};
use super::validation::{
    decode_input, ensure_provider_count, normalized_token, trimmed_bounded, valid_username,
    validate_decimal_id, validate_decimal_ids, validate_page,
};

const USERS_URL: &str = "https://api.x.com/2/users";
const USERS_BY_URL: &str = "https://api.x.com/2/users/by";
const ME_URL: &str = "https://api.x.com/2/users/me";
const SEARCH_URL: &str = "https://api.x.com/2/users/search";
const USER_FIELDS: &str = "id,name,username,description,created_at,location,url,profile_image_url,protected,verified,verified_type,public_metrics";

impl ConfiguredXToolRunner<'_> {
    pub(super) fn get_users(&self, call: AppToolCall) -> Result<PricedToolSuccess, ToolError> {
        let input: GetUsersInput = decode_input(call.input, InvalidInputReason::UserSelector)?;
        let query = profile_query();
        let (users, missing_values) = match input.lookup {
            UserLookup::Me => {
                if input.ids.is_some() || input.usernames.is_some() {
                    return Err(ToolError::InvalidInput(InvalidInputReason::UserSelector));
                }
                let response = self.http.send(user_request(
                    "GET",
                    ME_URL,
                    &call.installation_id,
                    query,
                    None,
                ));
                let provider: ProviderSingle<CompactUser> =
                    decode_read_response(response, "users.read")?;
                (vec![provider.data.ok_or(ToolError::NotFound)?], Vec::new())
            }
            UserLookup::Ids => {
                let Some(ids) = input.ids else {
                    return Err(ToolError::InvalidInput(InvalidInputReason::UserSelector));
                };
                if input.usernames.is_some() {
                    return Err(ToolError::InvalidInput(InvalidInputReason::UserSelector));
                }
                validate_decimal_ids(&ids, 10, InvalidInputReason::UserId)?;
                self.lookup_users(call.installation_id, USERS_URL, "ids", ids, false, query)?
            }
            UserLookup::Usernames => {
                let Some(usernames) = input.usernames else {
                    return Err(ToolError::InvalidInput(InvalidInputReason::UserSelector));
                };
                if input.ids.is_some()
                    || usernames.len() > 10
                    || usernames.is_empty()
                    || usernames.iter().any(|value| !valid_username(value))
                    || !all_unique_case_insensitive(&usernames)
                {
                    return Err(ToolError::InvalidInput(InvalidInputReason::Username));
                }
                self.lookup_users(
                    call.installation_id,
                    USERS_BY_URL,
                    "usernames",
                    usernames,
                    true,
                    query,
                )?
            }
        };
        if users.is_empty() {
            return Err(ToolError::NotFound);
        }
        let result_count = users.len();
        Ok(PricedToolSuccess {
            output: ToolSuccess::GetUsers(UsersOutput {
                users,
                missing_values,
                pagination_token: None,
                result_count,
            }),
            usage: metered(&[(USER_READ, result_count)]),
        })
    }

    pub(super) fn search_users(&self, call: AppToolCall) -> Result<PricedToolSuccess, ToolError> {
        let input: SearchUsersInput = decode_input(call.input, InvalidInputReason::UserQuery)?;
        let query_text = trimmed_bounded(input.query, 50, InvalidInputReason::UserQuery)?;
        if !query_text
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'\'' | b'_' | b' '))
        {
            return Err(ToolError::InvalidInput(InvalidInputReason::UserQuery));
        }
        validate_page(input.max_results)?;
        let token = normalized_token(input.pagination_token)?;
        let mut query = profile_query();
        query.insert(String::from("query"), query_text);
        query.insert(String::from("max_results"), input.max_results.to_string());
        if let Some(token) = token {
            query.insert(String::from("next_token"), token);
        }
        let response = self.http.send(user_request(
            "GET",
            SEARCH_URL,
            &call.installation_id,
            query,
            None,
        ));
        let provider: ProviderCollection<CompactUser> =
            decode_read_response(response, "users.read")?;
        ensure_provider_count(provider.data.len(), input.max_results as usize)?;
        let result_count = provider.data.len();
        Ok(PricedToolSuccess {
            output: ToolSuccess::SearchUsers(UsersOutput {
                users: provider.data,
                missing_values: Vec::new(),
                pagination_token: clean_token(provider.meta.next_token),
                result_count,
            }),
            usage: metered(&[(USER_READ, result_count)]),
        })
    }

    pub(super) fn get_relationships(
        &self,
        call: AppToolCall,
    ) -> Result<PricedToolSuccess, ToolError> {
        let input: GetRelationshipsInput =
            decode_input(call.input, InvalidInputReason::UserSelector)?;
        validate_decimal_id(&input.user_id, InvalidInputReason::UserId)?;
        validate_page(input.max_results)?;
        let token = normalized_token(input.pagination_token)?;
        let (suffix, scope) = match input.relationship {
            Relationship::Affiliates => ("affiliates", "users.read"),
            Relationship::Followers => ("followers", "follows.read"),
            Relationship::Following => ("following", "follows.read"),
            Relationship::Blocked => ("blocking", "block.read"),
            Relationship::Muted => ("muting", "mute.read"),
        };
        let url = format!("{USERS_URL}/{}/{suffix}", input.user_id);
        let mut query = profile_query();
        query.insert(String::from("max_results"), input.max_results.to_string());
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
        let provider: ProviderCollection<CompactUser> = decode_read_response(response, scope)?;
        ensure_provider_count(provider.data.len(), input.max_results as usize)?;
        let result_count = provider.data.len();
        Ok(PricedToolSuccess {
            output: ToolSuccess::GetRelationships(UsersOutput {
                users: provider.data,
                missing_values: Vec::new(),
                pagination_token: clean_token(provider.meta.next_token),
                result_count,
            }),
            usage: metered(&[(USER_READ, result_count)]),
        })
    }

    fn lookup_users(
        &self,
        installation_id: String,
        url: &str,
        selector: &str,
        requested: Vec<String>,
        compare_username: bool,
        mut query: BTreeMap<String, String>,
    ) -> Result<(Vec<CompactUser>, Vec<String>), ToolError> {
        query.insert(selector.to_owned(), requested.join(","));
        let response = self
            .http
            .send(user_request("GET", url, &installation_id, query, None));
        let provider: ProviderCollection<CompactUser> =
            decode_read_response(response, "users.read")?;
        ensure_provider_count(provider.data.len(), requested.len())?;
        let requested_keys: HashSet<String> = requested
            .iter()
            .map(|value| normalized_lookup_key(value, compare_username))
            .collect();
        let returned: HashSet<String> = provider
            .data
            .iter()
            .map(|user| {
                normalized_lookup_key(
                    if compare_username {
                        &user.username
                    } else {
                        &user.id
                    },
                    compare_username,
                )
            })
            .collect();
        if returned.len() != provider.data.len()
            || returned.iter().any(|value| !requested_keys.contains(value))
        {
            return Err(ToolError::ProviderResponseInvalid);
        }
        let missing = requested
            .into_iter()
            .filter(|value| !returned.contains(&normalized_lookup_key(value, compare_username)))
            .collect();
        Ok((provider.data, missing))
    }
}

fn profile_query() -> BTreeMap<String, String> {
    BTreeMap::from([(String::from("user.fields"), String::from(USER_FIELDS))])
}

fn all_unique_case_insensitive(values: &[String]) -> bool {
    let unique: HashSet<String> = values
        .iter()
        .map(|value| value.to_ascii_lowercase())
        .collect();
    unique.len() == values.len()
}

fn normalized_lookup_key(value: &str, compare_username: bool) -> String {
    if compare_username {
        value.to_ascii_lowercase()
    } else {
        value.to_owned()
    }
}

fn clean_token(token: Option<String>) -> Option<String> {
    token.filter(|value| !value.is_empty())
}
