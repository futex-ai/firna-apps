//! Fail-closed validation and classification of signed GitHub webhook payloads.

use crate::github::webhook_event_validation;
use crate::github::webhook_events;
use crate::github::webhook_host::{WebhookError, WebhookSigner};
use crate::github::webhook_types::{
    GitHubWebhookPayload, ProviderInstallationLifecycle, WebhookEnvelope, WebhookVerification,
};

const MAX_PAYLOAD_BYTES: usize = 262_144;

pub(super) fn is_published_event(event_type: &str) -> bool {
    webhook_events::is_published(event_type)
}

pub(super) fn verify(
    request: &str,
    signer: &dyn WebhookSigner,
) -> Result<WebhookVerification, WebhookError> {
    let envelope =
        serde_json::from_str::<WebhookEnvelope>(request).or(Err(WebhookError::InvalidEnvelope))?;
    if envelope.body.len() > MAX_PAYLOAD_BYTES {
        return Err(WebhookError::PayloadTooLarge);
    }
    let signature = required_header(&envelope, "x-hub-signature-256", HeaderKind::Signature)?;
    validate_signature(signature)?;
    let body_text =
        std::str::from_utf8(&envelope.body).or(Err(WebhookError::InvalidBodyEncoding))?;
    let expected = format!("sha256={}", signer.digest(body_text)?);
    if !constant_time_eq(signature.as_bytes(), expected.as_bytes()) {
        return Err(WebhookError::InvalidSignature);
    }
    let delivery = required_header(&envelope, "x-github-delivery", HeaderKind::Delivery)?;
    if !is_guid(delivery) {
        return Err(WebhookError::MalformedDelivery);
    }
    let event_type = required_header(&envelope, "x-github-event", HeaderKind::EventType)?;
    if !is_event_type(event_type) {
        return Err(WebhookError::MalformedEventType);
    }
    let payload = serde_json::from_slice::<GitHubWebhookPayload>(&envelope.body)
        .or(Err(WebhookError::InvalidJson))?;
    verify_payload_shape(event_type, &payload)?;
    build_verification(event_type, delivery, &payload)
}

fn build_verification(
    event_type: &str,
    delivery: &str,
    payload: &GitHubWebhookPayload,
) -> Result<WebhookVerification, WebhookError> {
    if event_type == "ping" {
        return Ok(WebhookVerification {
            provider_account_id: String::from("ping"),
            provider_installation_id: None,
            provider_event_id: delivery.to_owned(),
            provider_event_type: event_type.to_owned(),
            provider_user_id: None,
            provider_repository_id: None,
            installation_lifecycle: None,
        });
    }
    if event_type == "github_app_authorization" {
        let sender = payload
            .sender
            .as_ref()
            .ok_or(WebhookError::MissingAccount)?;
        return Ok(WebhookVerification {
            provider_account_id: sender.id.to_string(),
            provider_installation_id: None,
            provider_event_id: delivery.to_owned(),
            provider_event_type: event_type.to_owned(),
            provider_user_id: Some(sender.id.to_string()),
            provider_repository_id: None,
            installation_lifecycle: None,
        });
    }
    let installation = payload
        .installation
        .as_ref()
        .filter(|installation| installation.id != 0)
        .ok_or(WebhookError::MissingInstallation)?;
    if installation.account.id == 0 {
        return Err(WebhookError::MissingAccount);
    }
    let provider_repository_id = payload
        .repository
        .as_ref()
        .filter(|repository| repository.id > 0)
        .map(|repository| repository.id.to_string());
    if (webhook_events::is_published(event_type) || webhook_events::is_acknowledged(event_type))
        && provider_repository_id.is_none()
    {
        return Err(WebhookError::MissingRepository);
    }
    let installation_lifecycle = lifecycle(event_type, payload.action.as_deref());
    if !webhook_events::is_supported(event_type) {
        return Err(WebhookError::UnsupportedEvent);
    }
    Ok(WebhookVerification {
        provider_account_id: installation.account.id.to_string(),
        provider_installation_id: Some(installation.id.to_string()),
        provider_event_id: delivery.to_owned(),
        provider_event_type: event_type.to_owned(),
        provider_user_id: payload
            .sender
            .as_ref()
            .filter(|sender| sender.id != 0)
            .map(|sender| sender.id.to_string()),
        provider_repository_id,
        installation_lifecycle,
    })
}

fn verify_payload_shape(
    event_type: &str,
    payload: &GitHubWebhookPayload,
) -> Result<(), WebhookError> {
    if webhook_events::is_published(event_type) {
        webhook_event_validation::published(event_type, payload)
    } else if webhook_events::is_acknowledged(event_type) {
        require(
            payload
                .installation
                .as_ref()
                .is_some_and(|installation| installation.id > 0 && installation.account.id > 0)
                && payload
                    .repository
                    .as_ref()
                    .is_some_and(|repository| repository.id > 0)
                && payload.sender.as_ref().is_some_and(|sender| sender.id > 0),
        )
    } else {
        webhook_event_validation::control(event_type, payload)
    }
}

fn lifecycle(event_type: &str, action: Option<&str>) -> Option<ProviderInstallationLifecycle> {
    match (event_type, action) {
        ("installation", Some("deleted" | "suspend")) => {
            Some(ProviderInstallationLifecycle::Revoke)
        }
        ("installation", Some("created" | "unsuspend" | "new_permissions_accepted")) => {
            Some(ProviderInstallationLifecycle::Reconcile)
        }
        ("installation_repositories", Some("added" | "removed")) => {
            Some(ProviderInstallationLifecycle::Reconcile)
        }
        ("installation_target", Some("renamed" | "transferred")) => {
            Some(ProviderInstallationLifecycle::Reconcile)
        }
        _ => None,
    }
}

#[derive(Clone, Copy)]
enum HeaderKind {
    Signature,
    Delivery,
    EventType,
}

fn required_header<'a>(
    envelope: &'a WebhookEnvelope,
    name: &str,
    kind: HeaderKind,
) -> Result<&'a str, WebhookError> {
    let mut matches = envelope.headers.iter().filter(|header| header.name == name);
    let first = matches.next().map(|header| header.value.as_slice());
    if matches.next().is_some() {
        return Err(WebhookError::ConflictingHeader);
    }
    let value = first.filter(|value| !value.is_empty()).ok_or(match kind {
        HeaderKind::Signature => WebhookError::MissingSignature,
        HeaderKind::Delivery => WebhookError::MissingDelivery,
        HeaderKind::EventType => WebhookError::MissingEventType,
    })?;
    std::str::from_utf8(value).or(Err(malformed_header(kind)))
}

fn malformed_header(kind: HeaderKind) -> WebhookError {
    match kind {
        HeaderKind::Signature => WebhookError::MalformedSignature,
        HeaderKind::Delivery => WebhookError::MalformedDelivery,
        HeaderKind::EventType => WebhookError::MalformedEventType,
    }
}

fn validate_signature(signature: &str) -> Result<(), WebhookError> {
    let Some(digest) = signature.strip_prefix("sha256=") else {
        return Err(WebhookError::MalformedSignature);
    };
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(WebhookError::MalformedSignature);
    }
    Ok(())
}

fn is_guid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| match index {
            8 | 13 | 18 | 23 => byte == b'-',
            _ => byte.is_ascii_hexdigit(),
        })
}

fn is_event_type(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

fn require(condition: bool) -> Result<(), WebhookError> {
    if condition {
        Ok(())
    } else {
        Err(WebhookError::EventTypeDisagreement)
    }
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut difference = 0;
    for index in 0..left.len() {
        difference |= left[index] ^ right[index];
    }
    difference == 0
}
