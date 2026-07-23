//! HTTP app runtime conformance tests against the platform Wasm host.

use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use fna_apps_interface::manifest::AppManifest;
use fna_apps_wasm::WasmAppPackage;

/// Returns the HTTP manifest under test.
pub fn manifest() -> AppManifest {
    let path = app_root().join("manifest.yaml");
    let yaml = fs::read_to_string(path).expect("read HTTP manifest");
    AppManifest::from_yaml(&yaml).expect("HTTP manifest should parse")
}

/// Returns a freshly built HTTP component for runtime tests.
pub fn component_bytes() -> Vec<u8> {
    static COMPONENT_BYTES: OnceLock<Vec<u8>> = OnceLock::new();

    COMPONENT_BYTES.get_or_init(build_component_bytes).clone()
}

/// Returns the HTTP package tested by the platform runtime harness.
pub fn package() -> WasmAppPackage {
    WasmAppPackage {
        manifest: manifest(),
        component_bytes: component_bytes(),
    }
}

fn build_component_bytes() -> Vec<u8> {
    let app_root = app_root();
    let component_manifest = app_root.join("component").join("Cargo.toml");
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
        .expect("start HTTP component cargo build");
    assert!(cargo_status.success(), "HTTP component cargo build failed");

    let core_wasm = component_target_dir(&app_root, env::var_os("CARGO_TARGET_DIR"))
        .join("wasm32-unknown-unknown")
        .join("release")
        .join("fna_app_http_component.wasm");
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
        .expect("start wasm-tools component build");
    assert!(
        wasm_tools_status.success(),
        "HTTP component wasm-tools build failed"
    );

    let bytes = fs::read(&component_wasm).expect("read HTTP Wasm component");
    let _ = fs::remove_file(component_wasm);
    bytes
}

fn app_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn component_target_dir(app_root: &Path, cargo_target_dir: Option<OsString>) -> PathBuf {
    match cargo_target_dir {
        Some(path) => PathBuf::from(path),
        None => app_root.join("component").join("target"),
    }
}

fn component_filename() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after Unix epoch")
        .as_nanos();
    format!("fna-http-component-{}-{timestamp}.wasm", std::process::id())
}

#[cfg(test)]
#[path = "../http_package_tests.rs"]
mod http_package_tests;
#[cfg(test)]
#[path = "../http_runtime_support.rs"]
mod http_runtime_support;
#[cfg(test)]
#[path = "../http_tool_smoke_tests.rs"]
mod http_tool_smoke_tests;
#[cfg(test)]
#[path = "../target_directory_tests.rs"]
mod target_directory_tests;
