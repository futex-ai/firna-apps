//! Repository and code-search projection budget handling.

use serde_json::{Value, json};

use crate::github::error::GitHubError;
use crate::github::input::quoted_search_literal;
use crate::github::models::{CodeSearchItem, Repository};
use crate::github::projection::{
    DESCRIPTION_PREVIEW_BYTES, ExactKind, FRAGMENT_PREVIEW_BYTES, exact, fits_budget,
    nullable_exact, nullable_preview, preview,
};

pub(super) fn repository(repository: &Repository) -> Result<Value, GitHubError> {
    let (description, description_truncated) = nullable_preview(
        repository.description.0.as_deref(),
        DESCRIPTION_PREVIEW_BYTES,
    )?;
    Ok(json!({
        "id": repository.id,
        "full_name": exact(&repository.full_name, ExactKind::Identifier)?,
        "description": description,
        "description_truncated": description_truncated,
        "visibility": exact(&repository.visibility, ExactKind::EnumOrTimestamp)?,
        "archived": repository.archived,
        "fork": repository.fork,
        "default_branch": exact(&repository.default_branch, ExactKind::Identifier)?,
        "language": nullable_exact(repository.language.0.as_deref(), ExactKind::Identifier)?,
        "pushed_at": nullable_exact(
            repository.pushed_at.0.as_deref(),
            ExactKind::EnumOrTimestamp,
        )?,
        "html_url": exact(&repository.html_url, ExactKind::Url)?
    }))
}

pub(super) fn code_match(item: &CodeSearchItem) -> Result<Value, GitHubError> {
    let fragments = item
        .text_matches
        .iter()
        .map(|text_match| {
            let (fragment, truncated) = preview(&text_match.fragment, FRAGMENT_PREVIEW_BYTES)?;
            Ok(json!({ "fragment": fragment, "truncated": truncated }))
        })
        .collect::<Result<Vec<_>, GitHubError>>()?;
    Ok(json!({
        "repository_full_name": exact(&item.repository.full_name, ExactKind::Identifier)?,
        "path": exact(&item.path, ExactKind::PathOrTitle)?,
        "name": exact(&item.name, ExactKind::Identifier)?,
        "sha": exact(&item.sha, ExactKind::Identifier)?,
        "html_url": exact(&item.html_url, ExactKind::Url)?,
        "fragments": fragments
    }))
}

pub(super) fn build_search_query(
    query: &str,
    qualifier: Option<&(String, String)>,
    language: Option<&str>,
    path: Option<&str>,
) -> String {
    let mut terms = vec![quoted_search_literal(query)];
    if let Some((owner, repository)) = qualifier {
        let full_name = format!("{owner}/{repository}");
        terms.push(format!("repo:{}", quoted_search_literal(&full_name)));
    }
    if let Some(language) = language {
        terms.push(format!("language:{}", quoted_search_literal(language)));
    }
    if let Some(path) = path {
        terms.push(format!("path:{}", quoted_search_literal(path)));
    }
    terms.join(" ")
}

pub(super) fn reduce_repository_previews(output: &mut Value) -> Result<(), GitHubError> {
    if fits_budget(output) {
        return Ok(());
    }
    let count = array_length(output, "repositories")?;
    for index in (0..count).rev() {
        {
            let row = array_item_mut(output, "repositories", index)?;
            if let Some(description) = row.get_mut("description")
                && !description.is_null()
            {
                *description = json!("");
                row["description_truncated"] = json!(true);
            }
        }
        if fits_budget(output) {
            return Ok(());
        }
    }
    Err(GitHubError::InvalidProviderResponse)
}

pub(super) fn reduce_search_previews(output: &mut Value) -> Result<(), GitHubError> {
    if fits_budget(output) {
        return Ok(());
    }
    let match_count = array_length(output, "matches")?;
    for match_index in (0..match_count).rev() {
        let fragment_count = output
            .get("matches")
            .and_then(Value::as_array)
            .and_then(|matches| matches.get(match_index))
            .and_then(|row| row.get("fragments"))
            .and_then(Value::as_array)
            .map(Vec::len)
            .ok_or(GitHubError::InvalidProviderResponse)?;
        for fragment_index in (0..fragment_count).rev() {
            {
                let row = array_item_mut(output, "matches", match_index)?;
                let fragment = row
                    .get_mut("fragments")
                    .and_then(Value::as_array_mut)
                    .and_then(|fragments| fragments.get_mut(fragment_index))
                    .ok_or(GitHubError::InvalidProviderResponse)?;
                fragment["fragment"] = json!("");
                fragment["truncated"] = json!(true);
            }
            if fits_budget(output) {
                return Ok(());
            }
        }
    }
    Err(GitHubError::InvalidProviderResponse)
}

fn array_length(value: &Value, field: &str) -> Result<usize, GitHubError> {
    value
        .get(field)
        .and_then(Value::as_array)
        .map(Vec::len)
        .ok_or(GitHubError::InvalidProviderResponse)
}

fn array_item_mut<'a>(
    value: &'a mut Value,
    field: &str,
    index: usize,
) -> Result<&'a mut Value, GitHubError> {
    value
        .get_mut(field)
        .and_then(Value::as_array_mut)
        .and_then(|items| items.get_mut(index))
        .ok_or(GitHubError::InvalidProviderResponse)
}
