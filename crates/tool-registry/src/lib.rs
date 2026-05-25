//! Declarative Tool Registry primitives for ARPAGONA Agent Core.
//!
//! The Tool Registry declares which tools exist, what schemas they expose,
//! which permissions they require, and whether they are enabled.
//!
//! It deliberately does not execute tools, call external systems, open shells,
//! schedule work, drive browsers, access MCP, or bypass the Decision Gate.
//! Registry lookup is descriptive only: every future tool invocation must still
//! become a proposed action and pass through governance before any execution.

use arpagona_agent_core::{Permission, RiskLevel, ToolDefinition, ToolId, ToolStatus};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Registry-local alias for permissions required by a declared tool.
pub type ToolPermission = Permission;

/// Registry-local alias for the default risk level of a declared tool.
pub type ToolRiskLevel = RiskLevel;

/// Stable declarative category for a tool.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolKind {
    Deterministic,
    HumanWorkflow,
    DataLookup,
    FileOperation,
    NetworkOperation,
    ExternalService,
    CognitiveSupport,
}

/// Declarative capability advertised by a tool.
///
/// Capabilities describe what a tool can support. They do not grant execution
/// rights and they do not bypass permissions, policies or the Decision Gate.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCapability {
    ReadOnlyLookup,
    ReadFile,
    ListFiles,
    SearchText,
    DataTransformation,
    HumanNotificationDraft,
    FileReadDeclaration,
    FileWriteDeclaration,
    NetworkRequestDeclaration,
    ExternalServiceDeclaration,
    ShellAccess,
    EmailSend,
    BrowserAutomation,
    MCPAccess,
}

/// Cognitive role a tool can play in the agent loop.
///
/// These roles separate *what a tool does* (its kind/capability) from
/// *why it is used* (its cognitive role). The same tool could serve
/// Perception today and Validation tomorrow depending on context.
///
/// The following roles are considered safe for alpha read-only execution:
/// - Perception
/// - Inspection
/// - Validation
///
/// These roles exist as concepts but must remain **non-executable** in the
/// current alpha tool runtime:
/// - Transformation
/// - Execution
/// - Communication
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCognitiveRole {
    /// Observe the environment (list files, read metadata).
    Perception,
    /// Retrieve previously stored information.
    Recall,
    /// Inspect content in detail (read file content, search text).
    Inspection,
    /// Modify or transform data (write files, rename).
    Transformation,
    /// Verify assumptions or assert invariants.
    Validation,
    /// Execute a side-effecting operation.
    Execution,
    /// Send messages, notifications, emails.
    Communication,
    /// Reflect on execution outcomes.
    Reflection,
}

impl ToolCognitiveRole {
    /// Whether this role is safe for alpha read-only execution.
    pub fn is_safe_for_read_only(&self) -> bool {
        matches!(
            self,
            ToolCognitiveRole::Perception
                | ToolCognitiveRole::Inspection
                | ToolCognitiveRole::Validation
        )
    }

    /// Whether this role is currently non-executable by design.
    pub fn is_non_executable(&self) -> bool {
        matches!(
            self,
            ToolCognitiveRole::Transformation
                | ToolCognitiveRole::Execution
                | ToolCognitiveRole::Communication
        )
    }
}

/// Risk profile for a tool declaration.
///
/// This is a declarative annotation that helps tool selection and audit.
/// It does not authorise execution or bypass the Decision Gate.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolRiskProfile {
    pub default_risk: ToolRiskLevel,
    pub requires_human_approval: bool,
    pub requires_decision_gate: bool,
    pub audit_required: bool,
}

impl Default for ToolRiskProfile {
    fn default() -> Self {
        Self {
            default_risk: ToolRiskLevel::Low,
            requires_human_approval: false,
            requires_decision_gate: true,
            audit_required: true,
        }
    }
}

/// Declarative schema bundle for a tool.
///
/// The values are JSON Schema-compatible shapes, but the registry does not
/// validate payloads or execute anything. It only carries declarations.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolSchema {
    pub input_schema: Value,
    pub output_schema: Option<Value>,
}

impl ToolSchema {
    pub fn new(input_schema: Value, output_schema: Option<Value>) -> Self {
        Self {
            input_schema,
            output_schema,
        }
    }
}

