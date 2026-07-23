//! GitHub tool dispatch and shared service context.

mod common;
mod issues;
mod pull_requests;
mod repositories;
mod repository_file;
mod repository_projection;

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::github::error::{GitHubError, InvalidReason};
use crate::github::input::AppToolCall;
use crate::github::provider::{Clock, GitHubProvider};

pub(crate) struct GitHubToolService<'a> {
    provider: &'a dyn GitHubProvider,
    clock: &'a dyn Clock,
}

impl<'a> GitHubToolService<'a> {
    pub(crate) fn new(provider: &'a dyn GitHubProvider, clock: &'a dyn Clock) -> Self {
        Self { provider, clock }
    }

    pub(crate) fn call(&self, call: AppToolCall) -> Result<Value, GitHubError> {
        match call.tool_name.as_str() {
            "github_list_repositories" => repositories::list(self, call),
            "github_search_code" => repositories::search(self, call),
            "github_read_file" => repository_file::read(self, call),
            "github_read_pr" => pull_requests::read(self, call),
            "github_read_issue" => issues::read(self, call),
            _ => Err(GitHubError::InvalidRequest {
                reason: InvalidReason::UnknownTool,
            }),
        }
    }

    fn parse<T: DeserializeOwned>(&self, value: Value) -> Result<T, GitHubError> {
        match serde_json::from_value(value) {
            Ok(input) => Ok(input),
            Err(_) => Err(GitHubError::InvalidRequest {
                reason: InvalidReason::InvalidToolCall,
            }),
        }
    }
}
