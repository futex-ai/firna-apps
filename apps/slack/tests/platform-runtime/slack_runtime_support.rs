use std::sync::OnceLock;

use fna_apps_interface::runtime::WebhookHeader;
use fna_apps_wasm::{DynWasmHost, WasmComponentRuntime, WasmRuntimeLimits};

use crate::package;

pub(crate) fn runtime_with_host(host: DynWasmHost) -> WasmComponentRuntime {
    compiled_runtime().with_host(host)
}

pub(crate) fn webhook_header(name: &str, value: &str) -> WebhookHeader {
    WebhookHeader {
        name: name.to_owned(),
        value: value.as_bytes().to_vec(),
    }
}

fn compiled_runtime() -> &'static WasmComponentRuntime {
    static COMPILED_RUNTIME: OnceLock<WasmComponentRuntime> = OnceLock::new();

    COMPILED_RUNTIME.get_or_init(|| {
        WasmComponentRuntime::compile(package(), WasmRuntimeLimits::default()).unwrap()
    })
}