/// Human-readable governance notes attached to a tool declaration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolGovernance {
    pub requires_decision_gate: bool,
    pub requires_human_approval: bool,
    pub audit_required: bool,
    pub notes: Vec<String>,
}

impl Default for ToolGovernance {
    fn default() -> Self {
        Self {
            requires_decision_gate: true,
            requires_human_approval: false,
            audit_required: true,
            notes: Vec::new(),
        }
    }
}

/// A declared tool catalogue entry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RegisteredTool {
    pub definition: ToolDefinition,
    pub kind: ToolKind,
    pub capabilities: Vec<ToolCapability>,
    pub cognitive_role: Option<ToolCognitiveRole>,
    pub risk_profile: Option<ToolRiskProfile>,
    pub schema: ToolSchema,
    pub governance: ToolGovernance,
    pub tags: Vec<String>,
    pub registered_at: DateTime<Utc>,
}

impl RegisteredTool {
    pub fn new(
        definition: ToolDefinition,
        kind: ToolKind,
        schema: ToolSchema,
        cognitive_role: Option<ToolCognitiveRole>,
        risk_profile: Option<ToolRiskProfile>,
    ) -> Self {
        Self {
            definition,
            kind,
            capabilities: Vec::new(),
            cognitive_role,
            risk_profile,
            schema,
            governance: ToolGovernance::default(),
            tags: Vec::new(),
            registered_at: Utc::now(),
        }
    }

    pub fn id(&self) -> &ToolId {
        &self.definition.id
    }

    pub fn is_enabled(&self) -> bool {
        matches!(self.definition.status, ToolStatus::Available)
    }
}

/// Result of a pure registry lookup.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolLookupStatus {
    Found,
    Disabled,
    Deprecated,
    NotFound,
}

/// Descriptive lookup output. This is not execution approval.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolLookup {
    pub status: ToolLookupStatus,
    pub tool: Option<RegisteredTool>,
    pub reason: String,
}

/// In-memory declarative registry.
///
/// This type is intentionally simple and pure. It performs no I/O and has no
/// execution callback, runner, scheduler, shell, MCP client, browser driver, or
/// provider hook.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ToolRegistry {
    tools: Vec<RegisteredTool>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_tools(tools: Vec<RegisteredTool>) -> Result<Self, ToolRegistryError> {
        let mut registry = Self::new();
        for tool in tools {
            registry.register(tool)?;
        }
        Ok(registry)
    }

    pub fn register(&mut self, tool: RegisteredTool) -> Result<(), ToolRegistryError> {
        if self.tools.iter().any(|existing| existing.id() == tool.id()) {
            return Err(ToolRegistryError::DuplicateToolId(tool.id().clone()));
        }

        self.tools.push(tool);
        Ok(())
    }

    pub fn contains(&self, id: &ToolId) -> bool {
        self.tools.iter().any(|tool| tool.id() == id)
    }

    pub fn disable(&mut self, id: &ToolId) -> Result<(), ToolRegistryError> {
        match self.tools.iter_mut().find(|tool| tool.id() == id) {
            Some(tool) => {
                tool.definition.status = ToolStatus::Disabled;
                Ok(())
            }
            None => Err(ToolRegistryError::ToolNotFound(id.clone())),
        }
    }

    pub fn all(&self) -> &[RegisteredTool] {
        &self.tools
    }

    pub fn enabled_tools(&self) -> Vec<&RegisteredTool> {
        self.tools.iter().filter(|tool| tool.is_enabled()).collect()
    }

    pub fn lookup(&self, id: &ToolId) -> ToolLookup {
        match self.tools.iter().find(|tool| tool.id() == id) {
            Some(tool) if matches!(tool.definition.status, ToolStatus::Available) => ToolLookup {
                status: ToolLookupStatus::Found,
                tool: Some(tool.clone()),
                reason: "Tool is declared as available; this does not approve or execute it."
                    .to_owned(),
            },
            Some(tool) if matches!(tool.definition.status, ToolStatus::Disabled) => ToolLookup {
                status: ToolLookupStatus::Disabled,
                tool: Some(tool.clone()),
                reason:
                    "Tool is declared but disabled; no execution is possible from the registry."
                        .to_owned(),
            },
            Some(tool) => ToolLookup {
                status: ToolLookupStatus::Deprecated,
                tool: Some(tool.clone()),
                reason:
                    "Tool is declared but deprecated; no execution is possible from the registry."
                        .to_owned(),
            },
            None => ToolLookup {
                status: ToolLookupStatus::NotFound,
                tool: None,
                reason: "No tool declaration exists for this identifier.".to_owned(),
            },
        }
    }
}

