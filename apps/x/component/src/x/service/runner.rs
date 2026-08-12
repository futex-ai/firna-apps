//! X tool dispatch over the injected host HTTP boundary.

use crate::x::errors::{InvalidInputReason, ToolError};
use crate::x::host::XHttpClient;
use crate::x::types::common::{AppToolCall, PricedToolSuccess};

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
            "x_search_all_posts" => self.search_all_posts(call),
            "x_get_post_counts" => self.get_post_counts(call),
            "x_get_users" => self.get_users(call),
            "x_search_users" => self.search_users(call),
            "x_get_user_feed" => self.get_user_feed(call),
            "x_get_post_engagements" => self.get_post_engagements(call),
            "x_get_relationships" => self.get_relationships(call),
            "x_get_lists" => self.get_lists(call),
            "x_get_spaces" => self.get_spaces(call),
            "x_get_communities" => self.get_communities(call),
            "x_get_trends" => self.get_trends(call),
            "x_get_media" => self.get_media(call),
            "x_get_dms" => self.get_dms(call),
            "x_create_post" => self.create_post(call),
            "x_manage_post" => self.manage_post(call),
            "x_manage_relationship" => self.manage_relationship(call),
            "x_manage_list" => self.manage_list(call),
            "x_manage_dm" => self.manage_dm(call),
            "x_manage_media" => self.manage_media(call),
            "x_create_bookmark_folder" => self.create_bookmark_folder(call),
            _ => Err(ToolError::InvalidInput(InvalidInputReason::UnknownTool)),
        }
    }
}
