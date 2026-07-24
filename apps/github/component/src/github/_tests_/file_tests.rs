use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};

use crate::github::projection::OUTPUT_BUDGET_BYTES;

use super::support::{
    FILE_COMMIT_SHA, call, file_responses, path_entry_responses, response, tree_sha,
};

#[test]
fn rejects_tree_symlinks_before_contents_can_dereference_them() {
    let (output, requests) = call(
        "github_read_file",
        json!({
            "owner": "octo",
            "repository": "repo",
            "path": "docs/readme.md"
        }),
        path_entry_responses("docs/readme.md", "120000", "blob", "abc"),
    );

    assert_eq!(output["error"], "invalid_request");
    assert_eq!(output["reason"], "unsupported_content");
    assert_eq!(requests.len(), 3);
    assert_eq!(requests[0].path, "/repos/octo/repo/commits");
    assert_eq!(requests[0].query["per_page"], "1");
    assert_eq!(
        requests[1].path,
        format!("/repos/octo/repo/git/trees/{}", tree_sha(0))
    );
    assert_eq!(
        requests[2].path,
        format!("/repos/octo/repo/git/trees/{}", tree_sha(1))
    );
    assert!(
        requests
            .iter()
            .all(|request| !request.path.contains("/contents/"))
    );
}

#[test]
fn rejects_directories_symlinks_and_submodules_as_unsupported_content() {
    let directory = json!([{
        "type": "file",
        "path": "docs/readme.md"
    }]);
    assert_unsupported(directory);

    for (mode, object_type) in [("040000", "tree"), ("120000", "blob"), ("160000", "commit")] {
        let (output, requests) = call(
            "github_read_file",
            input(),
            path_entry_responses("docs", mode, object_type, "abc"),
        );
        assert_eq!(output["error"], "invalid_request");
        assert_eq!(output["reason"], "unsupported_content");
        assert_eq!(requests.len(), 2);
    }
}

#[test]
fn rejects_malformed_binary_and_oversized_file_content_without_truncation() {
    let cases = [
        file(json!("%%%"), json!(3), json!("base64")),
        file(json!(STANDARD.encode([0_u8])), json!(1), json!("base64")),
        file(json!(STANDARD.encode("hello")), json!(6), json!("base64")),
        file(json!(STANDARD.encode("hello")), json!(5), json!("utf-8")),
        file(
            json!(STANDARD.encode("hello")),
            json!(262_145),
            json!("base64"),
        ),
    ];

    for body in cases {
        let (output, _) = call("github_read_file", input(), file_responses("docs", body));
        assert!(
            matches!(
                output["reason"].as_str(),
                Some("unsupported_content" | "file_too_large")
            ) || output["error"] == "provider_contract_error",
            "unexpected output: {output}"
        );
        assert!(output.get("content").is_none());
    }
}

#[test]
fn rejects_missing_null_or_wrong_typed_required_file_fields() {
    for (field, replacement) in [
        ("sha", None),
        ("size", Some(Value::Null)),
        ("encoding", Some(json!(7))),
        ("content", Some(Value::Null)),
    ] {
        let mut body = file(json!(STANDARD.encode("hello")), json!(5), json!("base64"));
        let object = body.as_object_mut().expect("fixture should be an object");
        match replacement {
            Some(value) => {
                object.insert(field.to_owned(), value);
            }
            None => {
                object.remove(field);
            }
        }
        let (output, _) = call("github_read_file", input(), file_responses("docs", body));
        assert_eq!(output["error"], "provider_contract_error", "field {field}");
    }
}

#[test]
fn reads_the_exact_multibyte_file_limit_with_a_null_ref_and_url() {
    let content = "é".repeat(131_072);
    assert_eq!(content.len(), 256 * 1_024);
    let (output, _) = call(
        "github_read_file",
        input(),
        file_responses(
            "docs",
            file(
                json!(STANDARD.encode(&content)),
                json!(content.len()),
                json!("base64"),
            ),
        ),
    );
    assert_eq!(output["content"], content);
    assert_eq!(output["ref"], Value::Null);
    assert_eq!(output["html_url"], Value::Null);
    assert!(serde_json::to_vec(&output).unwrap().len() <= OUTPUT_BUDGET_BYTES);
}

#[test]
fn rejects_actual_content_over_the_file_limit_and_provider_path_mismatches() {
    let content = "a".repeat(256 * 1_024 + 1);
    let (output, _) = call(
        "github_read_file",
        input(),
        file_responses(
            "docs",
            file(
                json!(STANDARD.encode(&content)),
                json!(content.len()),
                json!("base64"),
            ),
        ),
    );
    assert_eq!(output["reason"], "file_too_large");

    let mut mismatch = file(json!(STANDARD.encode("hello")), json!(5), json!("base64"));
    mismatch["path"] = json!("other");
    let (output, _) = call(
        "github_read_file",
        input(),
        file_responses("docs", mismatch),
    );
    assert_eq!(output["reason"], "unsupported_content");
}

#[test]
fn pins_contents_to_the_resolved_commit_and_rejects_blob_sha_mismatches() {
    let mut responses = path_entry_responses("docs", "100644", "blob", "tree-blob");
    responses.push(response(
        200,
        file(json!(STANDARD.encode("hello")), json!(5), json!("base64")),
    ));
    let (output, requests) = call("github_read_file", input(), responses);

    assert_eq!(output["error"], "provider_contract_error");
    assert_eq!(requests[2].query["ref"], FILE_COMMIT_SHA);
}

fn assert_unsupported(body: Value) {
    let (output, _) = call("github_read_file", input(), file_responses("docs", body));
    assert_eq!(output["error"], "invalid_request");
    assert_eq!(output["reason"], "unsupported_content");
}

fn input() -> Value {
    json!({ "owner": "octo", "repository": "repo", "path": "docs" })
}

fn file(content: Value, size: Value, encoding: Value) -> Value {
    json!({
        "type": "file",
        "path": "docs",
        "sha": "abc",
        "size": size,
        "html_url": null,
        "encoding": encoding,
        "content": content
    })
}
