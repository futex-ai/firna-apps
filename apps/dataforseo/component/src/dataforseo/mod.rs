//! DataForSEO tool dispatch and provider integration.

mod envelope;
mod error;
mod host;
mod input;
mod output;
mod tools;
mod validation;

use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
struct AppToolCall {
    installation_id: String,
    tool_name: String,
    input: Value,
}

pub(crate) fn call_tool(request: &str) -> String {
    let result = match serde_json::from_str::<AppToolCall>(request) {
        Ok(call) => {
            let client = host::WasmProviderClient::new(&call.installation_id);
            tools::call(&client, &call.tool_name, call.input)
        }
        Err(_) => Err(error::Error::InvalidRequest("invalid_tool_call")),
    };
    match result {
        Ok(output) => output.to_string(),
        Err(error) => error.into_output().to_string(),
    }
}

#[cfg(test)]
#[path = "_tests_/mod.rs"]
mod tests;
