//! Repository, code-search, and content response models.

use serde::Deserialize;

use crate::github::models::{RequiredNullable, required_nullable};

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct Repository {
    pub(crate) id: u64,
    pub(crate) full_name: String,
    #[serde(deserialize_with = "required_nullable")]
    pub(crate) description: RequiredNullable<String>,
    pub(crate) visibility: String,
    pub(crate) archived: bool,
    pub(crate) fork: bool,
    pub(crate) default_branch: String,
    #[serde(deserialize_with = "required_nullable")]
    pub(crate) language: RequiredNullable<String>,
    #[serde(deserialize_with = "required_nullable")]
    pub(crate) pushed_at: RequiredNullable<String>,
    pub(crate) html_url: String,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct InstallationRepositoriesResponse {
    pub(crate) total_count: u64,
    pub(crate) repositories: Vec<Repository>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct CodeSearchResponse {
    pub(crate) total_count: u64,
    pub(crate) incomplete_results: bool,
    pub(crate) items: Vec<CodeSearchItem>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct CodeSearchItem {
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) sha: String,
    pub(crate) html_url: String,
    pub(crate) repository: RepositoryRef,
    #[serde(default)]
    pub(crate) text_matches: Vec<TextMatch>,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct RepositoryRef {
    pub(crate) full_name: String,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct TextMatch {
    pub(crate) fragment: String,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct FileContent {
    pub(crate) path: String,
    pub(crate) sha: String,
    pub(crate) size: u64,
    #[serde(deserialize_with = "required_nullable")]
    pub(crate) html_url: RequiredNullable<String>,
    pub(crate) encoding: String,
    pub(crate) content: String,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct CommitListEntry {
    sha: String,
    commit: CommitMetadata,
}

impl CommitListEntry {
    pub(crate) fn sha(&self) -> &str {
        &self.sha
    }

    pub(crate) fn root_tree_sha(&self) -> &str {
        &self.commit.tree.sha
    }
}

#[derive(Clone, Debug, Deserialize)]
struct CommitMetadata {
    tree: CommitTree,
}

#[derive(Clone, Debug, Deserialize)]
struct CommitTree {
    sha: String,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct GitTree {
    pub(crate) sha: String,
    pub(crate) tree: Vec<GitTreeEntry>,
    pub(crate) truncated: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct GitTreeEntry {
    pub(crate) path: String,
    pub(crate) mode: GitTreeEntryMode,
    #[serde(rename = "type")]
    pub(crate) object_type: GitObjectType,
    pub(crate) sha: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub(crate) enum GitTreeEntryMode {
    #[serde(rename = "100644")]
    File,
    #[serde(rename = "100755")]
    Executable,
    #[serde(rename = "040000")]
    Directory,
    #[serde(rename = "160000")]
    Submodule,
    #[serde(rename = "120000")]
    Symlink,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum GitObjectType {
    Blob,
    Tree,
    Commit,
}
