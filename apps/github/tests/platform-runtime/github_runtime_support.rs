use std::sync::OnceLock;

use fna_apps_wasm::{DynWasmHost, WasmComponentRuntime, WasmRuntimeLimits};

use crate::package;

pub(crate) fn runtime_with_host(host: DynWasmHost) -> WasmComponentRuntime {
    compiled_runtime().with_host(host)
}

fn compiled_runtime() -> &'static WasmComponentRuntime {
    static COMPILED_RUNTIME: OnceLock<WasmComponentRuntime> = OnceLock::new();

    COMPILED_RUNTIME.get_or_init(|| {
        WasmComponentRuntime::compile(package(), WasmRuntimeLimits::default()).unwrap()
    })
}
