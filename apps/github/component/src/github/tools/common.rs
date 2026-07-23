//! Shared provider-call and bounded-output helpers.

use std::collections::BTreeMap;

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::github::error::GitHubError;
use crate::github::input::AppToolCall;
use crate::github::provider::{ProviderMediaType, ProviderRequest};
use crate::github::provider_response;
use crate::github::tools::GitHubToolService;

impl GitHubToolService<'_> {
    pub(super) fn get<T: DeserializeOwned>(
        &self,
        call: &AppToolCall,
        path: String,
        query: BTreeMap<String, String>,
        media_type: ProviderMediaType,
        retry_input: Option<Value>,
    ) -> Result<(T, BTreeMap<String, String>), GitHubError> {
        let response = self.provider.get(ProviderRequest {
            path,
            query,
            installation_id: call.installation_id.clone(),
            media_type,
        })?;
        provider_response::decode(response, self.clock, retry_input)
    }
}

pub(super) fn remove_null_field(value: &mut Value, field: &str) {
    if value.get(field).is_some_and(Value::is_null)
        && let Some(object) = value.as_object_mut()
    {
        object.remove(field);
    }
}
