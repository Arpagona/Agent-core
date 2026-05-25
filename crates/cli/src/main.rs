use arpagona_agent_core::{
    ActionType, AgentId, AuditEvent, AuditEventId, AuditTraceSummary, CognitiveCycleResult,
    CorrectionTarget, Decision, DecisionId, DecisionStatus, DetectionSignal, DetectionSignalType,
    FailureClass, FailureInsight, FailureInsightId, InsightSeverity, MemoryWriteIntent,
    MemoryWriteKind, MemoryWriteProvenance, MemoryWriteTarget, ObjectiveDomain, Permission,
    ProposedAction, ProposedActionId, ProposedActionStatus, RiskLevel, SourceId, Task, TaskId,
    WorkspaceId,
};
use arpagona_decision_gate::{audit_event_for_decision, evaluate_proposed_action};
use arpagona_graph_memory::{
    demo_snapshot::{list_snapshots_in_directory, FailureInsightDemoSnapshot, EVIDENCE_ONLY_TOKEN},
    in_memory_graph_memory_store, AsyncGraphMemoryStore, GRAPH_MEMORY_SCHEMA,
};
use arpagona_tool_runtime::{ToolRuntime, ToolRuntimeConfig};
use chrono::Utc;
use clap::{Args, Parser, Subcommand, ValueEnum};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use std::error::Error;
use std::io::{self, Write};
use std::process::Command as ProcessCommand;

const DEFAULT_API_URL: &str = "http://127.0.0.1:3000";
const DEFAULT_WORKSPACE_ID: &str = "workspace-alpha";
const DEFAULT_AGENT_ID: &str = "agent-alpha";
const DEFAULT_TASK_ID: &str = "task-1";
const DEFAULT_TARGET: &str = "client@example.com";
const DEFAULT_RATIONALE: &str = "Préparer un brouillon sans l’envoyer";
const DEFAULT_PROVIDER: &str = "openai";
const DEFAULT_CHAT_PROVIDER: &str = "mock";
const DEFAULT_SNAPSHOT_DIR: &str = "target/demo-snapshots";

const ANSI_RESET: &str = "\x1b[0m";
const ANSI_BOLD: &str = "\x1b[1m";
const ANSI_DIM: &str = "\x1b[2m";

#[derive(Debug, Parser)]
#[command(name = "arpagona", version, about = "ARPAGONA alpha CLI")]
struct Cli {
    /// Base URL of the arpagona-api-server.
    #[arg(long, global = true, env = "ARPAGONA_API_URL", default_value = DEFAULT_API_URL)]
    api_url: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run the local API server through cargo.
    Serve,
    /// Start an interactive alpha terminal session.
    Chat(ChatArgs),
    /// Check API health.
    Health,
    /// Show a read-only local supervision overview.
    Status(StatusArgs),
    /// Show OpenAI auth status and setup guidance.
    Auth(AuthCommand),
    /// Manage tasks.
    Task(TaskCommand),
    /// Propose or evaluate actions.
    Action(ActionCommand),
    /// Ask an agent provider to propose actions.
    Agent(AgentCommand),
    /// Read audit events.
    Audit(AuditCommand),
    /// Inspect Failure-to-Insight vocabulary and readback conventions.
    Insight(InsightCommand),
    /// Inspect Graph Memory alpha status and readback conventions.
    Memory(MemoryCommand),
    /// Inspect and demo the alpha read-only cognitive tool runtime.
    Tool(ToolCommand),
    /// Run the General Cognitive Work Loop V0.
    Cognitive(CognitiveCommand),
}

#[derive(Debug, Args)]
pub struct CognitiveCommand {
    #[command(subcommand)]
    pub command: CognitiveSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum CognitiveSubcommand {
    /// Run the General Cognitive Work Loop with an objective.
    Run(CognitiveRunArgs),
}

#[derive(Debug, Args)]
pub struct CognitiveRunArgs {
    /// The objective text to work on.
    pub objective: String,
    /// Optional domain classification.
    #[arg(long)]
    pub domain: Option<String>,
    /// Optional context text (key:value pairs, one per line).
    #[arg(long)]
    pub context: Option<String>,
    /// Emit structured JSON instead of human-oriented text.
    #[arg(long)]
    pub json: bool,
    /// Run assessment bridge: convert ImprovementCandidates to FailureInsightCandidates.
    #[arg(long)]
    pub assess: bool,
}

#[derive(Debug, Args)]
struct ChatArgs {
    /// Agent proposer provider used by chat prompts.
    #[arg(long, default_value = DEFAULT_CHAT_PROVIDER)]
    provider: String,
    /// Workspace id.
    #[arg(long, default_value = DEFAULT_WORKSPACE_ID)]
    workspace_id: String,
    /// Task id linked to proposed actions.
    #[arg(long, default_value = DEFAULT_TASK_ID)]
    task_id: String,
    /// Permission granted when evaluating actions. Repeatable.
    #[arg(long = "permission", default_value = "simulate_email")]
    permissions: Vec<String>,
}

#[derive(Debug, Args)]
struct StatusArgs {
    /// Emit a structured JSON readback instead of human-oriented text.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct AuthCommand {
    #[command(subcommand)]
    command: AuthSubcommand,
}

#[derive(Debug, Subcommand)]
enum AuthSubcommand {
    /// Show whether OpenAI environment variables are configured.
    Status,
    /// Show safe setup instructions for OpenAI API key auth.
    Openai,
}

#[derive(Debug, Args)]
struct TaskCommand {
    #[command(subcommand)]
    command: TaskSubcommand,
}

#[derive(Debug, Subcommand)]
enum TaskSubcommand {
    /// Create a task.
    Create(CreateTaskArgs),
}

#[derive(Debug, Args)]
struct CreateTaskArgs {
    /// Task title.
    title: String,
    /// Optional task description.
    #[arg(long)]
    description: Option<String>,
    /// Workspace id.
    #[arg(long, default_value = DEFAULT_WORKSPACE_ID)]
    workspace_id: String,
}

#[derive(Debug, Args)]
struct ActionCommand {
    #[command(subcommand)]
    command: ActionSubcommand,
}

#[derive(Debug, Args)]
struct AgentCommand {
    #[command(subcommand)]
    command: AgentSubcommand,
}

#[derive(Debug, Subcommand)]
enum AgentSubcommand {
    /// Ask an LLM or mock provider to propose a pending action.
    Propose(ProposeAgentArgs),
}

#[derive(Debug, Args)]
struct ProposeAgentArgs {
    /// User prompt transformed into a ProposedAction through /agent/propose.
    prompt: String,
    /// Agent proposer provider.
    #[arg(long, default_value = DEFAULT_PROVIDER)]
    provider: String,
    /// Task id linked to the proposed action.
    #[arg(long, default_value = DEFAULT_TASK_ID)]
    task_id: String,
    /// Workspace id.
    #[arg(long, default_value = DEFAULT_WORKSPACE_ID)]
    workspace_id: String,
}

#[derive(Debug, Subcommand)]
enum ActionSubcommand {
    /// Propose an action.
    Propose(ProposeActionArgs),
    /// Evaluate a proposed action through the decision gate.
    Evaluate(EvaluateActionArgs),
}

#[derive(Debug, Args)]
struct ProposeActionArgs {
    /// Action type.
    #[arg(long = "type", default_value = "simulate_email")]
    action_type: String,
    /// Risk level.
    #[arg(long, value_enum, default_value_t = RiskArg::Medium)]
    risk: RiskArg,
    /// Permission required by the action. Repeatable.
    #[arg(long = "permission", default_value = "simulate_email")]
    permissions: Vec<String>,
    /// Target of the proposed action.
    #[arg(long, default_value = DEFAULT_TARGET)]
    target: String,
    /// Task id linked to the proposed action.
    #[arg(long, default_value = DEFAULT_TASK_ID)]
    task_id: String,
    /// Workspace id.
    #[arg(long, default_value = DEFAULT_WORKSPACE_ID)]
    workspace_id: String,
    /// Agent id.
    #[arg(long, default_value = DEFAULT_AGENT_ID)]
    proposed_by: String,
    /// Rationale recorded with the action.
    #[arg(long, default_value = DEFAULT_RATIONALE)]
    rationale: String,
    /// Memory proposal target entity type for governed memory-write action types.
    #[arg(long, default_value = "project")]
    memory_target_type: String,
    /// Memory proposal target entity id for governed memory-write action types.
    #[arg(long, default_value = "arpagona-agent-core")]
    memory_target_id: String,
    /// Memory proposal target attribute for governed memory-write action types.
    #[arg(long, default_value = "operational_note")]
    memory_target_attribute: String,
    /// Proposed memory value for governed memory-write action types. Accepts JSON or plain text.
    #[arg(long)]
    memory_value: Option<String>,
    /// Optional proposed Graph Memory fact id for governed memory-write action types.
    #[arg(long)]
    memory_fact_id: Option<String>,
    /// Optional related fact id for link/invalidation governed memory-write action types.
    #[arg(long)]
    memory_related_fact_id: Option<String>,
    /// Optional FailureInsight id for governed FailureInsight memory proposals.
    #[arg(long)]
    memory_failure_insight_id: Option<String>,
    /// Optional provenance source id for governed memory-write action types.
    #[arg(long)]
    memory_source_id: Option<String>,
    /// Provenance source label for governed memory-write action types.
    #[arg(long, default_value = "arpagona cli proposal")]
    memory_source_label: String,
    /// Provenance source kind for governed memory-write action types.
    #[arg(long, default_value = "local_operator_input")]
    memory_source_kind: String,
    /// Provenance evidence for governed memory-write action types.
    #[arg(
        long,
        default_value = "Memory write was proposed through the alpha CLI and still requires Decision Gate review."
    )]
    memory_evidence: String,
    /// Confidence for governed memory-write action types.
    #[arg(long, default_value_t = 0.5)]
    memory_confidence: f64,
    /// Future invalidation/supersession guidance for governed memory-write action types.
    #[arg(
        long,
        default_value = "Supersede or invalidate if the proposed operational note becomes stale."
    )]
    memory_invalidation_note: String,
}

#[derive(Clone, Debug, ValueEnum)]
enum RiskArg {
    Informational,
    Low,
    Medium,
    High,
    Critical,
}

impl RiskArg {
    fn as_api_value(&self) -> &'static str {
        match self {
            RiskArg::Informational => "informational",
            RiskArg::Low => "low",
            RiskArg::Medium => "medium",
            RiskArg::High => "high",
            RiskArg::Critical => "critical",
        }
    }
}

#[derive(Debug, Args)]
struct EvaluateActionArgs {
    /// Proposed action id to evaluate.
    proposed_action_id: String,
    /// Permission granted for the evaluation. Repeatable.
    #[arg(long = "permission", default_value = "simulate_email")]
    permissions: Vec<String>,
}

#[derive(Debug, Args)]
struct AuditCommand {
    #[command(subcommand)]
    command: AuditSubcommand,
}

#[derive(Debug, Args)]
struct InsightCommand {
    #[command(subcommand)]
    command: InsightSubcommand,
}

#[derive(Debug, Args)]
struct MemoryCommand {
    #[command(subcommand)]
    command: MemorySubcommand,
}

#[derive(Debug, Subcommand)]
enum InsightSubcommand {
    /// Show the read-only Failure-to-Insight schema and taxonomy.
    Schema(InsightSchemaArgs),
}

#[derive(Debug, Subcommand)]
enum MemorySubcommand {
    /// Show read-only Graph Memory alpha status.
    Status(MemoryStatusArgs),
    /// List read-only governed memory-write proposals from proposed actions.
    Proposals(MemoryProposalsArgs),
    /// Show one read-only governed memory-write proposal from proposed actions.
    Proposal(MemoryProposalArgs),
    /// Run local Graph Memory demos that exercise governed alpha paths.
    Demo(MemoryDemoCommand),
}

#[derive(Debug, Args)]
struct MemoryDemoCommand {
    #[command(subcommand)]
    command: MemoryDemoSubcommand,
}

#[derive(Debug, Subcommand)]
enum MemoryDemoSubcommand {
    /// Simulate a governed FailureInsight learning loop with in-memory persistence and readback.
    FailureInsight(MemoryDemoFailureInsightArgs),
    /// Read a FailureInsight demo snapshot from a JSON file for cross-invocation inspection.
    SnapshotRead(MemoryDemoSnapshotReadArgs),
    /// List all available demo snapshots in the snapshot directory.
    SnapshotList(MemoryDemoSnapshotListArgs),
}

#[derive(Debug, Args)]
struct InsightSchemaArgs {
    /// Emit structured JSON instead of human-oriented text.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct MemoryStatusArgs {
    /// Emit structured JSON instead of human-oriented text.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct MemoryProposalsArgs {
    /// Emit structured JSON instead of human-oriented text.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct MemoryProposalArgs {
    /// Proposed action id to inspect.
    proposal_id: String,
    /// Emit structured JSON instead of human-oriented text.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct MemoryDemoFailureInsightArgs {
    /// Inspect a specific FailureInsight id after the local governed demo persists it.
    #[arg(long = "inspect-id")]
    inspect_id: Option<String>,
    /// Optional path to write a demo snapshot JSON file for cross-invocation readback proof.
    /// When provided, the demo writes the readback state to disk after the in-memory demo succeeds.
    #[arg(long = "snapshot-path")]
    snapshot_path: Option<String>,
    /// Emit structured JSON instead of human-oriented text.
    #[arg(long)]
    json: bool,
    /// A custom failure description to use instead of the hardcoded default.
    /// When provided, the demo constructs a FailureInsight from this description.
    #[arg(long = "description")]
    description: Option<String>,
}

#[derive(Debug, Args)]
struct MemoryDemoSnapshotReadArgs {
    /// Path to the demo snapshot JSON file to read.
    snapshot_path: String,
    /// Emit structured JSON instead of human-oriented text.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct MemoryDemoSnapshotListArgs {
    /// Directory to scan for demo snapshot JSON files.
    #[arg(long = "snapshot-dir", default_value = DEFAULT_SNAPSHOT_DIR, env = "ARPAGONA_SNAPSHOT_DIR")]
    snapshot_dir: String,
    /// Emit structured JSON instead of human-oriented text.
    #[arg(long)]
    json: bool,
}

// ---------------------------------------------------------------------------
// Tool — alpha read-only tool runtime demo commands
// ---------------------------------------------------------------------------

#[derive(Debug, Args)]
struct ToolCommand {
    #[command(subcommand)]
    command: ToolSubcommand,
}

#[derive(Debug, Subcommand)]
enum ToolSubcommand {
    /// List all available tools in the alpha runtime.
    List(ToolListArgs),
    /// Show detailed information about a specific tool.
    Inspect(ToolInspectArgs),
    /// Run a read-only demo tool execution.
    Demo(ToolDemoCommand),
}

#[derive(Debug, Args)]
struct ToolListArgs {
    /// Emit structured JSON instead of human-oriented text.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct ToolInspectArgs {
    /// Name of the tool to inspect.
    tool_name: String,
    /// Emit structured JSON instead of human-oriented text.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct ToolDemoCommand {
    #[command(subcommand)]
    command: ToolDemoSubcommand,
}

#[derive(Debug, Subcommand)]
enum ToolDemoSubcommand {
    /// Demo the read_file tool.
    ReadFile(ToolDemoReadFileArgs),
    /// Demo the list_files tool.
    ListFiles(ToolDemoListFilesArgs),
    /// Demo the search_text tool.
    SearchText(ToolDemoSearchTextArgs),
    /// Run the full cognitive observation pipeline: tool execution → observation → assessment.
    Observe(ToolDemoObserveArgs),
}

#[derive(Debug, Args)]
struct ToolDemoReadFileArgs {
    /// Path to the file to read (relative to workspace).
    path: String,
    /// Emit structured JSON instead of human-oriented text.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct ToolDemoListFilesArgs {
    /// Path to the directory to list (relative to workspace, default: .).
    #[arg(default_value = ".")]
    path: String,
    /// Emit structured JSON instead of human-oriented text.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct ToolDemoSearchTextArgs {
    /// Text to search for.
    query: String,
    /// Path to search in (relative to workspace, default: .).
    #[arg(default_value = ".")]
    path: String,
    /// Emit structured JSON instead of human-oriented text.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct ToolDemoObserveArgs {
    /// Name of the tool to execute (read_file, list_files, search_text).
    tool_name: String,
    /// JSON arguments for the tool, e.g. '{"path": "Cargo.toml"}'.
    json_args: String,
    /// Emit structured JSON instead of human-oriented text.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Serialize)]
struct MemoryDemoFailureInsightReadback {
    signal: MemoryDemoSignalReadback,
    proposed_action_id: String,
    memory_write_kind: String,
    decision_id: String,
    decision_status: String,
    decision_reason: String,
    audit_event_id: String,
    persisted_failure_insight_id: Option<String>,
    inspected_failure_insight: Option<MemoryDemoFailureInsightInspectionReadback>,
    readback_found: bool,
    readback_audit_event_count: usize,
    readback_relation_count: usize,
    readback_warning: &'static str,
    functional_alpha_chain: &'static [&'static str],
    exact_local_command: &'static str,
    repeatable_demo_recipe: &'static [&'static str],
    next_safe_human_action: &'static str,
    warning: &'static str,
}

#[derive(Debug, Serialize)]
struct MemoryDemoSignalReadback {
    signal_type: &'static str,
    summary: String,
    correction_target: &'static str,
    provenance: &'static str,
}

#[derive(Debug, Serialize)]
struct MemoryDemoFailureInsightInspectionReadback {
    requested_failure_insight_id: String,
    found: bool,
    inspected_failure_insight_id: Option<String>,
    summary: Option<String>,
    correction_target: Option<String>,
    decision_id: Option<String>,
    audit_event_id: Option<String>,
    audit_event_count: usize,
    relation_count: usize,
    warning: &'static str,
}

#[derive(Debug, Subcommand)]
enum AuditSubcommand {
    /// List audit events.
    List,
    /// Show a read-only decision-scoped audit summary.
    /// Includes causal links, decision status, risk level and policies applied.
    DecisionSummary(DecisionSummaryArgs),
    /// Show a read-only task-scoped audit summary.
    /// Includes causal links, event boundaries and readback-only safety flags.
    TaskSummary(TaskSummaryArgs),
    /// Show a read-only workspace-scoped audit summary.
    /// Includes causal links, event boundaries and readback-only safety flags.
    WorkspaceSummary(WorkspaceSummaryArgs),
}

#[derive(Debug, Args)]
struct DecisionSummaryArgs {
    /// Decision id to summarize.
    decision_id: String,
    /// Emit a structured JSON readback instead of human-oriented text.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct TaskSummaryArgs {
    /// Task id to summarize.
    task_id: String,
    /// Emit a structured JSON readback instead of human-oriented text.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct WorkspaceSummaryArgs {
    /// Workspace id to summarize.
    workspace_id: String,
    /// Emit a structured JSON readback instead of human-oriented text.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Serialize)]
struct AuditDecisionReadback {
    summary: AuditTraceSummary,
    decision_status: Option<String>,
    explicit_reason: Option<String>,
    action_type: Option<String>,
    memory_write_kind: Option<String>,
    memory_target_type: Option<String>,
    memory_target_id: Option<String>,
    memory_target_attribute: Option<String>,
    memory_target_value: Option<Value>,
    memory_target_fact_id: Option<String>,
    memory_related_fact_id: Option<String>,
    memory_failure_insight_id: Option<String>,
    memory_provenance_source_id: Option<String>,
    memory_provenance_source_label: Option<String>,
    memory_provenance_source_kind: Option<String>,
    memory_provenance_evidence: Option<String>,
    memory_confidence: Option<f64>,
    memory_actor: Option<String>,
    memory_reason_for_remembering: Option<String>,
    memory_proposed_at: Option<String>,
    memory_invalidation_note: Option<String>,
    memory_decision_id: Option<String>,
    memory_audit_event_id: Option<String>,
    memory_persistence_readback_hint: Option<String>,
    memory_supersession_hint: Option<String>,
    risk_level: Option<String>,
    matched_policy_or_fallback_rule: Option<String>,
    required_permission: Option<String>,
    timestamp: Option<String>,
    suggested_next_action: Option<String>,
    block_reason_category: Option<String>,
    policies_applied: Vec<String>,
    warning: &'static str,
}

#[derive(Debug, Serialize)]
struct AuditTaskReadback {
    summary: AuditTraceSummary,
    warning: &'static str,
}

#[derive(Debug, Serialize)]
struct AuditWorkspaceReadback {
    summary: AuditTraceSummary,
    warning: &'static str,
}

#[derive(Debug, Serialize)]
struct StatusReadback {
    api_health: String,
    task_count: Option<usize>,
    proposed_action_count: Option<usize>,
    decision_count: Option<usize>,
    audit_event_count: Option<usize>,
    pending_decision_count: Option<usize>,
    needs_human_approval_count: Option<usize>,
    recent_audit_event_count: Option<usize>,
    last_audit_event_at: Option<String>,
    warning: &'static str,
}

#[derive(Debug, Serialize)]
struct InsightSchemaReadback {
    purpose: &'static str,
    minimum_fields: &'static [&'static str],
    failure_classes: &'static [&'static str],
    correction_targets: &'static [&'static str],
    statuses: &'static [&'static str],
    severities: &'static [&'static str],
    detection_signal_types: &'static [&'static str],
    alpha_limits: &'static [&'static str],
    warning: &'static str,
}

#[derive(Debug, Serialize)]
struct MemoryStatusReadback {
    graph_memory_support_compiled: bool,
    expected_backend: &'static str,
    configured_backend: Option<String>,
    surrealdb_adapter_available: bool,
    schema_available: bool,
    schema_bytes: usize,
    governed_persistence_helpers: &'static [&'static str],
    required_governance_controls: &'static [&'static str],
    alpha_limits: &'static [&'static str],
    not_implemented: &'static [&'static str],
    warning: &'static str,
}

#[derive(Debug, Serialize)]
struct MemoryProposalsReadback {
    proposals: Vec<MemoryProposalSummary>,
    warning: &'static str,
}

#[derive(Debug, Serialize)]
struct MemoryProposalDetailReadback {
    proposal: Option<MemoryProposalSummary>,
    warning: &'static str,
}

#[derive(Debug, Serialize)]
struct MemoryProposalSummary {
    id: String,
    workspace_id: String,
    task_id: Option<String>,
    proposed_by: String,
    action_type: String,
    status: String,
    risk_level: String,
    required_permissions: Vec<String>,
    target: Option<String>,
    rationale: String,
    created_at: String,
    memory_write_kind: Option<String>,
    target_type: Option<String>,
    target_id: Option<String>,
    target_attribute: Option<String>,
    target_value: Option<Value>,
    target_fact_id: Option<String>,
    related_fact_id: Option<String>,
    failure_insight_id: Option<String>,
    provenance_source_id: Option<String>,
    provenance_source_label: Option<String>,
    provenance_source_kind: Option<String>,
    provenance_evidence: Option<String>,
    confidence: Option<f64>,
    actor: Option<String>,
    reason_for_remembering: Option<String>,
    proposed_at: Option<String>,
    decision_id: Option<String>,
    audit_event_id: Option<String>,
    invalidation_note: Option<String>,
    persistence_readback_hint: String,
    supersession_hint: String,
    suggested_next_action: String,
}

