use std::fs;

use crate::{app_root, component_bytes, manifest};

#[test]
fn x_manifest_embeds_the_repo_owned_png_source() {
    let manifest = manifest();
    let icon = manifest.icon.expect("X icon");
    let base64 =
        fs::read_to_string(app_root().join("assets/icon.png.base64")).expect("read X icon base64");
    let png = fs::read(app_root().join("assets/icon.png")).expect("read X icon PNG");

    assert_eq!(icon.image.media_type.as_str(), "image/png");
    assert_eq!(icon.image.data_base64, base64.trim());
    assert_eq!(icon.color_pair.primary, "#000000");
    assert_eq!(icon.color_pair.secondary, "#FFFFFF");
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    assert!(png.len() < 64 * 1_024);
    assert_eq!(
        u32::from_be_bytes(png[16..20].try_into().expect("width")),
        128
    );
    assert_eq!(
        u32::from_be_bytes(png[20..24].try_into().expect("height")),
        128
    );
}

#[test]
fn x_component_bytes_are_a_component_binary() {
    let bytes = component_bytes();

    assert_eq!(&bytes[0..4], &[0x00, 0x61, 0x73, 0x6d]);
    assert_eq!(&bytes[4..8], &[0x0d, 0x00, 0x01, 0x00]);
}
