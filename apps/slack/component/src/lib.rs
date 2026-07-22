//! Rust guest source for the Slack app Wasm component.
#![warn(missing_docs, unreachable_pub)]

mod slack;

use bindings::Guest;

#[allow(missing_docs)]
mod bindings {
    wit_bindgen::generate!({
        inline: r#"
            package firna:app;

            world component {
                import host-http-request: func(request: string) -> string;
                import host-hmac-sha256: func(request: string) -> string;
                import host-log: func(request: string) -> string;
                export call-tool: func(request: string) -> string;
                export verify-webhook: func(request: string) -> string;
                export webhook-response: func(request: string) -> string;
                export normalize-event: func(request: string) -> string;
            }
        "#,
    });
}

struct SlackComponent;

impl Guest for SlackComponent {
    fn call_tool(request: String) -> String {
        slack::call_tool(&request)
    }

    fn verify_webhook(request: String) -> String {
        slack::verify_webhook(&request)
    }

    fn webhook_response(request: String) -> String {
        slack::webhook_response(&request)
    }

    fn normalize_event(request: String) -> String {
        slack::normalize_event(&request)
    }
}

bindings::export!(SlackComponent with_types_in bindings);

pub(crate) fn host_http_request(request: &str) -> String {
    bindings::host_http_request(request)
}

pub(crate) fn host_hmac_sha256(request: &str) -> String {
    bindings::host_hmac_sha256(request)
}
