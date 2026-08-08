//! X app runtime conformance tests against the Firna Wasm host.

use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use fna_apps_interface::manifest::AppManifest;
use fna_apps_wasm::WasmAppPackage;

/// Returns the X manifest under test.
pub fn manifest() -> AppManifest {
    let yaml = fs::read_to_string(app_root().join("manifest.yaml")).expect("read X manifest");
    AppManifest::from_yaml(&yaml).expect("X manifest should parse")
}

/// Returns a freshly built X component for runtime tests.
pub fn component_bytes() -> Vec<u8> {
    static COMPONENT_BYTES: OnceLock<Vec<u8>> = OnceLock::new();

    COMPONENT_BYTES.get_or_init(build_component_bytes).clone()
}

/// Returns the X package tested by the platform runtime harness.
pub fn package() -> WasmAppPackage {
    WasmAppPackage {
        manifest: manifest(),
        component_bytes: component_bytes(),
    }
}

fn build_component_bytes() -> Vec<u8> {
    let app_root = app_root();
    let component_manifest = app_root.join("component/Cargo.toml");
    let cargo_status = Command::new("cargo")
        .args([
            "build",
            "--manifest-path",
            component_manifest.to_string_lossy().as_ref(),
            "--target",
            "wasm32-unknown-unknown",
            "--release",
            "--locked",
        ])
        .status()
        .expect("start X component cargo build");
    assert!(cargo_status.success(), "X component cargo build failed");

    let core_wasm = component_target_dir(&app_root, env::var_os("CARGO_TARGET_DIR"))
        .join("wasm32-unknown-unknown/release/fna_app_x_component.wasm");
    let component_wasm = env::temp_dir().join(component_filename());
    let wasm_tools_status = Command::new("wasm-tools")
        .args([
            "component",
            "new",
            core_wasm.to_string_lossy().as_ref(),
            "-o",
            component_wasm.to_string_lossy().as_ref(),
        ])
        .status()
        .expect("start wasm-tools component conversion");
    assert!(wasm_tools_status.success(), "X component conversion failed");

    let bytes = fs::read(&component_wasm).expect("read X Wasm component");
    let _ = fs::remove_file(component_wasm);
    bytes
}

fn app_root() -> PathBuf {
    env::var_os("FIRNA_X_APP_ROOT").map_or_else(
        || PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
        PathBuf::from,
    )
}

fn component_target_dir(app_root: &Path, cargo_target_dir: Option<OsString>) -> PathBuf {
    match cargo_target_dir.map(PathBuf::from) {
        Some(path) if path.is_absolute() => path,
        Some(path) => env::current_dir()
            .expect("resolve current directory for Cargo target")
            .join(path),
        None => app_root.join("component/target"),
    }
}

fn component_filename() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after Unix epoch")
        .as_nanos();
    format!("fna-x-component-{}-{timestamp}.wasm", std::process::id())
}

#[cfg(test)]
#[path = "../target_directory_tests.rs"]
mod target_directory_tests;
#[cfg(test)]
#[path = "../x_error_tests.rs"]
mod x_error_tests;
#[cfg(test)]
#[path = "../x_metrics_error_tests.rs"]
mod x_metrics_error_tests;
#[cfg(test)]
#[path = "../x_metrics_smoke_tests.rs"]
mod x_metrics_smoke_tests;
#[cfg(test)]
#[path = "../x_oauth_lifecycle_tests.rs"]
mod x_oauth_lifecycle_tests;
#[cfg(test)]
#[path = "../x_package_tests.rs"]
mod x_package_tests;
#[cfg(test)]
#[path = "../x_read_smoke_tests.rs"]
mod x_read_smoke_tests;
#[cfg(test)]
#[path = "../x_runtime_support.rs"]
mod x_runtime_support;
#[cfg(test)]
#[path = "../x_test_support.rs"]
mod x_test_support;
#[cfg(test)]
#[path = "../x_write_smoke_tests.rs"]
mod x_write_smoke_tests;
