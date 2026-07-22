//! Compact closed output-envelope helpers.

mod common;
mod content_domain;
mod groups;
mod normalize;
mod page_business;

pub(super) use common::{bool_value, bounded_signed, number, signed, string, strings};
pub(super) use content_domain::{
    content_item, content_sentiment_item, technology_item, whois_item,
};
pub(super) use groups::{count_buckets, keyed_aggregates};
pub(super) use normalize::{business_listing, keyword_metric};
pub(super) use page_business::{business_info, instant_page};

use serde_json::{Value, json};

use super::envelope::ProviderResult;

pub(super) fn success(operation: &str, provider: ProviderResult, items: Vec<Value>) -> Value {
    json!({
        "ok": true,
        "provider": "dataforseo",
        "operation": operation,
        "task_id": provider.task_id,
        "cost_usd": provider.cost_usd,
        "rate_limit": {
            "limit_per_minute": provider.rate_limit.limit_per_minute,
            "remaining": provider.rate_limit.remaining,
        },
        "result_count": items.len(),
        "items": items,
    })
}
