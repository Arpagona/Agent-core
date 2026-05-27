//! JSON-RPC 2.0 and MCP (Model Context Protocol) types.
//!
//! Implements the minimal subset of the MCP specification (2025-11-25)
//! needed for tools/list, tools/call, and initialize.
//!
//! Spec: https://modelcontextprotocol.io

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ---------------------------------------------------------------------------
// JSON-RPC 2.0 Base Types
// ---------------------------------------------------------------------------

/// A unique identifier for a JSON-RPC request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    Number(u64),
    String(String),
}

/// A JSON-RPC 2.0 request object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: RequestId,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// A successful JSON-RPC 2.0 response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcSuccess {
    pub jsonrpc: String,
    pub id: RequestId,
    pub result: Value,
}

/// A JSON-RPC 2.0 error object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcErrorObject {
    pub code: i64,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// An error JSON-RPC 2.0 response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub jsonrpc: String,
    pub id: RequestId,
    pub error: JsonRpcErrorObject,
}

/// A JSON-RPC 2.0 notification (no id — no response expected).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// Any message that can be read from or written to an MCP transport.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum McpMessage {
    Request(JsonRpcRequest),
    Success(JsonRpcSuccess),
    Error(JsonRpcError),
    Notification(JsonRpcNotification),
}

// ---------------------------------------------------------------------------
// Standard JSON-RPC Error Codes
// ---------------------------------------------------------------------------

pub const PARSE_ERROR: i64 = -32700;
pub const INVALID_REQUEST: i64 = -32600;
pub const METHOD_NOT_FOUND: i64 = -32601;
pub const INVALID_PARAMS: i64 = -32602;
pub const INTERNAL_ERROR: i64 = -32603;

// ---------------------------------------------------------------------------
// MCP Protocol Types
// ---------------------------------------------------------------------------

/// Latest MCP protocol version supported by this server.
pub const MCP_PROTOCOL_VERSION: &str = "2025-11-25";

/// MCP server capabilities — what features this server supports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolCapabilities>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<Value>,
}

/// Tool-specific capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// MCP implementation metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Implementation {
    pub name: String,
    pub version: String,
}

/// Parameters for the `initialize` request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeParams {
    pub protocol_version: String,
    pub capabilities: Value,
    pub client_info: Implementation,
}

/// Result of the `initialize` request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResult {
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    pub server_info: Implementation,
}

/// An MCP tool definition exposed to clients.
///
/// Mirrors the MCP `Tool` interface from the specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    /// The name of the tool. Must be unique within this server.
    pub name: String,
    /// A human-readable description of what the tool does.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// JSON Schema object defining the expected parameters.
    pub input_schema: Value,
    /// Optional annotations for client behaviour hints.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<ToolAnnotations>,
}

/// Hints for client behaviour regarding a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolAnnotations {
    /// A human-readable title for this tool.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// If true, the tool does not modify its environment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub read_only_hint: Option<bool>,
}

/// Result of the `tools/list` request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListToolsResult {
    pub tools: Vec<McpTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// A content block in a `CallToolResult`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum McpContentBlock {
    #[serde(rename = "text")]
    Text {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
    },
    #[serde(rename = "resource")]
    Resource { resource: Value },
    #[serde(rename = "json")]
    Json { json: Value },
}

/// Result of the `tools/call` request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToolResult {
    pub content: Vec<McpContentBlock>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub structured_content: Option<Value>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_error: bool,
}

fn is_false(b: &bool) -> bool {
    !b
}

// ---------------------------------------------------------------------------
// Default capability providers
// ---------------------------------------------------------------------------

impl Default for ServerCapabilities {
    fn default() -> Self {
        Self {
            tools: Some(ToolCapabilities { list_changed: None }),
            resources: None,
            prompts: None,
            logging: None,
            experimental: None,
        }
    }
}

impl Default for ToolAnnotations {
    fn default() -> Self {
        Self {
            title: None,
            read_only_hint: Some(true),
        }
    }
}

// ---------------------------------------------------------------------------
// Error helpers
// ---------------------------------------------------------------------------

