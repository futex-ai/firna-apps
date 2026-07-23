//! Rust guest source for the Exa app Wasm component.
#![warn(missing_docs, unreachable_pub)]

mod exa;

use bindings::Guest;

#[allow(missing_docs)]
mod bindings {
    wit_bindgen::generate!({
        inline: r#"
            package firna:app;

            world component {
                import host-http-request: func(request: string) -> string;
                export call-tool: func(request: string) -> string;
            }
        "#,
    });
}

struct ExaComponent;

impl Guest for ExaComponent {
    fn call_tool(request: String) -> String {
        exa::call_tool(&request)
    }
}

bindings::export!(ExaComponent with_types_in bindings);

pub(crate) fn host_http_request(request: &str) -> String {
    bindings::host_http_request(request)
}
