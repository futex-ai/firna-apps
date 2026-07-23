//! Instant-page and business-tool conformance fixtures.

use serde_json::json;

use super::super::error::Error;
use super::support::{call, invalid};

#[test]
fn instant_page_request_disables_paid_crawl_and_rendering_options() {
    let (output, request) = call(
        "dataforseo_instant_page_audit",
        json!({"url": "https://example.com/page?q=rust"}),
        vec![json!({"items": [{
            "url": "https://example.com/page?q=rust",
            "checks": {
                "title_too_short": true,
                "is_broken": true,
                "irrelevant_title": false
            },
            "broken_links": true,
            "fetch_timing": {"duration_time": 0.25}
        }]})],
    );

    for field in [
        "store_raw_html",
        "load_resources",
        "enable_javascript",
        "enable_browser_rendering",
        "enable_xhr",
        "validate_micromarkup",
        "check_spell",
        "return_despite_timeout",
    ] {
        assert_eq!(request.task[field], false, "{field}");
    }
    assert_eq!(output["items"][0]["fetch_timing"]["duration_ms"], 250.0);
    assert_eq!(
        output["items"][0]["failed_checks"],
        json!(["broken_links", "broken_page", "title_too_short"])
    );
    assert!(matches!(
        invalid(
            "dataforseo_instant_page_audit",
            json!({"url": "https://user:secret@example.com/page"})
        ),
        Error::InvalidRequest(_)
    ));
}

#[test]
fn business_search_maps_query_only_to_title_and_builds_typed_filters() {
    let (_, request) = call(
        "dataforseo_business_search",
        json!({
            "latitude": 51.5,
            "longitude": -0.12,
            "radius_km": 4.5,
            "query": "coffee",
            "categories": ["cafe", "bakery"],
            "is_claimed": true,
            "min_rating": 4.25,
            "limit": 10,
            "offset": 5
        }),
        Vec::new(),
    );

    assert_eq!(request.task["location_coordinate"], "51.5,-0.12,4.5");
    assert_eq!(request.task["title"], "coffee");
    assert!(request.task.get("description").is_none());
    assert_eq!(
        request.task["filters"],
        json!([["rating.value", ">=", 4.25]])
    );
    assert_eq!(
        request.task["order_by"],
        json!(["rating.value,desc", "rating.votes_count,desc"])
    );
}

#[test]
fn business_search_rejects_missing_query_duplicate_categories_and_bad_coordinates() {
    for input in [
        json!({"latitude": 1.0, "longitude": 1.0, "radius_km": 2.0}),
        json!({
            "latitude": 1.0,
            "longitude": 1.0,
            "radius_km": 2.0,
            "categories": ["Cafe", " cafe "]
        }),
        json!({
            "latitude": 91.0,
            "longitude": 1.0,
            "radius_km": 2.0,
            "query": "coffee"
        }),
    ] {
        assert!(matches!(
            invalid("dataforseo_business_search", input),
            Error::InvalidRequest(_)
        ));
    }
}

#[test]
fn business_identities_are_host_generated_and_mutually_exclusive() {
    let (_, cid) = call(
        "dataforseo_business_info",
        json!({
            "cid": "12345",
            "location_code": 2840,
            "language_code": "en"
        }),
        Vec::new(),
    );
    assert_eq!(cid.task["keyword"], "cid:12345");

    let (_, place) = call(
        "dataforseo_business_info",
        json!({
            "place_id": "ChIJ-test",
            "location_code": 2840,
            "language_code": "en"
        }),
        Vec::new(),
    );
    assert_eq!(place.task["keyword"], "place_id:ChIJ-test");

    for identity in [
        json!({"business_name": "cid:123"}),
        json!({"business_name": "Cafe", "cid": "123"}),
    ] {
        let mut input = identity.as_object().unwrap().clone();
        input.insert(String::from("location_code"), json!(2840));
        input.insert(String::from("language_code"), json!("en"));
        assert!(matches!(
            invalid("dataforseo_business_info", serde_json::Value::Object(input)),
            Error::InvalidRequest(_)
        ));
    }
}

#[test]
fn business_output_caps_nested_data_and_sorts_popular_times() {
    let (output, _) = call(
        "dataforseo_business_info",
        json!({
            "business_name": "Example Cafe",
            "location_code": 2840,
            "language_code": "en"
        }),
        vec![json!({"items": [{
            "title": "Example Cafe",
            "reviews": [{"text": "must not survive"}],
            "questions": [{"text": "must not survive"}],
            "work_time": {"work_hours": {"timetable": {
                "monday": [
                    {"open": {"hour": 9, "minute": 0}, "close": {"hour": 17, "minute": 30}},
                    {"open": {"hour": 99, "minute": 0}, "close": {"hour": 17, "minute": 0}}
                ]
            }}},
            "popular_times": {"popular_times_by_days": {
                "monday": [
                    {"hour": 14, "minute": 0, "popularity": 101},
                    {"hour": 9, "minute": 30, "popularity": 80}
                ]
            }},
            "attributes": {
                "available_attributes": {"service": ["wifi", "delivery"]},
                "unavailable_attributes": {"access": ["parking"]}
            }
        }]})],
    );

    assert_eq!(output["items"][0]["hours"].as_array().unwrap().len(), 1);
    assert_eq!(output["items"][0]["popular_times"][0]["hour"], 9);
    assert_eq!(
        output["items"][0]["popular_times"][1]["popularity"],
        json!(null)
    );
    assert_eq!(
        output["items"][0]["attributes"].as_array().unwrap().len(),
        3
    );
    let encoded = output.to_string();
    assert!(!encoded.contains("reviews"));
    assert!(!encoded.contains("questions"));
    assert!(!encoded.contains("must not survive"));
}
