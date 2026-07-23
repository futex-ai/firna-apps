//! Trait-backed GitHub provider boundary.

use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::github::error::GitHubError;
use serde_json::Value;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProviderMediaType {
    Json,
    TextMatch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProviderRequest {
    pub(crate) path: String,
    pub(crate) query: BTreeMap<String, String>,
    pub(crate) installation_id: String,
    pub(crate) media_type: ProviderMediaType,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ProviderResponse {
    pub(crate) status: u16,
    pub(crate) headers: BTreeMap<String, String>,
    pub(crate) body: Value,
    pub(crate) body_truncated: bool,
}

#[cfg_attr(test, unimock::unimock(api = [GitHubProviderGet]))]
pub(crate) trait GitHubProvider {
    fn get(&self, request: ProviderRequest) -> Result<ProviderResponse, GitHubError>;
}

#[cfg_attr(test, unimock::unimock(api = [ClockNow]))]
pub(crate) trait Clock {
    fn now_unix_seconds(&self) -> u64;
}

pub(crate) struct HostGitHubProvider;

impl GitHubProvider for HostGitHubProvider {
    fn get(&self, request: ProviderRequest) -> Result<ProviderResponse, GitHubError> {
        crate::github::host::get(request)
    }
}

pub(crate) struct SystemClock;

impl Clock for SystemClock {
    fn now_unix_seconds(&self) -> u64 {
        match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(duration) => duration.as_secs(),
            Err(_) => 0,
        }
    }
}
