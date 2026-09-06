//! Typed GitHub status, workflow, merge-queue, and policy signal objects.

use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct PullRequestReference {
    pub(crate) number: u64,
    pub(crate) head: Option<SignalHead>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct SignalHead {
    #[serde(rename = "ref", alias = "head_branch")]
    pub(crate) name: Option<String>,
    pub(crate) sha: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct CheckRun {
    pub(crate) id: u64,
    pub(crate) name: String,
    pub(crate) status: String,
    pub(crate) conclusion: Option<String>,
    pub(crate) head_sha: String,
    pub(crate) html_url: Option<String>,
    pub(crate) check_suite: Option<SignalHead>,
    #[serde(default)]
    pub(crate) pull_requests: Vec<PullRequestReference>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct CheckSuite {
    pub(crate) id: u64,
    pub(crate) status: String,
    pub(crate) conclusion: Option<String>,
    pub(crate) head_branch: Option<String>,
    pub(crate) head_sha: String,
    #[serde(default)]
    pub(crate) pull_requests: Vec<PullRequestReference>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct WorkflowJob {
    pub(crate) id: u64,
    pub(crate) name: String,
    pub(crate) status: String,
    pub(crate) conclusion: Option<String>,
    pub(crate) head_branch: Option<String>,
    pub(crate) head_sha: String,
    pub(crate) html_url: Option<String>,
    #[serde(default)]
    pub(crate) pull_requests: Vec<PullRequestReference>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct WorkflowRun {
    pub(crate) id: u64,
    pub(crate) name: String,
    pub(crate) status: String,
    pub(crate) conclusion: Option<String>,
    pub(crate) head_branch: Option<String>,
    pub(crate) head_sha: String,
    pub(crate) html_url: Option<String>,
    #[serde(default)]
    pub(crate) pull_requests: Vec<PullRequestReference>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct StatusBranch {
    pub(crate) name: String,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct MergeGroup {
    pub(crate) head_ref: String,
    pub(crate) head_sha: String,
    pub(crate) base_ref: String,
    pub(crate) base_sha: String,
}
