//! GitHub app component implementation.

mod error;
mod host;
mod input;
mod input_validation;
mod models;
mod pagination;
mod projection;
mod provider;
mod provider_response;
mod tools;

use serde_json::Value;

use self::error::GitHubError;
use self::provider::{HostGitHubProvider, SystemClock};
use self::tools::GitHubToolService;

pub(crate) fn call_tool(request: &str) -> String {
    let provider = HostGitHubProvider;
    let clock = SystemClock;
    call_tool_with(request, &provider, &clock)
}

fn call_tool_with(
    request: &str,
    provider: &dyn provider::GitHubProvider,
    clock: &dyn provider::Clock,
) -> String {
    let result = match serde_json::from_str(request) {
        Ok(call) => GitHubToolService::new(provider, clock).call(call),
        Err(_) => Err(GitHubError::InvalidRequest {
            reason: error::InvalidReason::InvalidToolCall,
        }),
    };
    encode_result(result)
}

fn encode_result(result: Result<Value, GitHubError>) -> String {
    let value = match result {
        Ok(output) => output,
        Err(error) => error.into_value(),
    };
    match serde_json::to_string(&value) {
        Ok(encoded) => encoded,
        Err(_) => String::from(r#"{"ok":false,"error":"provider_contract_error"}"#),
    }
}

#[cfg(test)]
#[path = "_tests_/mod.rs"]
mod tests;
