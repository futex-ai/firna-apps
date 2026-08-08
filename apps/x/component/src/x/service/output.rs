//! Stable component success and error envelope encoding.

use serde::Serialize;

use crate::x::errors::{ErrorEnvelope, ToolError};
use crate::x::types::PricedToolSuccess;

#[derive(Serialize)]
#[serde(untagged)]
enum ComponentOutput {
    Success(PricedToolSuccess),
    Error(ErrorEnvelope),
}

pub(super) fn encode_output(output: Result<PricedToolSuccess, ToolError>) -> String {
    let output = match output {
        Ok(success) => ComponentOutput::Success(success),
        Err(error) => ComponentOutput::Error(error.envelope()),
    };
    serde_json::to_string(&output)
        .unwrap_or_else(|_| String::from("{\"ok\":false,\"error\":\"provider_contract_error\"}"))
}
