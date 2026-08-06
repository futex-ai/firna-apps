//! Slack webhook verification and event normalization.

use std::collections::BTreeMap;
use std::str;

use serde_json::{Value, json};

use crate::slack::host::{hmac_sha256, signing_credential};
use crate::slack::types::{
    VerifiedProviderEvent, WebhookEnvelope, WebhookHeader, WebhookResponseRequest,
};
use crate::slack::{encode_json, invalid_request};

const MAX_SIGNATURE_AGE_SECONDS: i64 = 300;

pub(crate) fn verify_webhook(request: &str) -> String {
    let Ok(envelope) = serde_json::from_str::<WebhookEnvelope>(request) else {
        return encode_json(invalid_request("invalid_webhook_envelope"));
    };
    let Ok(body_text) = String::from_utf8(envelope.body.clone()) else {
        return encode_json(invalid_request("invalid_webhook_body"));
    };
    let Ok(body) = serde_json::from_str::<Value>(&body_text) else {
        return encode_json(invalid_request("invalid_webhook_json"));
    };
    if let Some(error) = verify_signature(&envelope, &body_text) {
        return encode_json(error);
    }
    let provider_event_type = provider_event_type(&body);
    let provider_account_id = match team_id(&body) {
        Some(team_id) => team_id.to_owned(),
        None if provider_event_type == "url_verification" => String::from("url_verification"),
        None => return encode_json(invalid_request("missing_team_id")),
    };
    encode_json(json!({
        "provider_account_id": provider_account_id,
        "provider_event_id": provider_event_id(&body),
        "provider_event_type": provider_event_type,
        "provider_user_id": provider_user_id(&body)
    }))
}

pub(crate) fn normalize_event(request: &str) -> String {
    let Ok(event) = serde_json::from_str::<VerifiedProviderEvent>(request) else {
        return encode_json(invalid_request("invalid_verified_event"));
    };
    let Ok(body_text) = String::from_utf8(event.envelope.body.clone()) else {
        return encode_json(invalid_request("invalid_webhook_body"));
    };
    let Ok(body) = serde_json::from_str::<Value>(&body_text) else {
        return encode_json(invalid_request("invalid_webhook_json"));
    };
    let slack_event = body.get("event").unwrap_or(&body);
    encode_json(json!({
        "app_id": event.envelope.app_id,
        "installation_id": event.installation_id,
        "provider": "slack",
        "provider_event_id": event.verification.provider_event_id,
        "provider_event_type": event.verification.provider_event_type,
        "provider_account_id": event.verification.provider_account_id,
        "source": source(slack_event),
        "payload": payload(&body, slack_event)
    }))
}

pub(crate) fn webhook_response(request: &str) -> String {
    let Ok(request) = serde_json::from_str::<WebhookResponseRequest>(request) else {
        return encode_json(invalid_request("invalid_webhook_response_request"));
    };
    if request.verification.provider_event_type != "url_verification" {
        return String::from("null");
    }
    let Ok(body_text) = String::from_utf8(request.envelope.body.clone()) else {
        return encode_json(invalid_request("invalid_webhook_body"));
    };
    let Ok(body) = serde_json::from_str::<Value>(&body_text) else {
        return encode_json(invalid_request("invalid_webhook_json"));
    };
    let Some(challenge) = body.get("challenge").and_then(Value::as_str) else {
        return encode_json(invalid_request("missing_challenge"));
    };
    encode_json(json!({
        "status_code": 200,
        "content_type": "text/plain; charset=utf-8",
        "body": challenge
    }))
}

fn verify_signature(envelope: &WebhookEnvelope, body: &str) -> Option<Value> {
    let signature = match header_text(&envelope.headers, "x-slack-signature") {
        HeaderText::Value(value) => value,
        HeaderText::Missing => return Some(invalid_request("missing_slack_signature")),
        HeaderText::Invalid => return Some(invalid_request("invalid_slack_signature")),
    };
    let timestamp = match header_text(&envelope.headers, "x-slack-request-timestamp") {
        HeaderText::Value(value) => value,
        HeaderText::Missing => return Some(invalid_request("missing_slack_timestamp")),
        HeaderText::Invalid => return Some(invalid_request("invalid_slack_timestamp")),
    };
    let Ok(timestamp_seconds) = timestamp.parse::<i64>() else {
        return Some(invalid_request("invalid_slack_timestamp"));
    };
    if is_stale(timestamp_seconds, &envelope.received_at) {
        return Some(invalid_request("stale_slack_timestamp"));
    }
    let base = format!("v0:{timestamp}:{body}");
    let digest = match hmac_sha256(signing_credential(), base) {
        Ok(digest) => digest,
        Err(error) => return Some(error),
    };
    let expected = format!("v0={digest}");
    if constant_time_eq(signature.as_bytes(), expected.as_bytes()) {
        None
    } else {
        Some(invalid_request("invalid_slack_signature"))
    }
}