const AUDIT_READBACK_WARNING: &str =
    "Readback only: this summary is not approval, authorization, orchestration, or execution state.";

const INSIGHT_READBACK_WARNING: &str =
    "Readback only: FailureInsight vocabulary informs learning and supervision; it is not approval, authorization, self-modification, or execution state.";

const MEMORY_READBACK_WARNING: &str =
    "Readback only: Graph Memory status is not approval, authorization, orchestration, memory mutation, or execution state.";

const MEMORY_DEMO_WARNING: &str =
    "Local demo only: this in-memory Graph Memory proof is simulated/internal and is not broad memory mutation, authorization, autonomy, or external execution state.";

const FAILURE_INSIGHT_DEMO_CHAIN: &[&str] = &[
    "safe operational signal",
    "create_failure_insight_memory ProposedAction",
    "Decision Gate approval",
    "decision audit event",
    "approved local Graph Memory persistence",
    "FailureInsight readback with decision/audit trace proof",
    "demo snapshot written for cross-invocation readback proof",
];

const FAILURE_INSIGHT_DEMO_COMMAND: &str =
    "cargo run -q --bin arpagona -- memory demo failure-insight --json";

const FAILURE_INSIGHT_DEMO_INSPECT_COMMAND: &str =
    "cargo run -q --bin arpagona -- memory demo failure-insight --json --inspect-id insight-demo-governed-learning-loop";

#[allow(dead_code)]
const FAILURE_INSIGHT_DEMO_SNAPSHOT_COMMAND: &str =
    "cargo run -q --bin arpagona -- memory demo failure-insight --json --snapshot-path target/demo-snapshot.json";

#[allow(dead_code)]
const FAILURE_INSIGHT_DEMO_SNAPSHOT_READ_COMMAND: &str =
    "cargo run -q --bin arpagona -- memory demo snapshot-read target/demo-snapshot.json";

const FAILURE_INSIGHT_DEMO_RECIPE: &[&str] = &[
    "run the exact_local_command from the repository root",
    "verify decision_status is approved before treating persistence as expected demo behavior",
    "verify readback_found is true and readback_audit_event_count is at least 1",
    "optionally rerun with --inspect-id insight-demo-governed-learning-loop to inspect the persisted artifact by id",
    "run with --snapshot-path target/demo-snapshot.json to persist the readback as a JSON file",
    "in a separate terminal session, run snapshot-read target/demo-snapshot.json to prove cross-invocation readback",
    "treat all output as local evidence only, not authorization or durable user memory",
];

const INSIGHT_MINIMUM_FIELDS: &[&str] = &[
    "id",
    "failure_class",
    "severity",
    "status",
    "correction_target",
    "summary",
    "root_cause",
    "impact",
    "corrective_action",
    "owner_layer",
    "detection_signal",
    "confidence",
    "workspace_id",
    "task_id",
    "proposed_action_id",
    "decision_id",
    "audit_event_id",
    "linked_pr",
    "linked_test",
    "linked_doc",
    "created_at",
];

const INSIGHT_FAILURE_CLASSES: &[&str] = &[
    "missing_context",
    "stale_context",
    "bad_action_type",
    "policy_gap",
    "blocked_without_explanation",
    "wrong_compute_choice",
    "tool_mismatch",
    "unsafe_drift",
    "insufficient_observability",
    "test_gap",
    "documentation_gap",
];

const INSIGHT_CORRECTION_TARGETS: &[&str] = &["code", "test", "policy", "memory", "docs", "none"];

const INSIGHT_STATUSES: &[&str] = &[
    "proposed",
    "accepted",
    "applied",
    "superseded",
    "rejected",
    "no_change",
];

const INSIGHT_SEVERITIES: &[&str] = &["informational", "low", "medium", "high", "critical"];

const INSIGHT_DETECTION_SIGNAL_TYPES: &[&str] = &[
    "human_correction",
    "audit_event",
    "test_failure",
    "review_finding",
    "runtime_observation",
    "policy_review",
    "documentation_review",
];

const INSIGHT_ALPHA_LIMITS: &[&str] = &[
    "no automatic creation from audit events",
    "no persistence or Graph Memory mutation",
    "no Decision Gate influence",
    "no provider routing influence",
    "no self-modification",
    "no execution or external side effects",
];

const MEMORY_ALPHA_LIMITS: &[&str] = &[
    "read-only CLI status/proposal readback only",
    "approved persistence helpers are local alpha Graph Memory adapter capabilities, not CLI mutation commands",
    "SurrealDB adapter remains experimental",
    "no migration runner exposed through the CLI",
    "no broad semantic search or embeddings pipeline",
    "no hidden context injection into LLM prompts",
    "no personal or sensitive memory writes",
];

const MEMORY_GOVERNED_PERSISTENCE_HELPERS: &[&str] = &[
    "persist_approved_create_memory_fact",
    "persist_approved_failure_insight_memory",
];

const MEMORY_REQUIRED_GOVERNANCE_CONTROLS: &[&str] = &[
    "ProposedAction memory-write intent",
    "approved Decision Gate result",
    "matching decision audit event",
    "source/provenance readback when provided",
    "post-persistence fact or FailureInsight inspection path",
];

const MEMORY_NOT_IMPLEMENTED: &[&str] = &[
    "CLI memory mutation command",
    "automatic FailureInsight creation from audit events",
    "Decision Gate influence from memory readback",
    "Mission Control Graph Memory UI",
    "scheduler or autonomous memory expansion",
];

#[derive(Debug, PartialEq, Eq)]
enum ChatLine {
    Empty,
    Help,
    Quit,
    Status,
    Audit,
    Tasks,
    Actions,
    Evaluate(String),
    Provider(String),
    UnknownCommand(String),
    Prompt(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TermColor {
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    Gray,
}

#[derive(Debug, Deserialize)]
struct HealthResponse {
    status: String,
}

#[derive(Debug, Deserialize)]
struct EvaluateResponse {
    decision: DecisionView,
    audit_event: AuditEvent,
}

#[derive(Debug, Deserialize)]
struct AgentProposeResponse {
    kind: String,
    proposed_action: Option<ProposedAction>,
    message: Option<String>,
    question: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DecisionView {
    #[allow(dead_code)]
    id: String,
    #[allow(dead_code)]
    proposed_action_id: String,
    status: String,
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("{}", style_error(&format!("Error: {error}")));
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let api_url = cli.api_url.trim_end_matches('/').to_owned();
    let client = Client::new();

    match cli.command {
        Command::Serve => serve()?,
        Command::Chat(args) => chat(&client, &api_url, args).await?,
        Command::Health => health(&client, &api_url).await?,
        Command::Status(args) => status(&client, &api_url, args).await?,
        Command::Auth(auth) => match auth.command {
            AuthSubcommand::Status => auth_status(),
            AuthSubcommand::Openai => auth_openai_instructions(),
        },
        Command::Task(task) => match task.command {
            TaskSubcommand::Create(args) => create_task(&client, &api_url, args).await?,
        },
        Command::Action(action) => match action.command {
            ActionSubcommand::Propose(args) => propose_action(&client, &api_url, args).await?,
            ActionSubcommand::Evaluate(args) => evaluate_action(&client, &api_url, args).await?,
        },
        Command::Agent(agent) => match agent.command {
            AgentSubcommand::Propose(args) => propose_agent_action(&client, &api_url, args).await?,
        },
        Command::Audit(audit) => match audit.command {
            AuditSubcommand::List => list_audit(&client, &api_url).await?,
            AuditSubcommand::DecisionSummary(args) => {
                audit_decision_summary(&client, &api_url, args).await?
            }
            AuditSubcommand::TaskSummary(args) => {
                audit_task_summary(&client, &api_url, args).await?
            }
            AuditSubcommand::WorkspaceSummary(args) => {
                audit_workspace_summary(&client, &api_url, args).await?
            }
        },
        Command::Insight(insight) => match insight.command {
            InsightSubcommand::Schema(args) => insight_schema(args)?,
        },
        Command::Memory(memory) => match memory.command {
            MemorySubcommand::Status(args) => memory_status(args)?,
            MemorySubcommand::Proposals(args) => memory_proposals(&client, &api_url, args).await?,
            MemorySubcommand::Proposal(args) => memory_proposal(&client, &api_url, args).await?,
            MemorySubcommand::Demo(demo) => match demo.command {
                MemoryDemoSubcommand::FailureInsight(args) => {
                    memory_demo_failure_insight(args).await?
                }
                MemoryDemoSubcommand::SnapshotRead(args) => memory_demo_snapshot_read(args)?,
                MemoryDemoSubcommand::SnapshotList(args) => memory_demo_snapshot_list(args)?,
            },
        },
        Command::Tool(tool) => match tool.command {
            ToolSubcommand::List(args) => tool_list(args)?,
            ToolSubcommand::Inspect(args) => tool_inspect(args)?,
            ToolSubcommand::Demo(demo) => match demo.command {
                ToolDemoSubcommand::ReadFile(args) => tool_demo_read_file(args)?,
                ToolDemoSubcommand::ListFiles(args) => tool_demo_list_files(args)?,
                ToolDemoSubcommand::SearchText(args) => tool_demo_search_text(args)?,
                ToolDemoSubcommand::Observe(args) => tool_demo_observe(args)?,
            },
        },
        Command::Cognitive(cognitive) => match cognitive.command {
            CognitiveSubcommand::Run(args) => cognitive_run(args)?,
        },
    }

    Ok(())
}

fn serve() -> Result<(), Box<dyn Error>> {
    let status = ProcessCommand::new("cargo")
        .args(["run", "-p", "arpagona-api-server"])
        .status()?;

    if !status.success() {
        return Err(format!("arpagona-api-server exited with {status}").into());
    }

    Ok(())
}

async fn chat(client: &Client, api_url: &str, args: ChatArgs) -> Result<(), Box<dyn Error>> {
    let health_response = client
        .get(format!("{api_url}/health"))
        .send()
        .await
        .map_err(|error| {
            format!(
                "API unavailable at {api_url}: {error}. Start it with: cargo run -p arpagona-api-server"
            )
        })?;
    let health_result: Result<HealthResponse, Box<dyn Error>> = get_json(health_response).await;
    if let Err(error) = health_result {
        return Err(format!(
            "API unavailable at {api_url}: {error}. Start it with: cargo run -p arpagona-api-server"
        )
        .into());
    }

    let mut provider = args.provider;
    let permissions = normalize_permissions(args.permissions);

    print_chat_banner(api_url, &provider, &args.workspace_id, &args.task_id);

    loop {
        print!("{} ", style_prompt("You >"));
        io::stdout().flush()?;

        let mut line = String::new();
        if io::stdin().read_line(&mut line)? == 0 {
            println!("\n{}", style_dim("Goodbye."));
            break;
        }

        match parse_chat_line(&line) {
            ChatLine::Empty => {}
            ChatLine::Help => print_chat_help(),
            ChatLine::Quit => {
                println!("{}", style_dim("Goodbye."));
                break;
            }
            ChatLine::Status => status(client, api_url, StatusArgs { json: false }).await?,
            ChatLine::Audit => list_audit(client, api_url).await?,
            ChatLine::Tasks => list_tasks(client, api_url).await?,
            ChatLine::Actions => list_actions(client, api_url).await?,
            ChatLine::Evaluate(action_id) => {
                let response =
                    evaluate_action_request(client, api_url, &action_id, &permissions).await?;
                print_decision(&response)?;
            }
            ChatLine::Provider(next_provider) => {
                if matches!(next_provider.as_str(), "mock" | "openai") {
                    provider = next_provider;
                    println!("{} {}", style_success("Provider:"), provider);
                } else {
                    println!(
                        "{}",
                        style_error("Unsupported provider. Use mock or openai.")
                    );
                }
            }
            ChatLine::UnknownCommand(command) => {
                println!(
                    "{}",
                    style_warning(&format!("Unknown command: {command}. Type /help."))
                );
            }
            ChatLine::Prompt(prompt) => {
                match propose_agent_request(
                    client,
                    api_url,
                    &args.workspace_id,
                    &args.task_id,
                    &provider,
                    &prompt,
                )
                .await
                {
                    Ok(response) => print_agent_turn(&response)?,
                    Err(error) => {
                        println!("{}", format_provider_error(&provider, &error.to_string()))
                    }
                }
            }
        }
    }

    Ok(())
}

fn auth_status() {
    println!("{}", rainbow_text("ARPAGONA OpenAI Auth Status"));
    match env::var("OPENAI_API_KEY") {
        Ok(key) if !key.trim().is_empty() => {
            println!(
                "{} {}",
                style_success("OPENAI_API_KEY:"),
                mask_openai_key(&key)
            );
        }
        _ => {
            println!("{} not configured", style_warning("OPENAI_API_KEY:"));
            println!("Run: {}", style_info("arpagona auth openai"));
        }
    }

    match env::var("OPENAI_MODEL") {
        Ok(model) if !model.trim().is_empty() => {
            println!("{} {}", style_success("OPENAI_MODEL:"), model);
        }
        _ => {
            println!(
                "{} not set, provider default will be used",
                style_dim("OPENAI_MODEL:")
            );
        }
    }
}

fn auth_openai_instructions() {
    println!("{}", rainbow_text("Configure OpenAI for ARPAGONA"));
    println!(
        "{}",
        style_dim("Alpha uses API key auth. Full OAuth is post-alpha / provider-dependent.")
    );
    println!();
    println!("1. Create a local env file:");
    println!("   mkdir -p ~/.config/arpagona");
    println!("   nano ~/.config/arpagona/env");
    println!();
    println!("2. Add your key locally. Do not commit it:");
    println!("   export OPENAI_API_KEY=\"sk-...\"");
    println!("   export OPENAI_MODEL=\"gpt-4.1-mini\"  # optional");
    println!();
    println!("3. Load it:");
    println!("   source ~/.config/arpagona/env");
    println!();
    println!("4. Test:");
    println!("   arpagona auth status");
    println!("   arpagona chat --provider openai");
}

fn print_chat_banner(api_url: &str, provider: &str, workspace_id: &str, task_id: &str) {
    println!();
    println!("{}", rainbow_text("                 /\\"));
    println!("{}", rainbow_text("                /  \\"));
    println!("{}", rainbow_text("               / /\\ \\"));
    println!("{}", rainbow_text("              / /__\\ \\"));
    println!("{}", rainbow_text("             /  ____  \\"));
    println!("{}", rainbow_text("            /__/    \\__\\"));
    println!("{}", rainbow_text("              .  /\\  ."));
    println!("{}", rainbow_text("           .  . /  \\ .  ."));
    println!();
    println!(
        "{}",
        style_brand("    ___    ____  ____  ___   ______ ____  _   ______ ")
    );
    println!(
        "{}",
        style_brand("   /   |  / __ \\/ __ \\/   | / ____// __ \\/ | / /   |")
    );
    println!(
        "{}",
        style_brand("  / /| | / /_/ / /_/ / /| |/ / __ / / / /  |/ / /| |")
    );
    println!(
        "{}",
        style_brand(" / ___ |/ _, _/ ____/ ___ / /_/ // /_/ / /|  / ___ |")
    );
    println!(
        "{}",
        style_brand("/_/  |_/_/ |_/_/   /_/  |_\\____/ \\____/_/ |_/_/  |_|")
    );
    println!("{}", style_dim("        Cognitive Runtime Alpha Terminal"));
    println!();
    println!(
        "{} {} | {} {} | {} {} | {} {}",
        style_dim("provider:"),
        style_info(provider),
        style_dim("api:"),
        api_url,
        style_dim("workspace:"),
        workspace_id,
        style_dim("task:"),
        task_id,
    );
    println!(
        "{}",
        style_dim("Type /help for commands. Nothing is executed directly.")
    );
    println!();
}

fn print_chat_help() {
    println!("{}", style_info("Commands:"));
    println!(
        "  {}                 Show this help",
        style_command("/help")
    );
    println!(
        "  {} | {}         Leave chat",
        style_command("/quit"),
        style_command("/exit")
    );
    println!(
        "  {}               Show read-only supervision status",
        style_command("/status")
    );
    println!("  {}                List tasks", style_command("/tasks"));
    println!(
        "  {}              List proposed actions",
        style_command("/actions")
    );
    println!(
        "  {}    Evaluate a proposed action",
        style_command("/evaluate action-1")
    );
    println!(
        "  {}                List audit events",
        style_command("/audit")
    );
    println!(
        "  {}        Use mock provider",
        style_command("/provider mock")
    );
    println!(
        "  {}      Use OpenAI provider",
        style_command("/provider openai")
    );
    println!(
        "\n{}",
        style_dim("Any other text is routed as DirectReply, ClarifyingQuestion, or pending ProposedAction.")
    );
}

async fn health(client: &Client, api_url: &str) -> Result<(), Box<dyn Error>> {
    let response: HealthResponse =
        get_json(client.get(format!("{api_url}/health")).send().await?).await?;
    println!("{} {}", style_success("ARPAGONA API:"), response.status);
    Ok(())
}

async fn status(client: &Client, api_url: &str, args: StatusArgs) -> Result<(), Box<dyn Error>> {
    let readback = status_readback(client, api_url).await;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&readback)?);
    } else {
        print_status_readback(&readback);
    }
    Ok(())
}

async fn status_readback(client: &Client, api_url: &str) -> StatusReadback {
    let api_health = match client.get(format!("{api_url}/health")).send().await {
        Ok(response) => match get_json::<HealthResponse>(response).await {
            Ok(health) => health.status,
            Err(error) => format!("unavailable ({error})"),
        },
        Err(error) => format!("unavailable ({error})"),
    };

    if api_health != "ok" {
        return StatusReadback {
            api_health,
            task_count: None,
            proposed_action_count: None,
            decision_count: None,
            audit_event_count: None,
            pending_decision_count: None,
            needs_human_approval_count: None,
            recent_audit_event_count: None,
            last_audit_event_at: None,
            warning: AUDIT_READBACK_WARNING,
        };
    }

    let tasks = fetch_optional::<Vec<Task>>(client, api_url, "/tasks").await;
    let actions = fetch_optional::<Vec<ProposedAction>>(client, api_url, "/proposed-actions").await;
    let decisions = fetch_optional::<Vec<Decision>>(client, api_url, "/decisions").await;
    let audit_events = fetch_optional::<Vec<AuditEvent>>(client, api_url, "/audit").await;

    let pending_decision_count = actions.as_ref().map(|actions| {
        actions
            .iter()
            .filter(|action| action.status == ProposedActionStatus::PendingDecision)
            .count()
    });
    let action_needs_human_count = actions.as_ref().map(|actions| {
        actions
            .iter()
            .filter(|action| action.status == ProposedActionStatus::NeedsHumanApproval)
            .count()
    });
    let decision_needs_human_count = decisions.as_ref().map(|decisions| {
        decisions
            .iter()
            .filter(|decision| decision.status == DecisionStatus::NeedsHumanApproval)
            .count()
    });
    let needs_human_approval_count = match (action_needs_human_count, decision_needs_human_count) {
        (Some(actions), Some(decisions)) => Some(actions.max(decisions)),
        (Some(actions), None) => Some(actions),
        (None, Some(decisions)) => Some(decisions),
        (None, None) => None,
    };
    let recent_audit_event_count = audit_events
        .as_ref()
        .map(|events| events.iter().rev().take(5).count());
    let last_audit_event_at = audit_events.as_ref().and_then(|events| {
        events
            .iter()
            .max_by_key(|event| event.created_at)
            .map(|event| event.created_at.to_rfc3339())
    });

    StatusReadback {
        api_health,
        task_count: tasks.as_ref().map(Vec::len),
        proposed_action_count: actions.as_ref().map(Vec::len),
        decision_count: decisions.as_ref().map(Vec::len),
        audit_event_count: audit_events.as_ref().map(Vec::len),
        pending_decision_count,
        needs_human_approval_count,
        recent_audit_event_count,
        last_audit_event_at,
        warning: AUDIT_READBACK_WARNING,
    }
}

async fn fetch_optional<T: for<'de> Deserialize<'de>>(
    client: &Client,
    api_url: &str,
    path: &str,
) -> Option<T> {
    match client.get(format!("{api_url}{path}")).send().await {
        Ok(response) => get_json(response).await.ok(),
        Err(_) => None,
    }
}

fn print_status_readback(readback: &StatusReadback) {
    print!("{}", format_status_readback(readback));
}

fn format_status_readback(readback: &StatusReadback) -> String {
    let mut output = String::new();
    push_readback_line(&mut output, &style_info("ARPAGONA status"));
    push_readback_field(&mut output, "api_health:", &readback.api_health);
    push_readback_field(
        &mut output,
        "task_count:",
        &format_optional_usize(readback.task_count),
    );
    push_readback_field(
        &mut output,
        "proposed_action_count:",
        &format_optional_usize(readback.proposed_action_count),
    );
    push_readback_field(
        &mut output,
        "decision_count:",
        &format_optional_usize(readback.decision_count),
    );
    push_readback_field(
        &mut output,
        "audit_event_count:",
        &format_optional_usize(readback.audit_event_count),
    );
    push_readback_field(
        &mut output,
        "pending_decision_count:",
        &format_optional_usize(readback.pending_decision_count),
    );
    push_readback_field(
        &mut output,
        "needs_human_approval_count:",
        &format_optional_usize(readback.needs_human_approval_count),
    );
    push_readback_field(
        &mut output,
        "recent_audit_event_count:",
        &format_optional_usize(readback.recent_audit_event_count),
    );
    push_readback_field(
        &mut output,
        "last_audit_event_at:",
        readback.last_audit_event_at.as_deref().unwrap_or("-"),
    );
    push_readback_line(&mut output, &style_dim(readback.warning));
    output
}

fn format_optional_usize(value: Option<usize>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unavailable".to_owned())
}

fn format_optional_json(value: &Option<Value>) -> String {
    value
        .as_ref()
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_owned())
}

fn memory_status(args: MemoryStatusArgs) -> Result<(), Box<dyn Error>> {
    let readback = memory_status_readback();
    if args.json {
        println!("{}", serde_json::to_string_pretty(&readback)?);
    } else {
        print!("{}", format_memory_status_readback(&readback));
    }
    Ok(())
}

