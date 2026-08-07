//! GitHub Link-header pagination tests.

use std::collections::BTreeMap;

use crate::github::pagination::next_page;

#[test]
fn accepts_only_positive_signed_32_bit_next_pages() {
    let maximum = BTreeMap::from([(
        String::from("link"),
        String::from("<https://api.github.com/user/repos?page=2147483647>; rel=\"next\""),
    )]);
    assert_eq!(next_page(&maximum), Some(2_147_483_647));

    for value in ["0", "2147483648", "-1", "1.5", "next"] {
        let headers = BTreeMap::from([(
            String::from("Link"),
            format!("<https://api.github.com/user/repos?page={value}>; rel=\"next\""),
        )]);
        assert_eq!(next_page(&headers), None, "unexpected page {value}");
    }
}

#[test]
fn ignores_non_next_and_malformed_link_entries() {
    let headers = BTreeMap::from([(
        String::from("link"),
        String::from("garbage, <https://api.github.com/user/repos?page=2>; rel=\"last\""),
    )]);
    assert_eq!(next_page(&headers), None);
}
