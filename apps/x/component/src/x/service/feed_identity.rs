//! Connected-account resolution for feed requests without an explicit X user id.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::x::errors::ToolError;
use crate::x::host::user_request;
use crate::x::response::decode_read_response;
use crate::x::types::common::ProviderSingle;
use crate::x::types::posts::{GetUserFeedInput, UserFeed};

use super::runner::ConfiguredXToolRunner;
use super::validation::valid_post_id;

const ME_URL: &str = "https://api.x.com/2/users/me";

#[derive(Debug, Deserialize)]
struct CurrentUser {
    id: String,
}

impl ConfiguredXToolRunner<'_> {
    pub(super) fn resolve_feed_user_id(
        &self,
        installation_id: &str,
        input: &mut GetUserFeedInput,
    ) -> Result<usize, ToolError> {
        if input.user_id.is_some() || matches!(input.feed, UserFeed::RepostsOfMe) {
            return Ok(0);
        }
        let response = self.http.send(user_request(
            "GET",
            ME_URL,
            installation_id,
            BTreeMap::new(),
            None,
        ));
        let provider: ProviderSingle<CurrentUser> = decode_read_response(response, "users.read")?;
        let current = provider.data.ok_or(ToolError::NotFound)?;
        if !valid_post_id(&current.id) {
            return Err(ToolError::ProviderResponseInvalid);
        }
        input.user_id = Some(current.id);
        Ok(1)
    }
}
