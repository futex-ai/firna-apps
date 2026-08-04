//! Cargo target-directory resolution tests for the X component build.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::component_target_dir;

#[test]
fn component_target_dir_defaults_to_component_target() {
    let app_root = Path::new("/repo/apps/x");

    assert_eq!(
        component_target_dir(app_root, None),
        PathBuf::from("/repo/apps/x/component/target")
    );
}

#[test]
fn component_target_dir_uses_absolute_override() {
    let app_root = Path::new("/repo/apps/x");
    let configured = OsString::from("/Volumes/build-cache/firna");

    assert_eq!(
        component_target_dir(app_root, Some(configured)),
        PathBuf::from("/Volumes/build-cache/firna")
    );
}
