//! Typed GitHub REST response models.

mod common;
mod issues;
mod pull_requests;
mod repositories;

pub(crate) use common::{ProviderUser, RequiredNullable, required_nullable};
pub(crate) use issues::{IssueComment, IssueDetail, IssueLabel, IssueMilestone};
pub(crate) use pull_requests::{PullRequestDetail, PullRequestFile, PullRequestRef};
pub(crate) use repositories::{
    CodeSearchItem, CodeSearchResponse, CommitListEntry, FileContent, GitObjectType, GitTree,
    GitTreeEntry, GitTreeEntryMode, InstallationRepositoriesResponse, Repository,
};