fn memory_status_readback() -> MemoryStatusReadback {
    let configured_backend = env::var("ARPAGONA_GRAPH_MEMORY_BACKEND")
        .ok()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty());

    MemoryStatusReadback {
        graph_memory_support_compiled: true,
        expected_backend: "surrealdb",
        configured_backend,
        surrealdb_adapter_available: true,
        schema_available: !GRAPH_MEMORY_SCHEMA.trim().is_empty(),
        schema_bytes: GRAPH_MEMORY_SCHEMA.len(),
        governed_persistence_helpers: MEMORY_GOVERNED_PERSISTENCE_HELPERS,
        required_governance_controls: MEMORY_REQUIRED_GOVERNANCE_CONTROLS,
        alpha_limits: MEMORY_ALPHA_LIMITS,
        not_implemented: MEMORY_NOT_IMPLEMENTED,
        warning: MEMORY_READBACK_WARNING,
    }
}

fn format_memory_status_readback(readback: &MemoryStatusReadback) -> String {
    let mut output = String::new();
    push_readback_line(&mut output, &style_info("Graph Memory status"));
    push_readback_field(
        &mut output,
        "graph_memory_support_compiled:",
        &readback.graph_memory_support_compiled.to_string(),
    );
    push_readback_field(&mut output, "expected_backend:", readback.expected_backend);
    push_readback_field(
        &mut output,
        "configured_backend:",
        readback
            .configured_backend
            .as_deref()
            .unwrap_or("not configured"),
    );
    push_readback_field(
        &mut output,
        "surrealdb_adapter_available:",
        &readback.surrealdb_adapter_available.to_string(),
    );
    push_readback_field(
        &mut output,
        "schema_available:",
        &readback.schema_available.to_string(),
    );
    push_readback_field(
        &mut output,
        "schema_bytes:",
        &readback.schema_bytes.to_string(),
    );
    push_readback_field(
        &mut output,
        "governed_persistence_helpers:",
        &format_static_list(readback.governed_persistence_helpers),
    );
    push_readback_field(
        &mut output,
        "required_governance_controls:",
        &format_static_list(readback.required_governance_controls),
    );
    push_readback_field(
        &mut output,
        "alpha_limits:",
        &format_static_list(readback.alpha_limits),
    );
    push_readback_field(
        &mut output,
        "not_implemented:",
        &format_static_list(readback.not_implemented),
    );
    push_readback_line(&mut output, &style_dim(readback.warning));
    output
}

async fn memory_proposals(
    client: &Client,
    api_url: &str,
    args: MemoryProposalsArgs,
) -> Result<(), Box<dyn Error>> {
    let actions: Vec<ProposedAction> = get_json(
        client
            .get(format!("{api_url}/proposed-actions"))
            .send()
            .await?,
    )
    .await?;
    let readback = memory_proposals_readback_from_actions(actions);

    if args.json {
        println!("{}", serde_json::to_string_pretty(&readback)?);
    } else {
        print!("{}", format_memory_proposals_readback(&readback));
    }
    Ok(())
}

async fn memory_proposal(
    client: &Client,
    api_url: &str,
    args: MemoryProposalArgs,
) -> Result<(), Box<dyn Error>> {
    let actions: Vec<ProposedAction> = get_json(
        client
            .get(format!("{api_url}/proposed-actions"))
            .send()
            .await?,
    )
    .await?;
    let proposal = memory_proposals_readback_from_actions(actions)
        .proposals
        .into_iter()
        .find(|proposal| proposal.id == args.proposal_id);
    let readback = MemoryProposalDetailReadback {
        proposal,
        warning: MEMORY_READBACK_WARNING,
    };

    if args.json {
        println!("{}", serde_json::to_string_pretty(&readback)?);
    } else {
        print!("{}", format_memory_proposal_detail_readback(&readback));
    }
    Ok(())
}

async fn memory_demo_failure_insight(
    args: MemoryDemoFailureInsightArgs,
) -> Result<(), Box<dyn Error>> {
    let readback =
        memory_demo_failure_insight_readback(args.inspect_id.clone(), args.description.clone())
            .await?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&readback)?);
    } else {
        print!("{}", format_memory_demo_failure_insight_readback(&readback));
    }
    // If --snapshot-path is provided, write the readback JSON to disk for cross-invocation proof.
    if let Some(snapshot_path) = args.snapshot_path {
        let json_value = serde_json::to_value(&readback)?;
        let chain: Vec<String> = readback
            .functional_alpha_chain
            .iter()
            .map(|s| s.to_string())
            .collect();
        let snapshot = FailureInsightDemoSnapshot::new(json_value, chain);
        snapshot
            .write_to_file(std::path::Path::new(&snapshot_path))
            .map_err(|e| format!("failed to write snapshot to {snapshot_path}: {e}"))?;
        if args.json {
            // Merge the snapshot path info into the existing JSON output for cleaner piping.
            let mut output = serde_json::to_value(&readback)?;
            if let serde_json::Value::Object(ref mut map) = output {
                map.insert("snapshot_written".to_owned(), serde_json::json!(true));
                map.insert("snapshot_path".to_owned(), serde_json::json!(snapshot_path));
                map.insert(
                    "evidence_only_token".to_owned(),
                    serde_json::json!(EVIDENCE_ONLY_TOKEN),
                );
            }
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            eprintln!(
                "Snapshot written to {} (evidence_only_token: {})",
                snapshot_path, EVIDENCE_ONLY_TOKEN
            );
        }
    }
    Ok(())
}

async fn memory_demo_failure_insight_readback(
    inspect_id: Option<String>,
    description: Option<String>,
) -> Result<MemoryDemoFailureInsightReadback, Box<dyn Error>> {
    let workspace_id = WorkspaceId::new("workspace-demo-failure-insight");
    let task_id = TaskId::new("task-demo-failure-insight");
    let agent_id = AgentId::new("agent-demo-focus-loop");
    let proposed_action_id = ProposedActionId::new("action-demo-create-failure-insight-memory");
    let failure_insight_id = FailureInsightId::new("insight-demo-governed-learning-loop");
    let created_at = Utc::now();

    let (custom_signal_summary, custom_failure_summary, custom_impact, custom_root_cause, custom_action) = match &description {
        Some(desc) => (
            format!("safe bounded FailureInsight learning signal from operator: {desc}"),
            desc.clone(),
            format!("Operator reported: {desc}"),
            format!("Operator provided description: {desc}"),
            format!("Exercise the governed loop with operator-supplied description: {desc}"),
        ),
        None => (
            "safe bounded FailureInsight learning signal".to_owned(),
            "Governed FailureInsight learning loop needs repeatable local readback proof.".to_owned(),
            "Without an end-to-end local demo, operators must infer whether proposal, decision, audit, persistence, and readback remain connected.".to_owned(),
            "Human supervisors have weaker evidence that approved FailureInsights can be inspected without widening mutation authority.".to_owned(),
            "Exercise the loop with in-memory Graph Memory and explicit non-authorizing readback output.".to_owned(),
        ),
    };

    let signal = DetectionSignal::new(
        DetectionSignalType::RuntimeObservation,
        &custom_signal_summary,
    );

    let base_insight = FailureInsight::new(
        failure_insight_id.clone(),
        FailureClass::InsufficientObservability,
        InsightSeverity::Low,
        CorrectionTarget::Memory,
        &custom_failure_summary,
        &custom_impact,
        &custom_root_cause,
        &custom_action,
        "Graph Memory / Decision Gate alpha demo",
        signal.clone(),
        0.92,
        created_at,
    );

    let proposal_intent =
        failure_insight_demo_intent(&agent_id, &base_insight, None, None, created_at)?;
    let action = ProposedAction {
        id: proposed_action_id.clone(),
        workspace_id: workspace_id.clone(),
        task_id: Some(task_id.clone()),
        proposed_by: agent_id.clone(),
        action_type: ActionType::CreateFailureInsightMemory,
        target: Some("memory:failure_insight:insight-demo-governed-learning-loop".to_owned()),
        payload: json!({ "memory_write_intent": proposal_intent }),
        risk_level: RiskLevel::Low,
        required_permissions: vec![Permission::WriteMemory],
        rationale: "Demonstrate a governed create_failure_insight_memory proposal without durable or external memory mutation.".to_owned(),
        context_refs: vec![],
        status: ProposedActionStatus::PendingDecision,
        created_at,
    };

    let decision = evaluate_proposed_action(&action, &[], &[Permission::WriteMemory]);
    if decision.status != DecisionStatus::Approved {
        return Err(format!(
            "demo expected Decision Gate approval for low-risk local memory demo, got {:?}: {}",
            decision.status, decision.reason
        )
        .into());
    }
    let audit_event = audit_event_for_decision(&action, &decision);
    let linked_insight = base_insight.with_trace_links(
        Some(workspace_id.clone()),
        Some(task_id.clone()),
        Some(proposed_action_id.clone()),
        Some(decision.id.clone()),
        Some(audit_event.id.clone()),
    );
    let approved_intent = failure_insight_demo_intent(
        &agent_id,
        &linked_insight,
        Some(decision.id.clone()),
        Some(audit_event.id.clone()),
        created_at,
    )?;

    let store = in_memory_graph_memory_store("arpagona_demo", "failure_insight_loop").await?;
    let persisted = store
        .persist_approved_failure_insight_memory(
            approved_intent,
            decision.clone(),
            audit_event.clone(),
        )
        .await?;
    let readback = store
        .failure_insight_memory_readback(persisted.id.clone())
        .await?;
    let inspected_failure_insight = match inspect_id {
        Some(id) => {
            let requested_readback = store
                .failure_insight_memory_readback(FailureInsightId::new(id.clone()))
                .await?;
            Some(memory_demo_failure_insight_inspection_from_readback(
                id,
                requested_readback,
            ))
        }
        None => None,
    };

    Ok(MemoryDemoFailureInsightReadback {
        signal: MemoryDemoSignalReadback {
            signal_type: "runtime_observation",
            summary: custom_signal_summary,
            correction_target: "memory",
            provenance: "local in-memory demo",
        },
        proposed_action_id: proposed_action_id.to_string(),
        memory_write_kind: "create_failure_insight_memory".to_owned(),
        decision_id: decision.id.to_string(),
        decision_status: to_api_string(&decision.status)?,
        decision_reason: decision.reason,
        audit_event_id: audit_event.id.to_string(),
        persisted_failure_insight_id: readback.insight.map(|insight| insight.id.to_string()),
        inspected_failure_insight,
        readback_found: persisted.id == failure_insight_id,
        readback_audit_event_count: readback.decision_audit_events.len(),
        readback_relation_count: readback.insight_relations.len(),
        readback_warning: readback.warning,
        functional_alpha_chain: FAILURE_INSIGHT_DEMO_CHAIN,
        exact_local_command: FAILURE_INSIGHT_DEMO_COMMAND,
        repeatable_demo_recipe: FAILURE_INSIGHT_DEMO_RECIPE,
        next_safe_human_action: "Inspect the JSON/text readback and treat it as evidence only; do not treat it as approval, authorization, execution, or durable user memory.",
        warning: MEMORY_DEMO_WARNING,
    })
}

fn memory_demo_failure_insight_inspection_from_readback(
    requested_id: String,
    readback: arpagona_graph_memory::FailureInsightMemoryReadback,
) -> MemoryDemoFailureInsightInspectionReadback {
    let insight = readback.insight;
    MemoryDemoFailureInsightInspectionReadback {
        requested_failure_insight_id: requested_id,
        found: insight.is_some(),
        inspected_failure_insight_id: insight.as_ref().map(|insight| insight.id.to_string()),
        summary: insight.as_ref().map(|insight| insight.summary.clone()),
        correction_target: insight
            .as_ref()
            .and_then(|insight| to_api_string(&insight.correction_target).ok()),
        decision_id: insight
            .as_ref()
            .and_then(|insight| insight.decision_id.as_ref().map(ToString::to_string)),
        audit_event_id: insight
            .as_ref()
            .and_then(|insight| insight.audit_event_id.as_ref().map(ToString::to_string)),
        audit_event_count: readback.decision_audit_events.len(),
        relation_count: readback.insight_relations.len(),
        warning: readback.warning,
    }
}

fn memory_demo_snapshot_read(args: MemoryDemoSnapshotReadArgs) -> Result<(), Box<dyn Error>> {
    let snapshot =
        FailureInsightDemoSnapshot::read_from_file(std::path::Path::new(&args.snapshot_path))
            .map_err(|e| format!("failed to read snapshot: {e}"))?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&snapshot)?);
    } else {
        let mut output = String::new();
        output.push_str(&format!(
            "📸 Demo Snapshot Readback (file: {})\n\n",
            args.snapshot_path
        ));
        output.push_str(&format!(
            "Evidence token: {}\n\n",
            snapshot.evidence_only_token
        ));
        if !snapshot.functional_alpha_chain.is_empty() {
            output.push_str("Functional alpha chain achieved:\n");
            for step in &snapshot.functional_alpha_chain {
                output.push_str(&format!("  → {step}\n"));
            }
            output.push('\n');
        }
        output.push_str("Readback JSON content (truncated, use --json for full):\n\n");
        let json_str = serde_json::to_string_pretty(&snapshot.readback_json)?;
        let truncated = if json_str.len() > 500 {
            format!("{} ... [truncated]", &json_str[..500])
        } else {
            json_str
        };
        output.push_str(&truncated);
        output.push('\n');
        print!("{output}");
    }
    Ok(())
}

