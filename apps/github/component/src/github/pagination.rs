//! Numeric pagination helpers for GitHub `Link` headers.

use std::collections::BTreeMap;

pub(crate) fn next_page(headers: &BTreeMap<String, String>) -> Option<u32> {
    let link = header(headers, "link")?;
    for entry in link.split(',') {
        if !entry.contains("rel=\"next\"") {
            continue;
        }
        let start = entry.find('<')?.checked_add(1)?;
        let end = entry[start..].find('>')?.checked_add(start)?;
        let url = &entry[start..end];
        let query = url.split_once('?')?.1;
        for pair in query.split('&') {
            let (name, value) = pair.split_once('=')?;
            if name == "page"
                && let Ok(page) = value.parse::<u32>()
                && (1..=i32::MAX as u32).contains(&page)
            {
                return Some(page);
            }
        }
    }
    None
}

pub(crate) fn header<'a>(headers: &'a BTreeMap<String, String>, name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(candidate, _)| candidate.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str())
}
