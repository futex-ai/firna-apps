//! Repository listing, code search, and file-reading tools.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::github::error::{GitHubError, InvalidReason};
use crate::github::input::{
    AppToolCall, ListRepositoriesInput, ReadFileInput, SearchCodeInput, encoded_path,
    encoded_segment, git_ref, language, owner, page, page_size, path, repository, result_offset,
    search_path, search_term,
};
use crate::github::models::{CodeSearchResponse, FileContent, InstallationRepositoriesResponse};
use crate::github::pagination::next_page;
use crate::github::projection::{ExactKind, decode_file_content, exact, nullable_exact};
use crate::github::provider::ProviderMediaType;
use crate::github::tools::GitHubToolService;
use crate::github::tools::repository_projection::{
    build_search_query, code_match, reduce_repository_previews, reduce_search_previews,
    repository as project_repository,
};

pub(super) fn list(
    service: &GitHubToolService<'_>,
    call: AppToolCall,
) -> Result<Value, GitHubError> {
    let input = service.parse::<ListRepositoriesInput>(call.input.clone())?;
    let page = page(input.page)?;
    let per_page = page_size(input.per_page, 30, 50)?;
    let query = BTreeMap::from([
        (String::from("page"), page.to_string()),
        (String::from("per_page"), per_page.to_string()),
    ]);
    let (response, headers) = service.get::<InstallationRepositoriesResponse>(
        &call,
        String::from("/installation/repositories"),
        query,
        ProviderMediaType::Json,
        None,
    )?;
    if response.repositories.len() > per_page as usize
        || response.total_count < response.repositories.len() as u64
    {
        return Err(GitHubError::InvalidProviderResponse);
    }
    let rows = response
        .repositories
        .iter()
        .map(project_repository)
        .collect::<Result<Vec<_>, _>>()?;
    let mut output = json!({ "repositories": rows, "page": page });
    if let Some(next_page) = next_page(&headers) {
        output["next_page"] = json!(next_page);
    }
    reduce_repository_previews(&mut output)?;
    Ok(output)
}

pub(super) fn search(
    service: &GitHubToolService<'_>,
    call: AppToolCall,
) -> Result<Value, GitHubError> {
    let input = service.parse::<SearchCodeInput>(call.input.clone())?;
    let query_text = search_term(&input.query)?;
    let qualifier = match (input.owner.as_deref(), input.repository.as_deref()) {
        (Some(owner_value), Some(repository_value)) => {
            Some((owner(owner_value)?, repository(repository_value)?))
        }
        (None, None) => None,
        (None, Some(_)) => {
            return Err(GitHubError::InvalidRequest {
                reason: InvalidReason::RepositoryQualifierRequiresOwner,
            });
        }
        (Some(_), None) => {
            return Err(GitHubError::InvalidRequest {
                reason: InvalidReason::OwnerQualifierRequiresRepository,
            });
        }
    };
    let language = language(input.language)?;
    let search_path = search_path(input.path)?;
    let page = page(input.page)?;
    let per_page = page_size(input.per_page, 20, 20)?;
    if result_offset(page, per_page)? >= 1_000 {
        return Err(GitHubError::InvalidRequest {
            reason: InvalidReason::ResultWindowExceeded,
        });
    }
    let query = build_search_query(
        &query_text,
        qualifier.as_ref(),
        language.as_deref(),
        search_path.as_deref(),
    );
    let provider_query = BTreeMap::from([
        (String::from("q"), query),
        (String::from("page"), page.to_string()),
        (String::from("per_page"), per_page.to_string()),
    ]);
    let (response, headers) = service.get::<CodeSearchResponse>(
        &call,
        String::from("/search/code"),
        provider_query,
        ProviderMediaType::TextMatch,
        None,
    )?;
    if response.items.len() > per_page as usize
        || response
            .items
            .iter()
            .any(|item| item.text_matches.len() > 5)
    {
        return Err(GitHubError::InvalidProviderResponse);
    }
    let matches = response
        .items
        .iter()
        .map(code_match)
        .collect::<Result<Vec<_>, _>>()?;
    let mut output = json!({
        "total_count": response.total_count,
        "incomplete_results": response.incomplete_results,
        "matches": matches,
        "page": page
    });
    if let Some(candidate) = next_page(&headers)
        && result_offset(candidate, per_page)? < 1_000
    {
        output["next_page"] = json!(candidate);
    }
    reduce_search_previews(&mut output)?;
    Ok(output)
}

pub(super) fn read_file(
    service: &GitHubToolService<'_>,
    call: AppToolCall,
) -> Result<Value, GitHubError> {
    let input = service.parse::<ReadFileInput>(call.input.clone())?;
    let owner = owner(&input.owner)?;
    let repository = repository(&input.repository)?;
    let requested_path = path(&input.path)?;
    let git_ref = git_ref(input.git_ref)?;
    let mut query = BTreeMap::new();
    if let Some(git_ref) = &git_ref {
        query.insert(String::from("ref"), git_ref.clone());
    }
    let endpoint = format!(
        "/repos/{}/{}/contents/{}",
        encoded_segment(&owner),
        encoded_segment(&repository),
        encoded_path(&requested_path)
    );
    let (raw_response, _) =
        service.get::<Value>(&call, endpoint, query, ProviderMediaType::Json, None)?;
    let response = file_response(raw_response)?;
    if response.path != requested_path {
        return Err(GitHubError::InvalidRequest {
            reason: InvalidReason::UnsupportedContent,
        });
    }
    let content = decode_file_content(&response.encoding, &response.content, response.size)?;
    let output = json!({
        "repository_full_name": format!("{owner}/{repository}"),
        "path": exact(&response.path, ExactKind::PathOrTitle)?,
        "ref": git_ref,
        "sha": exact(&response.sha, ExactKind::Identifier)?,
        "size": response.size,
        "html_url": nullable_exact(response.html_url.0.as_deref(), ExactKind::Url)?,
        "content": content
    });
    crate::github::projection::ensure_budget(&output)?;
    Ok(output)
}

fn file_response(value: Value) -> Result<FileContent, GitHubError> {
    if value.is_array() {
        return Err(GitHubError::InvalidRequest {
            reason: InvalidReason::UnsupportedContent,
        });
    }
    match value.get("type") {
        Some(Value::String(content_type)) if content_type == "file" => {}
        Some(Value::String(_)) => {
            return Err(GitHubError::InvalidRequest {
                reason: InvalidReason::UnsupportedContent,
            });
        }
        _ => return Err(GitHubError::InvalidProviderResponse),
    }
    match serde_json::from_value(value) {
        Ok(response) => Ok(response),
        Err(_) => Err(GitHubError::InvalidProviderResponse),
    }
}