fn memory_demo_snapshot_list(args: MemoryDemoSnapshotListArgs) -> Result<(), Box<dyn Error>> {
    let dir = std::path::Path::new(&args.snapshot_dir);
    let list = list_snapshots_in_directory(dir);

    if args.json {
        println!("{}", serde_json::to_string_pretty(&list)?);
    } else {
        let mut output = String::new();
        output.push_str(&format!("📁 Demo Snapshots in: {}\n\n", args.snapshot_dir));
        if list.is_empty() {
            output.push_str("No demo snapshots found.\n");
            output.push_str(&format!(
                "Run `{}` in this project first.\n",
                "cargo run -q --bin arpagona -- memory demo failure-insight --snapshot-path target/demo-snapshots/demo.snapshot.json"
            ));
        } else {
            output.push_str(&format!("Found {} snapshot(s):\n\n", list.len()));
            for (i, (_path, listing)) in list.iter().enumerate() {
                output.push_str(&format!(
                    "{}. \u{1b}[1m{}\u{1b}[0m\n",
                    i + 1,
                    listing.file_name
                ));
                if let Some(preview) = &listing.description_preview {
                    output.push_str(&format!("   Description: {preview}\n"));
                }
                output.push_str(&format!(
                    "   Alpha chain steps: {}\n",
                    listing.chain_step_count
                ));
                if let Some(preview) = &listing.content_preview {
                    output.push_str(&format!("   Content: {preview}\n"));
                }
                output.push('\n');
            }
            output.push_str(&format!(
                "Use `{}` to inspect a specific snapshot.\n",
                "cargo run -q --bin arpagona -- memory demo snapshot-read <filename> --json"
            ));
        }
        print!("{output}");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tool runtime command implementations — alpha, read-only, local-only
// ---------------------------------------------------------------------------

const DEMO_TOOL_WARNING: &str = "⚠️  Alpha demo tool runtime — local read-only execution only. No authorization, no governance bypass. ⚠️";

fn tool_list(args: ToolListArgs) -> Result<(), Box<dyn Error>> {
    let tools = vec![
        (
            "read_file",
            "Read a file within the workspace",
            "Perception / Inspection",
        ),
        (
            "list_files",
            "List files and directories in a workspace path",
            "Perception",
        ),
        (
            "search_text",
            "Search for text patterns in workspace files",
            "Inspection",
        ),
    ];

    if args.json {
        let output: Vec<serde_json::Value> = tools
            .iter()
            .map(|(name, desc, role)| {
                serde_json::json!({
                    "name": name,
                    "description": desc,
                    "cognitive_role": role,
                    "read_only": true,
                    "alpha": true,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("{DEMO_TOOL_WARNING}");
        println!();
        println!("Available tools (alpha read-only runtime):");
        println!();
        for (name, desc, role) in &tools {
            println!("  {name}");
            println!("    Description:  {desc}");
            println!("    Cognitive role: {role}");
            println!("    Read-only:    yes");
            println!("    Alpha:        yes");
            println!();
        }
        println!("Use 'arpagona tool inspect <name>' for details.");
        println!("Use 'arpagona tool demo <name>' to execute.");
    }
    Ok(())
}

fn tool_inspect(args: ToolInspectArgs) -> Result<(), Box<dyn Error>> {
    let _runtime = ToolRuntime::new(ToolRuntimeConfig::new("."));

    match args.tool_name.as_str() {
        "read_file" => {
            let info = serde_json::json!({
                "name": "read_file",
                "description": "Read a file within the workspace",
                "cognitive_role": ["Perception", "Inspection"],
                "read_only": true,
                "alpha": true,
                "arguments": {
                    "path": {
                        "type": "string",
                        "description": "Path to the file (relative to workspace)",
                        "required": true
                    }
                },
                "security": {
                    "absolute_paths": "blocked",
                    "parent_traversal": "blocked",
                    "sensitive_files": "blocked (.env, .ssh, id_rsa, id_ed25519)",
                    "max_file_size": "1 MiB"
                },
                "workspace": "current directory"
            });
            if args.json {
                println!("{}", serde_json::to_string_pretty(&info)?);
            } else {
                println!("{DEMO_TOOL_WARNING}");
                println!();
                println!("Tool: read_file");
                println!("  Description:   Read a file within the workspace");
                println!("  Cognitive role: Perception, Inspection");
                println!("  Read-only:     yes");
                println!("  Workspace:     current directory");
                println!();
                println!("Arguments:");
                println!("  path (required): Path to the file (relative to workspace)");
                println!();
                println!("Security:");
                println!("  Absolute paths:           blocked");
                println!("  Parent traversal (..):    blocked");
                println!("  Sensitive files:          blocked (.env, .ssh, id_rsa, id_ed25519)");
                println!("  Max file size:            1 MiB");
            }
        }
        "list_files" => {
            let info = serde_json::json!({
                "name": "list_files",
                "description": "List files and directories in a workspace path",
                "cognitive_role": ["Perception"],
                "read_only": true,
                "alpha": true,
                "arguments": {
                    "path": {
                        "type": "string",
                        "description": "Directory path (relative to workspace, default: .)",
                        "required": false,
                        "default": "."
                    }
                },
                "security": {
                    "ignored_dirs": ".git, target, node_modules, .env, .ssh",
                    "max_results": 200,
                    "max_depth": 5
                },
                "workspace": "current directory"
            });
            if args.json {
                println!("{}", serde_json::to_string_pretty(&info)?);
            } else {
                println!("{DEMO_TOOL_WARNING}");
                println!();
                println!("Tool: list_files");
                println!("  Description:   List files and directories in a workspace path");
                println!("  Cognitive role: Perception");
                println!("  Read-only:     yes");
                println!("  Workspace:     current directory");
                println!();
                println!("Arguments:");
                println!("  path (optional): Directory path (default: .)");
                println!();
                println!("Security:");
                println!("  Ignored directories: .git, target, node_modules, .env, .ssh");
                println!("  Max results:         200");
                println!("  Max directory depth: 5");
            }
        }
        "search_text" => {
            let info = serde_json::json!({
                "name": "search_text",
                "description": "Search for text patterns in workspace files",
                "cognitive_role": ["Inspection"],
                "read_only": true,
                "alpha": true,
                "arguments": {
                    "query": {
                        "type": "string",
                        "description": "Text to search for",
                        "required": true
                    },
                    "path": {
                        "type": "string",
                        "description": "Directory to search (relative to workspace, default: .)",
                        "required": false,
                        "default": "."
                    }
                },
                "security": {
                    "ignored_dirs": ".git, target, node_modules, .env, .ssh",
                    "max_results": 100,
                    "max_file_size": "500 KiB"
                },
                "workspace": "current directory"
            });
            if args.json {
                println!("{}", serde_json::to_string_pretty(&info)?);
            } else {
                println!("{DEMO_TOOL_WARNING}");
                println!();
                println!("Tool: search_text");
                println!("  Description:   Search for text patterns in workspace files");
                println!("  Cognitive role: Inspection");
                println!("  Read-only:     yes");
                println!("  Workspace:     current directory");
                println!();
                println!("Arguments:");
                println!("  query (required): Text to search for");
                println!("  path (optional):  Directory to search (default: .)");
                println!();
                println!("Security:");
                println!("  Ignored directories: .git, target, node_modules, .env, .ssh");
                println!("  Max results:         100");
                println!("  Max file size:       500 KiB");
            }
        }
        other => {
            return Err(format!(
                "Unknown tool: {other}. Use 'arpagona tool list' to see available tools."
            )
            .into());
        }
    }
    Ok(())
}

fn tool_demo_read_file(args: ToolDemoReadFileArgs) -> Result<(), Box<dyn Error>> {
    let config = ToolRuntimeConfig::new(".");
    let runtime = ToolRuntime::new(config);

    let result = runtime.execute("read_file", &serde_json::json!({"path": args.path}));

    if args.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("{DEMO_TOOL_WARNING}");
        println!();
        println!("🧰 Tool demo: read_file");
        println!("   Path: {}", args.path);
        println!();
        match result.status {
            arpagona_agent_core::ToolExecutionStatus::Success => {
                println!("✅ Status: Success");
                println!("   {}", result.output_summary);
                println!();
                let payload = &result.observation.payload;
                if let Some(preview) = payload["content_preview"].as_str() {
                    println!("Preview (first 500 chars):");
                    println!("{}", preview);
                }
            }
            arpagona_agent_core::ToolExecutionStatus::Blocked => {
                println!("🔒 Status: Blocked (security)");
                if let Some(error) = &result.error {
                    println!("   Reason: {}", error.message);
                }
            }
            arpagona_agent_core::ToolExecutionStatus::Failed => {
                println!("❌ Status: Failed");
                if let Some(error) = &result.error {
                    println!("   Error: {}", error.message);
                }
            }
            arpagona_agent_core::ToolExecutionStatus::Warning => {
                println!("⚠️  Status: Warning");
                println!("   {}", result.output_summary);
            }
            arpagona_agent_core::ToolExecutionStatus::Skipped => {
                println!("⏭️  Status: Skipped");
            }
        }
        println!();
        println!("Full result available with --json");
    }
    Ok(())
}

fn tool_demo_list_files(args: ToolDemoListFilesArgs) -> Result<(), Box<dyn Error>> {
    let config = ToolRuntimeConfig::new(".");
    let runtime = ToolRuntime::new(config);

    let result = runtime.execute("list_files", &serde_json::json!({"path": args.path}));

    if args.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("{DEMO_TOOL_WARNING}");
        println!();
        println!("🧰 Tool demo: list_files");
        println!("   Path: {}", args.path);
        println!();
        match result.status {
            arpagona_agent_core::ToolExecutionStatus::Success
            | arpagona_agent_core::ToolExecutionStatus::Warning => {
                println!(
                    "✅ Status: {}",
                    if result.status == arpagona_agent_core::ToolExecutionStatus::Success {
                        "Success"
                    } else {
                        "Warning (truncated)"
                    }
                );
                println!("   {}", result.output_summary);
                println!();
                let payload = &result.observation.payload;
                if let Some(entries) = payload["entries"].as_array() {
                    for entry in entries {
                        let name = entry["name"].as_str().unwrap_or("?");
                        let is_dir = entry["is_directory"].as_bool().unwrap_or(false);
                        let icon = if is_dir { "📁" } else { "📄" };
                        println!("   {icon} {name}");
                    }
                }
            }
            arpagona_agent_core::ToolExecutionStatus::Failed => {
                println!("❌ Status: Failed");
                if let Some(error) = &result.error {
                    println!("   Error: {}", error.message);
                }
            }
            _ => {}
        }
        println!();
        println!("Full result available with --json");
    }
    Ok(())
}

fn tool_demo_search_text(args: ToolDemoSearchTextArgs) -> Result<(), Box<dyn Error>> {
    let config = ToolRuntimeConfig::new(".");
    let runtime = ToolRuntime::new(config);

    let result = runtime.execute(
        "search_text",
        &serde_json::json!({"query": args.query, "path": args.path}),
    );

    if args.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("{DEMO_TOOL_WARNING}");
        println!();
        println!("🧰 Tool demo: search_text");
        println!("   Query: {}", args.query);
        println!("   Path: {}", args.path);
        println!();
        match result.status {
            arpagona_agent_core::ToolExecutionStatus::Success
            | arpagona_agent_core::ToolExecutionStatus::Warning => {
                let status_label =
                    if result.status == arpagona_agent_core::ToolExecutionStatus::Success {
                        "Success"
                    } else {
                        "Warning (truncated)"
                    };
                println!("✅ Status: {status_label}");
                println!("   {}", result.output_summary);
                println!();
                let payload = &result.observation.payload;
                if let Some(matches) = payload["matches"].as_array() {
                    for m in matches {
                        let file = m["file"].as_str().unwrap_or("?");
                        let line = m["line"].as_u64().unwrap_or(0);
                        let snippet = m["snippet"].as_str().unwrap_or("");
                        println!("   📄 {file}:{line}");
                        println!("      {snippet}");
                        println!();
                    }
                }
            }
            arpagona_agent_core::ToolExecutionStatus::Failed => {
                println!("❌ Status: Failed");
                if let Some(error) = &result.error {
                    println!("   Error: {}", error.message);
                }
            }
            _ => {}
        }
        println!("Full result available with --json");
    }
    Ok(())
}

const OBSERVE_TOOL_WARNING: &str =
    "⚠️  Cognitive observation pipeline demo — read-only, no authorization, no governance bypass. ⚠️";

fn tool_demo_observe(args: ToolDemoObserveArgs) -> Result<(), Box<dyn Error>> {
    let config = ToolRuntimeConfig::new(".");
    let runtime = ToolRuntime::new(config);

    let arguments: Value = serde_json::from_str(&args.json_args)
        .map_err(|e| format!("Invalid JSON arguments: {e}"))?;

    let result = runtime.execute(&args.tool_name, &arguments);

    let obs = arpagona_agent_core::CognitiveObservation::from_tool_execution(&result);
    let assessment = arpagona_agent_core::assess_observation(&obs);

    if args.json {
        let output = serde_json::json!({
            "pipeline": "ToolExecutionResult → CognitiveObservation → ObservationAssessment",
            "warning": OBSERVE_TOOL_WARNING,
            "tool_execution_result": &result,
            "cognitive_observation": &obs,
            "assessment": &assessment,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("{OBSERVE_TOOL_WARNING}");
        println!();
        println!("🧩 Cognitive Observation Pipeline Demo");
        println!("   Pipeline: ToolExecutionResult → CognitiveObservation → ObservationAssessment");
        println!();

        // Tool execution result
        println!("━━━ Step 1: Tool Execution ━━━");
        println!("   Tool:      {}", result.tool_name);
        println!("   Status:    {:?}", result.status);
        println!("   Summary:   {}", result.output_summary);

        // Cognitive observation
        println!();
        println!("━━━ Step 2: Cognitive Observation ━━━");
        println!("   Kind:      {:?}", obs.kind);
        println!("   Status:    {:?}", obs.status);
        println!("   Usefulness: {:?}", obs.usefulness);
        println!("   Risk:       {:?}", obs.risk);
        println!("   Count:      {}", obs.count);
        println!("   Truncated:  {}", obs.truncated);

        if obs.failure_insight_candidate {
            let signal = if obs.is_positive_signal() {
                "🟢 positive"
            } else {
                "🟡 candidate"
            };
            println!(
                "   FailureInsight: {signal} ({:?})",
                obs.candidate_kind.unwrap()
            );
            println!("   Reason:      {}", obs.candidate_reason);
        } else {
            println!("   FailureInsight: ❌ none (clean observation)");
        }

        // Assessment
        println!();
        println!("━━━ Step 3: Assessment ━━━");
        println!("   Useful:     {}", assessment.is_useful);
        println!("   Complete:   {}", assessment.is_complete);

        if let Some(candidate) = &assessment.candidate {
            let signal_label = if candidate.is_positive_signal {
                "🟢 positive signal (safety boundary working)"
            } else {
                "🟡 candidate for FailureInsight creation"
            };
            println!("   Candidate:  {signal_label}");
            println!("   Kind:       {:?}", candidate.kind);
            println!("   Summary:    {}", candidate.summary);
        } else {
            println!("   Candidate:  ❌ none");
        }

        println!();
        println!("   Summary:    {}", assessment.assessment_summary);
        println!("   Next step:  {}", assessment.suggested_next_step);
        println!();
        println!("Full pipeline output available with --json");
    }
    Ok(())
}

fn failure_insight_demo_intent(
    agent_id: &AgentId,
    insight: &FailureInsight,
    decision_id: Option<DecisionId>,
    audit_event_id: Option<AuditEventId>,
    proposed_at: chrono::DateTime<Utc>,
) -> Result<MemoryWriteIntent, Box<dyn Error>> {
    Ok(MemoryWriteIntent::new(
        MemoryWriteKind::CreateFailureInsightMemory,
        MemoryWriteTarget {
            entity_type: "failure_insight".to_owned(),
            entity_id: insight.id.to_string(),
            attribute: Some("insight".to_owned()),
            value: Some(serde_json::to_value(insight)?),
            fact_id: None,
            related_fact_id: None,
            failure_insight_id: Some(insight.id.clone()),
        },
        MemoryWriteProvenance::new(
            Some(SourceId::new("source-demo-failure-insight-loop")),
            "local FailureInsight demo signal",
            "system_observation",
            "A local, in-memory demo exercises proposal, Decision Gate, audit, approved persistence, and readback.",
        ),
        insight.confidence,
        agent_id.clone(),
        "Remember this bounded FailureInsight only inside the local demo Graph Memory store for supervised readback proof.",
        proposed_at,
    )
    .with_audit_linkage(decision_id, audit_event_id))
}

fn format_memory_demo_failure_insight_readback(
    readback: &MemoryDemoFailureInsightReadback,
) -> String {
    let mut output = String::new();
    push_readback_line(&mut output, &style_info("FailureInsight memory demo"));
    push_readback_field(&mut output, "signal_type:", readback.signal.signal_type);
    push_readback_field(&mut output, "signal_summary:", &readback.signal.summary);
    push_readback_field(
        &mut output,
        "correction_target:",
        readback.signal.correction_target,
    );
    push_readback_field(&mut output, "provenance:", readback.signal.provenance);
    push_readback_field(
        &mut output,
        "proposed_action_id:",
        &readback.proposed_action_id,
    );
    push_readback_field(
        &mut output,
        "memory_write_kind:",
        &readback.memory_write_kind,
    );
    push_readback_field(&mut output, "decision_id:", &readback.decision_id);
    push_readback_field(&mut output, "decision_status:", &readback.decision_status);
    push_readback_field(&mut output, "decision_reason:", &readback.decision_reason);
    push_readback_field(&mut output, "audit_event_id:", &readback.audit_event_id);
    push_readback_field(
        &mut output,
        "persisted_failure_insight_id:",
        readback
            .persisted_failure_insight_id
            .as_deref()
            .unwrap_or("none"),
    );
    if let Some(inspection) = &readback.inspected_failure_insight {
        push_readback_field(
            &mut output,
            "inspected_failure_insight_id:",
            inspection
                .inspected_failure_insight_id
                .as_deref()
                .unwrap_or("none"),
        );
        push_readback_field(
            &mut output,
            "inspected_failure_insight_found:",
            &inspection.found.to_string(),
        );
        push_readback_field(
            &mut output,
            "inspected_failure_insight_summary:",
            inspection.summary.as_deref().unwrap_or("none"),
        );
        push_readback_field(
            &mut output,
            "inspect_command:",
            FAILURE_INSIGHT_DEMO_INSPECT_COMMAND,
        );
    }
    push_readback_field(
        &mut output,
        "readback_found:",
        &readback.readback_found.to_string(),
    );
    push_readback_field(
        &mut output,
        "readback_audit_event_count:",
        &readback.readback_audit_event_count.to_string(),
    );
    push_readback_field(
        &mut output,
        "readback_relation_count:",
        &readback.readback_relation_count.to_string(),
    );
    push_readback_field(
        &mut output,
        "functional_alpha_chain:",
        &format_static_list(readback.functional_alpha_chain),
    );
    push_readback_field(
        &mut output,
        "exact_local_command:",
        readback.exact_local_command,
    );
    push_readback_field(
        &mut output,
        "repeatable_demo_recipe:",
        &format_static_list(readback.repeatable_demo_recipe),
    );
    push_readback_field(
        &mut output,
        "next_safe_human_action:",
        readback.next_safe_human_action,
    );
    push_readback_line(&mut output, &style_dim(readback.readback_warning));
    push_readback_line(&mut output, &style_dim(readback.warning));
    output
}

fn memory_proposals_readback_from_actions(actions: Vec<ProposedAction>) -> MemoryProposalsReadback {
    let proposals = actions
        .into_iter()
        .filter_map(memory_proposal_summary_from_action)
        .collect();

    MemoryProposalsReadback {
        proposals,
        warning: MEMORY_READBACK_WARNING,
    }
}

fn memory_proposal_summary_from_action(action: ProposedAction) -> Option<MemoryProposalSummary> {
    let action_type = to_api_string(&action.action_type).ok()?;
    if !is_memory_write_action_type(&action_type) {
        return None;
    }

    let intent = action
        .payload
        .get("memory_write_intent")
        .unwrap_or(&action.payload);
    let target = intent.get("target").unwrap_or(&Value::Null);
    let provenance = intent.get("provenance").unwrap_or(&Value::Null);

    Some(MemoryProposalSummary {
        id: action.id.to_string(),
        workspace_id: action.workspace_id.to_string(),
        task_id: action.task_id.map(|task_id| task_id.to_string()),
        proposed_by: action.proposed_by.to_string(),
        action_type,
        status: to_api_string(&action.status).unwrap_or_else(|_| "unknown".to_owned()),
        risk_level: to_api_string(&action.risk_level).unwrap_or_else(|_| "unknown".to_owned()),
        required_permissions: action
            .required_permissions
            .iter()
            .filter_map(|permission| to_api_string(permission).ok())
            .collect(),
        target: action.target,
        rationale: action.rationale,
        created_at: action.created_at.to_rfc3339(),
        memory_write_kind: string_field(intent, "kind"),
        target_type: string_field(target, "entity_type"),
        target_id: string_field(target, "entity_id"),
        target_attribute: string_field(target, "attribute"),
        target_value: target.get("value").cloned(),
        target_fact_id: string_field(target, "fact_id"),
        related_fact_id: string_field(target, "related_fact_id"),
        failure_insight_id: string_field(target, "failure_insight_id"),
        provenance_source_id: string_field(provenance, "source_id"),
        provenance_source_label: string_field(provenance, "source_label"),
        provenance_source_kind: string_field(provenance, "source_kind"),
        provenance_evidence: string_field(provenance, "evidence"),
        confidence: intent.get("confidence").and_then(Value::as_f64),
        actor: string_field(intent, "actor"),
        reason_for_remembering: string_field(intent, "reason_for_remembering"),
        proposed_at: string_field(intent, "proposed_at"),
        decision_id: string_field(intent, "decision_id"),
        audit_event_id: string_field(intent, "audit_event_id"),
        invalidation_note: string_field(intent, "invalidation_note"),
        persistence_readback_hint: memory_proposal_persistence_readback_hint(
            &action.status,
            &string_field(target, "fact_id"),
            &string_field(target, "failure_insight_id"),
            &string_field(intent, "decision_id"),
            &string_field(intent, "audit_event_id"),
        ),
        supersession_hint: memory_proposal_supersession_hint(
            &string_field(target, "fact_id"),
            &string_field(target, "related_fact_id"),
            &string_field(target, "failure_insight_id"),
            &string_field(intent, "invalidation_note"),
        ),
        suggested_next_action: memory_proposal_next_action(&action.status),
    })
}

fn is_memory_write_action_type(action_type: &str) -> bool {
    matches!(
        action_type,
        "write_memory"
            | "create_memory_fact"
            | "link_memory_fact"
            | "invalidate_memory_fact"
            | "create_failure_insight_memory"
    )
}

fn memory_proposal_next_action(status: &ProposedActionStatus) -> String {
    match status {
        ProposedActionStatus::PendingDecision => {
            "Evaluate through Decision Gate before any memory persistence.".to_owned()
        }
        ProposedActionStatus::NeedsHumanApproval => {
            "Review Decision Gate result and obtain explicit human confirmation before persistence."
                .to_owned()
        }
        ProposedActionStatus::Approved => {
            "Persist only through an explicit governed Graph Memory path, then inspect readback."
                .to_owned()
        }
        ProposedActionStatus::Blocked => {
            "Do not persist; inspect explicit reason and correct proposal, policy, or evidence."
                .to_owned()
        }
        ProposedActionStatus::Cancelled => "No action; proposal was cancelled.".to_owned(),
    }
}

fn memory_proposal_persistence_readback_hint(
    status: &ProposedActionStatus,
    fact_id: &Option<String>,
    failure_insight_id: &Option<String>,
    decision_id: &Option<String>,
    audit_event_id: &Option<String>,
) -> String {
    if status != &ProposedActionStatus::Approved {
        return "Not persistable yet: inspect Decision Gate status before using Graph Memory helpers."
            .to_owned();
    }

    let artifact = fact_id
        .as_deref()
        .map(|id| format!("fact {id}"))
        .or_else(|| {
            failure_insight_id
                .as_deref()
                .map(|id| format!("FailureInsight {id}"))
        })
        .unwrap_or_else(|| "the generated Graph Memory artifact".to_owned());
    let decision = decision_id.as_deref().unwrap_or("the approved decision");
    let audit = audit_event_id
        .as_deref()
        .unwrap_or("the matching decision audit event");

    format!(
        "After explicit governed persistence, inspect {artifact}; verify it remains linked to decision {decision} and audit event {audit}."
    )
}

fn memory_proposal_supersession_hint(
    fact_id: &Option<String>,
    related_fact_id: &Option<String>,
    failure_insight_id: &Option<String>,
    invalidation_note: &Option<String>,
) -> String {
    if let Some(note) = invalidation_note {
        return format!("Future invalidation/supersession note: {note}");
    }
    if let Some(related_fact_id) = related_fact_id {
        return format!(
            "If this relationship becomes stale, propose invalidate_memory_fact or link supersession for related fact {related_fact_id}."
        );
    }
    if let Some(fact_id) = fact_id {
        return format!(
            "If this fact becomes stale, propose invalidate_memory_fact for {fact_id} before replacing it."
        );
    }
    if let Some(failure_insight_id) = failure_insight_id {
        return format!(
            "If this FailureInsight is superseded, create a later insight that references {failure_insight_id} and preserves audit linkage."
        );
    }

    "Future invalidation/supersession path must be proposed through governed memory-write intent before mutation."
        .to_owned()
}

fn format_memory_proposals_readback(readback: &MemoryProposalsReadback) -> String {
    let mut output = String::new();
    push_readback_line(&mut output, &style_info("Memory write proposals"));
    push_readback_field(
        &mut output,
        "proposal_count:",
        &readback.proposals.len().to_string(),
    );
    if readback.proposals.is_empty() {
        push_readback_line(
            &mut output,
            &style_dim("No governed memory-write proposals found."),
        );
    }
    for proposal in &readback.proposals {
        push_memory_proposal_fields(&mut output, proposal);
    }
    push_readback_line(&mut output, &style_dim(readback.warning));
    output
}

fn format_memory_proposal_detail_readback(readback: &MemoryProposalDetailReadback) -> String {
    let mut output = String::new();
    push_readback_line(&mut output, &style_info("Memory write proposal"));
    match &readback.proposal {
        Some(proposal) => push_memory_proposal_fields(&mut output, proposal),
        None => push_readback_line(
            &mut output,
            &style_dim("No governed memory-write proposal found for that id."),
        ),
    }
    push_readback_line(&mut output, &style_dim(readback.warning));
    output
}

fn push_memory_proposal_fields(output: &mut String, proposal: &MemoryProposalSummary) {
    push_readback_field(output, "id:", &proposal.id);
    push_readback_field(output, "action_type:", &proposal.action_type);
    push_readback_field(output, "status:", &proposal.status);
    push_readback_field(output, "risk_level:", &proposal.risk_level);
    push_readback_field(
        output,
        "required_permissions:",
        &format_policies(&proposal.required_permissions),
    );
    push_readback_field(output, "workspace_id:", &proposal.workspace_id);
    push_readback_field(
        output,
        "task_id:",
        proposal.task_id.as_deref().unwrap_or("-"),
    );
    push_readback_field(output, "proposed_by:", &proposal.proposed_by);
    push_readback_field(output, "target:", proposal.target.as_deref().unwrap_or("-"));
    push_readback_field(
        output,
        "memory_write_kind:",
        proposal.memory_write_kind.as_deref().unwrap_or("-"),
    );
    push_readback_field(
        output,
        "target_type:",
        proposal.target_type.as_deref().unwrap_or("-"),
    );
    push_readback_field(
        output,
        "target_id:",
        proposal.target_id.as_deref().unwrap_or("-"),
    );
    push_readback_field(
        output,
        "target_attribute:",
        proposal.target_attribute.as_deref().unwrap_or("-"),
    );
    push_readback_field(
        output,
        "target_value:",
        &format_optional_json(&proposal.target_value),
    );
    push_readback_field(
        output,
        "target_fact_id:",
        proposal.target_fact_id.as_deref().unwrap_or("-"),
    );
    push_readback_field(
        output,
        "related_fact_id:",
        proposal.related_fact_id.as_deref().unwrap_or("-"),
    );
    push_readback_field(
        output,
        "failure_insight_id:",
        proposal.failure_insight_id.as_deref().unwrap_or("-"),
    );
    push_readback_field(
        output,
        "provenance_source_id:",
        proposal.provenance_source_id.as_deref().unwrap_or("-"),
    );
    push_readback_field(
        output,
        "provenance_source_label:",
        proposal.provenance_source_label.as_deref().unwrap_or("-"),
    );
    push_readback_field(
        output,
        "provenance_source_kind:",
        proposal.provenance_source_kind.as_deref().unwrap_or("-"),
    );
    push_readback_field(
        output,
        "provenance_evidence:",
        proposal.provenance_evidence.as_deref().unwrap_or("-"),
    );
    push_readback_field(
        output,
        "confidence:",
        &proposal
            .confidence
            .map(|confidence| confidence.to_string())
            .unwrap_or_else(|| "-".to_owned()),
    );
    push_readback_field(output, "actor:", proposal.actor.as_deref().unwrap_or("-"));
    push_readback_field(
        output,
        "reason_for_remembering:",
        proposal.reason_for_remembering.as_deref().unwrap_or("-"),
    );
    push_readback_field(
        output,
        "proposed_at:",
        proposal.proposed_at.as_deref().unwrap_or("-"),
    );
    push_readback_field(
        output,
        "decision_id:",
        proposal.decision_id.as_deref().unwrap_or("-"),
    );
    push_readback_field(
        output,
        "audit_event_id:",
        proposal.audit_event_id.as_deref().unwrap_or("-"),
    );
    push_readback_field(
        output,
        "invalidation_note:",
        proposal.invalidation_note.as_deref().unwrap_or("-"),
    );
    push_readback_field(
        output,
        "persistence_readback_hint:",
        &proposal.persistence_readback_hint,
    );
    push_readback_field(output, "supersession_hint:", &proposal.supersession_hint);
    push_readback_field(output, "created_at:", &proposal.created_at);
    push_readback_field(output, "rationale:", &proposal.rationale);
    push_readback_field(
        output,
        "suggested_next_action:",
        &proposal.suggested_next_action,
    );
}

fn insight_schema(args: InsightSchemaArgs) -> Result<(), Box<dyn Error>> {
    let readback = insight_schema_readback();
    if args.json {
        println!("{}", serde_json::to_string_pretty(&readback)?);
    } else {
        print!("{}", format_insight_schema_readback(&readback));
    }
    Ok(())
}

fn insight_schema_readback() -> InsightSchemaReadback {
    InsightSchemaReadback {
        purpose: "Classify failures and corrections into durable, traceable learning artifacts for human supervision.",
        minimum_fields: INSIGHT_MINIMUM_FIELDS,
        failure_classes: INSIGHT_FAILURE_CLASSES,
        correction_targets: INSIGHT_CORRECTION_TARGETS,
        statuses: INSIGHT_STATUSES,
        severities: INSIGHT_SEVERITIES,
        detection_signal_types: INSIGHT_DETECTION_SIGNAL_TYPES,
        alpha_limits: INSIGHT_ALPHA_LIMITS,
        warning: INSIGHT_READBACK_WARNING,
    }
}

fn format_insight_schema_readback(readback: &InsightSchemaReadback) -> String {
    let mut output = String::new();
    push_readback_line(&mut output, &style_info("Failure-to-Insight schema"));
    push_readback_field(&mut output, "purpose:", readback.purpose);
    push_readback_field(
        &mut output,
        "minimum_fields:",
        &format_static_list(readback.minimum_fields),
    );
    push_readback_field(
        &mut output,
        "failure_classes:",
        &format_static_list(readback.failure_classes),
    );
    push_readback_field(
        &mut output,
        "correction_targets:",
        &format_static_list(readback.correction_targets),
    );
    push_readback_field(
        &mut output,
        "statuses:",
        &format_static_list(readback.statuses),
    );
    push_readback_field(
        &mut output,
        "severities:",
        &format_static_list(readback.severities),
    );
    push_readback_field(
        &mut output,
        "detection_signal_types:",
        &format_static_list(readback.detection_signal_types),
    );
    push_readback_field(
        &mut output,
        "alpha_limits:",
        &format_static_list(readback.alpha_limits),
    );
    push_readback_line(&mut output, &style_dim(readback.warning));
    output
}

fn format_static_list(values: &[&str]) -> String {
    if values.is_empty() {
        "-".to_owned()
    } else {
        values.join(", ")
    }
}

async fn create_task(
    client: &Client,
    api_url: &str,
    args: CreateTaskArgs,
) -> Result<(), Box<dyn Error>> {
    let response: Task = get_json(
        client
            .post(format!("{api_url}/tasks"))
            .json(&json!({
                "workspace_id": args.workspace_id,
                "title": args.title,
                "description": args.description,
            }))
            .send()
            .await?,
    )
    .await?;

    println!("{} {}", style_success("Created task:"), response.id);
    println!("{} {}", style_dim("Title:"), response.title);
    println!("{} {:?}", style_dim("Status:"), response.status);
    Ok(())
}

async fn list_tasks(client: &Client, api_url: &str) -> Result<(), Box<dyn Error>> {
    let tasks: Vec<Task> = get_json(client.get(format!("{api_url}/tasks")).send().await?).await?;

    if tasks.is_empty() {
        println!("{}", style_dim("No tasks."));
        return Ok(());
    }

    println!("{}", style_info("Tasks"));
    for task in tasks {
        println!("{} {}", style_dim("- id:"), task.id);
        println!("  {} {}", style_dim("title:"), task.title);
        println!(
            "  {} {}",
            style_dim("status:"),
            to_api_string(&task.status)?
        );
    }

    Ok(())
}

async fn propose_action(
    client: &Client,
    api_url: &str,
    args: ProposeActionArgs,
) -> Result<(), Box<dyn Error>> {
    let permissions = normalize_permissions(args.permissions.clone());
    let payload = default_payload(&args);
    let response: ProposedAction = get_json(
        client
            .post(format!("{api_url}/proposed-actions"))
            .json(&json!({
                "workspace_id": args.workspace_id,
                "task_id": args.task_id,
                "proposed_by": args.proposed_by,
                "action_type": normalize_action_type(&args.action_type),
                "target": args.target,
                "risk_level": args.risk.as_api_value(),
                "required_permissions": permissions,
                "rationale": args.rationale,
                "payload": payload,
            }))
            .send()
            .await?,
    )
    .await?;

    println!(
        "{} {}",
        style_success("Created proposed action:"),
        response.id
    );
    println!(
        "{} {}",
        style_dim("Status:"),
        to_api_string(&response.status)?
    );
    Ok(())
}

async fn propose_agent_action(
    client: &Client,
    api_url: &str,
    args: ProposeAgentArgs,
) -> Result<(), Box<dyn Error>> {
    let response = propose_agent_request(
        client,
        api_url,
        &args.workspace_id,
        &args.task_id,
        &args.provider,
        &args.prompt,
    )
    .await?;

    print_agent_turn(&response)
}

async fn propose_agent_request(
    client: &Client,
    api_url: &str,
    workspace_id: &str,
    task_id: &str,
    provider: &str,
    prompt: &str,
) -> Result<AgentProposeResponse, Box<dyn Error>> {
    get_json(
        client
            .post(format!("{api_url}/agent/propose"))
            .json(&json!({
                "workspace_id": workspace_id,
                "task_id": task_id,
                "prompt": prompt,
                "provider": provider,
            }))
            .send()
            .await?,
    )
    .await
}

fn print_agent_turn(response: &AgentProposeResponse) -> Result<(), Box<dyn Error>> {
    match response.kind.as_str() {
        "direct_reply" => {
            println!(
                "{} {}",
                style_info("DirectReply:"),
                response.message.as_deref().unwrap_or("")
            );
            Ok(())
        }
        "clarifying_question" => {
            println!(
                "{} {}",
                style_warning("ClarifyingQuestion:"),
                response.question.as_deref().unwrap_or("")
            );
            Ok(())
        }
        "proposed_action" => match response.proposed_action.as_ref() {
            Some(action) => print_agent_proposal(action),
            None => Err("API returned proposed_action without proposed_action payload".into()),
        },
        other => Err(format!("unknown agent turn kind: {other}").into()),
    }
}

fn print_agent_proposal(action: &ProposedAction) -> Result<(), Box<dyn Error>> {
    println!("{}", style_info("ProposedAction"));
    println!("{} {}", style_dim("id:"), action.id);
    println!(
        "{} {}",
        style_dim("type:"),
        to_api_string(&action.action_type)?
    );
    println!(
        "{} {}",
        style_dim("risk:"),
        style_risk(&to_api_string(&action.risk_level)?)
    );
    println!(
        "{} {}",
        style_dim("status:"),
        style_status(&to_api_string(&action.status)?)
    );
    println!("{} {}", style_dim("rationale:"), action.rationale);
    println!("{} /evaluate {}", style_dim("next:"), action.id);
    Ok(())
}

async fn evaluate_action(
    client: &Client,
    api_url: &str,
    args: EvaluateActionArgs,
) -> Result<(), Box<dyn Error>> {
    let response = evaluate_action_request(
        client,
        api_url,
        &args.proposed_action_id,
        &normalize_permissions(args.permissions),
    )
    .await?;

    print_decision(&response)
}

async fn evaluate_action_request(
    client: &Client,
    api_url: &str,
    proposed_action_id: &str,
    permissions: &[String],
) -> Result<EvaluateResponse, Box<dyn Error>> {
    get_json(
        client
            .post(format!("{api_url}/decision-gate/evaluate"))
            .json(&json!({
                "proposed_action_id": proposed_action_id,
                "granted_permissions": permissions,
            }))
            .send()
            .await?,
    )
    .await
}

fn print_decision(response: &EvaluateResponse) -> Result<(), Box<dyn Error>> {
    println!(
        "{} {}",
        style_success("Decision:"),
        style_status(&response.decision.status)
    );
    println!("{} {}", style_info("Audit:"), response.audit_event.id);
    Ok(())
}

async fn list_actions(client: &Client, api_url: &str) -> Result<(), Box<dyn Error>> {
    let actions: Vec<ProposedAction> = get_json(
        client
            .get(format!("{api_url}/proposed-actions"))
            .send()
            .await?,
    )
    .await?;

    if actions.is_empty() {
        println!("{}", style_dim("No proposed actions."));
        return Ok(());
    }

    println!("{}", style_info("Proposed actions"));
    for action in actions {
        println!("{} {}", style_dim("- id:"), action.id);
        println!(
            "  {} {}",
            style_dim("action_type:"),
            to_api_string(&action.action_type)?
        );
        println!(
            "  {} {}",
            style_dim("risk_level:"),
            style_risk(&to_api_string(&action.risk_level)?)
        );
        println!(
            "  {} {}",
            style_dim("status:"),
            style_status(&to_api_string(&action.status)?)
        );
    }

    Ok(())
}

async fn list_audit(client: &Client, api_url: &str) -> Result<(), Box<dyn Error>> {
    let events: Vec<AuditEvent> =
        get_json(client.get(format!("{api_url}/audit")).send().await?).await?;

    if events.is_empty() {
        println!("{}", style_dim("No audit events."));
        return Ok(());
    }

    println!("{}", style_info("Audit events"));
    for event in events {
        println!("{} {}", style_dim("- id:"), event.id);
        println!(
            "  {} {}",
            style_dim("event_type:"),
            to_api_string(&event.event_type)?
        );
        println!(
            "  {} {}",
            style_dim("proposed_action_id:"),
            event
                .proposed_action_id
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| "-".to_owned())
        );
        println!(
            "  {} {}",
            style_dim("decision_id:"),
            event
                .decision_id
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| "-".to_owned())
        );
        let metadata = decision_readback_metadata(std::slice::from_ref(&event));
        if metadata.decision_status.is_some()
            || metadata.explicit_reason.is_some()
            || metadata.action_type.is_some()
        {
            println!(
                "  {} {}",
                style_dim("decision_status:"),
                metadata.decision_status.as_deref().unwrap_or("-")
            );
            println!(
                "  {} {}",
                style_dim("explicit_reason:"),
                metadata.explicit_reason.as_deref().unwrap_or("-")
            );
            println!(
                "  {} {}",
                style_dim("action_type:"),
                metadata.action_type.as_deref().unwrap_or("-")
            );
            println!(
                "  {} {}",
                style_dim("risk:"),
                metadata.risk_level.as_deref().unwrap_or("-")
            );
            println!(
                "  {} {}",
                style_dim("matched_policy_or_fallback_rule:"),
                metadata
                    .matched_policy_or_fallback_rule
                    .as_deref()
                    .unwrap_or("-")
            );
            println!(
                "  {} {}",
                style_dim("required_permission:"),
                metadata.required_permission.as_deref().unwrap_or("-")
            );
            println!(
                "  {} {}",
                style_dim("timestamp:"),
                metadata.timestamp.as_deref().unwrap_or("-")
            );
            println!(
                "  {} {}",
                style_dim("suggested_next_action:"),
                metadata.suggested_next_action.as_deref().unwrap_or("-")
            );
        }
    }

    Ok(())
}

async fn audit_decision_summary(
    client: &Client,
    api_url: &str,
    args: DecisionSummaryArgs,
) -> Result<(), Box<dyn Error>> {
    let events: Vec<AuditEvent> =
        get_json(client.get(format!("{api_url}/audit")).send().await?).await?;
    let readback = decision_readback_from_audit_events(events, &args.decision_id);

    if args.json {
        println!("{}", serde_json::to_string_pretty(&readback)?);
    } else {
        print_audit_decision_readback(&readback);
    }
    Ok(())
}

async fn audit_task_summary(
    client: &Client,
    api_url: &str,
    args: TaskSummaryArgs,
) -> Result<(), Box<dyn Error>> {
    let events: Vec<AuditEvent> =
        get_json(client.get(format!("{api_url}/audit")).send().await?).await?;
    let readback = task_readback_from_audit_events(events, &args.task_id);

    if args.json {
        println!("{}", serde_json::to_string_pretty(&readback)?);
    } else {
        print_audit_task_readback(&readback);
    }
    Ok(())
}

async fn audit_workspace_summary(
    client: &Client,
    api_url: &str,
    args: WorkspaceSummaryArgs,
) -> Result<(), Box<dyn Error>> {
    let events: Vec<AuditEvent> =
        get_json(client.get(format!("{api_url}/audit")).send().await?).await?;
    let readback = workspace_readback_from_audit_events(events, &args.workspace_id);

    if args.json {
        println!("{}", serde_json::to_string_pretty(&readback)?);
    } else {
        print_audit_workspace_readback(&readback);
    }
    Ok(())
}

fn decision_readback_from_audit_events(
    events: Vec<AuditEvent>,
    decision_id: &str,
) -> AuditDecisionReadback {
    let decision_id = DecisionId::new(decision_id);
    let mut decision_events = events
        .into_iter()
        .filter(|event| event.decision_id.as_ref() == Some(&decision_id))
        .collect::<Vec<_>>();
    decision_events.sort_by_key(|event| event.created_at);

    let metadata = decision_readback_metadata(&decision_events);
    let mut summary = AuditTraceSummary::from_events(&decision_events);
    summary.decision_id = Some(decision_id);

    AuditDecisionReadback {
        summary,
        decision_status: metadata.decision_status,
        explicit_reason: metadata.explicit_reason,
        action_type: metadata.action_type,
        memory_write_kind: metadata.memory_write_kind,
        memory_target_type: metadata.memory_target_type,
        memory_target_id: metadata.memory_target_id,
        memory_target_attribute: metadata.memory_target_attribute,
        memory_target_value: metadata.memory_target_value,
        memory_target_fact_id: metadata.memory_target_fact_id,
        memory_related_fact_id: metadata.memory_related_fact_id,
        memory_failure_insight_id: metadata.memory_failure_insight_id,
        memory_provenance_source_id: metadata.memory_provenance_source_id,
        memory_provenance_source_label: metadata.memory_provenance_source_label,
        memory_provenance_source_kind: metadata.memory_provenance_source_kind,
        memory_provenance_evidence: metadata.memory_provenance_evidence,
        memory_confidence: metadata.memory_confidence,
        memory_actor: metadata.memory_actor,
        memory_reason_for_remembering: metadata.memory_reason_for_remembering,
        memory_proposed_at: metadata.memory_proposed_at,
        memory_invalidation_note: metadata.memory_invalidation_note,
        memory_decision_id: metadata.memory_decision_id,
        memory_audit_event_id: metadata.memory_audit_event_id,
        memory_persistence_readback_hint: metadata.memory_persistence_readback_hint,
        memory_supersession_hint: metadata.memory_supersession_hint,
        risk_level: metadata.risk_level,
        matched_policy_or_fallback_rule: metadata.matched_policy_or_fallback_rule,
        required_permission: metadata.required_permission,
        timestamp: metadata.timestamp,
        suggested_next_action: metadata.suggested_next_action,
        block_reason_category: metadata.block_reason_category,
        policies_applied: metadata.policies_applied,
        warning: AUDIT_READBACK_WARNING,
    }
}

fn task_readback_from_audit_events(events: Vec<AuditEvent>, task_id: &str) -> AuditTaskReadback {
    let task_id = TaskId::new(task_id);
    let mut task_events = events
        .into_iter()
        .filter(|event| event.task_id.as_ref() == Some(&task_id))
        .collect::<Vec<_>>();
    task_events.sort_by_key(|event| event.created_at);

    let mut summary = AuditTraceSummary::from_events(&task_events);
    summary.task_id = Some(task_id);

    AuditTaskReadback {
        summary,
        warning: AUDIT_READBACK_WARNING,
    }
}

fn workspace_readback_from_audit_events(
    events: Vec<AuditEvent>,
    workspace_id: &str,
) -> AuditWorkspaceReadback {
    let workspace_id = WorkspaceId::new(workspace_id);
    let mut workspace_events = events
        .into_iter()
        .filter(|event| event.workspace_id.as_ref() == Some(&workspace_id))
        .collect::<Vec<_>>();
    workspace_events.sort_by_key(|event| event.created_at);

    let mut summary = AuditTraceSummary::from_events(&workspace_events);
    summary.workspace_id = Some(workspace_id);

    AuditWorkspaceReadback {
        summary,
        warning: AUDIT_READBACK_WARNING,
    }
}

#[derive(Debug, Default)]
struct AuditDecisionMetadata {
    decision_status: Option<String>,
    explicit_reason: Option<String>,
    action_type: Option<String>,
    memory_write_kind: Option<String>,
    memory_target_type: Option<String>,
    memory_target_id: Option<String>,
    memory_target_attribute: Option<String>,
    memory_target_value: Option<Value>,
    memory_target_fact_id: Option<String>,
    memory_related_fact_id: Option<String>,
    memory_failure_insight_id: Option<String>,
    memory_provenance_source_id: Option<String>,
    memory_provenance_source_label: Option<String>,
    memory_provenance_source_kind: Option<String>,
    memory_provenance_evidence: Option<String>,
    memory_confidence: Option<f64>,
    memory_actor: Option<String>,
    memory_reason_for_remembering: Option<String>,
    memory_proposed_at: Option<String>,
    memory_invalidation_note: Option<String>,
    memory_decision_id: Option<String>,
    memory_audit_event_id: Option<String>,
    memory_persistence_readback_hint: Option<String>,
    memory_supersession_hint: Option<String>,
    risk_level: Option<String>,
    matched_policy_or_fallback_rule: Option<String>,
    required_permission: Option<String>,
    timestamp: Option<String>,
    suggested_next_action: Option<String>,
    block_reason_category: Option<String>,
    policies_applied: Vec<String>,
}

fn decision_readback_metadata(events: &[AuditEvent]) -> AuditDecisionMetadata {
    let mut metadata = AuditDecisionMetadata::default();

    for event in events {
        let trace = event.payload.get("causal_trace").unwrap_or(&event.payload);

        if metadata.decision_status.is_none() {
            metadata.decision_status = string_field(trace, "decision_status")
                .or_else(|| string_field(trace, "decision_outcome"));
        }
        if metadata.explicit_reason.is_none() {
            metadata.explicit_reason =
                string_field(trace, "explicit_reason").or_else(|| string_field(trace, "reason"));
        }
        if metadata.action_type.is_none() {
            metadata.action_type = string_field(trace, "action_type");
        }
        populate_memory_write_metadata(&mut metadata, trace.get("memory_write_intent"));
        if metadata.risk_level.is_none() {
            metadata.risk_level =
                string_field(trace, "risk_level").or_else(|| string_field(trace, "risk"));
        }
        if metadata.matched_policy_or_fallback_rule.is_none() {
            metadata.matched_policy_or_fallback_rule =
                string_field(trace, "matched_policy_or_fallback_rule");
        }
        if metadata.required_permission.is_none() {
            metadata.required_permission = string_field(trace, "required_permission");
        }
        if metadata.timestamp.is_none() {
            metadata.timestamp = string_field(trace, "timestamp");
        }
        if metadata.suggested_next_action.is_none() {
            metadata.suggested_next_action = string_field(trace, "suggested_next_action");
        }
        if metadata.block_reason_category.is_none() {
            metadata.block_reason_category = string_field(trace, "block_reason_category");
        }
        if metadata.policies_applied.is_empty() {
            metadata.policies_applied = string_array_field(trace, "policies_applied");
        }
    }

    metadata
}

fn populate_memory_write_metadata(metadata: &mut AuditDecisionMetadata, intent: Option<&Value>) {
    let Some(intent) = intent else {
        return;
    };
    let target = intent.get("target").unwrap_or(&Value::Null);
    let provenance = intent.get("provenance").unwrap_or(&Value::Null);

    if metadata.memory_write_kind.is_none() {
        metadata.memory_write_kind = string_field(intent, "kind");
    }
    if metadata.memory_target_type.is_none() {
        metadata.memory_target_type = string_field(target, "entity_type");
    }
    if metadata.memory_target_id.is_none() {
        metadata.memory_target_id = string_field(target, "entity_id");
    }
    if metadata.memory_target_attribute.is_none() {
        metadata.memory_target_attribute = string_field(target, "attribute");
    }
    if metadata.memory_target_value.is_none() {
        metadata.memory_target_value = target.get("value").cloned();
    }
    if metadata.memory_target_fact_id.is_none() {
        metadata.memory_target_fact_id = string_field(target, "fact_id");
    }
    if metadata.memory_related_fact_id.is_none() {
        metadata.memory_related_fact_id = string_field(target, "related_fact_id");
    }
    if metadata.memory_failure_insight_id.is_none() {
        metadata.memory_failure_insight_id = string_field(target, "failure_insight_id");
    }
    if metadata.memory_provenance_source_id.is_none() {
        metadata.memory_provenance_source_id = string_field(provenance, "source_id");
    }
    if metadata.memory_provenance_source_label.is_none() {
        metadata.memory_provenance_source_label = string_field(provenance, "source_label");
    }
    if metadata.memory_provenance_source_kind.is_none() {
        metadata.memory_provenance_source_kind = string_field(provenance, "source_kind");
    }
    if metadata.memory_provenance_evidence.is_none() {
        metadata.memory_provenance_evidence = string_field(provenance, "evidence");
    }
    if metadata.memory_confidence.is_none() {
        metadata.memory_confidence = intent.get("confidence").and_then(Value::as_f64);
    }
    if metadata.memory_actor.is_none() {
        metadata.memory_actor = string_field(intent, "actor");
    }
    if metadata.memory_reason_for_remembering.is_none() {
        metadata.memory_reason_for_remembering = string_field(intent, "reason_for_remembering");
    }
    if metadata.memory_proposed_at.is_none() {
        metadata.memory_proposed_at = string_field(intent, "proposed_at");
    }
    if metadata.memory_invalidation_note.is_none() {
        metadata.memory_invalidation_note = string_field(intent, "invalidation_note");
    }
    if metadata.memory_decision_id.is_none() {
        metadata.memory_decision_id = string_field(intent, "decision_id");
    }
    if metadata.memory_audit_event_id.is_none() {
        metadata.memory_audit_event_id = string_field(intent, "audit_event_id");
    }
    if metadata.memory_persistence_readback_hint.is_none() {
        metadata.memory_persistence_readback_hint = Some(memory_audit_persistence_readback_hint(
            metadata.decision_status.as_deref(),
            metadata.memory_target_fact_id.as_deref(),
            metadata.memory_failure_insight_id.as_deref(),
            metadata.memory_decision_id.as_deref(),
            metadata.memory_audit_event_id.as_deref(),
        ));
    }
    if metadata.memory_supersession_hint.is_none() {
        metadata.memory_supersession_hint = Some(memory_audit_supersession_hint(
            metadata.memory_target_fact_id.as_deref(),
            metadata.memory_related_fact_id.as_deref(),
            metadata.memory_failure_insight_id.as_deref(),
            metadata.memory_invalidation_note.as_deref(),
        ));
    }
}

fn memory_audit_persistence_readback_hint(
    decision_status: Option<&str>,
    fact_id: Option<&str>,
    failure_insight_id: Option<&str>,
    decision_id: Option<&str>,
    audit_event_id: Option<&str>,
) -> String {
    if decision_status != Some("approved") {
        return "Not persistable yet: inspect Decision Gate status before using Graph Memory helpers."
            .to_owned();
    }

    let artifact = fact_id
        .map(|id| format!("fact {id}"))
        .or_else(|| failure_insight_id.map(|id| format!("FailureInsight {id}")))
        .unwrap_or_else(|| "the generated Graph Memory artifact".to_owned());
    let decision = decision_id.unwrap_or("the approved decision");
    let audit = audit_event_id.unwrap_or("the matching decision audit event");

    format!(
        "After explicit governed persistence, inspect {artifact}; verify it remains linked to decision {decision} and audit event {audit}."
    )
}

fn memory_audit_supersession_hint(
    fact_id: Option<&str>,
    related_fact_id: Option<&str>,
    failure_insight_id: Option<&str>,
    invalidation_note: Option<&str>,
) -> String {
    if let Some(note) = invalidation_note {
        return format!("Future invalidation/supersession note: {note}");
    }
    if let Some(related_fact_id) = related_fact_id {
        return format!(
            "If this relationship becomes stale, propose invalidate_memory_fact or link supersession for related fact {related_fact_id}."
        );
    }
    if let Some(fact_id) = fact_id {
        return format!(
            "If this fact becomes stale, propose invalidate_memory_fact for {fact_id} before replacing it."
        );
    }
    if let Some(failure_insight_id) = failure_insight_id {
        return format!(
            "If this FailureInsight is superseded, create a later insight that references {failure_insight_id} and preserves audit linkage."
        );
    }

    "Future invalidation/supersession path must be proposed through governed memory-write intent before mutation."
        .to_owned()
}

fn string_field(value: &Value, field: &str) -> Option<String> {
    value.get(field).and_then(|field_value| match field_value {
        Value::String(value) => Some(value.clone()),
        Value::Null => None,
        other => Some(other.to_string()),
    })
}

fn string_array_field(value: &Value, field: &str) -> Vec<String> {
    value
        .get(field)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(|value| match value {
                    Value::String(value) => Some(value.clone()),
                    Value::Null => None,
                    other => Some(other.to_string()),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn print_audit_decision_readback(readback: &AuditDecisionReadback) {
    print!("{}", format_audit_decision_readback(readback));
}

fn print_audit_task_readback(readback: &AuditTaskReadback) {
    print!("{}", format_audit_task_readback(readback));
}

fn print_audit_workspace_readback(readback: &AuditWorkspaceReadback) {
    print!("{}", format_audit_workspace_readback(readback));
}

fn format_audit_decision_readback(readback: &AuditDecisionReadback) -> String {
    let summary = &readback.summary;
    let mut output = String::new();

    push_readback_line(&mut output, &style_info("Audit decision summary"));
    push_readback_field(
        &mut output,
        "decision_id:",
        &summary
            .decision_id
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "-".to_owned()),
    );
    push_readback_field(
        &mut output,
        "proposed_action_id:",
        &summary
            .proposed_action_id
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "-".to_owned()),
    );
    push_readback_field(
        &mut output,
        "workspace_id:",
        &summary
            .workspace_id
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "-".to_owned()),
    );
    push_readback_field(
        &mut output,
        "task_id:",
        &summary
            .task_id
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "-".to_owned()),
    );
    push_audit_summary_fields(&mut output, summary);
    push_readback_field(
        &mut output,
        "decision_status:",
        readback.decision_status.as_deref().unwrap_or("-"),
    );
    push_readback_field(
        &mut output,
        "explicit_reason:",
        readback.explicit_reason.as_deref().unwrap_or("-"),
    );
    push_readback_field(
        &mut output,
        "action_type:",
        readback.action_type.as_deref().unwrap_or("-"),
    );
    push_readback_field(
        &mut output,
        "memory_write_kind:",
        readback.memory_write_kind.as_deref().unwrap_or("-"),
    );
    push_readback_field(
        &mut output,
        "memory_target_type:",
        readback.memory_target_type.as_deref().unwrap_or("-"),
    );
    push_readback_field(
        &mut output,
        "memory_target_id:",
        readback.memory_target_id.as_deref().unwrap_or("-"),
    );
    push_readback_field(
        &mut output,
        "memory_target_attribute:",
        readback.memory_target_attribute.as_deref().unwrap_or("-"),
    );
    push_readback_field(
        &mut output,
        "memory_target_value:",
        &format_optional_json(&readback.memory_target_value),
    );
    push_readback_field(
        &mut output,
        "memory_target_fact_id:",
        readback.memory_target_fact_id.as_deref().unwrap_or("-"),
    );
    push_readback_field(
        &mut output,
        "memory_related_fact_id:",
        readback.memory_related_fact_id.as_deref().unwrap_or("-"),
    );
    push_readback_field(
        &mut output,
        "memory_failure_insight_id:",
        readback.memory_failure_insight_id.as_deref().unwrap_or("-"),
    );
    push_readback_field(
        &mut output,
        "memory_provenance_source_id:",
        readback
            .memory_provenance_source_id
            .as_deref()
            .unwrap_or("-"),
    );
    push_readback_field(
        &mut output,
        "memory_provenance_source_label:",
        readback
            .memory_provenance_source_label
            .as_deref()
            .unwrap_or("-"),
    );
    push_readback_field(
        &mut output,
        "memory_provenance_source_kind:",
        readback
            .memory_provenance_source_kind
            .as_deref()
            .unwrap_or("-"),
    );
    push_readback_field(
        &mut output,
        "memory_provenance_evidence:",
        readback
            .memory_provenance_evidence
            .as_deref()
            .unwrap_or("-"),
    );
    push_readback_field(
        &mut output,
        "memory_confidence:",
        &readback
            .memory_confidence
            .map(|confidence| confidence.to_string())
            .unwrap_or_else(|| "-".to_owned()),
    );
    push_readback_field(
        &mut output,
        "memory_actor:",
        readback.memory_actor.as_deref().unwrap_or("-"),
    );
    push_readback_field(
        &mut output,
        "memory_reason_for_remembering:",
        readback
            .memory_reason_for_remembering
            .as_deref()
            .unwrap_or("-"),
    );
    push_readback_field(
        &mut output,
        "memory_proposed_at:",
        readback.memory_proposed_at.as_deref().unwrap_or("-"),
    );
    push_readback_field(
        &mut output,
        "memory_invalidation_note:",
        readback.memory_invalidation_note.as_deref().unwrap_or("-"),
    );
    push_readback_field(
        &mut output,
        "memory_decision_id:",
        readback.memory_decision_id.as_deref().unwrap_or("-"),
    );
    push_readback_field(
        &mut output,
        "memory_audit_event_id:",
        readback.memory_audit_event_id.as_deref().unwrap_or("-"),
    );
    push_readback_field(
        &mut output,
        "memory_persistence_readback_hint:",
        readback
            .memory_persistence_readback_hint
            .as_deref()
            .unwrap_or("-"),
    );
    push_readback_field(
        &mut output,
        "memory_supersession_hint:",
        readback.memory_supersession_hint.as_deref().unwrap_or("-"),
    );
    push_readback_field(
        &mut output,
        "risk_level:",
        readback.risk_level.as_deref().unwrap_or("-"),
    );
    push_readback_field(
        &mut output,
        "matched_policy_or_fallback_rule:",
        readback
            .matched_policy_or_fallback_rule
            .as_deref()
            .unwrap_or("-"),
    );
    push_readback_field(
        &mut output,
        "required_permission:",
        readback.required_permission.as_deref().unwrap_or("-"),
    );
    push_readback_field(
        &mut output,
        "block_reason_category:",
        readback.block_reason_category.as_deref().unwrap_or("-"),
    );
    push_readback_field(
        &mut output,
        "timestamp:",
        readback.timestamp.as_deref().unwrap_or("-"),
    );
    push_readback_field(
        &mut output,
        "suggested_next_action:",
        readback.suggested_next_action.as_deref().unwrap_or("-"),
    );
    push_readback_field(
        &mut output,
        "policies_applied:",
        &format_policies(&readback.policies_applied),
    );
    push_audit_summary_flags(&mut output, summary);
    push_readback_line(&mut output, &style_dim(readback.warning));

    output
}

fn format_audit_task_readback(readback: &AuditTaskReadback) -> String {
    let summary = &readback.summary;
    let mut output = String::new();

    push_readback_line(&mut output, &style_info("Audit task summary"));
    push_readback_field(
        &mut output,
        "task_id:",
        &summary
            .task_id
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "-".to_owned()),
    );
    push_readback_field(
        &mut output,
        "workspace_id:",
        &summary
            .workspace_id
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "-".to_owned()),
    );
    push_readback_field(
        &mut output,
        "proposed_action_id:",
        &summary
            .proposed_action_id
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "-".to_owned()),
    );
    push_readback_field(
        &mut output,
        "decision_id:",
        &summary
            .decision_id
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "-".to_owned()),
    );
    push_audit_summary_fields(&mut output, summary);
    push_audit_summary_flags(&mut output, summary);
    push_readback_line(&mut output, &style_dim(readback.warning));

    output
}

