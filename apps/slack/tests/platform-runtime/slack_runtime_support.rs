use std::sync::OnceLock;

use fna_apps_interface::runtime::WebhookHeader;
use fna_apps_wasm::{DynWasmHost, WasmComponentRuntime, WasmRuntimeLimits};

use crate::package;

pub(crate) fn runtime_with_host(host: DynWasmHost) -> WasmComponentRuntime {
    compiled_runtime().with_host(host)
}

pub(crate) fn webhook_headers(timestamp: i64, signature: &str) -> Vec<WebhookHeader> {
    vec![
        WebhookHeader {
            name: String::from("x-slack-request-timestamp"),
            value: timestamp.to_string().into_bytes(),
        },
        WebhookHeader {
            name: String::from("x-slack-signature"),
            value: signature.as_bytes().to_vec(),
        },
    ]
}

fn compiled_runtime() -> &'static WasmComponentRuntime {
    static COMPILED_RUNTIME: OnceLock<WasmComponentRuntime> = OnceLock::new();

    COMPILED_RUNTIME.get_or_init(|| {
        WasmComponentRuntime::compile(package(), WasmRuntimeLimits::default()).unwrap()
    })
}
