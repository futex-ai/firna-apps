//! X tool dispatch over the injected host HTTP boundary.

use crate::x::errors::{InvalidInputReason, ToolError};
use crate::x::host::XHttpClient;
use crate::x::types::{AppToolCall, PricedToolSuccess};

use super::output::encode_output;

pub(crate) fn call_tool(request_json: &str, http: &dyn XHttpClient) -> String {
    let output = match serde_json::from_str::<AppToolCall>(request_json) {
        Ok(call) => ConfiguredXToolRunner { http }.run(call),
        Err(_) => Err(ToolError::InvalidInput(
            InvalidInputReason::MalformedToolCall,
        )),
    };
    encode_output(output)
}

pub(super) struct ConfiguredXToolRunner<'a> {
    pub(super) http: &'a dyn XHttpClient,
}

impl ConfiguredXToolRunner<'_> {
    fn run(&self, call: AppToolCall) -> Result<PricedToolSuccess, ToolError> {
        match call.tool_name.as_str() {
            "x_get_posts" => self.get_posts(call),
            "x_get_post_metrics" => self.get_post_metrics(call),
            "x_search_recent_posts" => self.search_recent_posts(call),
            "x_create_post" => self.create_post(call),
            _ => Err(ToolError::InvalidInput(InvalidInputReason::UnknownTool)),
        }
    }
}
