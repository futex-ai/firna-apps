use serde_json::Value;

use crate::manifest;

const PACKAGE_DOC: &str = include_str!("../../README.md");

#[test]
fn tool_catalog_documents_slack_manifest_tools() {
    let manifest = manifest();
    for tool in manifest.tools {
        let row = documented_tool_row(&tool.name);
        let prefix = format!("| slack | `{}` |", tool.name);
        assert!(
            row.starts_with(&prefix),
            "Slack tool catalog row for `{}` should start with `{}`",
            tool.name,
            prefix
        );
        assert!(
            row.contains(&tool.description),
            "Slack tool catalog row for `{}` should include the manifest description",
            tool.name
        );
        assert_documented_params(&tool.name, tool.input_schema.as_ref(), row);
    }
}

fn documented_tool_row(tool_name: &str) -> &'static str {
    let marker = format!("| `{tool_name}` |");
    let mut matches = PACKAGE_DOC.lines().filter(|line| line.contains(&marker));
    let row = matches
        .next()
        .unwrap_or_else(|| panic!("tool catalog is missing `{tool_name}`"));
    assert!(
        matches.next().is_none(),
        "tool catalog should document `{tool_name}` exactly once"
    );
    row
}

fn assert_documented_params(tool_name: &str, input_schema: Option<&Value>, row: &str) {
    for param in top_level_params(input_schema) {
        assert!(
            row.contains(&format!("`{param}")),
            "Slack tool catalog row for `{}` should document param `{}`",
            tool_name,
            param
        );
    }
}

fn top_level_params(input_schema: Option<&Value>) -> Vec<String> {
    let mut params = input_schema
        .and_then(|schema| schema.get("properties"))
        .and_then(Value::as_object)
        .map(|properties| properties.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    params.sort();
    params
}
