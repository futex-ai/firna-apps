use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;

use crate::github::projection::{decode_file_content, preview};

#[test]
fn truncates_multibyte_previews_on_utf8_boundaries() {
    let (value, truncated) = preview("ééé", 5).expect("preview should truncate");
    assert_eq!(value, "éé");
    assert!(truncated);
}

#[test]
fn decodes_exact_utf8_and_rejects_binary_or_mismatched_content() {
    let encoded = STANDARD.encode("hello\n");
    assert_eq!(
        decode_file_content("base64", &encoded, 6).expect("text should decode"),
        "hello\n"
    );
    assert!(decode_file_content("base64", &STANDARD.encode([0_u8]), 1).is_err());
    assert!(decode_file_content("base64", &encoded, 7).is_err());
    assert!(decode_file_content("utf-8", "hello", 5).is_err());
}
