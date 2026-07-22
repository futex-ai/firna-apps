//! JSON DTOs used by the HTTP component.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub(crate) struct AppToolCall {
    pub(crate) tool_name: String,
    pub(crate) input: Value,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HttpRequestInput {
    #[serde(default)]
    pub(crate) method: Option<String>,
    pub(crate) url: String,
    #[serde(default)]
    pub(crate) query: BTreeMap<String, String>,
    #[serde(default)]
    pub(crate) headers: BTreeMap<String, String>,
    #[serde(default)]
    pub(crate) body_json: Option<Value>,
    #[serde(default)]
    pub(crate) body_text: Option<String>,
    #[serde(default)]
    pub(crate) timeout_seconds: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl HttpMethod {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_uppercase().as_str() {
            "GET" => Some(Self::Get),
            "POST" => Some(Self::Post),
            "PUT" => Some(Self::Put),
            "PATCH" => Some(Self::Patch),
            "DELETE" => Some(Self::Delete),
            _ => None,
        }
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct NormalizedHttpRequest {
    pub(crate) method: String,
    pub(crate) url: String,
    pub(crate) query: BTreeMap<String, String>,
    pub(crate) headers: BTreeMap<String, String>,
    pub(crate) body_json: Option<Value>,
    pub(crate) body_text: Option<String>,
    pub(crate) timeout_seconds: Option<u64>,
    pub(crate) credential: Option<Value>,
    pub(crate) credential_injection: Option<Value>,
}
