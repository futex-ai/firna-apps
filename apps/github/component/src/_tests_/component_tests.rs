//! Tests for the credential-only component response.

use super::unsupported_tool_response;

#[test]
fn undeclared_tool_calls_fail_closed() {
    assert_eq!(
        unsupported_tool_response(),
        r#"{"ok":false,"error":"invalid_request","reason":"no_tools_declared"}"#
    );
}
