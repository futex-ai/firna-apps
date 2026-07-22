//! Slack tool handlers.

use serde_json::{Value, json};

use crate::slack::host::{bot_credential, slack_post, user_credential};
use crate::slack::types::{
    AppToolCall, SlackListChannelsRequest, SlackReadChannelHistoryRequest,
    SlackSearchMessagesRequest, SlackSendMessageRequest,
};
use crate::slack::{encode_json, invalid_request};

pub(crate) fn call_tool(request: &str) -> String {
    let Ok(call) = serde_json::from_str::<AppToolCall>(request) else {
        return encode_json(invalid_request("invalid_tool_call"));
    };
    let result = match call.tool_name.as_str() {
        "slack_list_channels" => list_channels(&call),
        "slack_read_channel_history" => read_channel_history(&call),
        "slack_send_message" => send_message(&call),
        "slack_search_messages" => search_messages(&call),
        _ => invalid_request("unknown_tool"),
    };
    encode_json(result)
}

fn list_channels(call: &AppToolCall) -> Value {
    let Ok(input) = serde_json::from_value::<SlackListChannelsRequest>(call.input.clone()) else {
        return invalid_request("invalid_list_channels_input");
    };
    let body = omit_null_fields(json!({
        "cursor": input.cursor,
        "exclude_archived": input.exclude_archived.unwrap_or(true),
        "limit": bounded_limit(input.limit, 100, 200),
        "types": input
            .types
            .unwrap_or_else(|| String::from("public_channel,private_channel,im,mpim"))
    }));
    let response = match slack_post(
        "conversations.list",
        bot_credential(&call.installation_id, "bot_token"),
        body,
    ) {
        Ok(response) => response,
        Err(error) => return error,
    };
    let channels = response
        .get("channels")
        .and_then(Value::as_array)
        .map(|channels| {
            channels
                .iter()
                .map(|channel| {
                    json!({
                        "id": string_field(channel, "id"),
                        "name": optional_string_field(channel, "name"),
                        "is_archived": channel
                            .get("is_archived")
                            .and_then(Value::as_bool)
                            .unwrap_or(false),
                        "is_member": channel
                            .get("is_member")
                            .and_then(Value::as_bool)
                            .unwrap_or(false)
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    json!({
        "channels": channels,
        "next_cursor": next_cursor(&response)
    })
}

fn read_channel_history(call: &AppToolCall) -> Value {
    let Ok(input) = serde_json::from_value::<SlackReadChannelHistoryRequest>(call.input.clone())
    else {
        return invalid_request("invalid_read_channel_history_input");
    };
    let body = omit_null_fields(json!({
        "channel": input.channel_id,
        "cursor": input.cursor,
        "latest": input.latest,
        "limit": bounded_limit(input.limit, 50, 200),
        "oldest": input.oldest
    }));
    let response = match slack_post(
        "conversations.history",
        bot_credential(&call.installation_id, "bot_token"),
        body,
    ) {
        Ok(response) => response,
        Err(error) => return error,
    };
    json!({
        "messages": messages(response.get("messages").and_then(Value::as_array)),
        "next_cursor": next_cursor(&response)
    })
}

fn send_message(call: &AppToolCall) -> Value {
    let Ok(input) = serde_json::from_value::<SlackSendMessageRequest>(call.input.clone()) else {
        return invalid_request("invalid_send_message_input");
    };
    let body = omit_null_fields(json!({
        "channel": input.channel_id,
        "client_msg_id": call.operation_id.as_deref(),
        "text": input.text,
        "thread_ts": input.thread_ts
    }));
    let response = match slack_post(
        "chat.postMessage",
        bot_credential(&call.installation_id, "bot_token"),
        body,
    ) {
        Ok(response) => response,
        Err(error) => return error,
    };
    json!({
        "channel_id": string_field(&response, "channel"),
        "ts": string_field(&response, "ts")
    })
}

fn search_messages(call: &AppToolCall) -> Value {
    let Some(user_id) = &call.effective_user_id else {
        return json!({
            "ok": false,
            "error": "auth_required",
            "auth_ids": ["slack_user_search"]
        });
    };
    let Ok(input) = serde_json::from_value::<SlackSearchMessagesRequest>(call.input.clone()) else {
        return invalid_request("invalid_search_messages_input");
    };
    let body = omit_null_fields(json!({
        "count": bounded_limit(input.limit, 20, 100),
        "cursor": input.cursor,
        "query": input.query,
        "sort": input.sort,
        "sort_dir": input.sort_dir
    }));
    let response = match slack_post(
        "search.messages",
        user_credential(&call.installation_id, user_id),
        body,
    ) {
        Ok(response) => response,
        Err(error) => return error,
    };
    let matches = response
        .get("messages")
        .and_then(|messages| messages.get("matches"))
        .and_then(Value::as_array);
    json!({
        "messages": messages(matches),
        "next_cursor": response
            .get("messages")
            .and_then(|messages| messages.get("pagination"))
            .and_then(|pagination| pagination.get("next_cursor"))
            .and_then(Value::as_str)
    })
}

fn messages(values: Option<&Vec<Value>>) -> Vec<Value> {
    values
        .map(|messages| {
            messages
                .iter()
                .map(|message| {
                    json!({
                        "ts": string_field(message, "ts"),
                        "user": optional_string_field(message, "user"),
                        "text": string_field(message, "text"),
                        "thread_ts": optional_string_field(message, "thread_ts")
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn next_cursor(response: &Value) -> Option<&str> {
    response
        .get("response_metadata")
        .and_then(|metadata| metadata.get("next_cursor"))
        .and_then(Value::as_str)
        .filter(|cursor| !cursor.is_empty())
}

fn bounded_limit(limit: Option<u64>, default: u64, max: u64) -> u64 {
    limit.unwrap_or(default).clamp(1, max)
}

fn omit_null_fields(mut value: Value) -> Value {
    if let Value::Object(fields) = &mut value {
        fields.retain(|_, field| !field.is_null());
    }
    value
}

fn string_field<'a>(value: &'a Value, key: &str) -> &'a str {
    value.get(key).and_then(Value::as_str).unwrap_or("")
}

fn optional_string_field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}
