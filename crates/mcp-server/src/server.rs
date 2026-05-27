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
    CallToolResult, GetPromptResult, Implementation, InitializeResult, JsonRpcError,
    JsonRpcNotification, JsonRpcRequest, JsonRpcSuccess, ListPromptsResult,
    ListResourceTemplatesResult, ListResourcesResult, ListToolsResult, McpContentBlock, McpPrompt,
    McpResource, McpResourceTemplate, McpTool, PromptArgument, PromptMessage, PromptMessageContent,
    ReadResourceResult, ResourceAnnotations, ResourceContents, ServerCapabilities, ToolAnnotations,
    MCP_PROTOCOL_VERSION,
};
use arpagona_agent_core::ToolExecutionStatus;
use arpagona_decision_gate::audit_event_for_decision;
use arpagona_tool_runtime::ToolRuntime;
use chrono::Utc;
use serde_json::Value;
use std::path::Path;
use tokio::sync::broadcast;

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
    /// Broadcast channel for sending notifications to connected clients
    /// (e.g. `notifications/tools/list_changed`).
    notification_tx: Option<broadcast::Sender<String>>,
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
            notification_tx: None,
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

    /// Set the broadcast notification channel for this server.
    ///
    /// Connected clients (e.g. SSE subscribers) will receive notifications
    /// such as `notifications/tools/list_changed` through this channel.
    pub fn set_notification_channel(&mut self, tx: broadcast::Sender<String>) {
        self.notification_tx = Some(tx);
    }

    /// Send a `notifications/tools/list_changed` notification to all
    /// connected clients.
    ///
    /// Call this whenever the tool list is modified (tools added, removed,
    /// or updated). Currently a no-op if no notification channel is set;
    /// in the future this will also write to the stdio transport when
    /// the server runs in stdio mode.
    pub fn notify_tools_list_changed(&self) {
        if let Some(ref tx) = self.notification_tx {
            let notification = JsonRpcNotification {
                jsonrpc: "2.0".to_owned(),
                method: "notifications/tools/list_changed".to_owned(),
                params: None,
            };
            if let Ok(json) = serde_json::to_string(&notification) {
                let _ = tx.send(json);
            }
        }
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
            "resources/list" => self
                .handle_resources_list(req)
                .map(|v| McpMessage::Success(JsonRpcSuccess::new(req.id.clone(), v))),
            "resources/templates/list" => self
                .handle_resources_templates_list(req)
                .map(|v| McpMessage::Success(JsonRpcSuccess::new(req.id.clone(), v))),
            "resources/read" => self
                .handle_resources_read(req)
                .map(|v| McpMessage::Success(JsonRpcSuccess::new(req.id.clone(), v))),
            "prompts/list" => self
                .handle_prompts_list(req)
                .map(|v| McpMessage::Success(JsonRpcSuccess::new(req.id.clone(), v))),
            "prompts/get" => self
                .handle_prompts_get(req)
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
            capabilities: ServerCapabilities::default(),
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

    // -----------------------------------------------------------------------
    // Handler: resources/list
    // -----------------------------------------------------------------------

    fn handle_resources_list(&mut self, req: &JsonRpcRequest) -> Result<Value, JsonRpcError> {
        if !self.initialized {
            return Err(JsonRpcError::new(
                req.id.clone(),
                -32000,
                "Server not initialized. Send 'initialize' first.",
            ));
        }

        let mut resources = vec![
            McpResource {
                uri: "arpagona://server/info".to_owned(),
                name: "Server Info".to_owned(),
                description: Some("MCP server metadata and configuration".to_owned()),
                mime_type: Some("application/json".to_owned()),
                annotations: Some(ResourceAnnotations {
                    title: Some("Server Info".to_owned()),
                    read_only_hint: Some(true),
                }),
            },
            McpResource {
                uri: "arpagona://tools/list".to_owned(),
                name: "Available Tools".to_owned(),
                description: Some("List of all tools exposed by this server".to_owned()),
                mime_type: Some("application/json".to_owned()),
                annotations: Some(ResourceAnnotations {
                    title: Some("Tools".to_owned()),
                    read_only_hint: Some(true),
                }),
            },
            McpResource {
                uri: "arpagona://audit/recent".to_owned(),
                name: "Recent Governance Audit".to_owned(),
                description: Some(
                    "Recent governance decisions (blocked/approved tool calls)".to_owned(),
                ),
                mime_type: Some("application/json".to_owned()),
                annotations: Some(ResourceAnnotations {
                    title: Some("Audit Log".to_owned()),
                    read_only_hint: Some(true),
                }),
            },
        ];

        if self.audit_store.is_some() {
            resources.push(McpResource {
                uri: "arpagona://audit/stats".to_owned(),
                name: "Audit Statistics".to_owned(),
                description: Some("Summary statistics of governance decisions".to_owned()),
                mime_type: Some("application/json".to_owned()),
                annotations: Some(ResourceAnnotations {
                    title: Some("Audit Stats".to_owned()),
                    read_only_hint: Some(true),
                }),
            });
        }

        serde_json::to_value(&ListResourcesResult {
            resources,
            next_cursor: None,
        })
        .map_err(|e| {
            JsonRpcError::internal_error(req.id.clone(), format!("Serialization error: {e}"))
        })
    }

    // -----------------------------------------------------------------------
    // Handler: resources/templates/list
    // -----------------------------------------------------------------------

    fn handle_resources_templates_list(
        &mut self,
        req: &JsonRpcRequest,
    ) -> Result<Value, JsonRpcError> {
        if !self.initialized {
            return Err(JsonRpcError::new(
                req.id.clone(),
                -32000,
                "Server not initialized. Send 'initialize' first.",
            ));
        }

        let templates = vec![McpResourceTemplate {
            uri_template: "arpagona://audit/by-id/{audit_id}".to_owned(),
            name: "Audit Record by ID".to_owned(),
            description: Some(
                "Retrieve a specific governance audit record by its audit event ID".to_owned(),
            ),
            mime_type: Some("application/json".to_owned()),
        }];

        serde_json::to_value(&ListResourceTemplatesResult {
            resource_templates: templates,
            next_cursor: None,
        })
        .map_err(|e| {
            JsonRpcError::internal_error(req.id.clone(), format!("Serialization error: {e}"))
        })
    }

    // -----------------------------------------------------------------------
    // Handler: resources/read
    // -----------------------------------------------------------------------

    fn handle_resources_read(&mut self, req: &JsonRpcRequest) -> Result<Value, JsonRpcError> {
        if !self.initialized {
            return Err(JsonRpcError::new(
                req.id.clone(),
                -32000,
                "Server not initialized. Send 'initialize' first.",
            ));
        }

        let params = req.params.as_ref().ok_or_else(|| {
            JsonRpcError::invalid_params(req.id.clone(), "Missing params for resources/read")
        })?;

        let uri = params.get("uri").and_then(Value::as_str).ok_or_else(|| {
            JsonRpcError::invalid_params(req.id.clone(), "Missing 'uri' in resources/read params")
        })?;

        let contents = match uri {
            "arpagona://server/info" => {
                let info = serde_json::json!({
                    "name": self.config.server_name,
                    "version": self.config.server_version,
                    "workspace_path": self.config.workspace_path,
                    "audit_enabled": self.audit_store.is_some(),
                });
                vec![ResourceContents {
                    uri: uri.to_owned(),
                    mime_type: Some("application/json".to_owned()),
                    text: serde_json::to_string_pretty(&info).unwrap_or_else(|_| "{}".to_owned()),
                }]
            }
            "arpagona://tools/list" => {
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
                let result = ListToolsResult {
                    tools,
                    next_cursor: None,
                };
                vec![ResourceContents {
                    uri: uri.to_owned(),
                    mime_type: Some("application/json".to_owned()),
                    text: serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_owned()),
                }]
            }
            "arpagona://audit/recent" => {
                let records = self
                    .audit_store
                    .as_ref()
                    .map(|store| store.recent(10).into_iter().cloned().collect::<Vec<_>>())
                    .unwrap_or_default();
                vec![ResourceContents {
                    uri: uri.to_owned(),
                    mime_type: Some("application/json".to_owned()),
                    text: serde_json::to_string_pretty(&records)
                        .unwrap_or_else(|_| "[]".to_owned()),
                }]
            }
            "arpagona://audit/stats" => {
                let summary = if let Some(ref store) = self.audit_store {
                    let all = store.all().to_vec();
                    let total = all.len();
                    let blocked = all.iter().filter(|r| r.outcome == "Blocked").count();
                    let approved = all.iter().filter(|r| r.outcome == "Approved").count();
                    let pending = all
                        .iter()
                        .filter(|r| r.outcome == "RequiresOverride")
                        .count();
                    serde_json::json!({
                        "total_governance_decisions": total,
                        "blocked": blocked,
                        "approved": approved,
                        "requires_override": pending,
                    })
                } else {
                    serde_json::json!({
                        "message": "Audit store not configured. Set audit_path in McpServerConfig."
                    })
                };
                vec![ResourceContents {
                    uri: uri.to_owned(),
                    mime_type: Some("application/json".to_owned()),
                    text: serde_json::to_string_pretty(&summary)
                        .unwrap_or_else(|_| "{}".to_owned()),
                }]
            }
            _ => {
                return Err(JsonRpcError::invalid_params(
                    req.id.clone(),
                    format!("Unknown resource URI: {uri}"),
                ));
            }
        };

        serde_json::to_value(&ReadResourceResult { contents }).map_err(|e| {
            JsonRpcError::internal_error(req.id.clone(), format!("Serialization error: {e}"))
        })
    }

    // -----------------------------------------------------------------------
    // Handler: prompts/list
    // -----------------------------------------------------------------------

    fn handle_prompts_list(&mut self, req: &JsonRpcRequest) -> Result<Value, JsonRpcError> {
        if !self.initialized {
            return Err(JsonRpcError::new(
                req.id.clone(),
                -32000,
                "Server not initialized. Send 'initialize' first.",
            ));
        }

        let prompts = vec![
            McpPrompt {
                name: "assess_governance".to_owned(),
                description: Some(
                    "Analyze governance decisions and identify patterns in blocked/approved tool calls"
                        .to_owned(),
                ),
                arguments: Some(vec![
                    PromptArgument {
                        name: "filter".to_owned(),
                        description: Some(
                            "Optional filter: 'blocked', 'approved', or 'all' (default)"
                                .to_owned(),
                        ),
                        required: false,
                    },
                    PromptArgument {
                        name: "limit".to_owned(),
                        description: Some(
                            "Maximum number of audit records to analyze (default: 10)"
                                .to_owned(),
                        ),
                        required: false,
                    },
                ]),
            },
            McpPrompt {
                name: "summarize_context".to_owned(),
                description: Some(
                    "Summarize the current server context: available tools, resources, and governance status"
                        .to_owned(),
                ),
                arguments: None,
            },
            McpPrompt {
                name: "inspect_audit_record".to_owned(),
                description: Some(
                    "Get a detailed analysis of a specific governance decision by audit ID"
                        .to_owned(),
                ),
                arguments: Some(vec![PromptArgument {
                    name: "audit_id".to_owned(),
                    description: Some("The audit event ID to inspect".to_owned()),
                    required: true,
                }]),
            },
        ];

        serde_json::to_value(&ListPromptsResult {
            prompts,
            next_cursor: None,
        })
        .map_err(|e| {
            JsonRpcError::internal_error(req.id.clone(), format!("Serialization error: {e}"))
        })
    }

    // -----------------------------------------------------------------------
    // Handler: prompts/get
    // -----------------------------------------------------------------------

    fn handle_prompts_get(&mut self, req: &JsonRpcRequest) -> Result<Value, JsonRpcError> {
        if !self.initialized {
            return Err(JsonRpcError::new(
                req.id.clone(),
                -32000,
                "Server not initialized. Send 'initialize' first.",
            ));
        }

        let params = req.params.as_ref().ok_or_else(|| {
            JsonRpcError::invalid_params(req.id.clone(), "Missing params for prompts/get")
        })?;

        let name = params.get("name").and_then(Value::as_str).ok_or_else(|| {
            JsonRpcError::invalid_params(req.id.clone(), "Missing 'name' in prompts/get params")
        })?;

        let arguments = params
            .get("arguments")
            .and_then(|a| a.as_object())
            .cloned()
            .unwrap_or_default();

        match name {
            "assess_governance" => {
                let filter = arguments
                    .get("filter")
                    .and_then(|v| v.as_str())
                    .unwrap_or("all");
                let limit = arguments
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(10);

                let records = self
                    .audit_store
                    .as_ref()
                    .map(|store| {
                        let all: Vec<_> = store
                            .all()
                            .iter()
                            .filter(|r| match filter {
                                "blocked" => r.outcome == "Blocked",
                                "approved" => r.outcome == "Approved",
                                _ => true,
                            })
                            .take(limit as usize)
                            .collect();
                        all
                    })
                    .unwrap_or_default();

                let records_json =
                    serde_json::to_string_pretty(&records).unwrap_or_else(|_| "[]".to_owned());

                let message = if records.is_empty() {
                    format!(
                        "No governance audit records were found for filter '{}'.\n\n\
                         The audit store is {}configured on this server.",
                        filter,
                        if self.audit_store.is_some() {
                            ""
                        } else {
                            "NOT "
                        }
                    )
                } else {
                    format!(
                        "Governance Analysis for filter: {} (showing {} of {})\n\n\
                         Below are the governance decision records matching your filter.\n\
                         Each record shows the tool name, outcome (Approved/Blocked/RequiresOverride),\n\
                         and the governance summary provided by the DecisionGate.\n\n\
                         {}\n\n\
                         Note: This analysis is informative only and does not override or bypass\n\
                         any governance decision. Use 'inspect_audit_record' to drill into a specific ID.",
                        filter,
                        records.len(),
                        self.audit_store
                            .as_ref()
                            .map(|s| s.len())
                            .unwrap_or(0),
                        records_json
                    )
                };

                serde_json::to_value(&GetPromptResult {
                    messages: vec![PromptMessage {
                        role: "assistant".to_owned(),
                        content: PromptMessageContent::Text {
                            text: message,
                            mime_type: None,
                        },
                    }],
                    description: Some(format!(
                        "Governance analysis with filter '{}', limit {}",
                        filter, limit
                    )),
                })
                .map_err(|e| {
                    JsonRpcError::internal_error(
                        req.id.clone(),
                        format!("Serialization error: {e}"),
                    )
                })
            }
            "summarize_context" => {
                let tool_count = known_tools().len();
                let resource_count = 4;
                let audit_enabled = self.audit_store.is_some();
                let audit_count = self.audit_store.as_ref().map(|s| s.len()).unwrap_or(0);

                let message = format!(
                    "Arpagona MCP Server Context Summary\n\n\
                     - Tools exposed: {}\n\
                     - Resources available: {}\n\
                     - Prompts available: 3\n\
                     - Audit store: {}\n\
                     - Governance decisions recorded: {}\n\n\
                     This server provides read-only access to Arpagona's tool runtime\n\
                     and governance audit trail. Every tool/call is evaluated through\n\
                     the DecisionGate before execution. Blocked calls produce structured\n\
                     error responses with governance reasons.",
                    tool_count,
                    resource_count,
                    if audit_enabled { "enabled" } else { "disabled" },
                    audit_count
                );

                serde_json::to_value(&GetPromptResult {
                    messages: vec![PromptMessage {
                        role: "assistant".to_owned(),
                        content: PromptMessageContent::Text {
                            text: message,
                            mime_type: None,
                        },
                    }],
                    description: Some("Server context summary".to_owned()),
                })
                .map_err(|e| {
                    JsonRpcError::internal_error(
                        req.id.clone(),
                        format!("Serialization error: {e}"),
                    )
                })
            }
            "inspect_audit_record" => {
                let audit_id = arguments
                    .get("audit_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let record = self.audit_store.as_ref().and_then(|store| {
                    store
                        .all()
                        .iter()
                        .find(|r| r.audit_event.id.to_string() == audit_id)
                });

                let message = if let Some(ref rec) = record {
                    let rec_json =
                        serde_json::to_string_pretty(rec).unwrap_or_else(|_| "{}".to_owned());
                    format!(
                        "Audit Record: {}\n\n\
                         Full record details:\n\
                         {}\n\n\
                         Governance Decision: {}\n\
                         Tool: {}\n\n\
                         This is a permanent record of a governance decision.\n\
                         It cannot be modified after creation.",
                        audit_id, rec_json, rec.outcome, rec.tool_name
                    )
                } else {
                    format!(
                        "No audit record found with ID: {}\n\n\
                         To see available audit records, use:\n\
                         - resources/read arpagona://audit/recent\n\
                         - assess_governance prompt to filter results",
                        audit_id
                    )
                };

                serde_json::to_value(&GetPromptResult {
                    messages: vec![PromptMessage {
                        role: "assistant".to_owned(),
                        content: PromptMessageContent::Text {
                            text: message,
                            mime_type: None,
                        },
                    }],
                    description: Some(format!("Inspect audit record: {}", audit_id)),
                })
                .map_err(|e| {
                    JsonRpcError::internal_error(
                        req.id.clone(),
                        format!("Serialization error: {e}"),
                    )
                })
            }
            other => Err(JsonRpcError::invalid_params(
                req.id.clone(),
                format!("Unknown prompt name: {other}"),
            )),
        }
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

    #[test]
    fn test_resources_list_returns_resources() {
        let mut server = McpServer::new(McpServerConfig::default());
        let init_req = make_request("initialize", None);
        server.handle_initialize(&init_req).unwrap();

        let req = make_request("resources/list", None);
        let result = server.handle_resources_list(&req).unwrap();
        let resources = result["resources"].as_array().unwrap();
        assert!(!resources.is_empty(), "Should have at least one resource");
        let uris: Vec<&str> = resources.iter().filter_map(|r| r["uri"].as_str()).collect();
        assert!(uris.contains(&"arpagona://server/info"));
        assert!(uris.contains(&"arpagona://tools/list"));
        assert!(uris.contains(&"arpagona://audit/recent"));
    }

    #[test]
    fn test_resources_list_before_initialize_fails() {
        let mut server = McpServer::new(McpServerConfig::default());
        let req = make_request("resources/list", None);
        let result = server.handle_resources_list(&req);
        assert!(result.is_err(), "Should fail before initialize");
    }

    #[test]
    fn test_resources_templates_list_returns_templates() {
        let mut server = McpServer::new(McpServerConfig::default());
        let init_req = make_request("initialize", None);
        server.handle_initialize(&init_req).unwrap();

        let req = make_request("resources/templates/list", None);
        let result = server.handle_resources_templates_list(&req).unwrap();
        let templates = result["resource_templates"].as_array().unwrap();
        assert!(!templates.is_empty(), "Should have at least one template");
        assert_eq!(
            templates[0]["uri_template"],
            "arpagona://audit/by-id/{audit_id}"
        );
    }

    #[test]
    fn test_resources_read_server_info() {
        let mut server = McpServer::new(McpServerConfig::default());
        let init_req = make_request("initialize", None);
        server.handle_initialize(&init_req).unwrap();

        let req = make_request(
            "resources/read",
            Some(json!({"uri": "arpagona://server/info"})),
        );
        let result = server.handle_resources_read(&req).unwrap();
        let contents = result["contents"].as_array().unwrap();
        assert!(!contents.is_empty());
        assert!(contents[0]["text"]
            .as_str()
            .unwrap_or("")
            .contains("arpagona-mcp"));
    }

    #[test]
    fn test_resources_read_tools_list() {
        let mut server = McpServer::new(McpServerConfig::default());
        let init_req = make_request("initialize", None);
        server.handle_initialize(&init_req).unwrap();

        let req = make_request(
            "resources/read",
            Some(json!({"uri": "arpagona://tools/list"})),
        );
        let result = server.handle_resources_read(&req).unwrap();
        let contents = result["contents"].as_array().unwrap();
        assert!(!contents.is_empty());
        assert!(contents[0]["text"]
            .as_str()
            .unwrap_or("")
            .contains("read_file"));
    }

    #[test]
    fn test_resources_read_unknown_uri_fails() {
        let mut server = McpServer::new(McpServerConfig::default());
        let init_req = make_request("initialize", None);
        server.handle_initialize(&init_req).unwrap();

        let req = make_request(
            "resources/read",
            Some(json!({"uri": "arpagona://unknown/resource"})),
        );
        let result = server.handle_resources_read(&req);
        assert!(result.is_err(), "Should fail for unknown URI");
    }

    #[test]
    fn test_prompts_list_returns_prompts() {
        let mut server = McpServer::new(McpServerConfig::default());
        let init_req = make_request("initialize", None);
        server.handle_initialize(&init_req).unwrap();

        let req = make_request("prompts/list", None);
        let result = server.handle_prompts_list(&req).unwrap();
        let prompts = result["prompts"].as_array().unwrap();
        assert!(!prompts.is_empty(), "Should have at least one prompt");
        let names: Vec<&str> = prompts.iter().filter_map(|p| p["name"].as_str()).collect();
        assert!(names.contains(&"assess_governance"));
        assert!(names.contains(&"summarize_context"));
        assert!(names.contains(&"inspect_audit_record"));
    }

    #[test]
    fn test_prompts_get_summarize_context() {
        let mut server = McpServer::new(McpServerConfig::default());
        let init_req = make_request("initialize", None);
        server.handle_initialize(&init_req).unwrap();

        let req = make_request("prompts/get", Some(json!({"name": "summarize_context"})));
        let result = server.handle_prompts_get(&req).unwrap();
        let messages = result["messages"].as_array().unwrap();
        assert!(!messages.is_empty(), "Should have at least one message");
        let text = messages[0]["content"]["text"].as_str().unwrap_or("");
        assert!(text.contains("Tools exposed"), "Should mention tools");
        assert!(
            text.contains("Resources available"),
            "Should mention resources"
        );
        assert!(text.contains("Prompts available"), "Should mention prompts");
    }

    #[test]
    fn test_prompts_get_unknown_name_fails() {
        let mut server = McpServer::new(McpServerConfig::default());
        let init_req = make_request("initialize", None);
        server.handle_initialize(&init_req).unwrap();

        let req = make_request("prompts/get", Some(json!({"name": "nonexistent_prompt"})));
        let result = server.handle_prompts_get(&req);
        assert!(result.is_err(), "Should fail for unknown prompt name");
    }

    #[test]
    fn test_prompts_get_before_initialize_fails() {
        let mut server = McpServer::new(McpServerConfig::default());
        let req = make_request("prompts/get", Some(json!({"name": "summarize_context"})));
        let result = server.handle_prompts_get(&req);
        assert!(result.is_err(), "Should fail before initialize");
    }

    #[test]
    fn test_resources_templates_list_before_initialize_fails() {
        let mut server = McpServer::new(McpServerConfig::default());
        let req = make_request("resources/templates/list", None);
        let result = server.handle_resources_templates_list(&req);
        assert!(result.is_err(), "Should fail before initialize");
    }

    #[test]
    fn test_resources_read_before_initialize_fails() {
        let mut server = McpServer::new(McpServerConfig::default());
        let req = make_request(
            "resources/read",
            Some(json!({"uri": "arpagona://server/info"})),
        );
        let result = server.handle_resources_read(&req);
        assert!(result.is_err(), "Should fail before initialize");
    }

    #[test]
    fn test_notification_channel_set_and_notify() {
        let (tx, mut rx) = broadcast::channel(10);
        let mut server = McpServer::new(McpServerConfig::default());

        // Before setting, notify should be a no-op (no panic)
        server.notify_tools_list_changed();

        // Set the channel
        server.set_notification_channel(tx);

        // Send notification
        server.notify_tools_list_changed();

        // Verify the notification was received
        let received = rx.try_recv().unwrap();
        assert!(
            received.contains("notifications/tools/list_changed"),
            "Should contain the MCP notification method name"
        );
        assert!(received.contains("2.0"), "Should be JSON-RPC 2.0 format");
    }

    #[test]
    fn test_notify_tools_list_changed_format() {
        let (tx, mut rx) = broadcast::channel(10);
        let mut server = McpServer::new(McpServerConfig::default());
        server.set_notification_channel(tx);

        server.notify_tools_list_changed();

        let received = rx.try_recv().unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&received).unwrap();
        assert_eq!(parsed["jsonrpc"], "2.0");
        assert_eq!(parsed["method"], "notifications/tools/list_changed");
        assert!(
            parsed.get("id").is_none(),
            "Notifications should not have an 'id' field"
        );
    }

    #[test]
    fn test_notification_broadcast_to_multiple_receivers() {
        let (tx, mut rx1) = broadcast::channel(10);
        let mut rx2 = tx.subscribe();
        let mut server = McpServer::new(McpServerConfig::default());
        server.set_notification_channel(tx);

        server.notify_tools_list_changed();

        // Both receivers should get the notification
        let msg1 = rx1.try_recv().unwrap();
        let msg2 = rx2.try_recv().unwrap();
        assert_eq!(
            msg1, msg2,
            "Both receivers should get the same notification"
        );
        assert!(
            msg1.contains("notifications/tools/list_changed"),
            "Should be tools/list_changed notification"
        );
    }
}
