//! Shared bounded text, selector, hostname, and URL validation.

use std::collections::BTreeSet;
use std::net::{IpAddr, Ipv6Addr};

use serde_json::{Map, Value, json};
use url::{Host, Url};

use super::error::{Error, Result};

pub(super) const DEFAULT_TIMEOUT_SECONDS: u64 = 180;

pub(super) fn text(value: String, max_chars: usize, reason: &'static str) -> Result<String> {
    let value = value.trim().to_owned();
    if value.is_empty() || value.chars().count() > max_chars || value.chars().any(char::is_control)
    {
        return Err(Error::InvalidRequest(reason));
    }
    Ok(value)
}

pub(super) fn unique_texts(
    values: Vec<String>,
    min: usize,
    max: usize,
    max_chars: usize,
    max_words: Option<usize>,
    reason: &'static str,
) -> Result<Vec<String>> {
    if !(min..=max).contains(&values.len()) {
        return Err(Error::InvalidRequest(reason));
    }
    let mut seen = BTreeSet::new();
    let mut normalized = Vec::with_capacity(values.len());
    for value in values {
        let value = text(value, max_chars, reason)?;
        if max_words.is_some_and(|limit| value.split_whitespace().count() > limit)
            || !seen.insert(value.to_lowercase())
        {
            return Err(Error::InvalidRequest(reason));
        }
        normalized.push(value);
    }
    Ok(normalized)
}

pub(super) fn timeout(value: Option<u64>) -> Result<u64> {
    bounded(
        value.unwrap_or(DEFAULT_TIMEOUT_SECONDS),
        1,
        300,
        "invalid_timeout_seconds",
    )
}

pub(super) fn bounded(value: u64, min: u64, max: u64, reason: &'static str) -> Result<u64> {
    (min..=max)
        .contains(&value)
        .then_some(value)
        .ok_or(Error::InvalidRequest(reason))
}

pub(super) fn location_language(
    location_name: Option<String>,
    location_code: Option<i64>,
    language_name: Option<String>,
    language_code: Option<String>,
) -> Result<Map<String, Value>> {
    let mut task = Map::new();
    match (location_name, location_code) {
        (Some(name), None) => {
            task.insert(
                "location_name".into(),
                json!(text(name, 255, "invalid_location")?),
            );
        }
        (None, Some(code @ 1..=2_147_483_647)) => {
            task.insert("location_code".into(), json!(code));
        }
        _ => return Err(Error::InvalidRequest("invalid_location_selector")),
    }
    match (language_name, language_code) {
        (Some(name), None) => {
            task.insert(
                "language_name".into(),
                json!(text(name, 100, "invalid_language")?),
            );
        }
        (None, Some(code)) => {
            task.insert(
                "language_code".into(),
                json!(text(code, 32, "invalid_language")?),
            );
        }
        _ => return Err(Error::InvalidRequest("invalid_language_selector")),
    }
    Ok(task)
}

pub(super) fn hostname(value: String, deny_www: bool) -> Result<String> {
    let value = text(value, 253, "invalid_hostname")?;
    if !value.is_ascii()
        || value != value.to_ascii_lowercase()
        || deny_www && value.starts_with("www.")
    {
        return Err(Error::InvalidRequest("invalid_hostname"));
    }
    let parsed = Url::parse(&format!("https://{value}"))
        .ok()
        .filter(|url| url.path() == "/" && url.query().is_none() && url.fragment().is_none())
        .ok_or(Error::InvalidRequest("invalid_hostname"))?;
    match parsed.host() {
        Some(Host::Domain(domain)) if domain == value && domain.contains('.') => Ok(value),
        _ => Err(Error::InvalidRequest("invalid_hostname")),
    }
}

pub(super) fn page_url(value: String) -> Result<String> {
    let value = text(value, 2_048, "invalid_page_url")?;
    let parsed = match Url::parse(&value) {
        Ok(parsed) => parsed,
        Err(_) => return Err(Error::InvalidRequest("invalid_page_url")),
    };
    if parsed.scheme() != "https"
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.fragment().is_some()
    {
        return Err(Error::InvalidRequest("invalid_page_url"));
    }
    match parsed.host() {
        Some(Host::Domain(domain)) if domain != "localhost" && !domain.ends_with(".localhost") => {}
        Some(Host::Ipv4(address)) if public_ip(IpAddr::V4(address)) => {}
        Some(Host::Ipv6(address)) if public_ip(IpAddr::V6(address)) => {}
        _ => return Err(Error::InvalidRequest("invalid_page_url")),
    }
    Ok(value)
}

pub(super) fn target(value: String, deny_www: bool) -> Result<String> {
    if value.trim().to_ascii_lowercase().starts_with("https://") {
        page_url(value)
    } else {
        hostname(value, deny_www)
    }
}

pub(super) fn premium_serp_operator(keyword: &str) -> bool {
    const OPERATORS: [&str; 16] = [
        "allinanchor:",
        "allintext:",
        "allintitle:",
        "allinurl:",
        "cache:",
        "define:",
        "definition:",
        "filetype:",
        "id:",
        "inanchor:",
        "info:",
        "intext:",
        "intitle:",
        "inurl:",
        "link:",
        "site:",
    ];
    let keyword = keyword.to_ascii_lowercase();
    OPERATORS.iter().any(|operator| keyword.contains(operator))
}

fn public_ip(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => {
            !(address.is_private()
                || address.is_loopback()
                || address.is_link_local()
                || address.is_multicast()
                || address.is_unspecified())
        }
        IpAddr::V6(address) => {
            !(address.is_loopback()
                || address.is_multicast()
                || address.is_unspecified()
                || unique_local(address)
                || unicast_link_local(address))
        }
    }
}

fn unique_local(address: Ipv6Addr) -> bool {
    address.segments()[0] & 0xfe00 == 0xfc00
}

fn unicast_link_local(address: Ipv6Addr) -> bool {
    address.segments()[0] & 0xffc0 == 0xfe80
}
