//! Existing-media metadata and subtitle action types.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MediaAction {
    SetAltText,
    AddSubtitles,
    DeleteSubtitles,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ManageMediaInput {
    pub(crate) action: MediaAction,
    pub(crate) media_id: String,
    pub(crate) alt_text: Option<String>,
    pub(crate) subtitle_media_id: Option<String>,
    pub(crate) display_name: Option<String>,
    pub(crate) language_code: Option<String>,
    pub(crate) media_category: Option<MediaCategory>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub(crate) enum MediaCategory {
    AmplifyVideo,
    TweetVideo,
}

#[derive(Debug, Serialize)]
pub(crate) struct ManageMediaOutput {
    pub(crate) action: MediaAction,
    pub(crate) media_id: String,
    pub(crate) applied: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct MediaMetadataBody {
    pub(crate) id: String,
    pub(crate) metadata: MediaMetadata,
}

#[derive(Debug, Serialize)]
pub(crate) struct MediaMetadata {
    pub(crate) alt_text: MediaAltText,
}

#[derive(Debug, Serialize)]
pub(crate) struct MediaAltText {
    pub(crate) text: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct AddSubtitlesBody {
    pub(crate) id: String,
    pub(crate) media_category: MediaCategory,
    pub(crate) subtitles: SubtitleTrack,
}

#[derive(Debug, Serialize)]
pub(crate) struct SubtitleTrack {
    pub(crate) display_name: String,
    pub(crate) id: String,
    pub(crate) language_code: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct DeleteSubtitlesBody {
    pub(crate) id: String,
    pub(crate) media_category: MediaCategory,
    pub(crate) language_code: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProviderMediaActionResponse {
    pub(crate) data: ProviderMediaActionData,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProviderMediaActionData {
    pub(crate) id: Option<String>,
    pub(crate) media_category: Option<String>,
    pub(crate) deleted: Option<bool>,
}
