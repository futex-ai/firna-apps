//! Pull-request detail and bounded file-page tool.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::github::error::GitHubError;
use crate::github::input::{
    AppToolCall, ReadPullRequestInput, encoded_segment, owner, page, page_size, positive_number,
    repository, result_offset,
};
use crate::github::models::{PullRequestDetail, PullRequestFile, PullRequestRef};
use crate::github::pagination::next_page;
use crate::github::projection::{
    BODY_PREVIEW_BYTES, ExactKind, PATCH_PREVIEW_BYTES, exact, fits_budget, nullable_exact,
    nullable_preview, user,
};
use crate::github::provider::ProviderMediaType;
use crate::github::tools::GitHubToolService;
use crate::github::tools::common::remove_null_field;

pub(super) fn read(
    service: &GitHubToolService<'_>,
    call: AppToolCall,
) -> Result<Value, GitHubError> {
    let input = service.parse::<ReadPullRequestInput>(call.input.clone())?;
    let owner = owner(&input.owner)?;
    let repository = repository(&input.repository)?;
    let number = positive_number(input.number)?;
    let include_files = input.include_files.unwrap_or(true);
    let files_page = page(input.files_page)?;
    let files_per_page = page_size(input.files_per_page, 10, 10)?;
    if result_offset(files_page, files_per_page)? >= 3_000 {
        return Err(crate::github::error::GitHubError::InvalidRequest {
            reason: crate::github::error::InvalidReason::ResultWindowExceeded,
        });
    }
    let base_path = format!(
        "/repos/{}/{}/pulls/{number}",
        encoded_segment(&owner),
        encoded_segment(&repository)
    );
    let (detail, _) = service.get::<PullRequestDetail>(
        &call,
        base_path.clone(),
        BTreeMap::new(),
        ProviderMediaType::Json,
        None,
    )?;
    if detail.number != u64::from(number) {
        return Err(GitHubError::InvalidProviderResponse);
    }
    let (files, next_files_page) = if include_files {
        read_files(
            service,
            &call,
            format!("{base_path}/files"),
            files_page,
            files_per_page,
        )?
    } else {
        (Vec::new(), None)
    };
    let mut output = project_detail(&detail, files, files_page, next_files_page)?;
    reduce_previews(&mut output)?;
    Ok(output)
}

fn read_files(
    service: &GitHubToolService<'_>,
    call: &AppToolCall,
    path: String,
    files_page: u32,
    files_per_page: u32,
) -> Result<(Vec<PullRequestFile>, Option<u32>), GitHubError> {
    let retry_input = (files_per_page > 1).then(|| {
        json!({
            "files_page": files_page,
            "files_per_page": (files_per_page / 2).max(1)
        })
    });
    let query = BTreeMap::from([
        (String::from("page"), files_page.to_string()),
        (String::from("per_page"), files_per_page.to_string()),
    ]);
    let (files, headers) = service.get::<Vec<PullRequestFile>>(
        call,
        path,
        query,
        ProviderMediaType::Json,
        retry_input,
    )?;
    if files.len() > files_per_page as usize {
        return Err(GitHubError::InvalidProviderResponse);
    }
    let next = match next_page(&headers) {
        Some(candidate) if result_offset(candidate, files_per_page)? < 3_000 => Some(candidate),
        _ => None,
    };
    Ok((files, next))
}

fn project_detail(
    detail: &PullRequestDetail,
    files: Vec<PullRequestFile>,
    files_page: u32,
    next_files_page: Option<u32>,
) -> Result<Value, GitHubError> {
    let (body, body_truncated) = nullable_preview(detail.body.0.as_deref(), BODY_PREVIEW_BYTES)?;
    let files = files
        .iter()
        .map(project_file)
        .collect::<Result<Vec<_>, _>>()?;
    let mut output = json!({
        "number": detail.number,
        "title": exact(&detail.title, ExactKind::PathOrTitle)?,
        "body": body,
        "body_truncated": body_truncated,
        "state": exact(&detail.state, ExactKind::EnumOrTimestamp)?,
        "draft": detail.draft,
        "merged": detail.merged,
        "mergeable": detail.mergeable.0,
        "author": user(&detail.user)?,
        "base": project_ref(&detail.base)?,
        "head": project_ref(&detail.head)?,
        "additions": detail.additions,
        "deletions": detail.deletions,
        "changed_file_count": detail.changed_files,
        "commit_count": detail.commits,
        "issue_comment_count": detail.comments,
        "review_comment_count": detail.review_comments,
        "created_at": exact(&detail.created_at, ExactKind::EnumOrTimestamp)?,
        "updated_at": exact(&detail.updated_at, ExactKind::EnumOrTimestamp)?,
        "closed_at": nullable_exact(detail.closed_at.0.as_deref(), ExactKind::EnumOrTimestamp)?,
        "merged_at": nullable_exact(detail.merged_at.0.as_deref(), ExactKind::EnumOrTimestamp)?,
        "html_url": exact(&detail.html_url, ExactKind::Url)?,
        "files": files,
        "files_page": files_page,
        "next_files_page": next_files_page
    });
    remove_null_field(&mut output, "next_files_page");
    Ok(output)
}

fn project_ref(reference: &PullRequestRef) -> Result<Value, GitHubError> {
    Ok(json!({
        "ref": exact(&reference.git_ref, ExactKind::Identifier)?,
        "sha": exact(&reference.sha, ExactKind::Identifier)?
    }))
}

fn project_file(file: &PullRequestFile) -> Result<Value, GitHubError> {
    let patch = file
        .patch
        .as_deref()
        .map(|patch| crate::github::projection::preview(patch, PATCH_PREVIEW_BYTES))
        .transpose()?;
    let (patch, patch_truncated) = match patch {
        Some((patch, truncated)) => (Some(patch), Some(truncated)),
        None => (None, None),
    };
    let mut output = json!({
        "filename": exact(&file.filename, ExactKind::PathOrTitle)?,
        "status": exact(&file.status, ExactKind::EnumOrTimestamp)?,
        "additions": file.additions,
        "deletions": file.deletions,
        "changes": file.changes,
        "sha": nullable_exact(file.sha.0.as_deref(), ExactKind::Identifier)?,
        "html_url": exact(&file.blob_url, ExactKind::Url)?,
        "previous_filename": nullable_exact(file.previous_filename.as_deref(), ExactKind::PathOrTitle)?,
        "patch": patch,
        "patch_truncated": patch_truncated
    });
    if file.previous_filename.is_none() {
        remove_null_field(&mut output, "previous_filename");
    }
    if file.patch.is_none() {
        remove_null_field(&mut output, "patch");
        remove_null_field(&mut output, "patch_truncated");
    }
    Ok(output)
}

fn reduce_previews(output: &mut Value) -> Result<(), GitHubError> {
    if fits_budget(output) {
        return Ok(());
    }
    let file_count = output
        .get("files")
        .and_then(Value::as_array)
        .map(Vec::len)
        .ok_or(GitHubError::InvalidProviderResponse)?;
    for index in (0..file_count).rev() {
        {
            let file = output
                .get_mut("files")
                .and_then(Value::as_array_mut)
                .and_then(|files| files.get_mut(index))
                .ok_or(GitHubError::InvalidProviderResponse)?;
            if file.get("patch").is_some() {
                file["patch"] = json!("");
                file["patch_truncated"] = json!(true);
            }
        }
        if fits_budget(output) {
            return Ok(());
        }
    }
    output["body"] = json!("");
    output["body_truncated"] = json!(true);
    if fits_budget(output) {
        Ok(())
    } else {
        Err(GitHubError::InvalidProviderResponse)
    }
}
