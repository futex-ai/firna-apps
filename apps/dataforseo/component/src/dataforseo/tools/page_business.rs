//! Instant-page and local-business request construction.

use serde_json::{Value, json};

use super::{decode_input, joined_filters, provider_items};
use crate::dataforseo::envelope::decode;
use crate::dataforseo::error::{Error, Result};
use crate::dataforseo::host::ProviderClient;
use crate::dataforseo::input::{BusinessInfoInput, BusinessSearchInput, InstantPageAuditInput};
use crate::dataforseo::output::{
    business_info as normalize_business_info, business_listing, instant_page, success,
};
use crate::dataforseo::validation::{
    bounded, location_language, page_url, text, timeout, unique_texts,
};

pub(super) fn instant_page_audit(client: &dyn ProviderClient, input: Value) -> Result<Value> {
    let input: InstantPageAuditInput = decode_input(input, "invalid_instant_page_audit_input")?;
    let request_timeout = timeout(input.timeout_seconds)?;
    let task = json!({
        "url": page_url(input.url)?,
        "store_raw_html": false,
        "load_resources": false,
        "enable_javascript": false,
        "enable_browser_rendering": false,
        "enable_xhr": false,
        "validate_micromarkup": false,
        "check_spell": false,
        "return_despite_timeout": false,
    });
    let provider = decode(client.post_task("/v3/on_page/instant_pages", task, request_timeout)?)?;
    let items = provider_items(&provider.results)
        .first()
        .map(instant_page)
        .into_iter()
        .collect();
    Ok(success("dataforseo.instant_page_audit", provider, items))
}

pub(super) fn business_search(client: &dyn ProviderClient, input: Value) -> Result<Value> {
    let input: BusinessSearchInput = decode_input(input, "invalid_business_search_input")?;
    validate_coordinates(input.latitude, input.longitude, input.radius_km)?;
    let query = input
        .query
        .map(|value| text(value, 200, "invalid_business_query"))
        .transpose()?;
    let categories = input
        .categories
        .map(|values| unique_texts(values, 1, 5, 100, None, "invalid_categories"))
        .transpose()?;
    if query.is_none() && categories.is_none() {
        return Err(Error::InvalidRequest(
            "business_query_or_categories_required",
        ));
    }
    let limit = bounded(input.limit.unwrap_or(25), 1, 50, "invalid_limit")?;
    let offset = bounded(input.offset.unwrap_or(0), 0, 1_000, "invalid_offset")?;
    let request_timeout = timeout(input.timeout_seconds)?;
    let mut task = serde_json::Map::new();
    task.insert(
        "location_coordinate".into(),
        json!(format!(
            "{},{},{}",
            input.latitude, input.longitude, input.radius_km
        )),
    );
    if let Some(query) = query {
        task.insert("title".into(), json!(query));
    }
    if let Some(categories) = categories {
        task.insert("categories".into(), json!(categories));
    }
    if let Some(is_claimed) = input.is_claimed {
        task.insert("is_claimed".into(), json!(is_claimed));
    }
    if let Some(rating) = input.min_rating {
        if !rating.is_finite() || !(0.0..=5.0).contains(&rating) {
            return Err(Error::InvalidRequest("invalid_min_rating"));
        }
        task.insert(
            "filters".into(),
            joined_filters(vec![json!(["rating.value", ">=", rating])])
                .unwrap_or(Value::Array(Vec::new())),
        );
    }
    task.insert("limit".into(), json!(limit));
    task.insert("offset".into(), json!(offset));
    task.insert(
        "order_by".into(),
        json!(["rating.value,desc", "rating.votes_count,desc"]),
    );
    let provider = decode(client.post_task(
        "/v3/business_data/business_listings/search/live",
        Value::Object(task),
        request_timeout,
    )?)?;
    let items = provider_items(&provider.results)
        .into_iter()
        .take(limit as usize)
        .map(|item| business_listing(&item))
        .collect();
    Ok(success("dataforseo.business_search", provider, items))
}

pub(super) fn business_info(client: &dyn ProviderClient, input: Value) -> Result<Value> {
    let input: BusinessInfoInput = decode_input(input, "invalid_business_info_input")?;
    let keyword = business_identity(input.business_name, input.cid, input.place_id)?;
    let request_timeout = timeout(input.timeout_seconds)?;
    let mut task = location_language(
        input.location_name,
        input.location_code,
        input.language_name,
        input.language_code,
    )?;
    task.insert("keyword".into(), json!(keyword));
    let provider = decode(client.post_task(
        "/v3/business_data/google/my_business_info/live",
        Value::Object(task),
        request_timeout,
    )?)?;
    let items = provider_items(&provider.results)
        .into_iter()
        .map(|item| normalize_business_info(&item))
        .collect();
    Ok(success("dataforseo.business_info", provider, items))
}

fn validate_coordinates(latitude: f64, longitude: f64, radius: f64) -> Result<()> {
    if !latitude.is_finite()
        || !longitude.is_finite()
        || !radius.is_finite()
        || !(-90.0..=90.0).contains(&latitude)
        || !(-180.0..=180.0).contains(&longitude)
        || !(1.0..=100.0).contains(&radius)
    {
        return Err(Error::InvalidRequest("invalid_business_coordinates"));
    }
    Ok(())
}

fn business_identity(
    business_name: Option<String>,
    cid: Option<String>,
    place_id: Option<String>,
) -> Result<String> {
    match (business_name, cid, place_id) {
        (Some(name), None, None) => {
            let name = text(name, 700, "invalid_business_name")?;
            if name.to_ascii_lowercase().starts_with("cid:")
                || name.to_ascii_lowercase().starts_with("place_id:")
            {
                return Err(Error::InvalidRequest("reserved_business_identity_prefix"));
            }
            Ok(name)
        }
        (None, Some(cid), None)
            if (1..=32).contains(&cid.len()) && cid.bytes().all(|byte| byte.is_ascii_digit()) =>
        {
            Ok(format!("cid:{cid}"))
        }
        (None, None, Some(place_id)) => Ok(format!(
            "place_id:{}",
            text(place_id, 255, "invalid_place_id")?
        )),
        _ => Err(Error::InvalidRequest("one_business_identity_required")),
    }
}
