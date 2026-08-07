//! Rust guest source for the GitHub app Wasm component.
#![warn(missing_docs, unreachable_pub)]

mod github;

use bindings::Guest;

#[allow(missing_docs)]
mod bindings {
    wit_bindgen::generate!({
        inline: r#"
            package firna:app;

            world component {
                import host-http-request: func(request: string) -> string;
                import host-hmac-sha256: func(request: string) -> string;
                export call-tool: func(request: string) -> string;
                export verify-webhook: func(request: string) -> string;
                export webhook-response: func(request: string) -> string;
                export normalize-event: func(request: string) -> string;
            }
        "#,
    });
}

struct GitHubComponent;

impl Guest for GitHubComponent {
    fn call_tool(request: String) -> String {
        github::call_tool(&request)
    }

    fn verify_webhook(request: String) -> String {
        github::verify_webhook(&request)
    }

    fn webhook_response(request: String) -> String {
        github::webhook_response(&request)
    }

    fn normalize_event(request: String) -> String {
        github::normalize_event(&request)
    }
}

bindings::export!(GitHubComponent with_types_in bindings);

pub(crate) fn host_http_request(request: &str) -> String {
    bindings::host_http_request(request)
}

pub(crate) fn host_hmac_sha256(request: &str) -> String {
    bindings::host_hmac_sha256(request)
}
