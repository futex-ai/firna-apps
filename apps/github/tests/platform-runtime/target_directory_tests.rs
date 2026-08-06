//! Cargo target-directory resolution tests for the GitHub component build.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::component_target_dir;

#[test]
fn component_target_dir_defaults_to_component_target() {
    let app_root = Path::new("/repo/apps/github");

    assert_eq!(
        component_target_dir(app_root, None),
        PathBuf::from("/repo/apps/github/component/target")
    );
}

#[test]
fn component_target_dir_uses_cargo_target_dir_override() {
    let app_root = Path::new("/repo/apps/github");
    let cargo_target_dir = OsString::from("/Volumes/build-cache/firna");

    assert_eq!(
        component_target_dir(app_root, Some(cargo_target_dir)),
        PathBuf::from("/Volumes/build-cache/firna")
    );
}
