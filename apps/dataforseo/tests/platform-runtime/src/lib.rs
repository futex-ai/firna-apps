//! DataForSEO app conformance tests against the platform Wasm host.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use fna_apps_interface::manifest::AppManifest;
use fna_apps_wasm::WasmAppPackage;

/// Returns the DataForSEO manifest under test.
pub fn manifest() -> AppManifest {
    let yaml =
        fs::read_to_string(app_root().join("manifest.yaml")).expect("read DataForSEO manifest");
    AppManifest::from_yaml(&yaml).expect("DataForSEO manifest should parse")
}

/// Returns a freshly built DataForSEO component for runtime tests.
pub fn component_bytes() -> Vec<u8> {
    static COMPONENT_BYTES: OnceLock<Vec<u8>> = OnceLock::new();
    COMPONENT_BYTES.get_or_init(build_component_bytes).clone()
}

/// Returns the DataForSEO package tested by the platform runtime harness.
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
        .expect("start DataForSEO component cargo build");
    assert!(
        cargo_status.success(),
        "DataForSEO component cargo build failed"
    );
    let core_wasm = cargo_target_dir(&app_root)
        .join("wasm32-unknown-unknown/release/fna_app_dataforseo_component.wasm");
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
        "DataForSEO component conversion failed"
    );
    let bytes = fs::read(&component_wasm).expect("read DataForSEO Wasm component");
    let _ = fs::remove_file(component_wasm);
    bytes
}

fn app_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn cargo_target_dir(app_root: &std::path::Path) -> PathBuf {
    let Some(configured) = env::var_os("CARGO_TARGET_DIR") else {
        return app_root.join("component/target");
    };
    let configured = PathBuf::from(configured);
    if configured.is_absolute() {
        configured
    } else {
        env::current_dir()
            .expect("resolve current directory for Cargo target")
            .join(configured)
    }
}

fn component_filename() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after Unix epoch")
        .as_nanos();
    format!(
        "fna-dataforseo-component-{}-{timestamp}.wasm",
        std::process::id()
    )
}

#[cfg(test)]
#[path = "../dataforseo_package_tests.rs"]
mod dataforseo_package_tests;
#[cfg(test)]
#[path = "../dataforseo_runtime_support.rs"]
mod dataforseo_runtime_support;
#[cfg(test)]
#[path = "../dataforseo_tool_cases.rs"]
mod dataforseo_tool_cases;
#[cfg(test)]
#[path = "../dataforseo_tool_smoke_tests.rs"]
mod dataforseo_tool_smoke_tests;
