//! X app component implementation.

mod errors;
mod host;
mod metrics_types;
mod response;
mod service;
mod types;

pub(crate) fn call_tool(request: &str) -> String {
    service::runner::call_tool(request, &host::ImportedXHttpClient)
}
