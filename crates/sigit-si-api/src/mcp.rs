//! JSON-RPC bridge to the MCP endpoint at `POST /api/v1/mcp`.
//!
//! Issues, pull requests, code search, and file reads have no REST surface on
//! sigit.si — they are served as MCP tools. Rather than wait for a second API,
//! the client speaks the same stateless JSON-RPC the agent does and decodes the
//! tool results into ordinary types. The transport is an implementation
//! detail; callers use [`crate::issues`], [`crate::pulls`], and
//! [`crate::code`].
//!
//! The endpoint is stateless (see `Mcp::Server`), so there is no `initialize`
//! handshake to perform — a `tools/call` stands on its own.

use {
    crate::{
        client::Client,
        error::{ApiError, Result},
    },
    serde::de::DeserializeOwned,
    serde_json::{json, Value},
    std::sync::atomic::{AtomicU64, Ordering},
    tracing::debug,
};

/// JSON-RPC ids only need to be unique within a connection; a process-wide
/// counter is plenty and keeps `call_tool` callable through `&self`.
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

impl Client {
    /// Invoke one MCP tool and return its decoded result.
    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<Value> {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": { "name": name, "arguments": arguments },
        });

        debug!(tool = name, "calling MCP tool");

        let builder = self
            .request(reqwest::Method::POST, "mcp")
            .header(reqwest::header::ACCEPT, "application/json")
            .json(&request);

        let response: Value = self.send(builder).await?;
        decode_tool_result(name, response)
    }

    /// [`Client::call_tool`], deserialized into a concrete type.
    pub async fn call_tool_as<T: DeserializeOwned>(
        &self,
        name: &str,
        arguments: Value,
    ) -> Result<T> {
        let value = self.call_tool(name, arguments).await?;
        serde_json::from_value(value)
            .map_err(|e| ApiError::Parse(format!("tool {name} returned an unexpected shape: {e}")))
    }
}

/// Unwrap a JSON-RPC envelope down to the tool's payload.
///
/// Three failure modes are distinct and all reachable: a protocol-level
/// `error` (unknown tool, malformed params), an in-band tool failure
/// (`isError: true`, e.g. "No pull request #9"), and a success whose text block
/// isn't the JSON we expect.
fn decode_tool_result(name: &str, response: Value) -> Result<Value> {
    if let Some(error) = response.get("error") {
        let message = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("The server rejected the request.");
        return Err(ApiError::Tool(message.to_string()));
    }

    let result = response
        .get("result")
        .ok_or_else(|| ApiError::Parse(format!("tool {name} returned no result")))?;

    let text = result
        .get("content")
        .and_then(Value::as_array)
        .and_then(|blocks| blocks.first())
        .and_then(|block| block.get("text"))
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::Parse(format!("tool {name} returned no content")))?;

    if result.get("isError").and_then(Value::as_bool) == Some(true) {
        return Err(ApiError::Tool(text.to_string()));
    }

    // Tools serialize their payload as pretty-printed JSON inside a text block.
    // A tool that legitimately returns prose would land in the fallback.
    serde_json::from_str(text).or_else(|_| Ok(Value::String(text.to_string())))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn envelope(content: Value, is_error: bool) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "content": [{ "type": "text", "text": content }],
                "isError": is_error,
            }
        })
    }

    #[test]
    fn a_successful_call_yields_the_parsed_payload() {
        let response = envelope(json!(r#"[{"number":3,"title":"Fix it"}]"#), false);
        let value = decode_tool_result("list_issues", response).unwrap();
        assert_eq!(value[0]["number"], 3);
    }

    #[test]
    fn an_in_band_tool_failure_becomes_a_tool_error() {
        let response = envelope(json!("No pull request #9 in a/b."), true);
        match decode_tool_result("get_pull_request", response) {
            Err(ApiError::Tool(message)) => assert!(message.contains("No pull request #9")),
            other => panic!("expected a Tool error, got {other:?}"),
        }
    }

    #[test]
    fn a_protocol_error_becomes_a_tool_error() {
        let response = json!({
            "jsonrpc": "2.0", "id": 1,
            "error": { "code": -32602, "message": "Unknown tool: nope" }
        });
        match decode_tool_result("nope", response) {
            Err(ApiError::Tool(message)) => assert!(message.contains("Unknown tool")),
            other => panic!("expected a Tool error, got {other:?}"),
        }
    }

    #[test]
    fn a_non_json_text_block_is_returned_as_a_string() {
        let response = envelope(json!("just prose"), false);
        let value = decode_tool_result("web_search", response).unwrap();
        assert_eq!(value, Value::String("just prose".into()));
    }

    #[test]
    fn a_missing_result_is_a_parse_error() {
        let response = json!({ "jsonrpc": "2.0", "id": 1 });
        assert!(matches!(
            decode_tool_result("list_issues", response),
            Err(ApiError::Parse(_))
        ));
    }
}