fn format_audit_workspace_readback(readback: &AuditWorkspaceReadback) -> String {
    let summary = &readback.summary;
    let mut output = String::new();

    push_readback_line(&mut output, &style_info("Audit workspace summary"));
    push_readback_field(
        &mut output,
        "workspace_id:",
        &summary
            .workspace_id
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "-".to_owned()),
    );
    push_readback_field(
        &mut output,
        "task_id:",
        &summary
            .task_id
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "-".to_owned()),
    );
    push_readback_field(
        &mut output,
        "proposed_action_id:",
        &summary
            .proposed_action_id
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "-".to_owned()),
    );
    push_readback_field(
        &mut output,
        "decision_id:",
        &summary
            .decision_id
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "-".to_owned()),
    );
    push_audit_summary_fields(&mut output, summary);
    push_audit_summary_flags(&mut output, summary);
    push_readback_line(&mut output, &style_dim(readback.warning));

    output
}

fn push_audit_summary_fields(output: &mut String, summary: &AuditTraceSummary) {
    push_readback_field(output, "event_count:", &summary.event_count.to_string());
    push_readback_field(
        output,
        "first_event_id:",
        &summary
            .first_event_id
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "-".to_owned()),
    );
    push_readback_field(
        output,
        "last_event_id:",
        &summary
            .last_event_id
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "-".to_owned()),
    );
    push_readback_field(
        output,
        "first_event_at:",
        &summary
            .first_event_at
            .map(|timestamp| timestamp.to_rfc3339())
            .unwrap_or_else(|| "-".to_owned()),
    );
    push_readback_field(
        output,
        "last_event_at:",
        &summary
            .last_event_at
            .map(|timestamp| timestamp.to_rfc3339())
            .unwrap_or_else(|| "-".to_owned()),
    );
}

