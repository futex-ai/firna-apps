use serde_json::json;

use super::super::error::Error;
use super::super::tools;
use super::super::validation::{hostname, page_url, premium_serp_operator};
use unimock::Unimock;

#[test]
fn network_targets_reject_unsafe_and_ambiguous_values() {
    assert!(hostname(String::from("example.com"), false).is_ok());
    assert!(page_url(String::from("https://example.com/a?q=1")).is_ok());

    for target in [
        "localhost",
        "127.0.0.1",
        "EXAMPLE.COM",
        "example.com:443",
        "https://example.com/path",
    ] {
        assert!(hostname(String::from(target), false).is_err());
    }
    for target in [
        "http://example.com",
        "https://localhost/a",
        "https://127.0.0.1/a",
        "https://user:password@example.com/a",
        "https://example.com/a#fragment",
    ] {
        assert!(page_url(String::from(target)).is_err());
    }
}

#[test]
fn search_operators_and_unknown_fields_fail_before_provider_work() {
    assert!(premium_serp_operator("site:example.com rust"));
    let client = Unimock::new(());

    let error = tools::call(
        &client,
        "dataforseo_google_serp",
        json!({
            "keyword": "rust",
            "location_code": 2840,
            "language_code": "en",
            "unreviewed": true
        }),
    )
    .unwrap_err();

    assert!(matches!(
        error,
        Error::InvalidRequest("invalid_google_serp_input")
    ));
}

#[test]
fn location_and_language_selectors_are_exclusive() {
    let client = Unimock::new(());
    let error = tools::call(
        &client,
        "dataforseo_google_serp",
        json!({
            "keyword": "rust",
            "location_code": 2840,
            "location_name": "United States",
            "language_code": "en"
        }),
    )
    .unwrap_err();

    assert!(matches!(
        error,
        Error::InvalidRequest("invalid_location_selector")
    ));
}
