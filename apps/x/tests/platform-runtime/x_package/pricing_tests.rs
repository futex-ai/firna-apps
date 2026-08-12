use fna_apps_interface::manifest::{AppPricing, AppToolPricingStructure};

use crate::manifest;

#[test]
fn x_manifest_prices_every_tool_with_a_finite_cap() {
    let manifest = manifest();
    let pricing = manifest.pricing.expect("X pricing declaration");

    assert_eq!(pricing.tools.len(), 23);
    assert_metered(
        &pricing,
        "x_get_posts",
        &[('p', 5_000, 10), ('u', 10_000, 10)],
    );
    assert_metered(&pricing, "x_get_post_metrics", &[('p', 5_000, 10)]);
    assert_metered(
        &pricing,
        "x_search_recent_posts",
        &[('p', 5_000, 25), ('u', 10_000, 25)],
    );
    assert_metered(
        &pricing,
        "x_search_all_posts",
        &[('p', 5_000, 25), ('u', 10_000, 25)],
    );
    assert_metered(&pricing, "x_get_users", &[('u', 10_000, 10)]);
    assert_metered(&pricing, "x_search_users", &[('u', 10_000, 25)]);
    assert_metered(
        &pricing,
        "x_get_user_feed",
        &[('p', 5_000, 25), ('u', 10_000, 25)],
    );
    assert_metered(
        &pricing,
        "x_get_post_engagements",
        &[('p', 5_000, 25), ('u', 10_000, 25)],
    );
    assert_metered(&pricing, "x_get_relationships", &[('u', 10_000, 25)]);
    assert_named_metered(
        &pricing,
        "x_get_lists",
        &["list_read", "post_read", "user_read"],
    );
    assert_named_metered(
        &pricing,
        "x_get_spaces",
        &["space_read", "post_read", "user_read"],
    );
    assert_named_metered(&pricing, "x_get_communities", &["community_read"]);
    assert_named_metered(&pricing, "x_get_trends", &["trend_read"]);
    assert_named_metered(&pricing, "x_get_dms", &["dm_event_read"]);
    assert_named_metered(&pricing, "x_get_media", &["media_read"]);
    for (tool, cap) in [
        ("x_create_post", 200_000),
        ("x_get_post_counts", 10_000),
        ("x_manage_post", 15_000),
        ("x_manage_relationship", 15_000),
        ("x_manage_list", 10_000),
        ("x_manage_dm", 15_000),
        ("x_manage_media", 5_000),
        ("x_create_bookmark_folder", 5_000),
    ] {
        assert_usage_reported(&pricing, tool, cap);
    }
}

fn assert_metered(pricing: &AppPricing, tool: &str, expected: &[(char, u64, u64)]) {
    let entry = pricing
        .tools
        .iter()
        .find(|entry| entry.tool == tool)
        .expect("metered pricing");
    let Some(AppToolPricingStructure::Metered { units }) = entry.structure.as_ref() else {
        panic!("{tool} should use metered pricing");
    };
    let actual = units
        .iter()
        .map(|unit| {
            let kind = if unit.unit == "post_read" { 'p' } else { 'u' };
            (kind, unit.price_usd_micros, unit.max_units_per_call)
        })
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
}

fn assert_named_metered(pricing: &AppPricing, tool: &str, names: &[&str]) {
    let entry = pricing
        .tools
        .iter()
        .find(|entry| entry.tool == tool)
        .expect("metered pricing");
    let Some(AppToolPricingStructure::Metered { units }) = entry.structure.as_ref() else {
        panic!("{tool} should use metered pricing");
    };
    assert_eq!(
        units
            .iter()
            .map(|unit| unit.unit.as_str())
            .collect::<Vec<_>>(),
        names
    );
    assert!(units.iter().all(|unit| unit.max_units_per_call > 0));
}

fn assert_usage_reported(pricing: &AppPricing, tool: &str, cap: u64) {
    let entry = pricing
        .tools
        .iter()
        .find(|entry| entry.tool == tool)
        .expect("usage-reported pricing");
    assert_eq!(
        entry.structure,
        Some(AppToolPricingStructure::UsageReported {
            max_cost_usd_micros_per_call: cap,
        })
    );
}
