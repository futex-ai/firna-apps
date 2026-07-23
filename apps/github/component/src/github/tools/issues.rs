//! Issue detail and bounded comment-page tool.

use std::collections::BTreeMap;

use serde_json::{Value, json};

use crate::github::error::GitHubError;
use crate::github::input::{
    AppToolCall, ReadIssueInput, encoded_segment, owner, page, page_size, positive_number,
    repository,
};
use crate::github::models::{IssueComment, IssueDetail, IssueLabel, IssueMilestone};
use crate::github::pagination::next_page;
use crate::github::projection::{
    BODY_PREVIEW_BYTES, COMMENT_PREVIEW_BYTES, ExactKind, exact, fits_budget, nullable_exact,
    nullable_preview, nullable_user,
};
use crate::github::provider::ProviderMediaType;
use crate::github::tools::GitHubToolService;
use crate::github::tools::common::remove_null_field;

pub(super) fn read(
    service: &GitHubToolService<'_>,
    call: AppToolCall,
) -> Result<Value, GitHubError> {
    let input = service.parse::<ReadIssueInput>(call.input.clone())?;
    let owner = owner(&input.owner)?;
    let repository = repository(&input.repository)?;
    let number = positive_number(input.number)?;
    let include_comments = input.include_comments.unwrap_or(true);
    let comments_page = page(input.comments_page)?;
    let comments_per_page = page_size(input.comments_per_page, 10, 10)?;
    let base_path = format!(
        "/repos/{}/{}/issues/{number}",
        encoded_segment(&owner),
        encoded_segment(&repository)
    );
    let (raw_detail, _) = service.get::<Value>(
        &call,
        base_path.clone(),
        BTreeMap::new(),
        ProviderMediaType::Json,
        None,
    )?;
    validate_issue_discriminator(&raw_detail)?;
    let detail = match serde_json::from_value::<IssueDetail>(raw_detail) {
        Ok(detail) => detail,
        Err(_) => return Err(GitHubError::InvalidProviderResponse),
    };
    if detail.number != u64::from(number) {
        return Err(GitHubError::InvalidProviderResponse);
    }
    let (comments, next_comments_page) = if include_comments {
        read_comments(
            service,
            &call,
            format!("{base_path}/comments"),
            comments_page,
            comments_per_page,
        )?
    } else {
        (Vec::new(), None)
    };
    let mut output = project_detail(&detail, comments, comments_page, next_comments_page)?;
    reduce_previews(&mut output)?;
    Ok(output)
}

fn validate_issue_discriminator(value: &Value) -> Result<(), GitHubError> {
    let Some(discriminator) = value.get("pull_request") else {
        return Ok(());
    };
    if discriminator.is_object() {
        return Err(GitHubError::UseGitHubReadPr);
    }
    Err(GitHubError::InvalidProviderResponse)
}

fn read_comments(
    service: &GitHubToolService<'_>,
    call: &AppToolCall,
    path: String,
    comments_page: u32,
    comments_per_page: u32,
) -> Result<(Vec<IssueComment>, Option<u32>), GitHubError> {
    let retry_input = (comments_per_page > 1).then(|| {
        json!({
            "comments_page": comments_page,
            "comments_per_page": (comments_per_page / 2).max(1)
        })
    });
    let query = BTreeMap::from([
        (String::from("page"), comments_page.to_string()),
        (String::from("per_page"), comments_per_page.to_string()),
    ]);
    let (comments, headers) = service.get::<Vec<IssueComment>>(
        call,
        path,
        query,
        ProviderMediaType::Json,
        retry_input,
    )?;
    if comments.len() > comments_per_page as usize {
        return Err(GitHubError::InvalidProviderResponse);
    }
    Ok((comments, next_page(&headers)))
}

