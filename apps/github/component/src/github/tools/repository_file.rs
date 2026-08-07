//! Exact repository file reads with Git object-type verification.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::github::error::{GitHubError, InvalidReason};
use crate::github::input::{
    AppToolCall, ReadFileInput, encoded_path, encoded_segment, git_ref, owner, path, repository,
};
use crate::github::models::{
    CommitListEntry, FileContent, GitObjectType, GitTree, GitTreeEntry, GitTreeEntryMode,
};
use crate::github::projection::{
    ExactKind, decode_file_content, ensure_budget, exact, nullable_exact,
};
use crate::github::provider::ProviderMediaType;
use crate::github::tools::GitHubToolService;

pub(super) fn read(
    service: &GitHubToolService<'_>,
    call: AppToolCall,
) -> Result<Value, GitHubError> {
    let input = service.parse::<ReadFileInput>(call.input.clone())?;
    let owner = owner(&input.owner)?;
    let repository = repository(&input.repository)?;
    let requested_path = path(&input.path)?;
    let requested_ref = git_ref(input.git_ref)?;
    let repository_path = format!(
        "/repos/{}/{}",
        encoded_segment(&owner),
        encoded_segment(&repository)
    );
    let (commit_sha, root_tree_sha) =
        resolve_revision(service, &call, &repository_path, requested_ref.as_deref())?;
    let blob_sha = verify_file_entry(
        service,
        &call,
        &repository_path,
        &root_tree_sha,
        &requested_path,
    )?;
    let endpoint = format!(
        "{repository_path}/contents/{}",
        encoded_path(&requested_path)
    );
    let query = BTreeMap::from([(String::from("ref"), commit_sha)]);
    let (raw_response, _) =
        service.get::<Value>(&call, endpoint, query, ProviderMediaType::Json, None)?;
    let response = file_response(raw_response)?;
    if response.path != requested_path {
        return unsupported_content();
    }
    let response_sha = exact(&response.sha, ExactKind::Identifier)?;
    if response_sha != blob_sha {
        return Err(GitHubError::InvalidProviderResponse);
    }
    let content = decode_file_content(&response.encoding, &response.content, response.size)?;
    let output = json!({
        "repository_full_name": format!("{owner}/{repository}"),
        "path": exact(&response.path, ExactKind::PathOrTitle)?,
        "ref": requested_ref,
        "sha": response_sha,
        "size": response.size,
        "html_url": nullable_exact(response.html_url.0.as_deref(), ExactKind::Url)?,
        "content": content
    });
    ensure_budget(&output)?;
    Ok(output)
}

fn resolve_revision(
    service: &GitHubToolService<'_>,
    call: &AppToolCall,
    repository_path: &str,
    requested_ref: Option<&str>,
) -> Result<(String, String), GitHubError> {
    let mut query = BTreeMap::from([(String::from("per_page"), String::from("1"))]);
    if let Some(requested_ref) = requested_ref {
        query.insert(String::from("sha"), requested_ref.to_owned());
    }
    let (commits, _) = service.get::<Vec<CommitListEntry>>(
        call,
        format!("{repository_path}/commits"),
        query,
        ProviderMediaType::Json,
        None,
    )?;
    let commit = match commits.as_slice() {
        [commit] => commit,
        [] => return Err(GitHubError::NotFoundOrNotAccessible),
        _ => return Err(GitHubError::InvalidProviderResponse),
    };
    Ok((
        exact(commit.sha(), ExactKind::Identifier)?,
        exact(commit.root_tree_sha(), ExactKind::Identifier)?,
    ))
}

fn verify_file_entry(
    service: &GitHubToolService<'_>,
    call: &AppToolCall,
    repository_path: &str,
    root_tree_sha: &str,
    requested_path: &str,
) -> Result<String, GitHubError> {
    let mut tree_sha = root_tree_sha.to_owned();
    let mut segments = requested_path.split('/').peekable();
    while let Some(segment) = segments.next() {
        let tree = fetch_tree(service, call, repository_path, &tree_sha)?;
        let entry = unique_entry(&tree, segment)?;
        let entry_sha = exact(&entry.sha, ExactKind::Identifier)?;
        match (classify_entry(entry)?, segments.peek().is_none()) {
            (TreeEntryKind::File, true) => return Ok(entry_sha),
            (TreeEntryKind::Directory, false) => tree_sha = entry_sha,
            _ => return unsupported_content(),
        }
    }
    Err(GitHubError::InvalidProviderResponse)
}

fn fetch_tree(
    service: &GitHubToolService<'_>,
    call: &AppToolCall,
    repository_path: &str,
    tree_sha: &str,
) -> Result<GitTree, GitHubError> {
    let (tree, _) = service.get::<GitTree>(
        call,
        format!("{repository_path}/git/trees/{}", encoded_segment(tree_sha)),
        BTreeMap::new(),
        ProviderMediaType::Json,
        None,
    )?;
    if tree.truncated {
        return Err(GitHubError::ProviderResponseTooLarge { retry_input: None });
    }
    if tree.sha != tree_sha {
        return Err(GitHubError::InvalidProviderResponse);
    }
    Ok(tree)
}

fn unique_entry<'a>(
    tree: &'a GitTree,
    requested_segment: &str,
) -> Result<&'a GitTreeEntry, GitHubError> {
    let mut matches = tree
        .tree
        .iter()
        .filter(|entry| entry.path == requested_segment);
    let entry = matches.next().ok_or(GitHubError::NotFoundOrNotAccessible)?;
    if matches.next().is_some() {
        return Err(GitHubError::InvalidProviderResponse);
    }
    Ok(entry)
}

#[derive(Clone, Copy)]
enum TreeEntryKind {
    File,
    Directory,
    Unsupported,
}

fn classify_entry(entry: &GitTreeEntry) -> Result<TreeEntryKind, GitHubError> {
    match (entry.mode, entry.object_type) {
        (GitTreeEntryMode::File | GitTreeEntryMode::Executable, GitObjectType::Blob) => {
            Ok(TreeEntryKind::File)
        }
        (GitTreeEntryMode::Directory, GitObjectType::Tree) => Ok(TreeEntryKind::Directory),
        (GitTreeEntryMode::Submodule, GitObjectType::Commit)
        | (GitTreeEntryMode::Symlink, GitObjectType::Blob) => Ok(TreeEntryKind::Unsupported),
        _ => Err(GitHubError::InvalidProviderResponse),
    }
}

fn file_response(value: Value) -> Result<FileContent, GitHubError> {
    if value.is_array() {
        return unsupported_content();
    }
    match value.get("type") {
        Some(Value::String(content_type)) if content_type == "file" => {}
        Some(Value::String(_)) => return unsupported_content(),
        _ => return Err(GitHubError::InvalidProviderResponse),
    }
    match serde_json::from_value(value) {
        Ok(response) => Ok(response),
        Err(_) => Err(GitHubError::InvalidProviderResponse),
    }
}

fn unsupported_content<T>() -> Result<T, GitHubError> {
    Err(GitHubError::InvalidRequest {
        reason: InvalidReason::UnsupportedContent,
    })
}