impl JsonRpcError {
    pub fn new(id: RequestId, code: i64, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_owned(),
            id,
            error: JsonRpcErrorObject {
                code,
                message: message.into(),
                data: None,
            },
        }
    }

    pub fn method_not_found(id: RequestId, method: &str) -> Self {
        Self::new(id, METHOD_NOT_FOUND, format!("Method not found: {method}"))
    }

    pub fn invalid_params(id: RequestId, message: impl Into<String>) -> Self {
        Self::new(id, INVALID_PARAMS, message)
    }

    pub fn internal_error(id: RequestId, message: impl Into<String>) -> Self {
        Self::new(id, INTERNAL_ERROR, message)
    }
}

impl JsonRpcSuccess {
    pub fn new(id: RequestId, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_owned(),
            id,
            result,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_rpc_request_serialization() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_owned(),
            id: RequestId::Number(1),
            method: "tools/list".to_owned(),
            params: None,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["jsonrpc"], "2.0");
        assert_eq!(json["id"], 1);
        assert_eq!(json["method"], "tools/list");
        assert!(json.get("params").is_none() || json["params"].is_null());
    }

    #[test]
    fn test_initialize_result_serialization() {
        let result = InitializeResult {
            protocol_version: MCP_PROTOCOL_VERSION.to_owned(),
            capabilities: ServerCapabilities::default(),
            server_info: Implementation {
                name: "arpagona".to_owned(),
                version: "0.1.0".to_owned(),
            },
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["protocol_version"], MCP_PROTOCOL_VERSION);
        assert!(json["capabilities"]["tools"].is_object());
        assert_eq!(json["server_info"]["name"], "arpagona");
    }

    #[test]
    fn test_tool_definition_roundtrip() {
        let tool = McpTool {
            name: "read_file".to_owned(),
            description: Some("Read a file from the workspace".to_owned()),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string", "description": "Path to the file"}
                },
                "required": ["path"]
            }),
            annotations: Some(ToolAnnotations {
                title: Some("Read File".to_owned()),
                read_only_hint: Some(true),
            }),
        };
        let json = serde_json::to_value(&tool).unwrap();
        assert_eq!(json["name"], "read_file");
        assert_eq!(json["input_schema"]["required"][0], "path");

        // Deserialize back
        let deserialized: McpTool = serde_json::from_value(json).unwrap();
        assert_eq!(deserialized.name, "read_file");
    }

    #[test]
    fn test_call_tool_result_text() {
        let result = CallToolResult {
            content: vec![McpContentBlock::Text {
                text: "File content here".to_owned(),
                mime_type: None,
            }],
            structured_content: None,
            is_error: false,
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["content"][0]["type"], "text");
        assert_eq!(json["content"][0]["text"], "File content here");
        // is_error=false is skipped by skip_serializing_if
        assert!(json.get("is_error").is_none() || json["is_error"].as_bool() == Some(false));
    }

    #[test]
    fn test_call_tool_result_error() {
        let result = CallToolResult {
            content: vec![McpContentBlock::Text {
                text: "File not found".to_owned(),
                mime_type: None,
            }],
            structured_content: None,
            is_error: true,
        };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["is_error"], true);
    }

    #[test]
    fn test_json_rpc_error_serialization() {
        let err = JsonRpcError::method_not_found(RequestId::Number(42), "unknown_method");
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["error"]["code"], METHOD_NOT_FOUND);
        assert!(json["error"]["message"]
            .as_str()
            .unwrap()
            .contains("unknown_method"));
    }

    #[test]
    fn test_mcp_message_untagged_dispatch() {
        // Sending a success message
        let msg = McpMessage::Success(JsonRpcSuccess::new(
            RequestId::Number(1),
            serde_json::json!({"tools": []}),
        ));
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["jsonrpc"], "2.0");
        assert_eq!(json["id"], 1);

        // Sending a request
        let msg2 = McpMessage::Request(JsonRpcRequest {
            jsonrpc: "2.0".to_owned(),
            id: RequestId::Number(1),
            method: "tools/list".to_owned(),
            params: None,
        });
        let json2 = serde_json::to_value(&msg2).unwrap();
        assert_eq!(json2["method"], "tools/list");
    }
}
