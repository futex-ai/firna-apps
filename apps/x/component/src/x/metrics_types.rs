//! Typed provider and output models for current X Post metrics.

use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GetPostMetricsInput {
    pub(crate) ids: Vec<String>,
    #[serde(default)]
    pub(crate) include_private_metrics: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct GetPostMetricsOutput {
    pub(crate) metrics: Vec<PostMetrics>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) missing_ids: Vec<String>,
    pub(crate) result_count: usize,
}

#[derive(Debug, Serialize)]
pub(crate) struct PostMetrics {
    pub(crate) id: String,
    pub(crate) public_metrics: PublicPostMetrics,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) private_metrics: Option<PrivatePostMetrics>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) unavailable_private_metrics: Vec<PrivateMetricName>,
}

#[derive(Debug, Serialize)]
pub(crate) struct PublicPostMetrics {
    pub(crate) impressions: u64,
    pub(crate) likes: u64,
    pub(crate) replies: u64,
    pub(crate) reposts: u64,
    pub(crate) quotes: u64,
    pub(crate) bookmarks: u64,
}

#[derive(Debug, Serialize)]
pub(crate) struct PrivatePostMetrics {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) engagements: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) url_clicks: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) profile_clicks: Option<u64>,
}

impl PrivatePostMetrics {
    fn is_empty(&self) -> bool {
        self.engagements.is_none() && self.url_clicks.is_none() && self.profile_clicks.is_none()
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PrivateMetricName {
    Engagements,
    UrlClicks,
    ProfileClicks,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct ProviderPostMetricsResponse {
    #[serde(default)]
    pub(crate) data: Vec<ProviderPostMetrics>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProviderPostMetrics {
    pub(crate) id: String,
    pub(crate) public_metrics: ProviderPublicPostMetrics,
    #[serde(default)]
    non_public_metrics: Option<ProviderPrivatePostMetrics>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProviderPublicPostMetrics {
    impression_count: u64,
    like_count: u64,
    reply_count: u64,
    retweet_count: u64,
    quote_count: u64,
    bookmark_count: u64,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct ProviderPrivatePostMetrics {
    #[serde(default, deserialize_with = "deserialize_optional_count")]
    engagements: Option<u64>,
    #[serde(default, deserialize_with = "deserialize_optional_count")]
    url_link_clicks: Option<u64>,
    #[serde(default, deserialize_with = "deserialize_optional_count")]
    user_profile_clicks: Option<u64>,
}

impl ProviderPostMetrics {
    pub(crate) fn into_output(self, include_private_metrics: bool) -> PostMetrics {
        let public_metrics = PublicPostMetrics {
            impressions: self.public_metrics.impression_count,
            likes: self.public_metrics.like_count,
            replies: self.public_metrics.reply_count,
            reposts: self.public_metrics.retweet_count,
            quotes: self.public_metrics.quote_count,
            bookmarks: self.public_metrics.bookmark_count,
        };
        let (private_metrics, unavailable_private_metrics) = if include_private_metrics {
            private_output(self.non_public_metrics.unwrap_or_default())
        } else {
            (None, Vec::new())
        };
        PostMetrics {
            id: self.id,
            public_metrics,
            private_metrics,
            unavailable_private_metrics,
        }
    }
}

fn private_output(
    provider: ProviderPrivatePostMetrics,
) -> (Option<PrivatePostMetrics>, Vec<PrivateMetricName>) {
    let mut unavailable = Vec::new();
    if provider.engagements.is_none() {
        unavailable.push(PrivateMetricName::Engagements);
    }
    if provider.url_link_clicks.is_none() {
        unavailable.push(PrivateMetricName::UrlClicks);
    }
    if provider.user_profile_clicks.is_none() {
        unavailable.push(PrivateMetricName::ProfileClicks);
    }
    let metrics = PrivatePostMetrics {
        engagements: provider.engagements,
        url_clicks: provider.url_link_clicks,
        profile_clicks: provider.user_profile_clicks,
    };
    let metrics = if metrics.is_empty() {
        None
    } else {
        Some(metrics)
    };
    (metrics, unavailable)
}

fn deserialize_optional_count<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: Deserializer<'de>,
{
    u64::deserialize(deserializer).map(Some)
}
