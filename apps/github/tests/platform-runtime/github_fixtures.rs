use serde_json::{Value, json};

pub(crate) fn repository(id: u64, description: Option<&str>) -> Value {
    json!({
        "id": id,
        "full_name": format!("octo/repo-{id}"),
        "description": description,
        "visibility": "private",
        "archived": false,
        "fork": false,
        "default_branch": "main",
        "language": "Rust",
        "pushed_at": "2026-07-20T10:00:00Z",
        "html_url": format!("https://github.com/octo/repo-{id}")
    })
}

pub(crate) fn code_search() -> Value {
    json!({
        "total_count": 1,
        "incomplete_results": false,
        "items": [{
            "name": "lib.rs",
            "path": "src/lib.rs",
            "sha": "abc",
            "html_url": "https://github.com/octo/repo/blob/abc/src/lib.rs",
            "repository": { "full_name": "octo/repo" },
            "text_matches": [{ "fragment": "pub fn call() {}" }]
        }]
    })
}

pub(crate) fn file() -> Value {
    json!({
        "type": "file",
        "path": "README.md",
        "sha": "abc",
        "size": 6,
        "html_url": "https://github.com/octo/repo/blob/abc/README.md",
        "encoding": "base64",
        "content": "aGVsbG8K"
    })
}

pub(crate) fn pull_request() -> Value {
    json!({
        "number": 7,
        "title": "Improve parser",
        "body": "Ready for review",
        "state": "open",
        "draft": false,
        "merged": false,
        "mergeable": true,
        "user": user("octo"),
        "base": { "ref": "main", "sha": "base" },
        "head": { "ref": "feature", "sha": "head" },
        "additions": 20,
        "deletions": 3,
        "changed_files": 1,
        "commits": 4,
        "comments": 5,
        "review_comments": 6,
        "created_at": "2026-07-18T10:00:00Z",
        "updated_at": "2026-07-20T10:00:00Z",
        "closed_at": null,
        "merged_at": null,
        "html_url": "https://github.com/octo/repo/pull/7"
    })
}

pub(crate) fn pull_request_files() -> Value {
    json!([{
        "filename": "src/lib.rs",
        "status": "modified",
        "additions": 2,
        "deletions": 1,
        "changes": 3,
        "sha": "abc",
        "blob_url": "https://github.com/octo/repo/blob/abc/src/lib.rs",
        "patch": "@@ -1 +1 @@"
    }])
}

pub(crate) fn issue() -> Value {
    json!({
        "number": 8,
        "title": "Bug report",
        "body": "Steps to reproduce",
        "state": "open",
        "state_reason": null,
        "user": user("octo"),
        "labels": [{ "name": "bug", "color": "ff0000", "description": null }],
        "assignees": [],
        "milestone": null,
        "comments": 1,
        "created_at": "2026-07-18T10:00:00Z",
        "updated_at": "2026-07-20T10:00:00Z",
        "closed_at": null,
        "html_url": "https://github.com/octo/repo/issues/8"
    })
}

pub(crate) fn issue_comments() -> Value {
    json!([{
        "id": 10,
        "user": user("octo"),
        "body": "Confirmed",
        "created_at": "2026-07-19T10:00:00Z",
        "updated_at": "2026-07-19T10:00:00Z",
        "html_url": "https://github.com/octo/repo/issues/8#issuecomment-10"
    }])
}

fn user(login: &str) -> Value {
    json!({
        "id": 1,
        "login": login,
        "html_url": format!("https://github.com/{login}")
    })
}