enum HeaderText<'a> {
    Missing,
    Invalid,
    Value(&'a str),
}

fn header_text<'a>(headers: &'a [WebhookHeader], name: &str) -> HeaderText<'a> {
    let mut matching = headers.iter().filter(|header| header.name == name);
    let Some(header) = matching.next() else {
        return HeaderText::Missing;
    };
    if matching.next().is_some() {
        return HeaderText::Invalid;
    }
    match str::from_utf8(&header.value) {
        Ok(value) => HeaderText::Value(value),
        Err(_) => HeaderText::Invalid,
    }
}

fn team_id(body: &Value) -> Option<&str> {
    body.get("team_id")
        .and_then(Value::as_str)
        .or_else(|| body.get("team").and_then(Value::as_str))
        .or_else(|| {
            body.get("authorizations")
                .and_then(Value::as_array)
                .and_then(|values| values.first())
                .and_then(|value| value.get("team_id"))
                .and_then(Value::as_str)
        })
}

fn provider_event_id(body: &Value) -> String {
    body.get("event_id")
        .and_then(Value::as_str)
        .or_else(|| {
            body.get("event")
                .and_then(|event| event.get("event_ts"))
                .and_then(Value::as_str)
        })
        .or_else(|| body.get("challenge").and_then(Value::as_str))
        .unwrap_or("unknown")
        .to_owned()
}

fn provider_event_type(body: &Value) -> String {
    if body.get("type").and_then(Value::as_str) == Some("url_verification") {
        return String::from("url_verification");
    }
    let event = body.get("event").unwrap_or(body);
    let event_type = event
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    if event_type != "message" {
        return event_type.to_owned();
    }
    match event.get("channel_type").and_then(Value::as_str) {
        Some("channel") => String::from("message.channels"),
        Some("group") => String::from("message.groups"),
        Some("im") => String::from("message.im"),
        Some("mpim") => String::from("message.mpim"),
        _ => String::from("message"),
    }
}

fn provider_user_id(body: &Value) -> Option<&str> {
    body.get("event")
        .unwrap_or(body)
        .get("user")
        .and_then(Value::as_str)
}

fn source(event: &Value) -> BTreeMap<String, String> {
    let mut source = BTreeMap::new();
    insert_string(&mut source, "user_id", event.get("user"));
    insert_string(&mut source, "channel_id", event.get("channel"));
    insert_string(&mut source, "thread_ts", event.get("thread_ts"));
    insert_string(&mut source, "ts", event.get("ts"));
    source
}

fn payload(body: &Value, event: &Value) -> Value {
    if body.get("type").and_then(Value::as_str) == Some("url_verification") {
        return json!({
            "type": "url_verification",
            "challenge": body.get("challenge").and_then(Value::as_str)
        });
    }
    json!({
        "type": event.get("type").and_then(Value::as_str),
        "text": event.get("text").and_then(Value::as_str),
        "channel": event.get("channel").and_then(Value::as_str),
        "user": event.get("user").and_then(Value::as_str),
        "ts": event.get("ts").and_then(Value::as_str),
        "thread_ts": event.get("thread_ts").and_then(Value::as_str)
    })
}

fn insert_string(target: &mut BTreeMap<String, String>, key: &str, value: Option<&Value>) {
    if let Some(value) = value.and_then(Value::as_str) {
        target.insert(key.to_owned(), value.to_owned());
    }
}

fn is_stale(timestamp_seconds: i64, received_at: &str) -> bool {
    let Some(received_seconds) = unix_seconds_from_rfc3339(received_at) else {
        return false;
    };
    (received_seconds - timestamp_seconds).abs() > MAX_SIGNATURE_AGE_SECONDS
}

fn unix_seconds_from_rfc3339(value: &str) -> Option<i64> {
    if value.len() < 19 {
        return None;
    }
    let year = value.get(0..4)?.parse::<i64>().ok()?;
    let month = value.get(5..7)?.parse::<i64>().ok()?;
    let day = value.get(8..10)?.parse::<i64>().ok()?;
    let hour = value.get(11..13)?.parse::<i64>().ok()?;
    let minute = value.get(14..16)?.parse::<i64>().ok()?;
    let second = value.get(17..19)?.parse::<i64>().ok()?;
    let days = days_from_civil(year, month, day);
    Some(days * 86_400 + hour * 3_600 + minute * 60 + second)
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = year - if month <= 2 { 1 } else { 0 };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut diff = 0;
    for index in 0..left.len() {
        diff |= left[index] ^ right[index];
    }
    diff == 0
}
