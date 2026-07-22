//! JSON DTOs used by the Slack component.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub(crate) struct AppToolCall {
    pub(crate) installation_id: String,
    pub(crate) tool_name: String,
    pub(crate) operation_id: Option<String>,
    pub(crate) input: Value,
    pub(crate) effective_user_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SlackListChannelsRequest {
    pub(crate) cursor: Option<String>,
    pub(crate) limit: Option<u64>,
    pub(crate) types: Option<String>,
    pub(crate) exclude_archived: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SlackReadChannelHistoryRequest {
    pub(crate) channel_id: String,
    pub(crate) cursor: Option<String>,
    pub(crate) limit: Option<u64>,
    pub(crate) oldest: Option<String>,
    pub(crate) latest: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SlackSendMessageRequest {
    pub(crate) channel_id: String,
    pub(crate) text: String,
    pub(crate) thread_ts: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SlackSearchMessagesRequest {
    pub(crate) query: String,
    pub(crate) cursor: Option<String>,
    pub(crate) limit: Option<u64>,
    pub(crate) sort: Option<String>,
    pub(crate) sort_dir: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct WebhookEnvelope {
    pub(crate) app_id: String,
    pub(crate) ingress_id: String,
    pub(crate) headers: BTreeMap<String, String>,
    pub(crate) body: Vec<u8>,
    pub(crate) received_at: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct VerifiedProviderEvent {
    pub(crate) installation_id: String,
    pub(crate) envelope: WebhookEnvelope,
    pub(crate) verification: WebhookVerification,
}

#[derive(Debug, Deserialize)]
pub(crate) struct WebhookResponseRequest {
    pub(crate) envelope: WebhookEnvelope,
    pub(crate) verification: WebhookVerification,
}

#[derive(Debug, Deserialize)]
pub(crate) struct WebhookVerification {
    pub(crate) provider_account_id: String,
    pub(crate) provider_event_id: String,
    pub(crate) provider_event_type: String,
}
