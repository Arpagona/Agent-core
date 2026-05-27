//! MCP server lifecycle, dispatch, and handlers.
//!
//! This module implements the core MCP server loop:
//! 1. Read JSON-RPC requests from stdin (via transport)
//! 2. Dispatch to the appropriate handler based on method
//! 3. Write JSON-RPC responses to stdout
//!
//! Phase 1 handlers:
//! - `initialize` — server handshake, returns capabilities
//! - `tools/list` — returns available tools
//! - `tools/call` — executes a tool via the Tool Runtime

use crate::audit_store::{McpGovernanceAuditRecord, McpGovernanceAuditStore};
use crate::governance::{evaluate_tool_call, GovernanceDecision};
use crate::transport::{read_message, write_message};
use crate::types::McpMessage;
use crate::types::{
    CallToolResult, Implementation, InitializeResult, JsonRpcError, JsonRpcRequest, JsonRpcSuccess,
    ListToolsResult, McpContentBlock, McpTool, ServerCapabilities, ToolAnnotations,
    MCP_PROTOCOL_VERSION,
};
use arpagona_agent_core::ToolExecutionStatus;
use arpagona_decision_gate::audit_event_for_decision;
use arpagona_tool_runtime::ToolRuntime;
use chrono::Utc;
use serde_json::Value;
use std::path::Path;

/// Configuration for the MCP server.
#[derive(Clone, Debug)]
pub struct McpServerConfig {
    /// Server name advertised during initialize.
    pub server_name: String,
    /// Server version advertised during initialize.
    pub server_version: String,
    /// Workspace path for the tool runtime.
    pub workspace_path: String,
    /// Optional path for the governance audit log file.
    /// When set, every governance decision is persisted as a JSON-lines entry.
    pub audit_path: Option<String>,
}

impl Default for McpServerConfig {
    fn default() -> Self {
        Self {
            server_name: "arpagona-mcp".to_owned(),
            server_version: "0.1.0".to_owned(),
            workspace_path: ".".to_owned(),
            audit_path: None,
        }
    }
}

/// A known tool descriptor used by the MCP server.
struct KnownTool {
    name: &'static str,
    description: &'static str,
    input_schema: Value,
    read_only: bool,
}

/// Build the list of tools known to the MCP server.
///
/// Phase 1: 3 tools from the read-only Tool Runtime.
/// Phase 2+: also read from Tool Registry for dynamic tools.
fn known_tools() -> Vec<KnownTool> {
    vec![
        KnownTool {
            name: "read_file",
            description: "Read a file within the workspace. Returns file content with metadata.",
            read_only: true,
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file (relative to workspace)"
                    }
                },
                "required": ["path"]
            }),
        },
        KnownTool {
            name: "list_files",
            description: "List files and directories in a workspace path.",
            read_only: true,
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Directory path (relative to workspace, default: .)"
                    }
                }
            }),
        },
        KnownTool {
            name: "search_text",
            description: "Search for text patterns in workspace files.",
            read_only: true,
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Text or pattern to search for"
                    },
                    "path": {
                        "type": "string",
                        "description": "Directory to search in (relative to workspace, default: .)"
                    }
                },
                "required": ["query"]
            }),
        },
    ]
}

/// The MCP server.
///
/// Holds the tool runtime and handles protocol dispatch.
pub struct McpServer {
    /// Server configuration.
    pub config: McpServerConfig,
    tool_runtime: ToolRuntime,
    /// Optional governance audit store for persisting governance decisions.
    audit_store: Option<McpGovernanceAuditStore>,
    /// True once `initialize` has been called.
    initialized: bool,
}

impl McpServer {
    /// Create a new MCP server with the given configuration.
    pub fn new(config: McpServerConfig) -> Self {
        let tool_runtime = ToolRuntime::new(arpagona_tool_runtime::ToolRuntimeConfig::new(
            Path::new(&config.workspace_path),
        ));

        // Initialize audit store if a path was provided
        let audit_store = config
            .audit_path
            .as_ref()
            .and_then(|path| McpGovernanceAuditStore::new(path).ok());

        Self {
            config,
            tool_runtime,
            audit_store,
            initialized: false,
        }
    }

    /// Returns whether the server has been initialized.
    pub fn initialized(&self) -> bool {
        self.initialized
    }

    /// Return a reference to the audit store, if one is configured.
    pub fn audit_store(&self) -> Option<&McpGovernanceAuditStore> {
        self.audit_store.as_ref()
    }

