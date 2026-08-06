//! Wasm component ABI for the credential-only GitHub app package.
#![warn(missing_docs, unreachable_pub)]

use bindings::Guest;

#[allow(missing_docs)]
mod bindings {
    wit_bindgen::generate!({
        inline: r#"
            package firna:app;

            world component {
                export call-tool: func(request: string) -> string;
            }
        "#,
    });
}

struct GithubComponent;

impl Guest for GithubComponent {
    fn call_tool(_request: String) -> String {
        unsupported_tool_response()
    }
}

bindings::export!(GithubComponent with_types_in bindings);

fn unsupported_tool_response() -> String {
    String::from(r#"{"ok":false,"error":"invalid_request","reason":"no_tools_declared"}"#)
}

#[cfg(test)]
#[path = "_tests_/component_tests.rs"]
mod component_tests;