fn push_audit_summary_flags(output: &mut String, summary: &AuditTraceSummary) {
    push_readback_field(
        output,
        "has_action_proposed:",
        &summary.has_action_proposed.to_string(),
    );
    push_readback_field(
        output,
        "has_decision_created:",
        &summary.has_decision_created.to_string(),
    );
    push_readback_field(
        output,
        "has_human_approval_request:",
        &summary.has_human_approval_request.to_string(),
    );
    push_readback_field(
        output,
        "has_human_outcome:",
        &summary.has_human_outcome.to_string(),
    );
    push_readback_field(
        output,
        "has_execution_event:",
        &summary.has_execution_event.to_string(),
    );
}

fn push_readback_field(output: &mut String, label: &str, value: &str) {
    push_readback_line(output, &format!("{} {}", style_dim(label), value));
}

fn push_readback_line(output: &mut String, line: &str) {
    output.push_str(line);
    output.push('\n');
}

fn format_policies(policies: &[String]) -> String {
    if policies.is_empty() {
        "-".to_owned()
    } else {
        policies.join(", ")
    }
}

async fn get_json<T: for<'de> Deserialize<'de>>(
    response: reqwest::Response,
) -> Result<T, Box<dyn Error>> {
    let status = response.status();
    let text = response.text().await?;
    if !status.is_success() {
        return Err(format!("API returned {status}: {text}").into());
    }
    Ok(serde_json::from_str(&text)?)
}

fn to_api_string<T: Serialize>(value: &T) -> Result<String, Box<dyn Error>> {
    match serde_json::to_value(value)? {
        Value::String(value) => Ok(value),
        other => Ok(other.to_string()),
    }
}

fn normalize_action_type(action_type: &str) -> Value {
    match action_type {
        "read_memory"
        | "read_tasks"
        | "read_proposed_actions"
        | "read_pending_actions"
        | "read_decisions"
        | "read_audit"
        | "read_status"
        | "system_check"
        | "write_memory"
        | "create_memory_fact"
        | "link_memory_fact"
        | "invalidate_memory_fact"
        | "create_failure_insight_memory"
        | "read_document"
        | "write_document"
        | "propose_tool_use"
        | "simulate_email"
        | "manage_task" => json!(action_type),
        custom => json!({ "custom": custom }),
    }
}

fn normalize_permissions(permissions: Vec<String>) -> Vec<String> {
    permissions
        .into_iter()
        .map(|permission| permission.trim().to_ascii_lowercase())
        .collect()
}

fn default_payload(args: &ProposeActionArgs) -> Value {
    if args.action_type == "simulate_email" {
        json!({
            "to": "client@example.com",
            "subject": "Simulation alpha ARPAGONA",
            "body": "Préparer un brouillon sans l’envoyer"
        })
    } else if is_memory_write_action_type(&args.action_type) {
        let target_value = memory_target_value(args);
        json!({
            "memory_write_intent": {
                "kind": args.action_type,
                "target": {
                    "entity_type": args.memory_target_type,
                    "entity_id": args.memory_target_id,
                    "attribute": args.memory_target_attribute,
                    "value": target_value,
                    "fact_id": args.memory_fact_id,
                    "related_fact_id": args.memory_related_fact_id,
                    "failure_insight_id": args.memory_failure_insight_id
                },
                "provenance": {
                    "source_id": args.memory_source_id,
                    "source_label": args.memory_source_label,
                    "source_kind": args.memory_source_kind,
                    "evidence": args.memory_evidence
                },
                "confidence": args.memory_confidence,
                "actor": args.proposed_by,
                "reason_for_remembering": args.rationale,
                "proposed_at": null,
                "decision_id": null,
                "audit_event_id": null,
                "invalidation_note": args.memory_invalidation_note
            }
        })
    } else {
        json!({})
    }
}

fn memory_target_value(args: &ProposeActionArgs) -> Value {
    args.memory_value
        .as_deref()
        .map(parse_memory_value)
        .unwrap_or_else(|| json!(args.rationale.clone()))
}

fn parse_memory_value(value: &str) -> Value {
    serde_json::from_str(value).unwrap_or_else(|_| json!(value))
}

fn parse_chat_line(line: &str) -> ChatLine {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return ChatLine::Empty;
    }

    if !trimmed.starts_with('/') {
        return ChatLine::Prompt(trimmed.to_owned());
    }

    let mut parts = trimmed.split_whitespace();
    match parts.next().unwrap_or_default() {
        "/help" => ChatLine::Help,
        "/quit" | "/exit" => ChatLine::Quit,
        "/status" => ChatLine::Status,
        "/audit" => ChatLine::Audit,
        "/tasks" => ChatLine::Tasks,
        "/actions" => ChatLine::Actions,
        "/evaluate" => match parts.next() {
            Some(action_id) => ChatLine::Evaluate(action_id.to_owned()),
            None => ChatLine::UnknownCommand("/evaluate requires an action id".to_owned()),
        },
        "/provider" => match parts.next() {
            Some(provider) => ChatLine::Provider(provider.to_owned()),
            None => ChatLine::UnknownCommand("/provider requires mock or openai".to_owned()),
        },
        command => ChatLine::UnknownCommand(command.to_owned()),
    }
}

fn format_provider_error(provider: &str, error: &str) -> String {
    let lower = error.to_ascii_lowercase();
    if provider == "openai" && (lower.contains("openai_api_key") || lower.contains("api key")) {
        return style_error(
            "OpenAI provider is not configured. Set OPENAI_API_KEY, then run: arpagona auth openai",
        );
    }
    style_error(&format!("Agent proposal failed: {error}"))
}

fn mask_openai_key(key: &str) -> String {
    let trimmed = key.trim();
    let char_count = trimmed.chars().count();
    if char_count <= 8 {
        return "***".to_owned();
    }

    let prefix: String = trimmed.chars().take(3).collect();
    let suffix: String = trimmed
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("{prefix}...{suffix}")
}

fn style_status(status: &str) -> String {
    match status {
        "approved" => style_text(status, TermColor::Green),
        "needs_human_approval" => style_text(status, TermColor::Yellow),
        "blocked" | "failed" | "rejected" => style_text(status, TermColor::Red),
        "pending_decision" => style_text(status, TermColor::Cyan),
        _ => style_text(status, TermColor::White),
    }
}

fn style_risk(risk: &str) -> String {
    match risk {
        "informational" | "low" => style_text(risk, TermColor::Green),
        "medium" => style_text(risk, TermColor::Yellow),
        "high" | "critical" => style_text(risk, TermColor::Red),
        _ => style_text(risk, TermColor::White),
    }
}

fn style_success(text: &str) -> String {
    style_text(text, TermColor::Green)
}

fn style_warning(text: &str) -> String {
    style_text(text, TermColor::Yellow)
}

fn style_error(text: &str) -> String {
    style_text(text, TermColor::Red)
}

fn style_info(text: &str) -> String {
    style_text(text, TermColor::Cyan)
}

fn style_brand(text: &str) -> String {
    format!("\x1b[38;5;151m{text}{ANSI_RESET}")
}

fn style_command(text: &str) -> String {
    format!(
        "{ANSI_BOLD}{}{ANSI_RESET}",
        style_text(text, TermColor::Magenta)
    )
}

fn style_prompt(text: &str) -> String {
    format!(
        "{ANSI_BOLD}{}{ANSI_RESET}",
        style_text(text, TermColor::Cyan)
    )
}

fn style_dim(text: &str) -> String {
    format!(
        "{ANSI_DIM}{}{ANSI_RESET}",
        style_text(text, TermColor::Gray)
    )
}

fn style_text(text: &str, color: TermColor) -> String {
    let code = match color {
        TermColor::Red => "31",
        TermColor::Green => "32",
        TermColor::Yellow => "33",
        TermColor::Blue => "34",
        TermColor::Magenta => "35",
        TermColor::Cyan => "36",
        TermColor::White => "37",
        TermColor::Gray => "90",
    };
    format!("\x1b[{code}m{text}{ANSI_RESET}")
}

fn rainbow_text(text: &str) -> String {
    let colors = [
        TermColor::Magenta,
        TermColor::Blue,
        TermColor::Cyan,
        TermColor::Green,
        TermColor::Yellow,
        TermColor::Red,
    ];
    let mut color_index = 0usize;
    let mut output = String::new();
    for character in text.chars() {
        if character.is_whitespace() {
            output.push(character);
            continue;
        }
        output.push_str(&style_text(
            &character.to_string(),
            colors[color_index % colors.len()],
        ));
        color_index += 1;
    }
    output
}

/// Run the General Cognitive Work Loop V0.
fn cognitive_run(args: CognitiveRunArgs) -> Result<(), Box<dyn Error>> {
    let domain = match args.domain.as_deref() {
        Some("general") | None => ObjectiveDomain::General,
        Some("business") => ObjectiveDomain::Business,
        Some("research") => ObjectiveDomain::Research,
        Some("teaching") => ObjectiveDomain::Teaching,
        Some("engineering") => ObjectiveDomain::Engineering,
        Some("administration") => ObjectiveDomain::Administration,
        Some("personal_productivity") | Some("personal-productivity") => {
            ObjectiveDomain::PersonalProductivity
        }
        Some("coding") => ObjectiveDomain::Coding,
        Some("unknown") => ObjectiveDomain::Unknown,
        Some(other) => {
            return Err(format!("Unknown domain '{}'. Valid values: general, business, research, teaching, engineering, administration, personal_productivity, coding, unknown", other).into());
        }
    };

    let result = arpagona_agent_core::cognitive_work::run_cognitive_work_cycle(
        &args.objective,
        Some(domain),
        args.context.as_deref(),
    );

    if args.json {
        let mut output = serde_json::to_value(&result)?;
        if let Some(obj) = output.as_object_mut() {
            // When --assess, inject failure_insight_candidates into working_memory
            if args.assess {
                if let Some(wm) = obj
                    .get_mut("working_memory")
                    .and_then(|v| v.as_object_mut())
                {
                    let fic = arpagona_agent_core::observation::FailureInsightCandidate::from_improvement_candidates(
                        &result.improvement_candidates,
                    );
                    wm.insert(
                        "failure_insight_candidates".to_owned(),
                        serde_json::to_value(&fic)?,
                    );
                }
            }
            obj.insert("assessed".to_owned(), serde_json::Value::Bool(args.assess));
        }
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        cognitive_print_readback(&result, args.assess);
    }

    Ok(())
}