    /// Run the MCP server loop: read requests from stdin and dispatch them.
    ///
    /// Blocks until stdin is closed (client disconnects) or an unrecoverable
    /// error occurs.
    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            let msg = match read_message()? {
                Some(msg) => msg,
                None => {
                    // EOF — client disconnected
                    break;
                }
            };

            self.dispatch(&msg);
        }
        Ok(())
    }

    /// Dispatch a single JSON-RPC request to the appropriate handler.
    fn dispatch(&mut self, req: &JsonRpcRequest) {
        let msg = self.handle_request_to_message(req);
        let _ = write_message(&msg);
    }

    /// Handle a JSON-RPC request and return the resulting MCP message.
    ///
    /// Returns either a `Success` or `Error` message. This method is the
    /// transport-agnostic bridge between the request and handler layers.
    /// HTTP transports (Axum) call this method directly instead of `dispatch`.
    pub fn handle_request_to_message(&mut self, req: &JsonRpcRequest) -> McpMessage {
        match req.method.as_str() {
            "initialize" => self
                .handle_initialize(req)
                .map(|v| McpMessage::Success(JsonRpcSuccess::new(req.id.clone(), v))),
            "tools/list" => self
                .handle_tools_list(req)
                .map(|v| McpMessage::Success(JsonRpcSuccess::new(req.id.clone(), v))),
            "tools/call" => self
                .handle_tools_call(req)
                .map(|v| McpMessage::Success(JsonRpcSuccess::new(req.id.clone(), v))),
            other => Err(JsonRpcError::method_not_found(req.id.clone(), other)),
        }
        .unwrap_or_else(|e| McpMessage::Error(e))
    }

    // -----------------------------------------------------------------------
    // Handler: initialize
    // -----------------------------------------------------------------------

    /// Handle the `initialize` handshake.
    ///
    /// Returns server capabilities and metadata.
    fn handle_initialize(&mut self, _req: &JsonRpcRequest) -> Result<Value, JsonRpcError> {
        self.initialized = true;

        let result = InitializeResult {
            protocol_version: MCP_PROTOCOL_VERSION.to_owned(),
            capabilities: ServerCapabilities {
                tools: Some(crate::types::ToolCapabilities { list_changed: None }),
                resources: None,
                prompts: None,
                logging: None,
                experimental: None,
            },
            server_info: Implementation {
                name: self.config.server_name.clone(),
                version: self.config.server_version.clone(),
            },
        };

        serde_json::to_value(&result).map_err(|e| {
            JsonRpcError::internal_error(
                _req.id.clone(),
                format!("Failed to serialize initialize result: {e}"),
            )
        })
    }

    // -----------------------------------------------------------------------
    // Handler: tools/list
    // -----------------------------------------------------------------------

    /// Handle `tools/list` — return all available tools as MCP tool definitions.
    fn handle_tools_list(&mut self, req: &JsonRpcRequest) -> Result<Value, JsonRpcError> {
        if !self.initialized {
            return Err(JsonRpcError::new(
                req.id.clone(),
                -32000,
                "Server not initialized. Send 'initialize' first.",
            ));
        }

        let tools: Vec<McpTool> = known_tools()
            .into_iter()
            .map(|kt| McpTool {
                name: kt.name.to_owned(),
                description: Some(kt.description.to_owned()),
                input_schema: kt.input_schema,
                annotations: Some(ToolAnnotations {
                    title: Some(kt.name.to_owned()),
                    read_only_hint: Some(kt.read_only),
                }),
            })
            .collect();

        serde_json::to_value(&ListToolsResult {
            tools,
            next_cursor: None,
        })
        .map_err(|e| {
            JsonRpcError::internal_error(req.id.clone(), format!("Serialization error: {e}"))
        })
    }

    // -----------------------------------------------------------------------
    // Handler: tools/call
    // -----------------------------------------------------------------------

    /// Handle `tools/call` — evaluate governance, then execute approved tools.
    ///
    /// Phase 2: every tool call is first evaluated through the DecisionGate
    /// governance layer. Blocked calls return structured error responses.
    fn handle_tools_call(&mut self, req: &JsonRpcRequest) -> Result<Value, JsonRpcError> {
        if !self.initialized {
            return Err(JsonRpcError::new(
                req.id.clone(),
                -32000,
                "Server not initialized. Send 'initialize' first.",
            ));
        }

        let params = req.params.as_ref().ok_or_else(|| {
            JsonRpcError::invalid_params(req.id.clone(), "Missing params for tools/call")
        })?;

        let tool_name = params.get("name").and_then(Value::as_str).ok_or_else(|| {
            JsonRpcError::invalid_params(req.id.clone(), "Missing 'name' in tools/call params")
        })?;

        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or(serde_json::json!({}));

        // Governance check — evaluate through DecisionGate
        let gov_result = evaluate_tool_call(tool_name, &arguments);

        // Create audit event for the governance decision
        let audit_event = audit_event_for_decision(
            &gov_result.proposed_action,
            &gov_result.decision_gate_decision,
        );

        // Persist audit event if audit store is configured
        if let Some(ref mut store) = self.audit_store {
            let outcome = match &gov_result.decision {
                GovernanceDecision::Approved { .. } => "Approved",
                GovernanceDecision::Blocked { .. } => "Blocked",
                GovernanceDecision::RequiresOverride { .. } => "RequiresOverride",
            };
            let record = McpGovernanceAuditRecord {
                outcome: outcome.to_owned(),
                tool_name: tool_name.to_owned(),
                arguments: arguments.clone(),
                summary: gov_result.decision.summary(),
                created_at: Utc::now(),
                audit_event: audit_event.clone(),
            };
            let _ = store.record(record);
        }

        if !gov_result.decision.is_approved() {
            return serde_json::to_value(&CallToolResult {
                content: vec![McpContentBlock::Text {
                    text: format!("Governance blocked: {}", gov_result.decision.summary()),
                    mime_type: None,
                }],
                structured_content: Some(serde_json::json!({
                    "governance": "blocked",
                    "reason": gov_result.decision.summary(),
                    "audit_event_id": audit_event.id,
                })),
                is_error: true,
            })
            .map_err(|e| {
                JsonRpcError::internal_error(req.id.clone(), format!("Serialization error: {e}"))
            });
        }

        // Execute the tool through the Tool Runtime (only after governance approval)
        let result = self.tool_runtime.execute(tool_name, &arguments);

        // Map the result to MCP CallToolResult format
        let (content, is_error) = match result.status {
            ToolExecutionStatus::Failed | ToolExecutionStatus::Blocked => {
                let error_msg = result
                    .error
                    .as_ref()
                    .map(|e| e.message.clone())
                    .unwrap_or_else(|| "Unknown error".to_owned());

                (
                    vec![McpContentBlock::Text {
                        text: format!("Error: {error_msg}"),
                        mime_type: None,
                    }],
                    true,
                )
            }
            _ => {
                // Success or Warning
                let obs_json = serde_json::to_value(&result.observation).unwrap_or_default();

                (
                    vec![
                        McpContentBlock::Text {
                            text: result.output_summary.clone(),
                            mime_type: None,
                        },
                        McpContentBlock::Json { json: obs_json },
                    ],
                    false,
                )
            }
        };

        serde_json::to_value(&CallToolResult {
            content,
            structured_content: Some(serde_json::json!({
                "execution_id": result.execution_id.to_string(),
                "tool_name": result.tool_name,
                "status": serde_json::to_value(&result.status).unwrap_or_default(),
            })),
            is_error,
        })
        .map_err(|e| {
            JsonRpcError::internal_error(req.id.clone(), format!("Serialization error: {e}"))
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::RequestId;
    use serde_json::json;

    fn make_request(method: &str, params: Option<Value>) -> JsonRpcRequest {
        JsonRpcRequest {
            jsonrpc: "2.0".to_owned(),
            id: RequestId::Number(1),
            method: method.to_owned(),
            params,
        }
    }

    #[test]
    fn test_known_tools_contains_three_tools() {
        let tools = known_tools();
        assert_eq!(tools.len(), 3, "Should have exactly 3 read-only tools");
        let names: Vec<&str> = tools.iter().map(|t| t.name).collect();
        assert!(names.contains(&"read_file"));
        assert!(names.contains(&"list_files"));
        assert!(names.contains(&"search_text"));
        assert!(
            tools.iter().all(|t| t.read_only),
            "All tools should be read-only"
        );
    }

    #[test]
    fn test_initialize_returns_capabilities() {
        let mut server = McpServer::new(McpServerConfig::default());
        let req = make_request(
            "initialize",
            Some(json!({
                "protocol_version": "2025-11-25",
                "capabilities": {},
                "client_info": {"name": "test", "version": "1.0"}
            })),
        );

        let result = server.handle_initialize(&req).unwrap();
        assert_eq!(result["protocol_version"], "2025-11-25");
        assert!(result["capabilities"]["tools"].is_object());
        assert_eq!(result["server_info"]["name"], "arpagona-mcp");
    }

    #[test]
    fn test_tools_list_after_initialize() {
        let mut server = McpServer::new(McpServerConfig::default());
        let init_req = make_request("initialize", None);
        server.handle_initialize(&init_req).unwrap();

        let req = make_request("tools/list", None);
        let result = server.handle_tools_list(&req).unwrap();
        let tools = result["tools"].as_array().unwrap();
        assert!(!tools.is_empty(), "Should have at least one tool");

        let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
        assert!(names.contains(&"read_file"));
        assert!(names.contains(&"list_files"));
        assert!(names.contains(&"search_text"));
    }

    #[test]
    fn test_tools_list_before_initialize_fails() {
        let mut server = McpServer::new(McpServerConfig::default());
        let req = make_request("tools/list", None);
        let result = server.handle_tools_list(&req);
        assert!(result.is_err(), "Should fail before initialize");
    }

    #[test]
    fn test_dispatch_unknown_method_does_not_panic() {
        let mut server = McpServer::new(McpServerConfig::default());
        let req = make_request("unknown_method", None);
        // dispatch writes to stdout — just verify it doesn't panic
        server.dispatch(&req);
    }

    #[test]
    fn test_tools_call_read_file_success() {
        let mut server = McpServer::new(McpServerConfig {
            workspace_path: ".".to_owned(),
            ..Default::default()
        });

        let init_req = make_request("initialize", None);
        server.handle_initialize(&init_req).unwrap();

        let req = make_request(
            "tools/call",
            Some(json!({
                "name": "read_file",
                "arguments": {"path": "Cargo.toml"}
            })),
        );

        let result = server.handle_tools_call(&req).unwrap();
        // is_error=false is skipped by skip_serializing_if, so null/missing = success
        assert!(
            result.get("is_error").is_none() || result["is_error"].as_bool() == Some(false),
            "Should succeed"
        );
        let content = result["content"].as_array().unwrap();
        assert!(!content.is_empty(), "Should have content blocks");
        assert!(
            content[0]["text"]
                .as_str()
                .unwrap_or("")
                .contains("Cargo.toml"),
            "Should mention the file path"
        );
    }

    #[test]
    fn test_tools_call_read_file_not_found() {
        let mut server = McpServer::new(McpServerConfig {
            workspace_path: ".".to_owned(),
            ..Default::default()
        });

        let init_req = make_request("initialize", None);
        server.handle_initialize(&init_req).unwrap();

        let req = make_request(
            "tools/call",
            Some(json!({
                "name": "read_file",
                "arguments": {"path": "nonexistent_file_xyz.txt"}
            })),
        );

        let result = server.handle_tools_call(&req).unwrap();
        assert_eq!(result["is_error"], true, "Should report error");
        let content = result["content"].as_array().unwrap();
        assert!(
            content[0]["text"].as_str().unwrap_or("").contains("Error"),
            "Should contain error message"
        );
    }

    #[test]
    fn test_tools_call_unknown_tool() {
        let mut server = McpServer::new(McpServerConfig::default());
        let init_req = make_request("initialize", None);
        server.handle_initialize(&init_req).unwrap();

        let req = make_request(
            "tools/call",
            Some(json!({
                "name": "nonexistent_tool",
                "arguments": {}
            })),
        );

        let result = server.handle_tools_call(&req).unwrap();
        assert_eq!(result["is_error"], true, "Unknown tool should error");
    }

    #[test]
    fn test_tools_call_without_initialize_fails() {
        let mut server = McpServer::new(McpServerConfig::default());
        let req = make_request(
            "tools/call",
            Some(json!({
                "name": "read_file",
                "arguments": {"path": "Cargo.toml"}
            })),
        );

        let result = server.handle_tools_call(&req);
        assert!(result.is_err(), "Should fail before initialize");
    }

    #[test]
    fn test_initialize_twice_is_idempotent() {
        let mut server = McpServer::new(McpServerConfig::default());
        let req = make_request("initialize", None);
        let result1 = server.handle_initialize(&req).unwrap();
        let result2 = server.handle_initialize(&req).unwrap();
        assert_eq!(result1["protocol_version"], result2["protocol_version"]);
    }
}
