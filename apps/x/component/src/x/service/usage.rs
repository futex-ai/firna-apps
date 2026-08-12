//! X pricing-unit construction for validated provider results.

use crate::x::types::common::{ToolUsageReport, ToolUsageUnit};

pub(super) const POST_READ: &str = "post_read";
pub(super) const USER_READ: &str = "user_read";
pub(super) const LIST_READ: &str = "list_read";
pub(super) const SPACE_READ: &str = "space_read";
pub(super) const COMMUNITY_READ: &str = "community_read";
pub(super) const TREND_READ: &str = "trend_read";
pub(super) const MEDIA_READ: &str = "media_read";
pub(super) const DM_EVENT_READ: &str = "dm_event_read";

pub(super) fn metered(units: &[(&'static str, usize)]) -> ToolUsageReport {
    ToolUsageReport::Metered {
        units: units
            .iter()
            .map(|(unit, quantity)| ToolUsageUnit {
                unit,
                quantity: *quantity as u64,
            })
            .collect(),
    }
}

pub(super) fn reported_cost(cost_usd_micros: u64) -> ToolUsageReport {
    ToolUsageReport::ReportedCost { cost_usd_micros }
}