/// Registry construction errors. These are catalogue errors, not runtime errors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolRegistryError {
    DuplicateToolId(ToolId),
    ToolNotFound(ToolId),
}

impl std::fmt::Display for ToolRegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateToolId(id) => write!(f, "duplicate tool id: {id}"),
            Self::ToolNotFound(id) => write!(f, "tool not found: {id}"),
        }
    }
}

impl std::error::Error for ToolRegistryError {}

pub fn declare_tool(
    id: impl Into<ToolId>,
    name: impl Into<String>,
    description: impl Into<String>,
    kind: ToolKind,
    schema: ToolSchema,
    required_permissions: Vec<ToolPermission>,
    default_risk_level: ToolRiskLevel,
    cognitive_role: Option<ToolCognitiveRole>,
    risk_profile: Option<ToolRiskProfile>,
) -> RegisteredTool {
    RegisteredTool::new(
        ToolDefinition {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            required_permissions,
            default_risk_level,
            status: ToolStatus::Available,
        },
        kind,
        schema,
        cognitive_role,
        risk_profile,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn schema() -> ToolSchema {
        ToolSchema::new(
            json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string" }
                },
                "required": ["query"]
            }),
            Some(json!({
                "type": "object",
                "properties": {
                    "summary": { "type": "string" }
                }
            })),
        )
    }

    fn declared_tool(id: &str, status: ToolStatus) -> RegisteredTool {
        let mut tool = declare_tool(
            id,
            "Knowledge lookup",
            "Declarative lookup tool used only for registry tests",
            ToolKind::DataLookup,
            schema(),
            Vec::new(),
            RiskLevel::Low,
            Some(ToolCognitiveRole::Recall),
            Some(ToolRiskProfile {
                default_risk: ToolRiskLevel::Low,
                requires_human_approval: false,
                requires_decision_gate: true,
                audit_required: true,
            }),
        );
        tool.definition.status = status;
        tool.capabilities = vec![ToolCapability::ReadOnlyLookup];
        tool
    }

    #[test]
    fn registry_registers_and_lists_enabled_declarations() {
        let available = declared_tool("knowledge.lookup", ToolStatus::Available);
        let disabled = declared_tool("email.send", ToolStatus::Disabled);
        let registry = ToolRegistry::from_tools(vec![available.clone(), disabled])
            .expect("unique declarations should register");

        assert_eq!(registry.all().len(), 2);
        assert_eq!(registry.enabled_tools(), vec![&available]);
        assert!(registry.contains(&ToolId::new("knowledge.lookup")));
        assert!(!registry.contains(&ToolId::new("missing")));
    }

    #[test]
    fn disable_changes_available_tool_status() {
        let mut registry = ToolRegistry::from_tools(vec![declared_tool(
            "knowledge.lookup",
            ToolStatus::Available,
        )])
        .expect("registry should build");

        registry
            .disable(&ToolId::new("knowledge.lookup"))
            .expect("existing tool should be disabled");

        assert_eq!(
            registry.lookup(&ToolId::new("knowledge.lookup")).status,
            ToolLookupStatus::Disabled
        );
        assert!(registry.enabled_tools().is_empty());
    }

    #[test]
    fn disabling_missing_tool_is_a_catalogue_error() {
        let mut registry = ToolRegistry::new();
        let err = registry
            .disable(&ToolId::new("missing"))
            .expect_err("missing tool should not be disabled");

        assert_eq!(err, ToolRegistryError::ToolNotFound(ToolId::new("missing")));
    }

    #[test]
    fn duplicate_tool_ids_are_rejected() {
        let err = ToolRegistry::from_tools(vec![
            declared_tool("same", ToolStatus::Available),
            declared_tool("same", ToolStatus::Disabled),
        ])
        .expect_err("duplicate ids must be rejected");

        assert_eq!(err, ToolRegistryError::DuplicateToolId(ToolId::new("same")));
    }

    #[test]
    fn lookup_reports_status_without_executing_or_approving() {
        let registry = ToolRegistry::from_tools(vec![
            declared_tool("available", ToolStatus::Available),
            declared_tool("disabled", ToolStatus::Disabled),
            declared_tool("deprecated", ToolStatus::Deprecated),
        ])
        .expect("registry should build");

        assert_eq!(
            registry.lookup(&ToolId::new("available")).status,
            ToolLookupStatus::Found
        );
        assert_eq!(
            registry.lookup(&ToolId::new("disabled")).status,
            ToolLookupStatus::Disabled
        );
        assert_eq!(
            registry.lookup(&ToolId::new("deprecated")).status,
            ToolLookupStatus::Deprecated
        );
        assert_eq!(
            registry.lookup(&ToolId::new("missing")).status,
            ToolLookupStatus::NotFound
        );
    }

    #[test]
    fn serialized_registry_does_not_contain_execution_hooks() {
        let registry = ToolRegistry::from_tools(vec![declared_tool(
            "knowledge.lookup",
            ToolStatus::Available,
        )])
        .expect("registry should build");

        let encoded = serde_json::to_string(&registry).expect("registry should serialize");
        assert!(!encoded.contains("execute"));
        assert!(!encoded.contains("runner"));
        assert!(!encoded.contains("shell"));
        assert!(!encoded.contains("scheduler"));
        assert!(!encoded.contains("mcp"));
        assert!(!encoded.contains("browser"));
    }

    #[test]
    fn read_only_tool_can_have_perception_role() {
        let tool = RegisteredTool::new(
            ToolDefinition {
                id: ToolId::new("read_file"),
                name: "Read file".to_owned(),
                description: "Read a file in workspace".to_owned(),
                required_permissions: vec![],
                default_risk_level: RiskLevel::Low,
                status: ToolStatus::Available,
            },
            ToolKind::FileOperation,
            schema(),
            Some(ToolCognitiveRole::Perception),
            Some(ToolRiskProfile::default()),
        );

        assert!(tool.cognitive_role.is_some());
        assert_eq!(tool.cognitive_role.unwrap(), ToolCognitiveRole::Perception);
    }

    #[test]
    fn dangerous_tool_can_exist_as_concept_but_remain_non_executable() {
        let write_tool = ToolCognitiveRole::Transformation;
        let exec_tool = ToolCognitiveRole::Execution;
        let comm_tool = ToolCognitiveRole::Communication;

        assert!(write_tool.is_non_executable());
        assert!(exec_tool.is_non_executable());
        assert!(comm_tool.is_non_executable());
    }

    #[test]
    fn safe_roles_are_marked_read_only_ready() {
        assert!(ToolCognitiveRole::Perception.is_safe_for_read_only());
        assert!(ToolCognitiveRole::Inspection.is_safe_for_read_only());
        assert!(ToolCognitiveRole::Validation.is_safe_for_read_only());
    }

    #[test]
    fn unsafe_roles_are_not_marked_read_only_ready() {
        assert!(!ToolCognitiveRole::Transformation.is_safe_for_read_only());
        assert!(!ToolCognitiveRole::Execution.is_safe_for_read_only());
        assert!(!ToolCognitiveRole::Communication.is_safe_for_read_only());
    }

    #[test]
    fn registry_does_not_produce_authorization() {
        let tool = RegisteredTool::new(
            ToolDefinition {
                id: ToolId::new("search_text"),
                name: "Search text".to_owned(),
                description: "Search text in workspace files".to_owned(),
                required_permissions: vec![],
                default_risk_level: RiskLevel::Low,
                status: ToolStatus::Available,
            },
            ToolKind::DataLookup,
            schema(),
            Some(ToolCognitiveRole::Inspection),
            None,
        );
        let mut registry = ToolRegistry::new();
        registry.register(tool).expect("tool should register");

        let lookup = registry.lookup(&ToolId::new("search_text"));

        assert_eq!(lookup.status, ToolLookupStatus::Found);
        assert!(!lookup.reason.contains("approved"));
        assert!(!lookup.reason.contains("executed"));
        assert!(lookup.reason.contains("does not approve"));
    }
}
