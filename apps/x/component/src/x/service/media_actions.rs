//! Existing X media metadata and subtitle management.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::x::errors::{InvalidInputReason, ToolError};
use crate::x::host::user_request;
use crate::x::response::decode_write_response;
use crate::x::types::common::{AppToolCall, PricedToolSuccess};
use crate::x::types::media_actions::{
    AddSubtitlesBody, DeleteSubtitlesBody, ManageMediaInput, ManageMediaOutput, MediaAction,
    MediaAltText, MediaCategory, MediaMetadata, MediaMetadataBody, ProviderMediaActionResponse,
    SubtitleTrack,
};
use crate::x::types::success::ToolSuccess;

use super::runner::ConfiguredXToolRunner;
use super::usage::reported_cost;
use super::validation::{decode_input, trimmed_bounded, validate_decimal_id};

const METADATA_URL: &str = "https://api.x.com/2/media/metadata";
const SUBTITLES_URL: &str = "https://api.x.com/2/media/subtitles";

impl ConfiguredXToolRunner<'_> {
    pub(super) fn manage_media(&self, call: AppToolCall) -> Result<PricedToolSuccess, ToolError> {
        let input: ManageMediaInput = decode_input(call.input, InvalidInputReason::MediaAction)?;
        validate_decimal_id(&input.media_id, InvalidInputReason::MediaAction)?;
        let request = media_request(&input)?;
        let response = self.http.send(user_request(
            request.method,
            request.url,
            &call.installation_id,
            BTreeMap::new(),
            Some(request.body),
        ));
        let provider: ProviderMediaActionResponse = decode_write_response(response, "media.write")?;
        if !confirmed(&provider, &input) {
            return Err(ToolError::WriteOutcomeUnknown);
        }
        Ok(PricedToolSuccess {
            output: ToolSuccess::ManageMedia(ManageMediaOutput {
                action: input.action,
                media_id: input.media_id,
                applied: true,
            }),
            usage: reported_cost(5_000),
        })
    }
}

struct MediaRequest {
    method: &'static str,
    url: &'static str,
    body: serde_json::Value,
}

fn media_request(input: &ManageMediaInput) -> Result<MediaRequest, ToolError> {
    match input.action {
        MediaAction::SetAltText => alt_text_request(input),
        MediaAction::AddSubtitles => add_subtitles_request(input),
        MediaAction::DeleteSubtitles => delete_subtitles_request(input),
    }
}

fn alt_text_request(input: &ManageMediaInput) -> Result<MediaRequest, ToolError> {
    if input.subtitle_media_id.is_some()
        || input.display_name.is_some()
        || input.language_code.is_some()
        || input.media_category.is_some()
    {
        return Err(ToolError::InvalidInput(InvalidInputReason::MediaAction));
    }
    let text = trimmed_bounded(
        input.alt_text.clone().unwrap_or_default(),
        1_000,
        InvalidInputReason::MediaAction,
    )?;
    encoded_request(
        "POST",
        METADATA_URL,
        MediaMetadataBody {
            id: input.media_id.clone(),
            metadata: MediaMetadata {
                alt_text: MediaAltText { text },
            },
        },
    )
}

fn add_subtitles_request(input: &ManageMediaInput) -> Result<MediaRequest, ToolError> {
    if input.alt_text.is_some() {
        return Err(ToolError::InvalidInput(InvalidInputReason::MediaAction));
    }
    let subtitle_media_id = input
        .subtitle_media_id
        .as_deref()
        .ok_or(ToolError::InvalidInput(InvalidInputReason::MediaAction))?;
    validate_decimal_id(subtitle_media_id, InvalidInputReason::MediaAction)?;
    let display_name = trimmed_bounded(
        input.display_name.clone().unwrap_or_default(),
        100,
        InvalidInputReason::MediaAction,
    )?;
    let language_code = language_code(input.language_code.as_deref())?;
    let media_category = input
        .media_category
        .ok_or(ToolError::InvalidInput(InvalidInputReason::MediaAction))?;
    encoded_request(
        "POST",
        SUBTITLES_URL,
        AddSubtitlesBody {
            id: input.media_id.clone(),
            media_category,
            subtitles: SubtitleTrack {
                display_name,
                id: subtitle_media_id.to_owned(),
                language_code,
            },
        },
    )
}

fn delete_subtitles_request(input: &ManageMediaInput) -> Result<MediaRequest, ToolError> {
    if input.alt_text.is_some() || input.subtitle_media_id.is_some() || input.display_name.is_some()
    {
        return Err(ToolError::InvalidInput(InvalidInputReason::MediaAction));
    }
    let language_code = language_code(input.language_code.as_deref())?;
    let media_category = input
        .media_category
        .ok_or(ToolError::InvalidInput(InvalidInputReason::MediaAction))?;
    encoded_request(
        "DELETE",
        SUBTITLES_URL,
        DeleteSubtitlesBody {
            id: input.media_id.clone(),
            media_category,
            language_code,
        },
    )
}

fn language_code(value: Option<&str>) -> Result<String, ToolError> {
    let value = value.unwrap_or_default();
    if value.len() == 2 && value.bytes().all(|byte| byte.is_ascii_uppercase()) {
        Ok(value.to_owned())
    } else {
        Err(ToolError::InvalidInput(InvalidInputReason::MediaAction))
    }
}

fn encoded_request(
    method: &'static str,
    url: &'static str,
    body: impl Serialize,
) -> Result<MediaRequest, ToolError> {
    let body = match serde_json::to_value(body) {
        Ok(body) => body,
        Err(_) => return Err(ToolError::ProviderResponseInvalid),
    };
    Ok(MediaRequest { method, url, body })
}

fn confirmed(response: &ProviderMediaActionResponse, input: &ManageMediaInput) -> bool {
    match input.action {
        MediaAction::SetAltText => response.data.id.as_deref() == Some(input.media_id.as_str()),
        MediaAction::AddSubtitles => {
            response.data.id.as_deref() == Some(input.media_id.as_str())
                && response.data.media_category.as_deref()
                    == input.media_category.map(category_name)
        }
        MediaAction::DeleteSubtitles => response.data.deleted == Some(true),
    }
}

fn category_name(category: MediaCategory) -> &'static str {
    match category {
        MediaCategory::AmplifyVideo => "AmplifyVideo",
        MediaCategory::TweetVideo => "TweetVideo",
    }
}
