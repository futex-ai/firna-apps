//! Runtime compilation tests for the GitHub tools and events component.

use fna_apps_wasm::{WasmComponentRuntime, WasmRuntimeLimits};

use crate::package;

#[test]
fn github_component_compiles_with_the_platform_runtime() {
    WasmComponentRuntime::compile(package(), WasmRuntimeLimits::default())
        .expect("GitHub component should compile");
}