fn project_detail(
    detail: &IssueDetail,
    comments: Vec<IssueComment>,
    comments_page: u32,
    next_comments_page: Option<u32>,
) -> Result<Value, GitHubError> {
    let (body, body_truncated) = nullable_preview(detail.body.as_deref(), BODY_PREVIEW_BYTES)?;
    let labels_truncated = detail.labels.len() > 100;
    let assignees_truncated = detail.assignees.len() > 20;
    let labels = detail
        .labels
        .iter()
        .take(100)
        .map(project_label)
        .collect::<Result<Vec<_>, _>>()?;
    let assignees = detail
        .assignees
        .iter()
        .take(20)
        .map(crate::github::projection::user)
        .collect::<Result<Vec<_>, _>>()?;
    let comments = comments
        .iter()
        .map(project_comment)
        .collect::<Result<Vec<_>, _>>()?;
    let mut output = json!({
        "number": detail.number,
        "title": exact(&detail.title, ExactKind::PathOrTitle)?,
        "body": body,
        "body_truncated": body_truncated,
        "state": exact(&detail.state, ExactKind::EnumOrTimestamp)?,
        "state_reason": nullable_exact(detail.state_reason.as_deref(), ExactKind::EnumOrTimestamp)?,
        "author": nullable_user(detail.user.0.as_ref())?,
        "labels": labels,
        "labels_truncated": labels_truncated,
        "assignees": assignees,
        "assignees_truncated": assignees_truncated,
        "milestone": project_milestone(detail.milestone.0.as_ref())?,
        "comment_count": detail.comments,
        "created_at": exact(&detail.created_at, ExactKind::EnumOrTimestamp)?,
        "updated_at": exact(&detail.updated_at, ExactKind::EnumOrTimestamp)?,
        "closed_at": nullable_exact(detail.closed_at.0.as_deref(), ExactKind::EnumOrTimestamp)?,
        "html_url": exact(&detail.html_url, ExactKind::Url)?,
        "comments": comments,
        "comments_page": comments_page,
        "next_comments_page": next_comments_page
    });
    remove_null_field(&mut output, "next_comments_page");
    Ok(output)
}

fn project_label(label: &IssueLabel) -> Result<Value, GitHubError> {
    Ok(json!({
        "name": exact(&label.name, ExactKind::Identifier)?,
        "color": exact(&label.color, ExactKind::EnumOrTimestamp)?,
        "description": nullable_exact(label.description.as_deref(), ExactKind::PathOrTitle)?
    }))
}

fn project_milestone(milestone: Option<&IssueMilestone>) -> Result<Value, GitHubError> {
    let Some(milestone) = milestone else {
        return Ok(Value::Null);
    };
    Ok(json!({
        "number": milestone.number,
        "title": exact(&milestone.title, ExactKind::PathOrTitle)?,
        "state": exact(&milestone.state, ExactKind::EnumOrTimestamp)?,
        "html_url": exact(&milestone.html_url, ExactKind::Url)?
    }))
}

fn project_comment(comment: &IssueComment) -> Result<Value, GitHubError> {
    let (body, body_truncated) =
        crate::github::projection::preview(&comment.body, COMMENT_PREVIEW_BYTES)?;
    Ok(json!({
        "id": comment.id,
        "author": nullable_user(comment.user.0.as_ref())?,
        "body": body,
        "body_truncated": body_truncated,
        "created_at": exact(&comment.created_at, ExactKind::EnumOrTimestamp)?,
        "updated_at": exact(&comment.updated_at, ExactKind::EnumOrTimestamp)?,
        "html_url": exact(&comment.html_url, ExactKind::Url)?
    }))
}

fn reduce_previews(output: &mut Value) -> Result<(), GitHubError> {
    if fits_budget(output) {
        return Ok(());
    }
    let count = output
        .get("comments")
        .and_then(Value::as_array)
        .map(Vec::len)
        .ok_or(GitHubError::InvalidProviderResponse)?;
    for index in (0..count).rev() {
        {
            let comment = output
                .get_mut("comments")
                .and_then(Value::as_array_mut)
                .and_then(|comments| comments.get_mut(index))
                .ok_or(GitHubError::InvalidProviderResponse)?;
            comment["body"] = json!("");
            comment["body_truncated"] = json!(true);
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
