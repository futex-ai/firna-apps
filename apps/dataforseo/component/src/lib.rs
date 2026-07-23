//! Rust guest source for the DataForSEO app WebAssembly component.
#![warn(missing_docs, unreachable_pub)]

mod dataforseo;

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

struct DataForSeoComponent;

impl Guest for DataForSeoComponent {
    fn call_tool(request: String) -> String {
        dataforseo::call_tool(&request)
    }
}

bindings::export!(DataForSeoComponent with_types_in bindings);

pub(crate) fn host_http_request(request: &str) -> String {
    bindings::host_http_request(request)
}