/// Print a human-readable cognitive cycle result.
fn cognitive_print_readback(result: &CognitiveCycleResult, assess: bool) {
    println!(
        "{}\n",
        style_brand("=== General Cognitive Work Loop V0 — Readback ===")
    );
    println!("{}", style_info("Objective"));
    println!("  id:           {}", result.objective.id);
    println!("  title:        {}", result.objective.title);
    println!("  domain:       {:?}", result.objective.domain);
    println!("  status:       {:?}", result.objective.status);
    println!();

    println!("{}", style_info("Working Memory"));
    println!(
        "  context_items:   {}",
        result.working_memory.context_items.len()
    );
    for item in &result.working_memory.context_items {
        println!(
            "    - {}: {} (source: {})",
            item.key, item.value, item.source
        );
    }
    println!(
        "  assumptions:     {}",
        result.working_memory.assumptions.len()
    );
    for assumption in &result.working_memory.assumptions {
        println!(
            "    - {} (confidence: {:.1})",
            assumption.description, assumption.confidence
        );
    }
    println!(
        "  constraints:     {}",
        result.working_memory.constraints.len()
    );
    for constraint in &result.working_memory.constraints {
        println!(
            "    - {} [kind: {}]",
            constraint.description, constraint.kind
        );
    }
    println!(
        "  missing_context: {}",
        result.working_memory.missing_context.len()
    );
    for mc in &result.working_memory.missing_context {
        println!("    - {}", mc.description);
        println!("      why: {}", mc.why_needed);
    }
    println!();

    println!("{}", style_info("Cognitive Plan"));
    for step in &result.plan.steps {
        println!("  {}. {}", step.order, step.description);
    }
    println!("  rationale: {}", result.plan.rationale);
    println!();

    println!("{}", style_info("Required Observations"));
    for obs in &result.required_observations {
        println!("  - {} ({})", obs.description, obs.why_needed);
    }
    println!();

    println!("{}", style_info("Proposed Next Action"));
    println!("  kind:       {:?}", result.proposed_next_action.kind);
    println!("  description: {}", result.proposed_next_action.description);
    println!("  rationale:  {}", result.proposed_next_action.rationale);
    println!(
        "  non_authorizing: {}",
        result.proposed_next_action.non_authorizing
    );
    println!();

    println!("{}", style_info("Improvement Candidates"));
    for candidate in &result.improvement_candidates {
        println!(
            "  - [{}] {} — {}",
            candidate.id, candidate.description, candidate.rationale
        );
    }
    println!();

    if assess {
        println!(
            "{}",
            style_info("Failure Insight Candidates (via --assess bridge)")
        );
        let fic =
            arpagona_agent_core::observation::FailureInsightCandidate::from_improvement_candidates(
                &result.improvement_candidates,
            );
        for candidate in &fic {
            println!(
                "  - [{:?}] {} — {}",
                candidate.kind, candidate.summary, candidate.reason
            );
        }
        println!();
    }

    println!("{}", style_dim(&format!("\u{26a0}  {}", result.warning)));
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_parses_action_propose_defaults() {
        let cli = Cli::parse_from(["arpagona", "action", "propose"]);
        match cli.command {
            Command::Action(ActionCommand {
                command: ActionSubcommand::Propose(args),
            }) => {
                assert_eq!(args.action_type, "simulate_email");
                assert_eq!(args.permissions, vec!["simulate_email"]);
                assert_eq!(args.task_id, "task-1");
                assert_eq!(args.target, "client@example.com");
                assert_eq!(args.rationale, "Préparer un brouillon sans l’envoyer");
                assert!(matches!(args.risk, RiskArg::Medium));
            }
            _ => panic!("expected action propose"),
        }
    }

    #[test]
    fn cli_parses_memory_action_proposal_controls() {
        let cli = Cli::parse_from([
            "arpagona",
            "action",
            "propose",
            "--type",
            "create_memory_fact",
            "--permission",
            "write_memory",
            "--memory-target-type",
            "person",
            "--memory-target-id",
            "client-1",
            "--memory-target-attribute",
            "preference",
            "--memory-value",
            "{\"language\":\"fr\"}",
            "--memory-fact-id",
            "fact-client-1",
            "--memory-source-id",
            "source-note-1",
            "--memory-source-label",
            "operator note",
            "--memory-source-kind",
            "local_note",
            "--memory-evidence",
            "Client explicitly asked for French.",
            "--memory-confidence",
            "0.77",
            "--memory-invalidation-note",
            "Supersede if preference changes.",
            "--proposed-by",
            "agent-alpha",
            "--rationale",
            "Remember client language preference.",
        ]);
        match cli.command {
            Command::Action(ActionCommand {
                command: ActionSubcommand::Propose(args),
            }) => {
                assert_eq!(args.action_type, "create_memory_fact");
                assert_eq!(args.permissions, vec!["write_memory"]);
                assert_eq!(args.memory_target_type, "person");
                assert_eq!(args.memory_target_id, "client-1");
                assert_eq!(args.memory_target_attribute, "preference");
                assert_eq!(args.memory_value.as_deref(), Some("{\"language\":\"fr\"}"));
                assert_eq!(args.memory_fact_id.as_deref(), Some("fact-client-1"));
                assert_eq!(args.memory_source_id.as_deref(), Some("source-note-1"));
                assert_eq!(args.memory_source_label, "operator note");
                assert_eq!(args.memory_source_kind, "local_note");
                assert_eq!(args.memory_evidence, "Client explicitly asked for French.");
                assert_eq!(args.memory_confidence, 0.77);
                assert_eq!(
                    args.memory_invalidation_note,
                    "Supersede if preference changes."
                );

                let payload = default_payload(&args);
                assert_eq!(
                    payload["memory_write_intent"]["target"]["value"],
                    json!({"language": "fr"})
                );
                assert_eq!(
                    payload["memory_write_intent"]["target"]["fact_id"],
                    "fact-client-1"
                );
                assert_eq!(
                    payload["memory_write_intent"]["provenance"]["source_id"],
                    "source-note-1"
                );
                assert_eq!(payload["memory_write_intent"]["actor"], "agent-alpha");
                assert_eq!(
                    payload["memory_write_intent"]["reason_for_remembering"],
                    "Remember client language preference."
                );
            }
            _ => panic!("expected action propose"),
        }
    }

    #[test]
    fn memory_payload_falls_back_to_plain_text_value() {
        let cli = Cli::parse_from([
            "arpagona",
            "action",
            "propose",
            "--type",
            "create_failure_insight_memory",
            "--permission",
            "write_memory",
            "--memory-value",
            "plain text observation",
            "--memory-failure-insight-id",
            "insight-1",
        ]);
        match cli.command {
            Command::Action(ActionCommand {
                command: ActionSubcommand::Propose(args),
            }) => {
                let payload = default_payload(&args);
                assert_eq!(
                    payload["memory_write_intent"]["target"]["value"],
                    "plain text observation"
                );
                assert_eq!(
                    payload["memory_write_intent"]["target"]["failure_insight_id"],
                    "insight-1"
                );
                assert_eq!(
                    payload["memory_write_intent"]["reason_for_remembering"],
                    DEFAULT_RATIONALE
                );
            }
            _ => panic!("expected action propose"),
        }
    }

    #[test]
    fn cli_parses_memory_demo_failure_insight_json() {
        let cli = Cli::parse_from(["arpagona", "memory", "demo", "failure-insight", "--json"]);
        match cli.command {
            Command::Memory(MemoryCommand {
                command:
                    MemorySubcommand::Demo(MemoryDemoCommand {
                        command: MemoryDemoSubcommand::FailureInsight(args),
                    }),
            }) => {
                assert!(args.json);
                assert!(args.inspect_id.is_none());
            }
            _ => panic!("expected memory demo failure-insight"),
        }
    }

    #[test]
    fn cli_parses_memory_demo_failure_insight_inspect_id() {
        let cli = Cli::parse_from([
            "arpagona",
            "memory",
            "demo",
            "failure-insight",
            "--json",
            "--inspect-id",
            "insight-demo-governed-learning-loop",
        ]);
        match cli.command {
            Command::Memory(MemoryCommand {
                command:
                    MemorySubcommand::Demo(MemoryDemoCommand {
                        command: MemoryDemoSubcommand::FailureInsight(args),
                    }),
            }) => {
                assert!(args.json);
                assert_eq!(
                    args.inspect_id.as_deref(),
                    Some("insight-demo-governed-learning-loop")
                );
            }
            _ => panic!("expected memory demo failure-insight inspect id"),
        }
    }

    #[test]
    fn cli_parses_memory_demo_failure_insight_with_description() {
        let cli = Cli::parse_from([
            "arpagona",
            "memory",
            "demo",
            "failure-insight",
            "--description",
            "the proposal lacked a confidence field",
        ]);
        match cli.command {
            Command::Memory(MemoryCommand {
                command:
                    MemorySubcommand::Demo(MemoryDemoCommand {
                        command: MemoryDemoSubcommand::FailureInsight(args),
                    }),
            }) => {
                assert_eq!(
                    args.description.as_deref(),
                    Some("the proposal lacked a confidence field")
                );
                assert!(!args.json);
            }
            _ => panic!("expected memory demo failure-insight with description"),
        }
    }

    #[test]
    fn cli_parses_memory_demo_snapshot_list() {
        let cli = Cli::parse_from(["arpagona", "memory", "demo", "snapshot-list"]);
        match cli.command {
            Command::Memory(MemoryCommand {
                command:
                    MemorySubcommand::Demo(MemoryDemoCommand {
                        command: MemoryDemoSubcommand::SnapshotList(args),
                    }),
            }) => {
                assert!(!args.json);
                assert_eq!(args.snapshot_dir, DEFAULT_SNAPSHOT_DIR);
            }
            _ => panic!("expected memory demo snapshot-list"),
        }
    }

    #[test]
    fn cli_parses_memory_demo_snapshot_list_json() {
        let cli = Cli::parse_from(["arpagona", "memory", "demo", "snapshot-list", "--json"]);
        match cli.command {
            Command::Memory(MemoryCommand {
                command:
                    MemorySubcommand::Demo(MemoryDemoCommand {
                        command: MemoryDemoSubcommand::SnapshotList(args),
                    }),
            }) => {
                assert!(args.json);
                assert_eq!(args.snapshot_dir, DEFAULT_SNAPSHOT_DIR);
            }
            _ => panic!("expected memory demo snapshot-list --json"),
        }
    }

    #[test]
    fn cli_parses_memory_demo_snapshot_list_with_custom_dir() {
        let cli = Cli::parse_from([
            "arpagona",
            "memory",
            "demo",
            "snapshot-list",
            "--snapshot-dir",
            "/tmp/snapshots",
        ]);
        match cli.command {
            Command::Memory(MemoryCommand {
                command:
                    MemorySubcommand::Demo(MemoryDemoCommand {
                        command: MemoryDemoSubcommand::SnapshotList(args),
                    }),
            }) => {
                assert!(!args.json);
                assert_eq!(args.snapshot_dir, "/tmp/snapshots");
            }
            _ => panic!("expected memory demo snapshot-list --snapshot-dir"),
        }
    }

    #[test]
    fn cli_parses_memory_demo_snapshot_list_json_with_custom_dir() {
        let cli = Cli::parse_from([
            "arpagona",
            "memory",
            "demo",
            "snapshot-list",
            "--json",
            "--snapshot-dir",
            "/tmp/snapshots",
        ]);
        match cli.command {
            Command::Memory(MemoryCommand {
                command:
                    MemorySubcommand::Demo(MemoryDemoCommand {
                        command: MemoryDemoSubcommand::SnapshotList(args),
                    }),
            }) => {
                assert!(args.json);
                assert_eq!(args.snapshot_dir, "/tmp/snapshots");
            }
            _ => panic!("expected memory demo snapshot-list --json --snapshot-dir"),
        }
    }

    #[tokio::test]
    async fn memory_demo_failure_insight_readback_proves_governed_loop_without_authorizing() {
        let readback = memory_demo_failure_insight_readback(None, None)
            .await
            .expect("demo readback succeeds");

        assert_eq!(readback.signal.signal_type, "runtime_observation");
        assert_eq!(readback.memory_write_kind, "create_failure_insight_memory");
        assert_eq!(readback.decision_status, "approved");
        assert!(readback.readback_found);
        assert_eq!(
            readback.persisted_failure_insight_id.as_deref(),
            Some("insight-demo-governed-learning-loop")
        );
        assert_eq!(readback.exact_local_command, FAILURE_INSIGHT_DEMO_COMMAND);
        assert!(readback.inspected_failure_insight.is_none());
        assert!(readback.repeatable_demo_recipe.contains(
            &"verify readback_found is true and readback_audit_event_count is at least 1"
        ));
        assert!(readback.repeatable_demo_recipe.contains(
            &"optionally rerun with --inspect-id insight-demo-governed-learning-loop to inspect the persisted artifact by id"
        ));
        assert_eq!(readback.readback_audit_event_count, 1);
        assert_eq!(readback.readback_relation_count, 2);
        assert!(readback.readback_warning.contains("Readback only"));
        assert!(readback.warning.contains("Local demo only"));

        let formatted = format_memory_demo_failure_insight_readback(&readback);
        assert!(formatted.contains("FailureInsight memory demo"));
        assert!(formatted.contains("create_failure_insight_memory"));
        assert!(formatted.contains(FAILURE_INSIGHT_DEMO_COMMAND));
        assert!(formatted.contains("repeatable_demo_recipe"));
        assert!(formatted.contains("Readback only"));
    }

    #[tokio::test]
    async fn memory_demo_description_propagates_through_governed_loop_signal_to_readback() {
        // End-to-end proof that operator-supplied --description text flows through the full
        // governed path: signal -> proposal -> DecisionGate -> audit -> persistence -> readback.
        // Asserts the custom description appears in both the signal summary and the inspected
        // FailureInsight fields. This is the functional-alpha chain's description propagation proof.
        let custom_desc = "custom test governed loop description";
        let readback = memory_demo_failure_insight_readback(
            Some("insight-demo-governed-learning-loop".to_owned()),
            Some(custom_desc.to_owned()),
        )
        .await
        .expect("demo readback with description succeeds");

        // 1. Signal summary reflects the custom description
        assert!(
            readback.signal.summary.contains(custom_desc),
            "signal summary should contain custom description: expected to contain '{}', got '{}'",
            custom_desc,
            readback.signal.summary
        );

        // 2. The full governed path still works with description (decision approved, persisted)
        assert_eq!(readback.memory_write_kind, "create_failure_insight_memory");
        assert_eq!(readback.decision_status, "approved");
        assert!(readback.readback_found, "readback_found should be true");
        assert_eq!(
            readback.persisted_failure_insight_id.as_deref(),
            Some("insight-demo-governed-learning-loop")
        );
        assert_eq!(readback.readback_audit_event_count, 1);
        assert_eq!(readback.readback_relation_count, 2);

        // 3. Inspected FailureInsight summary also contains the custom description
        let inspection = readback
            .inspected_failure_insight
            .as_ref()
            .expect("requested inspection is included when inspect_id is provided");
        assert!(inspection.found);
        assert_eq!(
            inspection.inspected_failure_insight_id.as_deref(),
            Some("insight-demo-governed-learning-loop")
        );
        assert!(
            inspection
                .summary
                .as_deref()
                .unwrap_or("")
                .contains(custom_desc),
            "inspected FailureInsight summary should contain custom description: got '{:?}'",
            inspection.summary
        );

        // 4. Formatted text readback also contains the description
        let formatted = format_memory_demo_failure_insight_readback(&readback);
        assert!(formatted.contains(custom_desc));

        // 5. Warning and evidence-only guards remain present despite custom description
        assert!(readback.readback_warning.contains("Readback only"));
        assert!(readback.warning.contains("Local demo only"));
        assert!(inspection.warning.contains("Readback only"));
    }

    #[tokio::test]
    async fn memory_demo_failure_insight_inspect_id_proves_persisted_artifact_readback() {
        let readback = memory_demo_failure_insight_readback(
            Some("insight-demo-governed-learning-loop".to_owned()),
            None,
        )
        .await
        .expect("demo inspection readback succeeds");

        let inspection = readback
            .inspected_failure_insight
            .as_ref()
            .expect("requested inspection is included");
        assert!(inspection.found);
        assert_eq!(
            inspection.inspected_failure_insight_id.as_deref(),
            Some("insight-demo-governed-learning-loop")
        );
        assert_eq!(inspection.audit_event_count, 1);
        assert_eq!(inspection.relation_count, 2);
        assert!(inspection.summary.as_deref().unwrap_or_default().contains(
            "Governed FailureInsight learning loop needs repeatable local readback proof"
        ));
        assert!(inspection.warning.contains("Readback only"));

        let formatted = format_memory_demo_failure_insight_readback(&readback);
        assert!(formatted.contains("inspected_failure_insight_id"));
        assert!(formatted.contains("inspected_failure_insight_found"));
        assert!(formatted.contains(FAILURE_INSIGHT_DEMO_INSPECT_COMMAND));
    }

    #[tokio::test]
    async fn memory_demo_failure_insight_inspect_id_reports_missing_artifact_without_authorizing() {
        let readback =
            memory_demo_failure_insight_readback(Some("insight-demo-missing".to_owned()), None)
                .await
                .expect("demo missing inspection readback succeeds");

        let inspection = readback
            .inspected_failure_insight
            .as_ref()
            .expect("requested inspection is included");
        assert_eq!(
            inspection.requested_failure_insight_id,
            "insight-demo-missing"
        );
        assert!(!inspection.found);
        assert!(inspection.inspected_failure_insight_id.is_none());
        assert_eq!(inspection.audit_event_count, 0);
        assert_eq!(inspection.relation_count, 0);
        assert!(inspection.warning.contains("Readback only"));
    }

    #[test]
    fn cli_parses_agent_propose_defaults() {
        let cli = Cli::parse_from([
            "arpagona",
            "agent",
            "propose",
            "Prépare un brouillon de réponse client",
        ]);
        match cli.command {
            Command::Agent(AgentCommand {
                command: AgentSubcommand::Propose(args),
            }) => {
                assert_eq!(args.prompt, "Prépare un brouillon de réponse client");
                assert_eq!(args.provider, "openai");
                assert_eq!(args.task_id, "task-1");
                assert_eq!(args.workspace_id, "workspace-alpha");
            }
            _ => panic!("expected agent propose"),
        }
    }

    #[test]
    fn cli_parses_chat_defaults() {
        let cli = Cli::parse_from(["arpagona", "chat"]);
        match cli.command {
            Command::Chat(args) => {
                assert_eq!(args.provider, "mock");
                assert_eq!(args.task_id, "task-1");
                assert_eq!(args.workspace_id, "workspace-alpha");
                assert_eq!(args.permissions, vec!["simulate_email"]);
            }
            _ => panic!("expected chat"),
        }
    }

    #[test]
    fn cli_parses_chat_provider_openai() {
        let cli = Cli::parse_from(["arpagona", "chat", "--provider", "openai"]);
        match cli.command {
            Command::Chat(args) => assert_eq!(args.provider, "openai"),
            _ => panic!("expected chat"),
        }
    }

    #[test]
    fn cli_parses_auth_commands() {
        let status = Cli::parse_from(["arpagona", "auth", "status"]);
        assert!(matches!(
            status.command,
            Command::Auth(AuthCommand {
                command: AuthSubcommand::Status
            })
        ));

        let openai = Cli::parse_from(["arpagona", "auth", "openai"]);
        assert!(matches!(
            openai.command,
            Command::Auth(AuthCommand {
                command: AuthSubcommand::Openai
            })
        ));
    }

    #[test]
    fn cli_parses_action_evaluate_with_permission() {
        let cli = Cli::parse_from([
            "arpagona",
            "action",
            "evaluate",
            "action-1",
            "--permission",
            "simulate_email",
        ]);
        match cli.command {
            Command::Action(ActionCommand {
                command: ActionSubcommand::Evaluate(args),
            }) => {
                assert_eq!(args.proposed_action_id, "action-1");
                assert_eq!(args.permissions, vec!["simulate_email"]);
            }
            _ => panic!("expected action evaluate"),
        }
    }

    #[test]
    fn cli_parses_audit_decision_summary_command() {
        let cli = Cli::parse_from([
            "arpagona",
            "audit",
            "decision-summary",
            "decision-1",
            "--json",
        ]);
        match cli.command {
            Command::Audit(AuditCommand {
                command: AuditSubcommand::DecisionSummary(args),
            }) => {
                assert_eq!(args.decision_id, "decision-1");
                assert!(args.json);
            }
            _ => panic!("expected audit decision-summary"),
        }
    }

    #[test]
    fn cli_parses_audit_task_summary_command() {
        let cli = Cli::parse_from(["arpagona", "audit", "task-summary", "task-1", "--json"]);
        match cli.command {
            Command::Audit(AuditCommand {
                command: AuditSubcommand::TaskSummary(args),
            }) => {
                assert_eq!(args.task_id, "task-1");
                assert!(args.json);
            }
            _ => panic!("expected audit task-summary"),
        }
    }

    #[test]
    fn cli_parses_audit_workspace_summary_command() {
        let cli = Cli::parse_from([
            "arpagona",
            "audit",
            "workspace-summary",
            "workspace-1",
            "--json",
        ]);
        match cli.command {
            Command::Audit(AuditCommand {
                command: AuditSubcommand::WorkspaceSummary(args),
            }) => {
                assert_eq!(args.workspace_id, "workspace-1");
                assert!(args.json);
            }
            _ => panic!("expected audit workspace-summary"),
        }
    }

    #[test]
    fn audit_workspace_summary_filters_and_orders_events_without_authorizing() {
        use arpagona_agent_core::{AuditEventId, DecisionId, ProposedActionId};

        let events = vec![
            serde_json::from_value::<AuditEvent>(json!({
                "id": "audit-2",
                "event_type": "decision_created",
                "actor": "system",
                "workspace_id": "workspace-1",
                "task_id": "task-1",
                "proposed_action_id": "action-1",
                "decision_id": "decision-1",
                "payload": {},
                "created_at": "2026-01-01T00:05:00Z"
            }))
            .unwrap(),
            serde_json::from_value::<AuditEvent>(json!({
                "id": "audit-unrelated",
                "event_type": "execution_started",
                "actor": "system",
                "workspace_id": "workspace-2",
                "task_id": "task-2",
                "proposed_action_id": "action-2",
                "decision_id": "decision-2",
                "payload": {},
                "created_at": "2026-01-01T00:10:00Z"
            }))
            .unwrap(),
            serde_json::from_value::<AuditEvent>(json!({
                "id": "audit-1",
                "event_type": "action_proposed",
                "actor": "system",
                "workspace_id": "workspace-1",
                "task_id": "task-1",
                "proposed_action_id": "action-1",
                "decision_id": null,
                "payload": {},
                "created_at": "2026-01-01T00:00:00Z"
            }))
            .unwrap(),
        ];
        let first_at = events[2].created_at;
        let last_at = events[0].created_at;

        let readback = workspace_readback_from_audit_events(events, "workspace-1");
        let summary = &readback.summary;

        assert_eq!(summary.workspace_id, Some(WorkspaceId::new("workspace-1")));
        assert_eq!(summary.event_count, 2);
        assert_eq!(summary.first_event_id, Some(AuditEventId::new("audit-1")));
        assert_eq!(summary.last_event_id, Some(AuditEventId::new("audit-2")));
        assert_eq!(summary.first_event_at, Some(first_at));
        assert_eq!(summary.last_event_at, Some(last_at));
        assert_eq!(summary.task_id, Some(TaskId::new("task-1")));
        assert_eq!(
            summary.proposed_action_id,
            Some(ProposedActionId::new("action-1"))
        );
        assert_eq!(summary.decision_id, Some(DecisionId::new("decision-1")));
        assert!(summary.has_action_proposed);
        assert!(summary.has_decision_created);
        assert!(!summary.has_execution_event);

        let formatted = format_audit_workspace_readback(&readback);
        assert!(formatted.contains("Audit workspace summary"));
        assert!(formatted.contains("workspace_id:"));
        assert!(formatted.contains("workspace-1"));
        assert!(formatted.contains("event_count:"));
        assert!(formatted.contains("Readback only"));

        let json = serde_json::to_value(&readback).unwrap();
        assert_eq!(json["summary"]["workspace_id"], "workspace-1");
        assert_eq!(json["summary"]["event_count"], 2);
        assert!(json["warning"].as_str().unwrap().contains("Readback only"));
    }

    #[test]
    fn audit_workspace_summary_preserves_empty_workspace_scope() {
        let readback = workspace_readback_from_audit_events(vec![], "workspace-empty");
        let summary = &readback.summary;

        assert_eq!(
            summary.workspace_id,
            Some(WorkspaceId::new("workspace-empty"))
        );
        assert_eq!(summary.event_count, 0);
        assert_eq!(summary.first_event_id, None);
        assert_eq!(summary.last_event_id, None);
        assert_eq!(summary.first_event_at, None);
        assert_eq!(summary.last_event_at, None);
        assert!(!summary.has_decision_created);
        assert!(!summary.has_execution_event);
    }

    #[test]
    fn audit_task_summary_filters_and_orders_events_without_authorizing() {
        use arpagona_agent_core::{AuditEventId, DecisionId, ProposedActionId, WorkspaceId};

        let events = vec![
            serde_json::from_value::<AuditEvent>(json!({
                "id": "audit-2",
                "event_type": "decision_created",
                "actor": "system",
                "workspace_id": "workspace-1",
                "task_id": "task-1",
                "proposed_action_id": "action-1",
                "decision_id": "decision-1",
                "payload": {},
                "created_at": "2026-01-01T00:05:00Z"
            }))
            .unwrap(),
            serde_json::from_value::<AuditEvent>(json!({
                "id": "audit-unrelated",
                "event_type": "execution_started",
                "actor": "system",
                "workspace_id": "workspace-1",
                "task_id": "task-2",
                "proposed_action_id": "action-2",
                "decision_id": "decision-2",
                "payload": {},
                "created_at": "2026-01-01T00:10:00Z"
            }))
            .unwrap(),
            serde_json::from_value::<AuditEvent>(json!({
                "id": "audit-1",
                "event_type": "action_proposed",
                "actor": "system",
                "workspace_id": "workspace-1",
                "task_id": "task-1",
                "proposed_action_id": "action-1",
                "decision_id": null,
                "payload": {},
                "created_at": "2026-01-01T00:00:00Z"
            }))
            .unwrap(),
        ];
        let first_at = events[2].created_at;
        let last_at = events[0].created_at;

        let readback = task_readback_from_audit_events(events, "task-1");
        let summary = &readback.summary;

        assert_eq!(summary.task_id, Some(TaskId::new("task-1")));
        assert_eq!(summary.event_count, 2);
        assert_eq!(summary.first_event_id, Some(AuditEventId::new("audit-1")));
        assert_eq!(summary.last_event_id, Some(AuditEventId::new("audit-2")));
        assert_eq!(summary.first_event_at, Some(first_at));
        assert_eq!(summary.last_event_at, Some(last_at));
        assert_eq!(summary.workspace_id, Some(WorkspaceId::new("workspace-1")));
        assert_eq!(
            summary.proposed_action_id,
            Some(ProposedActionId::new("action-1"))
        );
        assert_eq!(summary.decision_id, Some(DecisionId::new("decision-1")));
        assert!(summary.has_action_proposed);
        assert!(summary.has_decision_created);
        assert!(!summary.has_execution_event);

        let formatted = format_audit_task_readback(&readback);
        assert!(formatted.contains("Audit task summary"));
        assert!(formatted.contains("task_id:"));
        assert!(formatted.contains("task-1"));
        assert!(formatted.contains("event_count:"));
        assert!(formatted.contains("Readback only"));

        let json = serde_json::to_value(&readback).unwrap();
        assert_eq!(json["summary"]["task_id"], "task-1");
        assert_eq!(json["summary"]["event_count"], 2);
        assert!(json["warning"].as_str().unwrap().contains("Readback only"));
    }

    #[test]
    fn audit_task_summary_preserves_empty_task_scope() {
        let readback = task_readback_from_audit_events(vec![], "task-empty");
        let summary = &readback.summary;

        assert_eq!(summary.task_id, Some(TaskId::new("task-empty")));
        assert_eq!(summary.event_count, 0);
        assert_eq!(summary.first_event_id, None);
        assert_eq!(summary.last_event_id, None);
        assert_eq!(summary.first_event_at, None);
        assert_eq!(summary.last_event_at, None);
        assert!(!summary.has_decision_created);
        assert!(!summary.has_execution_event);
    }

    #[test]
    fn audit_decision_summary_filters_and_orders_events_without_authorizing() {
        use arpagona_agent_core::{AuditEventId, ProposedActionId, TaskId, WorkspaceId};

        let events = vec![
            serde_json::from_value::<AuditEvent>(json!({
                "id": "audit-2",
                "event_type": "decision_created",
                "actor": "system",
                "workspace_id": "workspace-1",
                "task_id": "task-1",
                "proposed_action_id": "action-1",
                "decision_id": "decision-1",
                "payload": {
                    "causal_trace": {
                        "decision_status": "needs_human_approval",
                        "action_type": "create_memory_fact",
                        "risk_level": "high",
                        "policies_applied": ["policy-human-approval"],
                        "memory_write_intent": {
                            "kind": "create_memory_fact",
                            "target": {
                                "entity_type": "project",
                                "entity_id": "arpagona-agent-core",
                                "attribute": "operational_note",
                                "value": "governed memory proposals are visible",
                                "fact_id": "fact-audit-memory-1",
                                "related_fact_id": "fact-audit-memory-prior",
                                "failure_insight_id": null
                            },
                            "provenance": {
                                "source_id": "source-audit-memory",
                                "source_label": "focus loop",
                                "source_kind": "operational_report",
                                "evidence": "Issue #47 requires governed memory observability."
                            },
                            "confidence": 0.88,
                            "actor": "agent-alpha",
                            "reason_for_remembering": "Keep memory-write proposal context visible in audit readback.",
                            "proposed_at": "2026-05-21T10:00:00Z",
                            "decision_id": "decision-1",
                            "audit_event_id": "audit-2",
                            "invalidation_note": "Supersede when priority changes."
                        }
                    }
                },
                "created_at": "2026-01-01T00:05:00Z"
            }))
            .unwrap(),
            serde_json::from_value::<AuditEvent>(json!({
                "id": "audit-unrelated",
                "event_type": "execution_started",
                "actor": "system",
                "workspace_id": "workspace-1",
                "task_id": "task-2",
                "proposed_action_id": "action-2",
                "decision_id": "decision-2",
                "payload": {},
                "created_at": "2026-01-01T00:10:00Z"
            }))
            .unwrap(),
            serde_json::from_value::<AuditEvent>(json!({
                "id": "audit-1",
                "event_type": "action_proposed",
                "actor": "system",
                "workspace_id": "workspace-1",
                "task_id": "task-1",
                "proposed_action_id": "action-1",
                "decision_id": "decision-1",
                "payload": {},
                "created_at": "2026-01-01T00:00:00Z"
            }))
            .unwrap(),
        ];
        let first_at = events[2].created_at;
        let last_at = events[0].created_at;

        let readback = decision_readback_from_audit_events(events, "decision-1");
        let summary = &readback.summary;

        assert_eq!(summary.decision_id, Some(DecisionId::new("decision-1")));
        assert_eq!(summary.event_count, 2);
        assert_eq!(summary.first_event_id, Some(AuditEventId::new("audit-1")));
        assert_eq!(summary.last_event_id, Some(AuditEventId::new("audit-2")));
        assert_eq!(summary.first_event_at, Some(first_at));
        assert_eq!(summary.last_event_at, Some(last_at));
        assert_eq!(summary.workspace_id, Some(WorkspaceId::new("workspace-1")));
        assert_eq!(summary.task_id, Some(TaskId::new("task-1")));
        assert_eq!(
            summary.proposed_action_id,
            Some(ProposedActionId::new("action-1"))
        );
        assert!(summary.has_action_proposed);
        assert!(summary.has_decision_created);
        assert!(!summary.has_execution_event);
        assert_eq!(
            readback.decision_status.as_deref(),
            Some("needs_human_approval")
        );
        assert_eq!(readback.risk_level.as_deref(), Some("high"));
        assert_eq!(readback.policies_applied, vec!["policy-human-approval"]);
        assert_eq!(readback.action_type.as_deref(), Some("create_memory_fact"));
        assert_eq!(
            readback.memory_write_kind.as_deref(),
            Some("create_memory_fact")
        );
        assert_eq!(readback.memory_target_type.as_deref(), Some("project"));
        assert_eq!(
            readback.memory_target_value,
            Some(json!("governed memory proposals are visible"))
        );
        assert_eq!(
            readback.memory_target_id.as_deref(),
            Some("arpagona-agent-core")
        );
        assert_eq!(
            readback.memory_target_fact_id.as_deref(),
            Some("fact-audit-memory-1")
        );
        assert_eq!(
            readback.memory_related_fact_id.as_deref(),
            Some("fact-audit-memory-prior")
        );
        assert_eq!(
            readback.memory_provenance_source_id.as_deref(),
            Some("source-audit-memory")
        );
        assert_eq!(
            readback.memory_provenance_source_label.as_deref(),
            Some("focus loop")
        );
        assert_eq!(readback.memory_confidence, Some(0.88));
        assert_eq!(readback.memory_decision_id.as_deref(), Some("decision-1"));
        assert_eq!(readback.memory_audit_event_id.as_deref(), Some("audit-2"));
        assert!(readback
            .memory_persistence_readback_hint
            .as_deref()
            .unwrap()
            .contains("Not persistable yet"));
        assert!(readback
            .memory_supersession_hint
            .as_deref()
            .unwrap()
            .contains("Supersede when priority changes"));
        assert_eq!(
            readback.memory_reason_for_remembering.as_deref(),
            Some("Keep memory-write proposal context visible in audit readback.")
        );

        let formatted = format_audit_decision_readback(&readback);
        assert!(formatted.contains("decision_status:"));
        assert!(formatted.contains("needs_human_approval"));
        assert!(formatted.contains("risk_level:"));
        assert!(formatted.contains("high"));
        assert!(formatted.contains("policies_applied:"));
        assert!(formatted.contains("policy-human-approval"));
        assert!(formatted.contains("memory_write_kind:"));
        assert!(formatted.contains("create_memory_fact"));
        assert!(formatted.contains("memory_target_id:"));
        assert!(formatted.contains("arpagona-agent-core"));
        assert!(formatted.contains("memory_target_fact_id:"));
        assert!(formatted.contains("fact-audit-memory-1"));
        assert!(formatted.contains("memory_provenance_source_id:"));
        assert!(formatted.contains("source-audit-memory"));
        assert!(formatted.contains("memory_persistence_readback_hint:"));
        assert!(formatted.contains("memory_supersession_hint:"));
        assert!(formatted.contains("memory_reason_for_remembering:"));
        assert!(formatted.contains("Keep memory-write proposal context visible in audit readback."));
        assert!(formatted.contains("Readback only"));

        let json = serde_json::to_value(&readback).unwrap();
        assert_eq!(json["decision_status"], "needs_human_approval");
        assert_eq!(json["risk_level"], "high");
        assert_eq!(json["policies_applied"], json!(["policy-human-approval"]));
        assert_eq!(json["memory_write_kind"], "create_memory_fact");
        assert_eq!(json["memory_target_id"], "arpagona-agent-core");
        assert_eq!(json["memory_target_fact_id"], "fact-audit-memory-1");
        assert_eq!(json["memory_related_fact_id"], "fact-audit-memory-prior");
        assert_eq!(json["memory_provenance_source_id"], "source-audit-memory");
        assert_eq!(json["memory_decision_id"], "decision-1");
        assert_eq!(json["memory_audit_event_id"], "audit-2");
        assert!(json["memory_persistence_readback_hint"]
            .as_str()
            .unwrap()
            .contains("Not persistable yet"));
        assert_eq!(
            json["memory_reason_for_remembering"],
            "Keep memory-write proposal context visible in audit readback."
        );
        assert_eq!(json["summary"]["event_count"], 2);
    }

    #[test]
    fn audit_decision_summary_preserves_empty_decision_scope() {
        let readback = decision_readback_from_audit_events(vec![], "decision-empty");
        let summary = &readback.summary;

        assert_eq!(summary.decision_id, Some(DecisionId::new("decision-empty")));
        assert_eq!(summary.event_count, 0);
        assert_eq!(summary.first_event_id, None);
        assert_eq!(summary.last_event_id, None);
        assert_eq!(summary.first_event_at, None);
        assert_eq!(summary.last_event_at, None);
        assert_eq!(readback.decision_status, None);
        assert_eq!(readback.risk_level, None);
        assert!(readback.policies_applied.is_empty());
        assert!(!summary.has_execution_event);
    }

    #[test]
    fn cli_parses_status_command() {
        let cli = Cli::parse_from(["arpagona", "status", "--json"]);
        match cli.command {
            Command::Status(args) => assert!(args.json),
            _ => panic!("expected status"),
        }
    }

    #[test]
    fn cli_parses_insight_schema_command() {
        let cli = Cli::parse_from(["arpagona", "insight", "schema", "--json"]);
        match cli.command {
            Command::Insight(InsightCommand {
                command: InsightSubcommand::Schema(args),
            }) => assert!(args.json),
            _ => panic!("expected insight schema"),
        }
    }

    #[test]
    fn cli_parses_memory_status_command() {
        let cli = Cli::parse_from(["arpagona", "memory", "status", "--json"]);
        match cli.command {
            Command::Memory(MemoryCommand {
                command: MemorySubcommand::Status(args),
            }) => assert!(args.json),
            _ => panic!("expected memory status"),
        }
    }

    #[test]
    fn cli_parses_memory_proposals_commands() {
        let list = Cli::parse_from(["arpagona", "memory", "proposals", "--json"]);
        match list.command {
            Command::Memory(MemoryCommand {
                command: MemorySubcommand::Proposals(args),
            }) => assert!(args.json),
            _ => panic!("expected memory proposals"),
        }

        let detail = Cli::parse_from(["arpagona", "memory", "proposal", "action-1", "--json"]);
        match detail.command {
            Command::Memory(MemoryCommand {
                command: MemorySubcommand::Proposal(args),
            }) => {
                assert_eq!(args.proposal_id, "action-1");
                assert!(args.json);
            }
            _ => panic!("expected memory proposal detail"),
        }
    }

    #[test]
    fn memory_proposal_readback_filters_and_explains_memory_write_intent() {
        let actions = vec![
            serde_json::from_value::<ProposedAction>(json!({
                "id": "action-memory-1",
                "workspace_id": "workspace-1",
                "task_id": "task-1",
                "proposed_by": "agent-alpha",
                "action_type": "create_memory_fact",
                "target": "project:arpagona-agent-core",
                "payload": {
                    "memory_write_intent": {
                        "kind": "create_memory_fact",
                        "target": {
                            "entity_type": "project",
                            "entity_id": "arpagona-agent-core",
                            "attribute": "current_priority",
                            "value": "governed memory priority is inspectable",
                            "fact_id": "fact-memory-1",
                            "related_fact_id": null,
                            "failure_insight_id": null
                        },
                        "provenance": {
                            "source_id": "source-focus-loop",
                            "source_label": "focus loop",
                            "source_kind": "operational_report",
                            "evidence": "Issue #47 asks for memory proposal observability."
                        },
                        "confidence": 0.91,
                        "actor": "agent-alpha",
                        "reason_for_remembering": "Keep the current governed memory priority visible.",
                        "proposed_at": "2026-05-21T10:00:00Z",
                        "decision_id": "decision-memory-1",
                        "audit_event_id": "audit-memory-1",
                        "invalidation_note": "Supersede when focus loop priority changes."
                    }
                },
                "risk_level": "medium",
                "required_permissions": ["write_memory"],
                "rationale": "Remember project priority only after governance.",
                "context_refs": [],
                "status": "needs_human_approval",
                "created_at": "2026-05-21T10:00:00Z"
            }))
            .unwrap(),
            serde_json::from_value::<ProposedAction>(json!({
                "id": "action-email-1",
                "workspace_id": "workspace-1",
                "task_id": "task-1",
                "proposed_by": "agent-alpha",
                "action_type": "simulate_email",
                "target": "client@example.com",
                "payload": {},
                "risk_level": "medium",
                "required_permissions": ["simulate_email"],
                "rationale": "Draft an email.",
                "context_refs": [],
                "status": "pending_decision",
                "created_at": "2026-05-21T10:01:00Z"
            }))
            .unwrap(),
        ];

        let readback = memory_proposals_readback_from_actions(actions);

        assert_eq!(readback.proposals.len(), 1);
        let proposal = &readback.proposals[0];
        assert_eq!(proposal.id, "action-memory-1");
        assert_eq!(proposal.action_type, "create_memory_fact");
        assert_eq!(
            proposal.memory_write_kind.as_deref(),
            Some("create_memory_fact")
        );
        assert_eq!(proposal.target_type.as_deref(), Some("project"));
        assert_eq!(
            proposal.target_value,
            Some(json!("governed memory priority is inspectable"))
        );
        assert_eq!(
            proposal.provenance_source_label.as_deref(),
            Some("focus loop")
        );
        assert_eq!(proposal.target_fact_id.as_deref(), Some("fact-memory-1"));
        assert_eq!(
            proposal.provenance_source_id.as_deref(),
            Some("source-focus-loop")
        );
        assert!(proposal
            .persistence_readback_hint
            .contains("Not persistable yet"));
        assert!(proposal
            .supersession_hint
            .contains("Supersede when focus loop priority changes"));
        assert_eq!(proposal.required_permissions, vec!["write_memory"]);
        assert!(proposal
            .suggested_next_action
            .contains("Review Decision Gate result"));
        assert!(readback.warning.contains("not approval"));

        let formatted = format_memory_proposals_readback(&readback);
        assert!(formatted.contains("Memory write proposals"));
        assert!(formatted.contains("action-memory-1"));
        assert!(formatted.contains("reason_for_remembering:"));
        assert!(formatted.contains("target_fact_id:"));
        assert!(formatted.contains("persistence_readback_hint:"));
        assert!(formatted.contains("supersession_hint:"));
        assert!(formatted.contains("Readback only"));

        let json = serde_json::to_value(&readback).unwrap();
        assert_eq!(
            json["proposals"][0]["memory_write_kind"],
            "create_memory_fact"
        );
        assert_eq!(json["proposals"][0]["confidence"], 0.91);
    }

    #[test]
    fn memory_status_readback_describes_alpha_state_without_authorizing() {
        let readback = memory_status_readback();

        assert!(readback.graph_memory_support_compiled);
        assert_eq!(readback.expected_backend, "surrealdb");
        assert!(readback.surrealdb_adapter_available);
        assert!(readback
            .alpha_limits
            .contains(&"read-only CLI status/proposal readback only"));
        assert!(readback
            .governed_persistence_helpers
            .contains(&"persist_approved_create_memory_fact"));
        assert!(readback
            .governed_persistence_helpers
            .contains(&"persist_approved_failure_insight_memory"));
        assert!(readback
            .required_governance_controls
            .contains(&"approved Decision Gate result"));
        assert!(!readback
            .not_implemented
            .contains(&"approved Graph Memory write path"));
        assert!(readback
            .not_implemented
            .contains(&"CLI memory mutation command"));

        let formatted = format_memory_status_readback(&readback);
        assert!(formatted.contains("Graph Memory status"));
        assert!(formatted.contains("graph_memory_support_compiled:"));
        assert!(formatted.contains("surrealdb_adapter_available:"));
        assert!(formatted.contains("governed_persistence_helpers:"));
        assert!(formatted.contains("persist_approved_create_memory_fact"));
        assert!(formatted.contains("required_governance_controls:"));
        assert!(formatted.contains("read-only"));
        assert!(formatted.contains("not approval"));

        let json = serde_json::to_value(&readback).unwrap();
        assert_eq!(json["expected_backend"], "surrealdb");
        assert_eq!(json["surrealdb_adapter_available"], true);
        assert_eq!(
            json["governed_persistence_helpers"][0],
            "persist_approved_create_memory_fact"
        );
        assert!(json["required_governance_controls"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value == "approved Decision Gate result"));
        assert!(json["warning"].as_str().unwrap().contains("not approval"));
    }

    #[test]
    fn insight_schema_readback_lists_taxonomy_without_authorizing() {
        let readback = insight_schema_readback();

        assert!(readback.failure_classes.contains(&"missing_context"));
        assert!(readback
            .failure_classes
            .contains(&"insufficient_observability"));
        assert!(readback.correction_targets.contains(&"docs"));
        assert!(readback.minimum_fields.contains(&"audit_event_id"));
        assert!(readback
            .alpha_limits
            .contains(&"no execution or external side effects"));

        let formatted = format_insight_schema_readback(&readback);
        assert!(formatted.contains("Failure-to-Insight schema"));
        assert!(formatted.contains("failure_classes:"));
        assert!(formatted.contains("Readback only"));
        assert!(formatted.contains("not approval"));

        let json = serde_json::to_value(&readback).unwrap();
        assert_eq!(json["failure_classes"][0], "missing_context");
        assert!(json["warning"].as_str().unwrap().contains("not approval"));
    }

    #[test]
    fn status_readback_formats_counts_and_readback_warning() {
        let readback = StatusReadback {
            api_health: "ok".to_owned(),
            task_count: Some(2),
            proposed_action_count: Some(3),
            decision_count: Some(1),
            audit_event_count: Some(4),
            pending_decision_count: Some(2),
            needs_human_approval_count: Some(1),
            recent_audit_event_count: Some(4),
            last_audit_event_at: Some("2026-01-01T00:00:00+00:00".to_owned()),
            warning: AUDIT_READBACK_WARNING,
        };

        let formatted = format_status_readback(&readback);

        assert!(formatted.contains("ARPAGONA status"));
        assert!(formatted.contains("api_health:"));
        assert!(formatted.contains("task_count:"));
        assert!(formatted.contains("pending_decision_count:"));
        assert!(formatted.contains("needs_human_approval_count:"));
        assert!(formatted.contains("last_audit_event_at:"));
        assert!(formatted.contains("Readback only"));
    }

    #[test]
    fn status_readback_formats_unavailable_counts() {
        let readback = StatusReadback {
            api_health: "unavailable (connection refused)".to_owned(),
            task_count: None,
            proposed_action_count: None,
            decision_count: None,
            audit_event_count: None,
            pending_decision_count: None,
            needs_human_approval_count: None,
            recent_audit_event_count: None,
            last_audit_event_at: None,
            warning: AUDIT_READBACK_WARNING,
        };

        let formatted = format_status_readback(&readback);

        assert!(formatted.contains("unavailable"));
        assert!(formatted.contains("last_audit_event_at:"));
        assert!(formatted.contains("Readback only"));
    }

    #[test]
    fn parses_chat_internal_commands() {
        assert_eq!(parse_chat_line(""), ChatLine::Empty);
        assert_eq!(parse_chat_line("/help"), ChatLine::Help);
        assert_eq!(parse_chat_line("/quit"), ChatLine::Quit);
        assert_eq!(parse_chat_line("/exit"), ChatLine::Quit);
        assert_eq!(parse_chat_line("/status"), ChatLine::Status);
        assert_eq!(parse_chat_line("/audit"), ChatLine::Audit);
        assert_eq!(parse_chat_line("/tasks"), ChatLine::Tasks);
        assert_eq!(parse_chat_line("/actions"), ChatLine::Actions);
        assert_eq!(
            parse_chat_line("/evaluate action-1"),
            ChatLine::Evaluate("action-1".to_owned())
        );
        assert_eq!(
            parse_chat_line("/provider mock"),
            ChatLine::Provider("mock".to_owned())
        );
        assert_eq!(
            parse_chat_line("Prépare un brouillon"),
            ChatLine::Prompt("Prépare un brouillon".to_owned())
        );
    }

    #[test]
    fn masks_openai_key_without_leaking_secret() {
        let secret = "sk-proj-1234567890abcdef";
        let masked = mask_openai_key(secret);
        assert!(masked.starts_with("sk-"));
        assert!(masked.ends_with("cdef"));
        assert!(!masked.contains("1234567890"));
        assert_ne!(masked, secret);
    }

    #[test]
    fn short_openai_key_mask_is_safe() {
        assert_eq!(mask_openai_key("sk-123"), "***");
    }

    #[test]
    fn openai_provider_error_points_to_auth_help_without_secret() {
        let message = format_provider_error("openai", "OPENAI_API_KEY is missing");
        assert!(message.contains("arpagona auth openai"));
        assert!(!message.contains("sk-"));
    }

    #[test]
    fn command_definition_is_valid() {
        Cli::command().debug_assert();
    }
}
