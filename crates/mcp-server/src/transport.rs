//! stdio transport for the MCP server.
//!
//! Reads JSON-RPC 2.0 messages from stdin and writes responses to stdout,
//! one JSON object per line (newline-delimited JSON — NDJSON).
//!
//! This is the simplest MCP transport: the server is launched as a subprocess
//! by the MCP client (e.g. Claude Desktop, Cursor, VS Code), and communicates
//! over standard I/O streams.

use crate::types::{JsonRpcError, JsonRpcRequest, JsonRpcSuccess, McpMessage, RequestId};
use serde_json::Value;
use std::io::{self, BufRead, Write};

/// Read one JSON-RPC message from stdin.
///
/// Returns `None` on EOF (stdin closed by the client).
pub fn read_message() -> io::Result<Option<JsonRpcRequest>> {
    let stdin = io::stdin();
    let mut line = String::new();
    let n = stdin.lock().read_line(&mut line)?;

    if n == 0 {
        // EOF — client disconnected
        return Ok(None);
    }

    let trimmed = line.trim();
    if trimmed.is_empty() {
        // Skip blank lines (some clients send keep-alive newlines)
        return read_message();
    }

    let parsed: Value =
        serde_json::from_str(trimmed).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    // Validate that it has the required JSON-RPC fields
    let method = parsed
        .get("method")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "Missing 'method' field in MCP message",
            )
        })?;

    let id = parsed.get("id").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "Missing 'id' field in MCP request",
        )
    })?;

    let id: RequestId = serde_json::from_value(id.clone()).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Invalid request id: {e}"),
        )
    })?;

    let params = parsed.get("params").cloned();

    Ok(Some(JsonRpcRequest {
        jsonrpc: "2.0".to_owned(),
        id,
        method: method.to_owned(),
        params,
    }))
}

/// Write a JSON-RPC response message to stdout.
///
/// The message is serialized as a single JSON line followed by a newline.
pub fn write_message(msg: &McpMessage) -> io::Result<()> {
    let json = serde_json::to_string(msg).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Serialization error: {e}"),
        )
    })?;

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    writeln!(handle, "{json}")?;
    handle.flush()?;
    Ok(())
}

/// Convenience: write a successful JSON-RPC response.
pub fn write_success(id: RequestId, result: Value) -> io::Result<()> {
    write_message(&McpMessage::Success(JsonRpcSuccess::new(id, result)))
}

/// Convenience: write a JSON-RPC error response.
pub fn write_error(err: JsonRpcError) -> io::Result<()> {
    write_message(&McpMessage::Error(err))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::INTERNAL_ERROR;

    #[test]
    fn test_serialize_success_message() {
        let msg = McpMessage::Success(JsonRpcSuccess::new(
            RequestId::Number(1),
            serde_json::json!({"tools": []}),
        ));
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""jsonrpc":"2.0""#));
        assert!(json.contains(r#""id":1"#));
        assert!(json.contains(r#""tools""#));
    }

    #[test]
    fn test_serialize_error_message() {
        let err = JsonRpcError::new(
            RequestId::Number(42),
            INTERNAL_ERROR,
            "something went wrong",
        );
        let msg = McpMessage::Error(err);
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""code":-32603"#));
        assert!(json.contains(r#""something went wrong""#));
    }

    #[test]
    fn test_serialize_request_to_string() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_owned(),
            id: RequestId::Number(1),
            method: "tools/list".to_owned(),
            params: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains(r#""method":"tools/list""#));
        assert!(json.starts_with("{"));
        assert!(json.ends_with("}"));
    }
}
