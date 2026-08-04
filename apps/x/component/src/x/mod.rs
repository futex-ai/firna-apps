//! X app component implementation.

mod errors;
mod host;
mod response;
mod service;
mod types;

pub(crate) fn call_tool(request: &str) -> String {
    service::call_tool(request, &host::ImportedXHttpClient)
}
