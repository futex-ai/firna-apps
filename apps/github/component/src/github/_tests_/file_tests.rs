use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};

use crate::github::projection::OUTPUT_BUDGET_BYTES;

use super::support::{call, response};

#[test]
fn rejects_directories_symlinks_and_submodules_as_unsupported_content() {
    let directory = json!([{
        "type": "file",
        "path": "docs/readme.md"
    }]);
    assert_unsupported(directory);

    for content_type in ["dir", "symlink", "submodule"] {
        assert_unsupported(json!({
            "type": content_type,
            "path": "docs",
            "sha": "abc",
            "size": 0,
            "html_url": "https://github.com/octo/repo/tree/main/docs",
            "encoding": "base64",
            "content": ""
        }));
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
        let (output, _) = call("github_read_file", input(), vec![response(200, body)]);
        assert!(
            matches!(
                output["reason"].as_str(),
                Some("unsupported_content" | "file_too_large")
            ) || output["error"] == "invalid_provider_response",
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
        let (output, _) = call("github_read_file", input(), vec![response(200, body)]);
        assert_eq!(
            output["error"], "invalid_provider_response",
            "field {field}"
        );
    }
}

#[test]
fn reads_the_exact_multibyte_file_limit_with_a_null_ref_and_url() {
    let content = "é".repeat(131_072);
    assert_eq!(content.len(), 256 * 1_024);
    let (output, _) = call(
        "github_read_file",
        input(),
        vec![response(
            200,
            file(
                json!(STANDARD.encode(&content)),
                json!(content.len()),
                json!("base64"),
            ),
        )],
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
        vec![response(
            200,
            file(
                json!(STANDARD.encode(&content)),
                json!(content.len()),
                json!("base64"),
            ),
        )],
    );
    assert_eq!(output["reason"], "file_too_large");

    let mut mismatch = file(json!(STANDARD.encode("hello")), json!(5), json!("base64"));
    mismatch["path"] = json!("other");
    let (output, _) = call("github_read_file", input(), vec![response(200, mismatch)]);
    assert_eq!(output["reason"], "unsupported_content");
}

fn assert_unsupported(body: Value) {
    let (output, _) = call("github_read_file", input(), vec![response(200, body)]);
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
