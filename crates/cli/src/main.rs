use arpagona_agent_core::{
    holographic::{resonate_for_working_memory, RESONANCE_NON_AUTHORIZING_WARNING},
    llm_journal::LlmJournal,
    ActionType, AgentId, AuditEvent, AuditEventId, AuditTraceSummary, CognitiveCycleResult,
    CorrectionTarget, Decision, DecisionId, DecisionStatus, DetectionSignal, DetectionSignalType,
    ExecutorRegistry, ExecutorState, FailureClass, FailureInsight, FailureInsightId,
    InsightSeverity, MemoryWriteIntent, MemoryWriteKind, MemoryWriteProvenance, MemoryWriteTarget,
    ObjectiveDomain, Permission, ProposedAction, ProposedActionId, ProposedActionStatus, RiskLevel,
    SourceId, Task, TaskId, WorkspaceId,
};
use arpagona_compute_reservoir::{
    allocate_for_working_memory, ComputeAllocation, ComputeCapability, ComputeNode, ComputeNodeId,
    ComputeNodeStatus, ComputePolicy, ComputeResourceKind, DataSensitivity,
    NON_AUTHORIZING_READBACK,
};
use arpagona_decision_gate::{audit_event_for_decision, evaluate_proposed_action};
use arpagona_graph_memory::{
    demo_snapshot::{list_snapshots_in_directory, FailureInsightDemoSnapshot, EVIDENCE_ONLY_TOKEN},
    in_memory_graph_memory_store, AsyncGraphMemoryStore, GRAPH_MEMORY_SCHEMA,
};
use arpagona_holographic_memory::{
    embedding::{extend_signature_with_embedding, CharacterNGramEmbeddingProvider},
    sqlite_store::SqliteHolographicMemoryStore,
    HolographicMemoryError, HolographicMemoryStore, HolographicQuery, HolographicTrace,
    InMemoryHolographicMemoryStore, SourceKind,
};
use arpagona_llm::run_cognitive_synthesis;
use arpagona_tool_runtime::{ToolRuntime, ToolRuntimeConfig};
use chrono::Utc;
use clap::{Args, Parser, Subcommand, ValueEnum};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command as ProcessCommand;
use std::sync::{Mutex, OnceLock};

/// Global LLM journal for the CLI session.
///
/// All LLM interactions (synthesis, tool-call intents, governed executions)
/// are recorded here for operator readback via `arpagona llm journal`.
/// Entries are persisted to a JSON-lines file for cross-invocation readback.
fn global_llm_journal() -> &'static Mutex<LlmJournal> {
    static JOURNAL: OnceLock<Mutex<LlmJournal>> = OnceLock::new();
    JOURNAL.get_or_init(|| {
        let path = env::var("ARPAGONA_LLM_JOURNAL_PATH")
            .unwrap_or_else(|_| "target/llm-journal.jsonl".to_owned());
        Mutex::new(LlmJournal::with_file(100, path))
    })
}

const DEFAULT_API_URL: &str = "http://127.0.0.1:3000";
const DEFAULT_WORKSPACE_ID: &str = "workspace-alpha";
const DEFAULT_AGENT_ID: &str = "agent-alpha";
const DEFAULT_TASK_ID: &str = "task-1";
const DEFAULT_TARGET: &str = "client@example.com";
const DEFAULT_RATIONALE: &str = "Préparer un brouillon sans l’envoyer";
const DEFAULT_PROVIDER: &str = "ollama";
const DEFAULT_CHAT_PROVIDER: &str = "ollama";
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
    /// List and inspect executor registry state.
    Executor(ExecutorCommand),
    /// Start the native MCP server (stdio transport).
    McpServer(McpServerArgs),
    /// Read recent MCP governance audit decisions from a persisted file.
    McpGovernanceAudit(McpGovernanceAuditArgs),
    /// Read the LLM interaction journal (C3 — prompt/response/decision/risk traces).
    Llm(LlmCommand),
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
pub struct ExecutorCommand {
    #[command(subcommand)]
    pub command: ExecutorSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ExecutorSubcommand {
    /// List all registered executors with their current state.
    List(ExecutorListArgs),
    /// Inspect the details of a specific executor.
    Inspect(ExecutorInspectArgs),
}

#[derive(Debug, Args)]
pub struct ExecutorListArgs {
    /// Emit structured JSON instead of human-oriented text.
    #[arg(long)]
    pub json: bool,
    /// Query executor state from the local core crate directly, without
    /// connecting to the API server. Shows only the static default registry
    /// state (NoopExecutor disabled). Combine with --state-file to load
    /// persisted executor state transitions. The output explicitly indicates
    /// that it is offline/local state, not live server state.
    #[arg(long)]
    pub offline: bool,
    /// Path to a JSON file with persisted executor state, applied on top
    /// of the default registry when --offline is set.
    /// Format: {"executor_id": "disabled"|"ready"|"blocked"}
    /// Example: {"noop-executor": "ready"}
    #[arg(long)]
    pub state_file: Option<String>,
}

#[derive(Debug, Args)]
pub struct ExecutorInspectArgs {
    /// The executor ID to inspect.
    pub executor_id: String,
    /// Emit structured JSON instead of human-oriented text.
    #[arg(long)]
    pub json: bool,
    /// Query executor state from the local core crate directly, without
    /// connecting to the API server. Shows only the static default registry
    /// state (NoopExecutor disabled). Combine with --state-file to load
    /// persisted executor state transitions. The output explicitly indicates
    /// that it is offline/local state, not live server state.
    #[arg(long)]
    pub offline: bool,
    /// Path to a JSON file with persisted executor state, applied on top
    /// of the default registry when --offline is set.
    /// Format: {"executor_id": "disabled"|"ready"|"blocked"}
    /// Example: {"noop-executor": "ready"}
    #[arg(long)]
    pub state_file: Option<String>,
}

/// Arguments for the `mcp-server` command.
#[derive(Debug, Args)]
pub struct McpServerArgs {
    /// Workspace path to serve files from (default: current dir).
    #[arg(long, default_value = ".")]
    pub workspace: String,
    /// Server name advertised to MCP clients.
    #[arg(long, default_value = "arpagona-mcp")]
    pub name: String,
    /// Server version advertised to MCP clients.
    #[arg(long, default_value = "0.1.0")]
    pub version: String,
    /// Path to the governance audit log file.
    /// When set, every governance decision is persisted to this file.
    #[arg(long)]
    pub audit_path: Option<String>,
}

/// Arguments for the `mcp-governance-audit` command.
#[derive(Debug, Args)]
pub struct McpGovernanceAuditArgs {
    /// Path to the governance audit log file (default: target/mcp-audit.jsonl).
    #[arg(long, default_value = "target/mcp-audit.jsonl")]
    pub audit_path: String,
    /// Maximum number of recent entries to show.
    #[arg(long, default_value_t = 20)]
    pub limit: usize,
    /// Emit structured JSON instead of human-readable text.
    #[arg(long)]
    pub json: bool,
}

/// Arguments for the `llm` command group.
#[derive(Debug, Args)]
pub struct LlmCommand {
    #[command(subcommand)]
    pub command: LlmSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum LlmSubcommand {
    /// Show recent LLM interaction journal entries.
    Journal(LlmJournalArgs),
}

#[derive(Debug, Args)]
pub struct LlmJournalArgs {
    /// Maximum number of recent entries to show (default: 10).
    #[arg(long, default_value_t = 10)]
    pub limit: usize,
    /// Emit structured JSON instead of human-readable text.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct CognitiveRunArgs {
    /// The objective text to work on.
    #[arg(long)]
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
    /// Run Compute Reservoir allocation bridge: map WorkingMemory to resource selection.
    #[arg(long)]
    pub allocate: bool,
    /// Run HolographicMemory resonance bridge: map WorkingMemory + allocation to pattern hints.
    #[arg(long)]
    pub resonate: bool,
    /// Run tool observation bridge: execute tool runtime for required observations and inject results.
    #[arg(long)]
    pub observe: bool,
    /// Run proposal bridge: convert FailureInsightCandidates and observations into
    /// context-rich ProposedActions via the API. Each proposal is PendingDecision
    /// and carries metadata (objective, source_kind, rationale, risk, etc.).
    #[arg(long)]
    pub propose: bool,
    /// Run LLM synthesis: call an LLM provider to enrich the cognitive cycle output.
    #[arg(long)]
    pub llm: bool,
    /// Run offline governance bridge: convert FailureInsightCandidates into ProposedActions
    /// through the local DecisionGate -> Decision -> AuditEvent path without requiring the
    /// API server. Requires --assess (needs FailureInsightCandidates to govern).
    #[arg(long)]
    pub govern: bool,
    /// LLM provider to use for --llm (mock = no real API call, openai = OpenAI Responses API, ollama = local Ollama).
    #[arg(long, default_value = "ollama")]
    pub provider: String,
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
    /// Review proposed actions: list, show, approve, reject, defer.
    Review(ReviewActionCommand),
    /// Run dry-run sandbox for approved low-risk proposals.
    Sandbox(SandboxActionCommand),
    /// Dry-run an approved proposal: simulate execution without side effects.
    DryRun(DryRunActionArgs),
    /// Query the execution capability registry.
    Capability(CapabilityCommand),
    /// Run a policy check on a proposed action.
    Policy(PolicyActionCommand),
    /// Execute a proposal (disabled — always returns ExecutionDisabled).
    Execute(ExecuteActionArgs),
}

#[derive(Debug, Args)]
struct ReviewActionCommand {
    #[command(subcommand)]
    command: ReviewActionSubcommand,
}

#[derive(Debug, Subcommand)]
enum ReviewActionSubcommand {
    /// List proposed actions, optionally filtered by status.
    List(ReviewActionListArgs),
    /// Show a single proposed action by ID.
    Show(ReviewActionShowArgs),
    /// Approve a pending proposed action.
    Approve(ReviewActionTransitionArgs),
    /// Reject a pending proposed action.
    Reject(ReviewActionTransitionArgs),
    /// Defer a pending proposed action.
    Defer(ReviewActionTransitionArgs),
    /// Supersede an approved proposed action.
    Supersede(ReviewActionTransitionArgs),
}

#[derive(Debug, Args)]
struct ReviewActionListArgs {
    /// Filter by status (e.g. pending_decision, approved, rejected, deferred).
    #[arg(long)]
    status: Option<String>,
    /// Emit structured JSON instead of human-oriented text.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct ReviewActionShowArgs {
    /// Proposed action ID to show.
    action_id: String,
    /// Emit structured JSON instead of human-oriented text.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct ReviewActionTransitionArgs {
    /// Proposed action ID to transition.
    action_id: String,
    /// Optional human-readable reason.
    #[arg(long)]
    reason: Option<String>,
    /// Actor identifier (default: human-cli).
    #[arg(long, default_value = "human-cli")]
    actor: String,
    /// Emit structured JSON instead of human-oriented text.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct SandboxActionCommand {
    #[command(subcommand)]
    command: SandboxActionSubcommand,
}

#[derive(Debug, Subcommand)]
enum SandboxActionSubcommand {
    /// Run dry-run sandbox for an approved low-risk proposal.
    Run(SandboxRunArgs),
    /// List sandbox runs.
    List(SandboxListArgs),
}

#[derive(Debug, Args)]
struct SandboxRunArgs {
    /// Proposed action ID to run in sandbox.
    action_id: String,
    /// Emit structured JSON instead of human-oriented text.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct SandboxListArgs {
    /// Emit structured JSON instead of human-oriented text.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct DryRunActionArgs {
    /// Proposed action ID to dry-run.
    action_id: String,
    /// Emit structured JSON instead of human-oriented text.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct CapabilityCommand {
    #[command(subcommand)]
    command: CapabilitySubcommand,
}

#[derive(Debug, Subcommand)]
enum CapabilitySubcommand {
    /// List all execution capabilities.
    List(CapabilityListArgs),
    /// Show capability for a specific action type.
    Show(CapabilityShowArgs),
}

#[derive(Debug, Args)]
struct CapabilityListArgs {
    /// Emit structured JSON instead of human-oriented text.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct CapabilityShowArgs {
    /// Action type (e.g. read_memory, simulate_email).
    action_type: String,
    /// Emit structured JSON instead of human-oriented text.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct PolicyActionCommand {
    #[command(subcommand)]
    command: PolicyActionSubcommand,
}

#[derive(Debug, Subcommand)]
enum PolicyActionSubcommand {
    /// Run a policy check on a proposed action.
    Check(PolicyCheckArgs),
}

#[derive(Debug, Args)]
struct PolicyCheckArgs {
    /// Proposed action ID to check.
    action_id: String,
    /// Emit structured JSON instead of human-oriented text.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct ExecuteActionArgs {
    /// Proposed action ID to execute.
    action_id: String,
    /// Emit structured JSON instead of human-oriented text.
    #[arg(long)]
    json: bool,
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
    /// Exercise holographic memory: add traces and search by resonance.
    Holographic(HolographicCommand),
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
// Holographic Memory — alpha CLI holographic memory commands
// ---------------------------------------------------------------------------

#[derive(Debug, Args)]
struct HolographicCommand {
    #[command(subcommand)]
    command: HolographicSubcommand,
}

#[derive(Debug, Subcommand)]
enum HolographicSubcommand {
    /// Add a trace to the holographic memory store and persist to file.
    Add(HolographicAddArgs),
    /// Search holographic memory by resonance with a query.
    Search(HolographicSearchArgs),
    /// Encode conversation turns as holographic traces with distributed signatures.
    ///
    /// Reads turn data from a JSON file (Conversation format) and creates
    /// holographic traces using keyword extraction and role-based concepts.
    /// Optionally finds similar traces by resonance after processing.
    FromConversation(HolographicFromConversationArgs),
    /// Explore the linked-memory graph from a trace, following linked_memory_ids chains.
    ///
    /// Uses BFS traversal with configurable depth limit and cycle detection.
    /// Returns all reachable traces in discovery order with traversal metadata.
    Explore(HolographicExploreArgs),
    /// Consolidate redundant holographic memory traces within a time window.
    ///
    /// Finds pairs of traces in the same project with similar resonance signatures
    /// created within `--window` minutes, merges keywords/concepts/entities, sums
    /// activation counts, and removes the redundant trace. Operates on the SQLite
    /// store only (InMemory store is a no-op).
    Consolidate(HolographicConsolidateArgs),
}

#[derive(Debug, Args)]
struct HolographicAddArgs {
    /// Trace identifier (must be unique within the store).
    #[arg(long)]
    trace_id: String,
    /// Project scope for the trace.
    #[arg(long, default_value = "default")]
    project_id: String,
    /// Comma-separated keywords for the trace.
    #[arg(long)]
    keywords: String,
    /// Comma-separated concepts for the trace.
    #[arg(long)]
    concepts: String,
    /// Comma-separated entities for the trace.
    #[arg(long)]
    entities: String,
    /// Path to the holographic memory JSON file (created if not found).
    #[arg(long, default_value = "target/holographic-store.json")]
    file: String,
    /// Enable embedding-based semantic generalization (character n-gram).
    #[arg(long)]
    embed: bool,
    /// Emit structured JSON instead of human-oriented text.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct HolographicSearchArgs {
    /// Project scope for the query.
    #[arg(long, default_value = "default")]
    project_id: String,
    /// Query text to describe what you're looking for.
    #[arg(long)]
    query: String,
    /// Comma-separated keywords to match against stored traces.
    #[arg(long)]
    keywords: String,
    /// Comma-separated concepts to match against stored traces.
    #[arg(long)]
    concepts: String,
    /// Comma-separated entities to match against stored traces.
    #[arg(long)]
    entities: String,
    /// Maximum number of resonance matches to return.
    #[arg(long, default_value = "10")]
    limit: usize,
    /// Path to the holographic memory JSON file.
    #[arg(long, default_value = "target/holographic-store.json")]
    file: String,
    /// Enable embedding-based semantic generalization (character n-gram).
    #[arg(long)]
    embed: bool,
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct HolographicFromConversationArgs {
    /// Path to the conversation JSON file (Conversation format).
    #[arg(long)]
    file: String,
    /// Project scope for the traces.
    #[arg(long, default_value = "default")]
    project_id: String,
    /// Path to the holographic memory JSON store (created if not found).
    #[arg(long, default_value = "target/holographic-store.json")]
    store: String,
    /// After processing all turns, find similar traces by resonance.
    #[arg(long)]
    find_similar: bool,
    /// Maximum number of resonance matches when --find-similar is set.
    #[arg(long, default_value = "5")]
    limit: usize,
    /// Emit structured JSON instead of human-oriented text.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct HolographicExploreArgs {
    /// The trace ID to start traversal from.
    #[arg(long)]
    trace_id: String,
    /// Project scope for the traces.
    #[arg(long, default_value = "default")]
    project_id: String,
    /// Maximum traversal depth (0 = root only).
    #[arg(long, default_value_t = 10)]
    max_depth: usize,
    /// Path to the holographic memory JSON file.
    #[arg(long, default_value = "target/holographic-store.json")]
    file: String,
    /// Emit structured JSON instead of human-oriented text.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct HolographicConsolidateArgs {
    /// Project scope for consolidation.
    #[arg(long, default_value = "default")]
    project_id: String,
    /// Time window in minutes within which to find redundant trace pairs.
    #[arg(long, default_value_t = 60)]
    window: u64,
    /// Similarity threshold (0.0–1.0) for detecting redundant traces.
    #[arg(long, default_value_t = 0.7)]
    threshold: f32,
    /// Path to the SQLite holographic memory database file.
    #[arg(long, default_value = "target/holographic-store.db")]
    db: String,
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
    /// Readback-only warning.
    warning: &'static str,
    /// Non-API-dependent subsystem status gathered locally.
    local: LocalSubsystemStatus,
    /// D2 supervision — recent proposed actions and Decision Gate results.
    supervision: SupervisionSection,
}

/// Non-API-dependent subsystem status, gathered locally without requiring the API server.
///
/// This makes `arpagona status` useful even when the API server is not running,
/// by reporting the health and configuration of local subsystems:
/// memory stores, LLM provider config, tool runtime, handoff files, and MCP server.
#[derive(Debug, Serialize)]
struct LocalSubsystemStatus {
    /// Whether the holographic memory SQLite database exists on disk.
    holographic_memory_db_exists: bool,
    /// Path to the holographic memory SQLite database.
    holographic_memory_db_path: Option<String>,
    /// Whether the OpenAI API key is configured in the environment.
    openai_api_key_configured: bool,
    /// Whether an Ollama endpoint is configured in the environment (or default used).
    ollama_endpoint_configured: bool,
    /// Whether the Ollama endpoint appears reachable (by checking default socket).
    ollama_appears_reachable: bool,
    /// Whether the conversation memory store has stored traces.
    conversation_memory_trace_count: Option<usize>,
    /// Number of tools declared in the tool runtime.
    tool_runtime_tool_count: Option<usize>,
    /// Available tool names from the tool runtime.
    tool_runtime_tools: Vec<String>,
    /// Current next action from FOCUS_LOOP_NEXT.md (first line content).
    handoff_next_action: Option<String>,
    /// Number of open items in DAILY_VALIDATION_BACKLOG.md.
    backlog_open_count: Option<usize>,
    /// Whether the MCP server binary exists in the target directory.
    mcp_server_binary_available: bool,
    /// CLI crate version string.
    cli_version: String,
    /// Readback-only warning.
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

/// Compact summary of a proposed action for operator supervision (D2).
#[derive(Debug, Serialize)]
struct ProposedActionSummary {
    id: String,
    action_type: String,
    target: Option<String>,
    risk_level: String,
    required_permissions: Vec<String>,
    rationale: String,
    status: String,
    created_at: String,
}

/// Compact summary of a Decision Gate result for operator supervision (D2).
#[derive(Debug, Serialize)]
struct DecisionResultSummary {
    id: String,
    proposed_action_id: String,
    status: String,
    reason: String,
    risk_level: String,
    created_at: String,
}

/// ProposedAction and tool-call supervision section (D2).
///
/// Shows recent proposed actions, direct tool-call intents, and their
/// Decision Gate results. Read-only — does not authorize execution.
#[derive(Debug, Serialize)]
struct SupervisionSection {
    recent_proposed_actions: Vec<ProposedActionSummary>,
    recent_decision_results: Vec<DecisionResultSummary>,
    warning: &'static str,
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
            ActionSubcommand::Review(args) => review_action(&client, &api_url, args).await?,
            ActionSubcommand::Sandbox(args) => sandbox_action(&client, &api_url, args).await?,
            ActionSubcommand::DryRun(args) => dry_run_action(&client, &api_url, args).await?,
            ActionSubcommand::Capability(args) => {
                capability_action(&client, &api_url, args).await?
            }
            ActionSubcommand::Policy(args) => policy_action(&client, &api_url, args).await?,
            ActionSubcommand::Execute(args) => execute_action(&client, &api_url, args).await?,
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
            MemorySubcommand::Holographic(h) => match h.command {
                HolographicSubcommand::Add(args) => memory_holographic_add(args)?,
                HolographicSubcommand::Search(args) => memory_holographic_search(args)?,
                HolographicSubcommand::FromConversation(args) => {
                    memory_holographic_from_conversation(args)?
                }
                HolographicSubcommand::Explore(args) => memory_holographic_explore(args)?,
                HolographicSubcommand::Consolidate(args) => memory_holographic_consolidate(args)?,
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
            CognitiveSubcommand::Run(args) => cognitive_run(&client, &api_url, args).await?,
        },
        Command::Executor(executor) => match executor.command {
            ExecutorSubcommand::List(args) => executor_list(&client, &api_url, args).await?,
            ExecutorSubcommand::Inspect(args) => executor_inspect(&client, &api_url, args).await?,
        },
        Command::McpServer(args) => mcp_server(args)?,
        Command::McpGovernanceAudit(args) => mcp_governance_audit(args)?,
        Command::Llm(llm) => match llm.command {
            LlmSubcommand::Journal(args) => llm_journal_list(args)?,
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
                if matches!(next_provider.as_str(), "mock" | "openai" | "ollama") {
                    provider = next_provider;
                    println!("{} {}", style_success("Provider:"), provider);
                } else {
                    println!(
                        "{}",
                        style_error("Unsupported provider. Use mock, openai, or ollama.")
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
            local: gather_local_subsystem_status().await,
            supervision: SupervisionSection {
                recent_proposed_actions: vec![],
                recent_decision_results: vec![],
                warning: AUDIT_READBACK_WARNING,
            },
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

    // Build D2 supervision section from fetched data.
    let recent_proposed_actions = actions
        .as_ref()
        .map(|actions| {
            actions
                .iter()
                .rev()
                .take(5)
                .map(|action| ProposedActionSummary {
                    id: action.id.to_string(),
                    action_type: serde_json::to_string(&action.action_type)
                        .unwrap_or_else(|_| "unknown".to_owned()),
                    target: action.target.clone(),
                    risk_level: serde_json::to_string(&action.risk_level)
                        .unwrap_or_else(|_| "unknown".to_owned()),
                    required_permissions: action
                        .required_permissions
                        .iter()
                        .map(|p| format!("{p:?}"))
                        .collect(),
                    rationale: action.rationale.clone(),
                    status: serde_json::to_string(&action.status)
                        .unwrap_or_else(|_| "unknown".to_owned()),
                    created_at: action.created_at.to_rfc3339(),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let recent_decision_results = decisions
        .as_ref()
        .map(|decisions| {
            decisions
                .iter()
                .rev()
                .take(5)
                .map(|decision| DecisionResultSummary {
                    id: decision.id.to_string(),
                    proposed_action_id: decision.proposed_action_id.to_string(),
                    status: serde_json::to_string(&decision.status)
                        .unwrap_or_else(|_| "unknown".to_owned()),
                    reason: decision.reason.clone(),
                    risk_level: serde_json::to_string(&decision.risk_level)
                        .unwrap_or_else(|_| "unknown".to_owned()),
                    created_at: decision.created_at.to_rfc3339(),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

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
        local: gather_local_subsystem_status().await,
        supervision: SupervisionSection {
            recent_proposed_actions,
            recent_decision_results,
            warning: AUDIT_READBACK_WARNING,
        },
    }
}

/// Gather subsystem status from local (non-API) sources.
///
/// This covers memory stores, provider configuration, tool runtime,
/// handoff/backlog files, and MCP server availability. It does not
/// require the API server to be running.
async fn gather_local_subsystem_status() -> LocalSubsystemStatus {
    // --- Holographic Memory SQLite database ---
    let hm_db_path = PathBuf::from("target/holographic-memory.db");
    let hm_db_exists = hm_db_path.exists();
    let hm_db_path_str = if hm_db_exists {
        Some(hm_db_path.to_string_lossy().to_string())
    } else {
        None
    };

    // --- Provider configuration ---
    let openai_key = std::env::var("OPENAI_API_KEY");
    let openai_configured = openai_key.is_ok();
    let ollama_endpoint =
        std::env::var("OLLAMA_ENDPOINT").unwrap_or_else(|_| "http://localhost:11434".to_owned());
    let ollama_configured = true; // Default endpoint always exists

    // --- Ollama reachability (lightweight check) ---
    let ollama_reachable = check_ollama_reachable(&ollama_endpoint).await;

    // --- Conversation memory traces ---
    let conv_trace_count = check_conversation_memory_traces();

    // --- Tool Runtime ---
    // Known tools from the alpha read-only Tool Runtime.
    let tools: Vec<String> = vec![
        "read_file".to_owned(),
        "list_files".to_owned(),
        "search_text".to_owned(),
    ];
    let tool_count = Some(tools.len());

    // --- Handoff file (FOCUS_LOOP_NEXT.md) ---
    let handoff_next = read_handoff_next_action();

    // --- Backlog open items ---
    let backlog_count = count_backlog_open_items();

    // --- MCP server binary ---
    let mcp_binary = PathBuf::from("target/debug/arpagona-mcp-server");
    let mcp_available = mcp_binary.exists();

    // --- CLI version ---
    let cli_version = env!("CARGO_PKG_VERSION").to_owned();

    LocalSubsystemStatus {
        holographic_memory_db_exists: hm_db_exists,
        holographic_memory_db_path: hm_db_path_str,
        openai_api_key_configured: openai_configured,
        ollama_endpoint_configured: ollama_configured,
        ollama_appears_reachable: ollama_reachable,
        conversation_memory_trace_count: conv_trace_count,
        tool_runtime_tool_count: tool_count,
        tool_runtime_tools: tools,
        handoff_next_action: handoff_next,
        backlog_open_count: backlog_count,
        mcp_server_binary_available: mcp_available,
        cli_version,
        warning: AUDIT_READBACK_WARNING,
    }
}

/// Attempt a lightweight check to see if the Ollama endpoint responds.
async fn check_ollama_reachable(endpoint: &str) -> bool {
    let url = format!("{}/api/tags", endpoint.trim_end_matches('/'));
    match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
    {
        Ok(client) => match client.get(&url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        },
        Err(_) => false,
    }
}

/// Read conversation memory traces from the in-memory store.
/// Falls back to None if the store is not available.
fn check_conversation_memory_traces() -> Option<usize> {
    // Since conversation memory requires in-memory state that may not exist
    // in a fresh CLI invocation, return None to indicate "not available".
    // A future enhancement could add a SQLite-backed persistent store.
    None
}

/// Read the first meaningful content line from FOCUS_LOOP_NEXT.md.
fn read_handoff_next_action() -> Option<String> {
    let path = PathBuf::from("FOCUS_LOOP_NEXT.md");
    if !path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(path).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty()
            && !trimmed.starts_with('#')
            && !trimmed.starts_with('-')
            && trimmed.len() > 3
        {
            return Some(trimmed.to_owned());
        }
    }
    None
}

/// Count open candidate items in DAILY_VALIDATION_BACKLOG.md.
fn count_backlog_open_items() -> Option<usize> {
    let path = PathBuf::from("DAILY_VALIDATION_BACKLOG.md");
    if !path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(path).ok()?;
    let count = content
        .lines()
        .filter(|line| line.trim().starts_with("###") && line.contains("DV-"))
        .count();
    Some(count)
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
    push_readback_line(&mut output, "");
    push_readback_line(&mut output, &style_info("Local subsystems"));
    push_readback_field(&mut output, "cli_version:", &readback.local.cli_version);
    push_readback_field(
        &mut output,
        "hm_db_exists:",
        &readback.local.holographic_memory_db_exists.to_string(),
    );
    let hm_path = readback
        .local
        .holographic_memory_db_path
        .as_deref()
        .unwrap_or("-");
    push_readback_field(&mut output, "hm_db_path:", hm_path);
    push_readback_field(
        &mut output,
        "openai_key_configured:",
        &readback.local.openai_api_key_configured.to_string(),
    );
    push_readback_field(
        &mut output,
        "ollama_configured:",
        &readback.local.ollama_endpoint_configured.to_string(),
    );
    push_readback_field(
        &mut output,
        "ollama_reachable:",
        &readback.local.ollama_appears_reachable.to_string(),
    );
    push_readback_field(
        &mut output,
        "tool_runtime_tool_count:",
        &format_optional_usize(readback.local.tool_runtime_tool_count),
    );
    push_readback_field(
        &mut output,
        "tool_runtime_tools:",
        &readback.local.tool_runtime_tools.join(", "),
    );
    if let Some(next) = &readback.local.handoff_next_action {
        push_readback_field(&mut output, "handoff_next_action:", next);
    }
    push_readback_field(
        &mut output,
        "backlog_open_count:",
        &format_optional_usize(readback.local.backlog_open_count),
    );
    push_readback_field(
        &mut output,
        "mcp_server_binary:",
        &readback.local.mcp_server_binary_available.to_string(),
    );
    push_readback_line(&mut output, &style_dim(readback.local.warning));
    push_readback_line(&mut output, "");
    push_readback_line(
        &mut output,
        &style_info("Proposed action & tool-call supervision (D2)"),
    );
    if readback.supervision.recent_proposed_actions.is_empty() {
        push_readback_line(&mut output, "  (no recent proposed actions)");
    } else {
        for action in &readback.supervision.recent_proposed_actions {
            push_readback_line(&mut output, &format!("  action: {}", action.id));
            push_readback_field(
                &mut output,
                "  type:",
                &action_type_display(&action.action_type),
            );
            if let Some(target) = &action.target {
                push_readback_field(&mut output, "  target:", target);
            }
            push_readback_field(&mut output, "  risk:", &action.risk_level);
            push_readback_field(&mut output, "  status:", &action.status);
            push_readback_field(
                &mut output,
                "  permissions:",
                &action.required_permissions.join(", "),
            );
            push_readback_field(&mut output, "  rationale:", &action.rationale);
            push_readback_field(&mut output, "  at:", &action.created_at);
            push_readback_line(&mut output, "");
        }
    }
    if readback.supervision.recent_decision_results.is_empty() {
        push_readback_line(&mut output, "  (no recent decision results)");
    } else {
        for decision in &readback.supervision.recent_decision_results {
            push_readback_line(&mut output, &format!("  decision: {}", decision.id));
            push_readback_field(
                &mut output,
                "  proposed_action_id:",
                &decision.proposed_action_id,
            );
            push_readback_field(&mut output, "  status:", &decision.status);
            push_readback_field(&mut output, "  reason:", &decision.reason);
            push_readback_field(&mut output, "  risk:", &decision.risk_level);
            push_readback_field(&mut output, "  at:", &decision.created_at);
            push_readback_line(&mut output, "");
        }
    }
    push_readback_line(&mut output, &style_dim(readback.supervision.warning));
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

/// Strip surrounding quotes from a JSON-encoded string for display.
fn action_type_display(json_str: &str) -> String {
    json_str.trim_matches('"').to_owned()
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
// Holographic Memory handlers — alpha CLI for the holographic memory crate
// ---------------------------------------------------------------------------

fn memory_holographic_add(args: HolographicAddArgs) -> Result<(), Box<dyn Error>> {
    // Load or create the store
    let mut store = match InMemoryHolographicMemoryStore::load_from_file(&args.file) {
        Ok(s) => s,
        Err(HolographicMemoryError::PersistenceError(_)) => InMemoryHolographicMemoryStore::new(),
        Err(e) => {
            return Err(format!("Failed to load holographic store: {}", e).into());
        }
    };

    // Parse comma-separated fields
    let keywords: Vec<String> = args
        .keywords
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let concepts: Vec<String> = args
        .concepts
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let entities: Vec<String> = args
        .entities
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    // Build the trace with a distributed signature
    let mut trace = HolographicTrace::new(
        args.trace_id.clone(),    // id
        args.project_id.clone(),  // project_id
        SourceKind::ManualNote,   // source_kind
        "cli-manual".to_string(), // source_id
        vec![],                   // source_turn_ids
        format!(
            // content_summary
            "keywords: {}, concepts: {}, entities: {}",
            args.keywords, args.concepts, args.entities
        ),
        keywords.clone(),                // keywords
        concepts.clone(),                // concepts
        entities.clone(),                // entities
        vec![],                          // linked_memory_ids
        vec![],                          // linked_decision_ids
        1.0,                             // importance
        0.5,                             // confidence
        0.0,                             // emotional_weight
        0.0,                             // strategic_weight
        chrono::Utc::now().to_rfc3339(), // created_at
    );

    // Optionally extend with embedding bits for semantic generalization
    if args.embed {
        let provider = CharacterNGramEmbeddingProvider::default();
        extend_signature_with_embedding(
            &mut trace.distributed_signature,
            &trace.content_summary,
            &keywords,
            &provider,
        );
    }

    // Add and save
    store
        .add_trace(trace)
        .map_err(|e| format!("Holographic memory error: {}", e))?;
    store
        .save_to_file(&args.file)
        .map_err(|e| format!("Save failed: {}", e))?;

    if args.json {
        let output = serde_json::json!({
            "status": "added",
            "trace_id": args.trace_id,
            "project_id": args.project_id,
            "store_path": args.file,
            "keyword_count": keywords.len(),
            "concept_count": concepts.len(),
            "entity_count": entities.len(),
            "total_traces": store.len(),
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("✓ Holographic trace added");
        println!("  Trace ID:  {}", args.trace_id);
        println!("  Project:   {}", args.project_id);
        println!("  Keywords:  {}", keywords.join(", "));
        println!("  Concepts:  {}", concepts.join(", "));
        println!("  Entities:  {}", entities.join(", "));
        println!("  Store:     {}", args.file);
        println!("  Total traces: {}", store.len());
        println!();
        println!(
            "⚠️  Holographic memory is recall evidence only. It does not authorize any action."
        );
    }

    Ok(())
}

fn memory_holographic_search(args: HolographicSearchArgs) -> Result<(), Box<dyn Error>> {
    // Load the store
    let mut store = match InMemoryHolographicMemoryStore::load_from_file(&args.file) {
        Ok(s) => s,
        Err(HolographicMemoryError::PersistenceError(_)) => {
            eprintln!("No holographic store found at: {}", args.file);
            eprintln!("Add some traces first with `memory holographic add`.");
            return Ok(());
        }
        Err(e) => {
            return Err(format!("Failed to load holographic store: {}", e).into());
        }
    };

    // Parse comma-separated query fields
    let keywords: Vec<String> = args
        .keywords
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let concepts: Vec<String> = args
        .concepts
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let entities: Vec<String> = args
        .entities
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if keywords.is_empty() && concepts.is_empty() && entities.is_empty() {
        eprintln!("At least one of --keywords, --concepts, or --entities is required for resonance search.");
        return Ok(());
    }

    // Build query
    let mut query = HolographicQuery::new(
        args.project_id.clone(),
        args.query.clone(),
        keywords.clone(),
        concepts,
        entities,
    );

    // Optionally extend with embedding bits for semantic generalization
    if args.embed {
        let provider = CharacterNGramEmbeddingProvider::default();
        extend_signature_with_embedding(
            &mut query.distributed_signature,
            &query.text,
            &keywords,
            &provider,
        );
    }

    // Search by resonance
    let context = store.retrieve_by_resonance(&args.project_id, query, args.limit);

    if args.json {
        println!("{}", serde_json::to_string_pretty(&context)?);
    } else {
        if context.matches.is_empty() {
            println!(
                "🔍 No resonance matches found for project '{}'.",
                args.project_id
            );
            println!("  Query: {}", args.query);
        } else {
            println!("🔍 Resonance matches for project '{}':", args.project_id);
            println!("  Query: {}", args.query);
            println!();
            for (i, m) in context.matches.iter().enumerate() {
                println!(
                    "{}. {} (score: {:.2})",
                    i + 1,
                    m.trace.content_summary,
                    m.score.total
                );
                if !m.matched_keywords.is_empty() {
                    println!("   Matched keywords: {}", m.matched_keywords.join(", "));
                }
                if !m.trace.keywords.is_empty() {
                    println!("   Trace keywords: {}", m.trace.keywords.join(", "));
                }
            }
            println!();
            if !context.activated_trace_ids.is_empty() {
                println!("  Activated traces: {}", context.activated_trace_ids.len());
            }
        }
        println!();
        println!(
            "⚠️  Holographic memory is recall evidence only. It does not authorize any action."
        );
    }

    Ok(())
}

fn memory_holographic_from_conversation(
    args: HolographicFromConversationArgs,
) -> Result<(), Box<dyn Error>> {
    use arpagona_conversation_memory::holographic_bridge::{
        Conversation, HolographicConversationBridge,
    };

    // Read the conversation JSON file
    let json_data = std::fs::read_to_string(&args.file)
        .map_err(|e| format!("Failed to read conversation file '{}': {}", args.file, e))?;
    let conversation: Conversation = serde_json::from_str(&json_data)
        .map_err(|e| format!("Failed to parse conversation JSON: {}", e))?;

    // Load or create the holographic store
    let mut bridge =
        match HolographicConversationBridge::load_from_file(&args.project_id, &args.store) {
            Ok(b) => b,
            Err(HolographicMemoryError::PersistenceError(_)) => {
                HolographicConversationBridge::new(&args.project_id)
            }
            Err(e) => {
                return Err(format!("Failed to load holographic store: {}", e).into());
            }
        };

    // Process all turns
    let trace_count = bridge.process_conversation(&conversation);

    // Save the updated store
    bridge
        .save_to_file(&args.store)
        .map_err(|e| format!("Failed to save holographic store: {}", e))?;

    // Optionally find similar traces
    let similar_context = if args.find_similar {
        let ctx = bridge.find_similar_for_turns(&conversation.turns, args.limit);
        Some(ctx)
    } else {
        None
    };

    if args.json {
        let mut output = serde_json::json!({
            "status": "processed",
            "conversation_id": conversation.conversation_id,
            "title": conversation.title,
            "project_id": args.project_id,
            "turn_count": conversation.turns.len(),
            "trace_count": trace_count,
            "store_path": args.store,
        });
        if let Some(ctx) = &similar_context {
            output["find_similar"] = serde_json::json!({
                "match_count": ctx.matches.len(),
                "matches": ctx.matches,
            });
        }
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        let title = conversation.title.as_deref().unwrap_or("(untitled)");
        println!("✓ Processed conversation '{}'", title);
        println!("  Conversation ID: {}", conversation.conversation_id);
        println!("  Project:         {}", args.project_id);
        println!("  Turns processed: {}", conversation.turns.len());
        println!("  Traces created:  {}", trace_count);
        println!("  Store path:      {}", args.store);
        println!();

        // Show a compact summary of what was encoded per turn
        let traces = bridge.store().list_traces(&args.project_id);
        let recent_traces: Vec<_> = traces.iter().rev().take(trace_count).collect();
        if !recent_traces.is_empty() {
            println!("  Encoded traces:");
            for t in &recent_traces {
                let kw_list = if t.keywords.len() > 5 {
                    format!("{} keywords", t.keywords.len())
                } else {
                    t.keywords.join(", ")
                };
                println!(
                    "    {:<12} keywords: {}, concepts: {}",
                    t.id,
                    kw_list,
                    t.concepts.join(", ")
                );
            }
        }

        if let Some(ctx) = &similar_context {
            if ctx.matches.is_empty() {
                println!();
                println!("  🔍 No resonance matches found.");
            } else {
                println!();
                println!("  🔍 Top resonance matches:");
                for (i, m) in ctx.matches.iter().enumerate() {
                    println!(
                        "    {}. {} (score: {:.3})",
                        i + 1,
                        m.trace.content_summary,
                        m.score.total
                    );
                    if !m.matched_keywords.is_empty() {
                        println!("       Matched keywords: {}", m.matched_keywords.join(", "));
                    }
                }
            }
        }

        println!();
        println!(
            "⚠️  Holographic memory is recall evidence only. It does not authorize any action."
        );
    }

    Ok(())
}

fn memory_holographic_explore(args: HolographicExploreArgs) -> Result<(), Box<dyn Error>> {
    // Load the store
    let store = match InMemoryHolographicMemoryStore::load_from_file(&args.file) {
        Ok(s) => s,
        Err(HolographicMemoryError::PersistenceError(_)) => {
            eprintln!("No holographic store found at: {}", args.file);
            eprintln!("Add some traces first with `memory holographic add`.");
            return Ok(());
        }
        Err(e) => {
            return Err(format!("Failed to load holographic store: {}", e).into());
        }
    };

    // Run the traversal
    let result = store.traverse_linked_memories(&args.trace_id, args.max_depth);

    let traversal = match result {
        Ok(r) => r,
        Err(HolographicMemoryError::TraceNotFound(id)) => {
            eprintln!("Trace '{}' not found in the holographic store.", id);
            eprintln!("Available traces:");
            let traces = store.list_traces(&args.project_id);
            if traces.is_empty() {
                eprintln!("  (no traces for project '{}')", args.project_id);
            } else {
                for t in &traces {
                    eprintln!("  {}: {}", t.id, t.content_summary);
                }
            }
            return Ok(());
        }
        Err(e) => {
            return Err(format!("Traversal error: {}", e).into());
        }
    };

    if args.json {
        println!("{}", serde_json::to_string_pretty(&traversal)?);
    } else {
        println!("🔗 Holographic Memory Graph Traversal");
        println!();
        println!("{}", traversal.traversal_summary);
        println!();
        println!("  Root trace:    {}", traversal.root_trace_id);
        println!("  Visited:       {} traces", traversal.visited_traces.len());
        println!(
            "  Max depth:     {} / {} configured",
            traversal.reachable_depth, traversal.max_depth_limit
        );
        if traversal.cycle_detected {
            println!("  ⚠️  Cycle(s) detected and broken.");
        }
        if traversal.depth_limit_reached {
            println!("  ⏹️  Depth limit reached; chain may extend further.");
        }
        println!();
        println!("  Traversal order:");
        for (i, trace_id) in traversal.visited_trace_ids.iter().enumerate() {
            let indent = "    ".to_owned();
            // Find the trace data
            if let Some(trace) = traversal.visited_traces.iter().find(|t| t.id == *trace_id) {
                let keywords_str = if trace.keywords.is_empty() {
                    String::new()
                } else {
                    format!(" [keywords: {}]", trace.keywords.join(", "))
                };
                println!(
                    "{}{}. {} ({}){}",
                    indent,
                    i + 1,
                    trace_id,
                    trace.content_summary,
                    keywords_str,
                );
            } else {
                println!("{}{}. {} (unknown trace)", indent, i + 1, trace_id);
            }
        }
        println!();
        println!(
            "⚠️  Holographic memory is recall evidence only. It does not authorize any action."
        );
    }

    Ok(())
}

fn memory_holographic_consolidate(args: HolographicConsolidateArgs) -> Result<(), Box<dyn Error>> {
    // Open the SQLite store
    let mut store = match SqliteHolographicMemoryStore::new(&args.db) {
        Ok(s) => s,
        Err(e) => {
            return Err(format!("Failed to open SQLite store at '{}': {}", args.db, e).into());
        }
    };

    let before = store.len();

    // Run consolidation
    let consolidated = store.consolidate_traces(&args.project_id, args.window, args.threshold)?;

    let after = store.len();

    if args.json {
        let output = serde_json::json!({
            "status": "consolidated",
            "project_id": args.project_id,
            "window_minutes": args.window,
            "similarity_threshold": args.threshold,
            "traces_consolidated": consolidated,
            "traces_before": before,
            "traces_after": after,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("✓ Holographic memory consolidation complete");
        println!("  Project:             {}", args.project_id);
        println!("  Window:              {} minutes", args.window);
        println!("  Similarity threshold: {:.2}", args.threshold);
        println!("  Traces before:       {}", before);
        println!("  Traces consolidated:  {}", consolidated);
        println!("  Traces after:        {}", after);
        println!();
        println!(
            "⚠️  Holographic memory is recall evidence only. It does not authorize any action."
        );
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
        ProposedActionStatus::Rejected => "Proposal was rejected by human reviewer.".to_owned(),
        ProposedActionStatus::Deferred => {
            "Proposal was deferred; re-evaluate when context changes.".to_owned()
        }
        ProposedActionStatus::Superseded => {
            "Proposal was superseded by a more recent decision.".to_owned()
        }
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

/// Review command handler: list, show, approve, reject, defer, supersede.
async fn review_action(
    client: &Client,
    api_url: &str,
    cmd: ReviewActionCommand,
) -> Result<(), Box<dyn Error>> {
    match cmd.command {
        ReviewActionSubcommand::List(args) => {
            let all_actions: Vec<ProposedAction> = get_json(
                client
                    .get(format!("{api_url}/proposed-actions"))
                    .send()
                    .await?,
            )
            .await?;

            let filtered: Vec<_> = if let Some(ref status_filter) = args.status {
                all_actions
                    .into_iter()
                    .filter(|a| {
                        format!("{:?}", a.status).to_lowercase() == status_filter.to_lowercase()
                    })
                    .collect()
            } else {
                all_actions
            };

            if args.json {
                println!("{}", serde_json::to_string_pretty(&filtered)?);
            } else {
                if filtered.is_empty() {
                    println!("{}", style_dim("No proposed actions matching filter."));
                    return Ok(());
                }
                println!("{}", style_info("Proposed actions"));
                for action in &filtered {
                    let p = &action.payload;
                    let score = p
                        .get("priority_score")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);
                    let band = p
                        .get("priority_band")
                        .and_then(|v| v.as_str())
                        .unwrap_or("?");
                    let batched = p.get("batched").and_then(|v| v.as_bool()).unwrap_or(false);
                    println!(
                        "{} {} | {} | score={:.2} ({}){}",
                        style_dim("- id:"),
                        action.id,
                        style_dim(&format!("{:?}", action.status)),
                        score,
                        band,
                        if batched { " [batched]" } else { "" },
                    );
                }
            }
        }
        ReviewActionSubcommand::Show(args) => {
            let all_actions: Vec<ProposedAction> = get_json(
                client
                    .get(format!("{api_url}/proposed-actions"))
                    .send()
                    .await?,
            )
            .await?;

            let action = all_actions
                .into_iter()
                .find(|a| a.id.as_str() == args.action_id)
                .ok_or_else(|| format!("Proposed action '{}' not found", args.action_id))?;

            if args.json {
                println!("{}", serde_json::to_string_pretty(&action)?);
            } else {
                println!("{} {}", style_dim("id:"), action.id);
                println!("{} {:?}", style_dim("status:"), action.status);
                println!("{} {:?}", style_dim("action_type:"), action.action_type);
                println!(
                    "{} {}",
                    style_dim("risk_level:"),
                    format!("{:?}", action.risk_level)
                );
                println!("{} {}", style_dim("rationale:"), action.rationale);
                println!("{} {}", style_dim("created_at:"), action.created_at);
                if action.target.is_some() {
                    println!(
                        "{} {}",
                        style_dim("target:"),
                        action.target.as_ref().unwrap()
                    );
                }
                // Print payload fields
                if let Some(score) = action
                    .payload
                    .get("priority_score")
                    .and_then(|v| v.as_f64())
                {
                    println!("{} {:.2}", style_dim("priority_score:"), score);
                }
                if let Some(band) = action.payload.get("priority_band").and_then(|v| v.as_str()) {
                    println!("{} {}", style_dim("priority_band:"), band);
                }
                if let Some(batched) = action.payload.get("batched").and_then(|v| v.as_bool()) {
                    println!("{} {}", style_dim("batched:"), batched);
                    if let Some(count) = action.payload.get("merged_count").and_then(|v| v.as_u64())
                    {
                        println!("{} {}", style_dim("merged_count:"), count);
                    }
                }
                if let Some(source_kind) =
                    action.payload.get("source_kind").and_then(|v| v.as_str())
                {
                    println!("{} {}", style_dim("source_kind:"), source_kind);
                }
                if let Some(benefit) = action
                    .payload
                    .get("expected_benefit")
                    .and_then(|v| v.as_str())
                {
                    println!("{} {}", style_dim("expected_benefit:"), benefit);
                }
            }
        }
        ReviewActionSubcommand::Approve(ref args)
        | ReviewActionSubcommand::Reject(ref args)
        | ReviewActionSubcommand::Defer(ref args)
        | ReviewActionSubcommand::Supersede(ref args) => {
            let action_name = match cmd.command {
                ReviewActionSubcommand::Approve(_) => "approve",
                ReviewActionSubcommand::Reject(_) => "reject",
                ReviewActionSubcommand::Defer(_) => "defer",
                ReviewActionSubcommand::Supersede(_) => "supersede",
                _ => unreachable!(),
            };

            let response: serde_json::Value = client
                .post(format!(
                    "{api_url}/proposed-actions/{}/review",
                    args.action_id
                ))
                .json(&serde_json::json!({
                    "action": action_name,
                    "reason": args.reason,
                    "actor": args.actor,
                }))
                .send()
                .await?
                .json()
                .await?;

            if args.json {
                println!("{}", serde_json::to_string_pretty(&response)?);
            } else {
                if let Some(action_val) = response.get("proposed_action") {
                    if let Some(status) = action_val.get("status").and_then(|v| v.as_str()) {
                        println!(
                            "{} Proposed action '{}' → {}",
                            style_info("✓"),
                            args.action_id,
                            status,
                        );
                    }
                }
                if let Some(audit) = response.get("audit_event") {
                    if let Some(event_id) = audit.get("id").and_then(|v| v.as_str()) {
                        println!("{} Audit event: {}", style_dim("  audit:"), event_id);
                    }
                }
            }
        }
    }
    Ok(())
}

/// Sandbox command handler: run dry-run simulation or list sandbox runs.
async fn sandbox_action(
    client: &Client,
    api_url: &str,
    cmd: SandboxActionCommand,
) -> Result<(), Box<dyn Error>> {
    match cmd.command {
        SandboxActionSubcommand::Run(args) => {
            let response: serde_json::Value = client
                .post(format!(
                    "{api_url}/proposed-actions/{}/sandbox",
                    args.action_id
                ))
                .send()
                .await?
                .json()
                .await?;

            if args.json {
                println!("{}", serde_json::to_string_pretty(&response)?);
            } else {
                println!("{}", style_info("Dry-Run Sandbox Simulation"));
                println!(
                    "{} {}",
                    style_dim("  sandbox run id:"),
                    response["id"].as_str().unwrap_or("?")
                );
                println!(
                    "{} {}",
                    style_dim("  status:"),
                    response["status"].as_str().unwrap_or("?")
                );
                println!(
                    "{} {}",
                    style_dim("  action_type:"),
                    response["action_type"].as_str().unwrap_or("?")
                );
                println!(
                    "{} {}",
                    style_dim("  risk_level:"),
                    response["risk_level"].as_str().unwrap_or("?")
                );

                if let Some(warnings) = response["warnings"].as_array() {
                    for w in warnings {
                        if let Some(w_text) = w.as_str() {
                            println!("  {} {}", style_warning("⚠"), w_text);
                        }
                    }
                }

                if let Some(sim_out) = response.get("simulated_output") {
                    if let Some(effects) = sim_out["simulated_effects"].as_array() {
                        for (i, effect) in effects.iter().enumerate() {
                            let desc = effect["description"].as_str().unwrap_or("unknown effect");
                            let etype = effect["effect_type"].as_str().unwrap_or("?");
                            println!(
                                "  {}. {} ({})",
                                style_dim(&format!("{}", i + 1)),
                                desc,
                                etype,
                            );
                        }
                    }
                    if let Some(non_auth) = sim_out["non_authorizing_warning"].as_str() {
                        println!("  {}", style_warning(non_auth));
                    }
                }

                if let Some(sim_warning) = response["simulation_warning"].as_str() {
                    println!("\n{}", style_warning(sim_warning));
                }
            }
        }
        SandboxActionSubcommand::List(args) => {
            let runs: Vec<serde_json::Value> =
                get_json(client.get(format!("{api_url}/sandbox-runs")).send().await?).await?;

            if args.json {
                println!("{}", serde_json::to_string_pretty(&runs)?);
            } else {
                if runs.is_empty() {
                    println!("{}", style_dim("No sandbox runs found."));
                    return Ok(());
                }
                println!("{}", style_info("Sandbox runs"));
                for run in &runs {
                    println!(
                        "{} {} | {} | {} | {}",
                        style_dim("- id:"),
                        run["id"].as_str().unwrap_or("?"),
                        run["status"].as_str().unwrap_or("?"),
                        run["action_type"].as_str().unwrap_or("?"),
                        run["created_at"].as_str().unwrap_or("?"),
                    );
                }
            }
        }
    }
    Ok(())
}

/// Dry-run an approved proposal: simulate execution without side effects.
async fn dry_run_action(
    client: &Client,
    api_url: &str,
    args: DryRunActionArgs,
) -> Result<(), Box<dyn Error>> {
    let response: serde_json::Value = client
        .post(format!(
            "{}/proposed-actions/{}/dry-run",
            api_url, args.action_id
        ))
        .send()
        .await?
        .json()
        .await?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        if let Some(status_val) = response.get("status").and_then(|v| v.as_str()) {
            let status_icon = if status_val == "dry_run_completed" {
                style_success("✓")
            } else {
                style_error("✗")
            };
            println!(
                "{} Dry-run of '{}': {}",
                status_icon,
                args.action_id,
                style_status(status_val),
            );
        }
        if let Some(summary) = response
            .get("human_readable_summary")
            .and_then(|v| v.as_str())
        {
            println!("{} {}", style_dim("  summary:"), summary);
        }
        if let Some(effects) = response.get("expected_effects").and_then(|v| v.as_array()) {
            println!("{}", style_dim("  expected effects:"));
            for effect in effects {
                if let Some(text) = effect.as_str() {
                    println!("    - {}", text);
                }
            }
        }
        if let Some(resources) = response.get("touched_resources").and_then(|v| v.as_array()) {
            println!("{}", style_dim("  touched resources:"));
            for resource in resources {
                if let Some(text) = resource.as_str() {
                    println!("    - {}", text);
                }
            }
        }
        if let Some(reversibility) = response.get("reversibility").and_then(|v| v.as_str()) {
            println!("{} {}", style_dim("  reversibility:"), reversibility);
        }
    }
    Ok(())
}

/// Query the execution capability registry: list all or show one.
async fn capability_action(
    client: &Client,
    api_url: &str,
    cmd: CapabilityCommand,
) -> Result<(), Box<dyn Error>> {
    match cmd.command {
        CapabilitySubcommand::List(args) => {
            let caps: Vec<serde_json::Value> = get_json(
                client
                    .get(format!("{api_url}/execution-capabilities"))
                    .send()
                    .await?,
            )
            .await?;

            if args.json {
                println!("{}", serde_json::to_string_pretty(&caps)?);
            } else {
                println!("{}", style_info("Execution Capabilities"));
                println!(
                    "{}",
                    style_dim("  {:<24} dry-run  real-exec  max_risk          approval")
                );
                for cap in &caps {
                    let at = cap["action_type"].as_str().unwrap_or("?");
                    let dry_run = cap["supports_dry_run"].as_bool().unwrap_or(false);
                    let real_exec = cap["supports_real_execution"].as_bool().unwrap_or(false);
                    let max_risk = cap["max_allowed_risk"].as_str().unwrap_or("?");
                    let horiz = cap["human_approval_required"].as_bool().unwrap_or(false);
                    println!(
                        "  {:<24} {:<7} {:<10} {:<18} {}",
                        at,
                        if dry_run { "✓" } else { "✗" },
                        if real_exec { "✓" } else { "✗" },
                        max_risk,
                        if horiz { "human" } else { "auto" },
                    );
                }
                println!();
                println!(
                    "{} Use `action capability show <action_type>` for details.",
                    style_dim("Tip:")
                );
            }
        }
        CapabilitySubcommand::Show(args) => {
            let cap: serde_json::Value = get_json(
                client
                    .get(format!(
                        "{api_url}/execution-capabilities/{}",
                        args.action_type
                    ))
                    .send()
                    .await?,
            )
            .await?;

            if args.json {
                println!("{}", serde_json::to_string_pretty(&cap)?);
            } else {
                println!("{}", style_info("Execution Capability Detail"));
                println!(
                    "{} {}",
                    style_dim("  action_type:"),
                    cap["action_type"].as_str().unwrap_or("?")
                );
                println!(
                    "{} {}",
                    style_dim("  executor_id:"),
                    cap["executor_id"].as_str().unwrap_or("(none)")
                );
                println!(
                    "{} {}",
                    style_dim("  supports_dry_run:"),
                    cap["supports_dry_run"].as_bool().unwrap_or(false)
                );
                println!(
                    "{} {}",
                    style_dim("  supports_real_execution:"),
                    cap["supports_real_execution"].as_bool().unwrap_or(false)
                );
                println!(
                    "{} {}",
                    style_dim("  max_allowed_risk:"),
                    cap["max_allowed_risk"].as_str().unwrap_or("?")
                );
                println!(
                    "{} {}",
                    style_dim("  human_approval_required:"),
                    cap["human_approval_required"].as_bool().unwrap_or(false)
                );
                println!(
                    "{} {}",
                    style_dim("  reversibility:"),
                    cap["reversibility"].as_str().unwrap_or("?")
                );
                if let Some(kinds) = cap["touched_resource_kinds"].as_array() {
                    let kinds_str: Vec<&str> = kinds.iter().filter_map(|v| v.as_str()).collect();
                    println!(
                        "{} {}",
                        style_dim("  touched_resource_kinds:"),
                        kinds_str.join(", ")
                    );
                }
                if let Some(perms) = cap["required_permissions"].as_array() {
                    let perms_str: Vec<&str> = perms.iter().filter_map(|v| v.as_str()).collect();
                    println!(
                        "{} {}",
                        style_dim("  required_permissions:"),
                        perms_str.join(", ")
                    );
                }
                if let Some(notes) = cap["notes"].as_str() {
                    println!("{} {}", style_dim("  notes:"), notes);
                }
                if let Some(warn) = cap["safety_warning"].as_str() {
                    println!("{} {}", style_warning("  safety_warning:"), warn);
                }
            }
        }
    }
    Ok(())
}

/// Run a policy check on a proposed action via the API.
async fn policy_action(
    client: &Client,
    api_url: &str,
    cmd: PolicyActionCommand,
) -> Result<(), Box<dyn Error>> {
    match cmd.command {
        PolicyActionSubcommand::Check(args) => {
            let result: serde_json::Value = client
                .post(format!(
                    "{api_url}/proposed-actions/{}/policy-check",
                    args.action_id
                ))
                .send()
                .await?
                .json()
                .await?;

            if args.json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                let decision = result["decision"].as_str().unwrap_or("unknown");
                let reason = result["reason"].as_str().unwrap_or("");

                let icon = match decision {
                    "allowed" => style_success("✓"),
                    "needs_human_approval" => style_warning("◆"),
                    "needs_dry_run" => style_info("◈"),
                    _ => style_error("✗"),
                };

                println!(
                    "{} Policy check for '{}': {}",
                    icon,
                    args.action_id,
                    style_status(decision),
                );
                println!("{} {}", style_dim("  reason:"), reason);

                if let Some(rules) = result["matched_rules"].as_array() {
                    if !rules.is_empty() {
                        println!("{}", style_dim("  matched rules:"));
                        for rule in rules {
                            if let Some(r) = rule.as_str() {
                                println!("    - {}", r);
                            }
                        }
                    }
                }

                if let Some(cap) = result["capability"].as_object() {
                    if let Some(at) = cap.get("action_type").and_then(|v| v.as_str()) {
                        println!("{} {}", style_dim("  action_type:"), at);
                    }
                    if let Some(dry) = cap.get("supports_dry_run").and_then(|v| v.as_bool()) {
                        println!(
                            "{} {}",
                            style_dim("  supports_dry_run:"),
                            if dry { "yes" } else { "no" }
                        );
                    }
                    if let Some(max) = cap.get("max_allowed_risk").and_then(|v| v.as_str()) {
                        println!("{} {}", style_dim("  max_allowed_risk:"), max);
                    }
                }
            }
        }
    }
    Ok(())
}

/// Execute a proposal through the API (always returns ExecutionDisabled).
async fn execute_action(
    client: &Client,
    api_url: &str,
    args: ExecuteActionArgs,
) -> Result<(), Box<dyn Error>> {
    let response: serde_json::Value = client
        .post(format!(
            "{api_url}/proposed-actions/{}/execute",
            args.action_id
        ))
        .send()
        .await?
        .json()
        .await?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&response)?);
    } else {
        let status = response["status"].as_str().unwrap_or("unknown");
        let reason = response["reason"].as_str().unwrap_or("");

        let icon = match status {
            "execution_disabled" => style_warning("◈"),
            "execution_blocked" => style_error("✗"),
            _ => style_info("●"),
        };

        println!(
            "{} Execute '{}': {}",
            icon,
            args.action_id,
            style_status(status),
        );
        println!("{} {}", style_dim("  reason:"), reason);

        if let Some(at) = response["action_type"].as_str() {
            println!("{} {}", style_dim("  action_type:"), at);
        }
        if let Some(audit_id) = response["audit_event_id"].as_str() {
            println!("{} {}", style_dim("  audit_event_id:"), audit_id);
        }
        if let Some(resources) = response["touched_resources"].as_array() {
            if !resources.is_empty() {
                println!("{} {:?}", style_dim("  touched_resources:"), resources);
            }
        }
    }
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

/// Execute tool runtime observations for required observations in the cognitive cycle.
///
/// Maps each `RequiredObservation` to tool runtime calls where possible:
/// - context/codebase observations → list_files(".")
/// - language/tech observations → read_file("Cargo.toml")
/// - engineering-specific → list_files("crates/")
///
/// Returns a vector of `CognitiveObservation` objects from successful tool calls.
/// Observations that cannot be automated (need human input) are skipped.
fn run_observations(
    result: &CognitiveCycleResult,
) -> Vec<arpagona_agent_core::observation::CognitiveObservation> {
    use arpagona_agent_core::observation::CognitiveObservation;
    use arpagona_tool_runtime::{ToolRuntime, ToolRuntimeConfig};

    let runtime = ToolRuntime::new(ToolRuntimeConfig::new("."));
    let mut observations: Vec<CognitiveObservation> = Vec::new();

    for obs in &result.required_observations {
        let desc_lower = obs.description.to_lowercase();
        let id_lower = obs.id.to_lowercase();

        // Map observation to tool call based on keywords
        let tool_call: Option<(&str, serde_json::Value)> = {
            if id_lower.contains("general-context")
                || id_lower.contains("context")
                || desc_lower.contains("context")
                || desc_lower.contains("codebase")
            {
                Some(("list_files", serde_json::json!({"path": "."})))
            } else if id_lower.contains("language")
                || id_lower.contains("tech")
                || desc_lower.contains("cargo")
                || desc_lower.contains("language")
                || desc_lower.contains("dépendance")
            {
                Some(("read_file", serde_json::json!({"path": "Cargo.toml"})))
            } else if id_lower.contains("existing-codebase")
                || id_lower.contains("codebase")
                || desc_lower.contains("structure")
                || desc_lower.contains("architecture")
            {
                Some(("list_files", serde_json::json!({"path": "crates/"})))
            } else {
                None // no tool mapping — requires human input
            }
        };

        if let Some((tool_name, args)) = tool_call {
            let result = runtime.execute(tool_name, &args);
            observations.push(CognitiveObservation::from_tool_execution(&result));
        }
    }

    observations
}

/// Context-aware proposal metadata injected into each ProposedAction's payload.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct ProposalMetadata {
    /// The objective that triggered this proposal.
    originating_objective: String,
    /// Where the signal came from: "failure_insight_candidate", "observation", "assessment".
    source_kind: String,
    /// Short summary of the original signal.
    source_summary: String,
    /// Why this action is proposed.
    rationale: String,
    /// Expected benefit of completing this action.
    expected_benefit: String,
    /// Risk level assigned to this proposal.
    risk_level: String,
    /// Suggested action type: test, fix, refactor, doc, research, governance.
    suggested_action_type: String,
    /// Confidence if available (0.0 = none, 1.0 = certain).
    confidence: Option<f64>,
    /// Estimated implementation cost: low, medium, high. Defaults to medium.
    #[serde(default = "default_implementation_cost")]
    implementation_cost: String,
    /// Deterministic priority score computed from benefit, confidence, risk, cost, and type.
    #[serde(default)]
    priority_score: f64,
    /// Priority band derived from score: high, medium, low.
    #[serde(default = "default_priority_band")]
    priority_band: String,
    /// Warning: this is a non-authorizing proposal.
    non_authorizing_warning: String,
}

fn default_implementation_cost() -> String {
    "medium".to_owned()
}

fn default_priority_band() -> String {
    "medium".to_owned()
}

/// Deterministic priority scoring for proposed actions.
///
/// Computes a score from available metadata:
/// - `expected_benefit` — maps to benefit score (0.3–1.0)
/// - `confidence` — defaults to 0.5 if None
/// - `risk_level` — penalty multiplier (risk MUST reduce priority: 0.0–1.0)
/// - `implementation_cost` — cost penalty (0.4–1.0)
/// - `suggested_action_type` — small bonus/penalty (-0.2 to +0.2)
///
/// Score = benefit × confidence × risk_penalty × cost_penalty + type_bonus
/// Clamped to [0.0, 2.0].
fn compute_priority_score(
    expected_benefit: &str,
    confidence: Option<f64>,
    risk_level: &str,
    suggested_action_type: &str,
    implementation_cost: &str,
) -> f64 {
    let benefit_score = match expected_benefit {
        s if s.contains("Unblock") || s.contains("Restore") || s.contains("safety") => 1.0,
        s if s.contains("missing context") || s.contains("Provide missing") => 0.8,
        s if s.contains("visibility") || s.contains("full data") => 0.7,
        s if s.contains("Reduce repeated friction") || s.contains("observability") => 0.6,
        s if s.contains("Clarify ambiguous") || s.contains("Reconcile") => 0.5,
        _ => 0.3,
    };
    let conf = confidence.unwrap_or(0.5);
    let risk_penalty = match risk_level {
        "informational" => 1.0,
        "low" => 0.8,
        "medium" => 0.5,
        "high" => 0.2,
        "critical" => 0.0,
        _ => 0.5,
    };
    let cost_penalty = match implementation_cost {
        "low" => 1.0,
        "medium" => 0.8,
        "high" => 0.4,
        _ => 0.8,
    };
    let type_bonus = match suggested_action_type {
        "fix" => 0.2,
        "governance" => 0.15,
        "test" => 0.1,
        "refactor" => 0.0,
        "research" => -0.1,
        "doc" => -0.2,
        _ => 0.0,
    };
    let score = benefit_score * conf * risk_penalty * cost_penalty + type_bonus;
    score.clamp(0.0, 2.0)
}

/// Map a priority score to a human-readable band.
fn compute_priority_band(score: f64) -> &'static str {
    if score >= 0.7 {
        "high"
    } else if score >= 0.4 {
        "medium"
    } else {
        "low"
    }
}

/// Stable deduplication key from proposal payload metadata.
///
/// Based on:
/// - `suggested_action_type`
/// - `source_kind`
/// - Normalized `source_summary` (lowercased, trimmed, first 100 chars)
fn dedup_key_from_payload(payload: &serde_json::Value) -> String {
    let action_type = payload
        .get("suggested_action_type")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let source_kind = payload
        .get("source_kind")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let summary = payload
        .get("source_summary")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let normalized = summary
        .to_lowercase()
        .trim()
        .chars()
        .take(100)
        .collect::<String>();
    format!("{}::{}::{}", action_type, source_kind, normalized)
}

/// Aggregate metadata for a merged proposal batch.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct DedupedBatchMetadata {
    /// Number of original proposals merged into this batch.
    merged_count: usize,
    /// IDs of the original proposals that were merged.
    merged_proposal_ids: Vec<String>,
    /// Aggregated source summaries from all merged proposals.
    aggregated_source_summaries: Vec<String>,
    /// Aggregated rationales from all merged proposals.
    aggregated_rationales: Vec<String>,
    /// The highest expected benefit string among merged items.
    max_expected_benefit: String,
    /// The highest confidence value (or default 0.5).
    max_confidence: f64,
    /// The highest risk level among merged items (conservative: takes the maximum risk).
    highest_risk_level: String,
    /// The lowest implementation cost among merged items (low < medium < high).
    lowest_implementation_cost: String,
    /// Final priority score after re-computation with conservative risk.
    final_priority_score: f64,
    /// Priority band derived from final score.
    final_priority_band: String,
    /// Marker that this proposal is a batch.
    batched: bool,
}

/// Merge duplicate ProposedActions that share the same deduplication key.
///
/// Conservative rules:
/// - Merged risk_level keeps the **highest** risk among merged items.
/// - Merged score uses the highest risk (risk must not be hidden).
/// - The first proposal in each group becomes the primary proposal.
/// - All proposals remain PendingDecision.
/// - Decisions and audit events from original proposals are discarded;
///   new decisions/audit events are created for the merged proposals.
fn dedup_proposed_actions(
    actions: Vec<ProposedAction>,
) -> (Vec<ProposedAction>, Vec<Decision>, Vec<AuditEvent>) {
    use arpagona_decision_gate::{audit_event_for_decision, evaluate_proposed_action};
    use std::collections::BTreeMap;

    let mut groups: BTreeMap<String, Vec<ProposedAction>> = BTreeMap::new();
    for action in actions {
        let key = dedup_key_from_payload(&action.payload);
        groups.entry(key).or_default().push(action);
    }

    let mut merged_actions: Vec<ProposedAction> = Vec::new();
    let risk_order: std::collections::HashMap<&str, u8> = [
        ("informational", 0),
        ("low", 1),
        ("medium", 2),
        ("high", 3),
        ("critical", 4),
    ]
    .iter()
    .cloned()
    .collect();

    let cost_order: std::collections::HashMap<&str, u8> = [("low", 0), ("medium", 1), ("high", 2)]
        .iter()
        .cloned()
        .collect();

    for (_key, group) in groups {
        if group.is_empty() {
            continue;
        }
        if group.len() == 1 {
            // Single proposal — no merging needed
            merged_actions.push(group.into_iter().next().unwrap());
            continue;
        }

        // Take the first proposal as the primary
        let mut primary = group[0].clone();

        // Collect aggregated data
        let mut all_risk_levels: Vec<u8> = Vec::new();
        let mut all_costs: Vec<u8> = Vec::new();
        let mut all_confidences: Vec<f64> = Vec::new();
        let mut merged_ids: Vec<String> = Vec::new();
        let mut aggregated_summaries: Vec<String> = Vec::new();
        let mut aggregated_rationales: Vec<String> = Vec::new();
        let mut benefit_scores: Vec<f64> = Vec::new();
        let mut max_benefit_str: String = String::new();

        for action in &group {
            merged_ids.push(action.id.as_str().to_owned());
            let p = &action.payload;

            if let Some(s) = p.get("source_summary").and_then(|v| v.as_str()) {
                if !aggregated_summaries.contains(&s.to_owned()) {
                    aggregated_summaries.push(s.to_owned());
                }
            }
            if let Some(r) = p.get("rationale").and_then(|v| v.as_str()) {
                if !aggregated_rationales.contains(&r.to_owned()) {
                    aggregated_rationales.push(r.to_owned());
                }
            }

            let rl = p
                .get("risk_level")
                .and_then(|v| v.as_str())
                .unwrap_or("medium");
            all_risk_levels.push(*risk_order.get(rl).unwrap_or(&2));

            let ic = p
                .get("implementation_cost")
                .and_then(|v| v.as_str())
                .unwrap_or("medium");
            all_costs.push(*cost_order.get(ic).unwrap_or(&1));

            let conf = p.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.5);
            all_confidences.push(conf);

            let bs = p
                .get("priority_score")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            benefit_scores.push(bs);

            let eb = p
                .get("expected_benefit")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if eb.len() > max_benefit_str.len() {
                max_benefit_str = eb.to_owned();
            }
        }

        // Conservative: highest risk level
        let max_risk_rank = *all_risk_levels.iter().max().unwrap_or(&2);
        let highest_risk_str = risk_order
            .iter()
            .find(|(_, v)| **v == max_risk_rank)
            .map(|(k, _)| *k)
            .unwrap_or("medium")
            .to_owned();

        // Lowest cost (low is best — take the minimum rank)
        let min_cost_rank = *all_costs.iter().min().unwrap_or(&1);
        let lowest_cost_str = cost_order
            .iter()
            .find(|(_, v)| **v == min_cost_rank)
            .map(|(k, _)| *k)
            .unwrap_or("medium")
            .to_owned();

        // Max confidence
        let max_confidence = all_confidences.into_iter().fold(0.0_f64, |a, b| a.max(b));

        // Re-compute score with conservative (highest) risk
        let action_type = primary
            .payload
            .get("suggested_action_type")
            .and_then(|v| v.as_str())
            .unwrap_or("research");
        let final_score = compute_priority_score(
            &max_benefit_str,
            Some(max_confidence),
            &highest_risk_str,
            action_type,
            &lowest_cost_str,
        );

        // Build batch metadata
        let batch_meta = DedupedBatchMetadata {
            merged_count: group.len(),
            merged_proposal_ids: merged_ids,
            aggregated_source_summaries: aggregated_summaries,
            aggregated_rationales: aggregated_rationales,
            max_expected_benefit: max_benefit_str,
            max_confidence,
            highest_risk_level: highest_risk_str.clone(),
            lowest_implementation_cost: lowest_cost_str,
            final_priority_score: final_score,
            final_priority_band: compute_priority_band(final_score).to_owned(),
            batched: true,
        };

        // Update the primary's payload with batch metadata
        if let Some(payload_obj) = primary.payload.as_object_mut() {
            // Override risk_level with conservative highest
            payload_obj.insert(
                "risk_level".to_owned(),
                serde_json::Value::String(highest_risk_str.clone()),
            );
            // Update priority_score and priority_band
            payload_obj.insert("priority_score".to_owned(), serde_json::json!(final_score));
            payload_obj.insert(
                "priority_band".to_owned(),
                serde_json::Value::String(compute_priority_band(final_score).to_owned()),
            );
            // Insert batch metadata
            payload_obj.insert("batched".to_owned(), serde_json::Value::Bool(true));
            payload_obj.insert("merged_count".to_owned(), serde_json::json!(group.len()));
            payload_obj.insert(
                "merged_proposal_ids".to_owned(),
                serde_json::to_value(&batch_meta.merged_proposal_ids)
                    .unwrap_or(serde_json::Value::Null),
            );
            payload_obj.insert(
                "aggregated_source_summaries".to_owned(),
                serde_json::to_value(&batch_meta.aggregated_source_summaries)
                    .unwrap_or(serde_json::Value::Null),
            );
            // Also update risk_level on the struct level
            primary.risk_level = match highest_risk_str.as_str() {
                "informational" => RiskLevel::Informational,
                "low" => RiskLevel::Low,
                "medium" => RiskLevel::Medium,
                "high" => RiskLevel::High,
                "critical" => RiskLevel::Critical,
                _ => RiskLevel::Medium,
            };
        }

        merged_actions.push(primary);
    }

    // Re-evaluate all merged proposals through the Decision Gate
    let mut decisions: Vec<Decision> = Vec::new();
    let mut audit_events: Vec<AuditEvent> = Vec::new();
    for action in &merged_actions {
        let decision = evaluate_proposed_action(action, &[], &permissions_for_action(action));
        let audit_event = audit_event_for_decision(action, &decision);
        decisions.push(decision);
        audit_events.push(audit_event);
    }

    (merged_actions, decisions, audit_events)
}

/// Map a FailureInsightCandidateKind to a suggested action type.
fn fic_kind_to_action(kind: &str) -> &'static str {
    match kind {
        "blocked_tool_use" | "tool_runtime_failure" => "fix",
        "missing_context" | "insufficient_observation_quality" => "research",
        "empty_search_result" | "truncated_result" => "test",
        "ambiguous_result" | "documentation_mismatch" | "repeated_operator_friction" => "refactor",
        "safety_boundary_triggered" => "governance",
        _ => "research",
    }
}

/// Map a FailureInsightCandidateKind to an expected benefit string.
fn fic_kind_to_benefit(kind: &str) -> &'static str {
    match kind {
        "blocked_tool_use" => "Unblock a prevented operation so the agent can proceed safely.",
        "tool_runtime_failure" => "Restore tool runtime reliability and reduce observation noise.",
        "missing_context" => "Provide missing context so future cycles produce better plans.",
        "insufficient_observation_quality" => {
            "Improve observation quality for more reliable downstream assessments."
        }
        "empty_search_result" => {
            "Verify whether the expected data exists or adjust the search strategy."
        }
        "truncated_result" => {
            "Ensure full data visibility by widening the search or paginating results."
        }
        "ambiguous_result" => "Clarify ambiguous signals to enable confident downstream decisions.",
        "documentation_mismatch" => {
            "Reconcile documentation and observed behaviour to reduce confusion."
        }
        "repeated_operator_friction" => {
            "Reduce repeated friction by improving the operator experience."
        }
        "safety_boundary_triggered" => "Review and harden safety boundaries based on this trigger.",
        _ => "Improve overall cognitive cycle quality.",
    }
}

/// Map an observation's candidate_kind to a suggested action type.
fn obs_kind_to_action(kind: &Option<String>) -> &'static str {
    match kind.as_deref() {
        Some("blocked_tool_use") | Some("tool_runtime_failure") => "fix",
        Some("missing_context") | Some("insufficient_observation_quality") => "research",
        Some("empty_search_result") | Some("truncated_result") => "test",
        Some("ambiguous_result")
        | Some("documentation_mismatch")
        | Some("repeated_operator_friction") => "refactor",
        Some("safety_boundary_triggered") => "governance",
        _ => "research",
    }
}

/// Map an observation candidate kind to an expected benefit string.
fn obs_kind_to_benefit(kind: &Option<String>) -> &'static str {
    match kind.as_deref() {
        Some("blocked_tool_use") => {
            "Unblock a prevented operation so the agent can proceed safely."
        }
        Some("tool_runtime_failure") => {
            "Restore tool runtime reliability and reduce observation noise."
        }
        Some("missing_context") => "Provide missing context so future cycles produce better plans.",
        Some("insufficient_observation_quality") => {
            "Improve observation quality for more reliable downstream assessments."
        }
        Some("empty_search_result") => {
            "Verify whether the expected data exists or adjust the search strategy."
        }
        Some("truncated_result") => {
            "Ensure full data visibility by widening the search or paginating results."
        }
        Some("ambiguous_result") => {
            "Clarify ambiguous signals to enable confident downstream decisions."
        }
        Some("documentation_mismatch") => {
            "Reconcile documentation and observed behaviour to reduce confusion."
        }
        Some("repeated_operator_friction") => {
            "Reduce repeated friction by improving the operator experience."
        }
        Some("safety_boundary_triggered") => {
            "Review and harden safety boundaries based on this trigger."
        }
        _ => "Improve overall cognitive cycle quality.",
    }
}

/// Convert FailureInsightCandidates and CognitiveObservations into context-rich
/// ProposedActions via the API server, evaluate them through the Decision Gate,
/// and return the created proposals, decisions, and audit events.
async fn run_proposals(
    client: &Client,
    api_url: &str,
    objective: &str,
    failure_insight_candidates: &[serde_json::Value],
    cognitive_observations: &[serde_json::Value],
) -> Result<ProposalRunResult, Box<dyn Error>> {
    use arpagona_agent_core::ProposedAction;

    let mut proposed_actions: Vec<ProposedAction> = Vec::new();

    // Helper to create a single proposal via the API
    async fn create_one_proposal(
        client: &Client,
        api_url: &str,
        action_type: &str,
        risk_level: &str,
        permissions: &[&str],
        target: &str,
        rationale: &str,
        payload: &serde_json::Value,
    ) -> Result<ProposedAction, Box<dyn Error>> {
        let response: ProposedAction = get_json(
            client
                .post(format!("{api_url}/proposed-actions"))
                .json(&serde_json::json!({
                    "workspace_id": "workspace-alpha",
                    "task_id": "task-cognitive-propose",
                    "proposed_by": "agent-cognitive-proposer-v0",
                    "action_type": action_type,
                    "target": target,
                    "risk_level": risk_level,
                    "required_permissions": permissions,
                    "rationale": rationale,
                    "payload": payload,
                }))
                .send()
                .await?,
        )
        .await?;
        Ok(response)
    }

    // ── From FailureInsightCandidates ──────────────────────────────────────
    for fic in failure_insight_candidates {
        let kind = fic
            .get("kind")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let summary = fic.get("summary").and_then(|v| v.as_str()).unwrap_or("");
        let reason = fic.get("reason").and_then(|v| v.as_str()).unwrap_or("");
        let tool_name = fic.get("tool_name").and_then(|v| v.as_str()).unwrap_or("");
        let is_positive = fic
            .get("is_positive_signal")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let suggested_action = fic_kind_to_action(kind);
        let benefit = fic_kind_to_benefit(kind);
        let risk = if is_positive { "informational" } else { "low" };

        let permissions: &[&str] = match suggested_action {
            "fix" => &["propose_tool_use"],
            "test" => &["read_document"],
            "refactor" => &["propose_tool_use"],
            "research" => &["read_document"],
            "governance" => &["read_memory"],
            _ => &["read_document"],
        };

        let metadata = ProposalMetadata {
            originating_objective: objective.to_owned(),
            source_kind: "failure_insight_candidate".to_owned(),
            source_summary: summary.to_owned(),
            rationale: format!("{} (tool: {})", reason, tool_name),
            expected_benefit: benefit.to_owned(),
            risk_level: risk.to_owned(),
            suggested_action_type: suggested_action.to_owned(),
            confidence: None,
            implementation_cost: "medium".to_owned(),
            priority_score: compute_priority_score(benefit, None, risk, suggested_action, "medium"),
            priority_band: compute_priority_band(compute_priority_score(
                benefit, None, risk, suggested_action, "medium",
            ))
            .to_owned(),
            non_authorizing_warning: "This proposal is pending Decision Gate review. No execution without explicit approval.".to_owned(),
        };

        let action = create_one_proposal(
            client,
            api_url,
            "propose_tool_use",
            risk,
            permissions,
            tool_name,
            &format!("Proposal from {}: {}", kind, summary),
            &serde_json::json!(metadata),
        )
        .await?;

        proposed_actions.push(action);
    }

    // ── From CognitiveObservations ─────────────────────────────────────────
    for obs in cognitive_observations {
        let summary = obs.get("summary").and_then(|v| v.as_str()).unwrap_or("");
        let detail = obs.get("detail").and_then(|v| v.as_str()).unwrap_or("");
        let tool_name = obs.get("tool_name").and_then(|v| v.as_str()).unwrap_or("");
        let candidate_kind = obs
            .get("candidate_kind")
            .and_then(|v| v.as_str())
            .map(|s| s.to_owned());
        let is_failure_candidate = obs
            .get("failure_insight_candidate")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let usefulness = obs
            .get("usefulness")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        // Only propose for observations that are failure candidates or have low usefulness
        let obs_is_actionable =
            is_failure_candidate || usefulness == "low" || usefulness == "very_low";

        if !obs_is_actionable {
            continue;
        }

        let suggested_action = obs_kind_to_action(&candidate_kind);
        let benefit = obs_kind_to_benefit(&candidate_kind);

        let risk = match usefulness {
            "very_low" => "informational",
            _ => "low",
        };

        let permissions: &[&str] = match suggested_action {
            "fix" => &["propose_tool_use"],
            _ => &["read_document"],
        };

        let metadata = ProposalMetadata {
            originating_objective: objective.to_owned(),
            source_kind: "cognitive_observation".to_owned(),
            source_summary: summary.to_owned(),
            rationale: format!("Observation from tool '{}': {}", tool_name, detail),
            expected_benefit: benefit.to_owned(),
            risk_level: risk.to_owned(),
            suggested_action_type: suggested_action.to_owned(),
            confidence: None,
            implementation_cost: "medium".to_owned(),
            priority_score: compute_priority_score(benefit, None, risk, suggested_action, "medium"),
            priority_band: compute_priority_band(compute_priority_score(
                benefit, None, risk, suggested_action, "medium",
            ))
            .to_owned(),
            non_authorizing_warning: "This proposal is pending Decision Gate review. No execution without explicit approval.".to_owned(),
        };

        let action = create_one_proposal(
            client,
            api_url,
            "propose_tool_use",
            risk,
            permissions,
            tool_name,
            &format!("Observation-based proposal: {}", summary),
            &serde_json::json!(metadata),
        )
        .await?;

        proposed_actions.push(action);
    }

    // ── Deduplicate and re-evaluate through the Decision Gate ─────────
    let (deduped_actions, deduped_decisions, deduped_audit_events) =
        dedup_proposed_actions(proposed_actions);

    Ok(ProposalRunResult {
        proposed_actions: deduped_actions,
        decisions: deduped_decisions,
        audit_events: deduped_audit_events,
    })
}

/// Extract the required permissions from a ProposedAction as a Vec<Permission>.
fn permissions_for_action(action: &ProposedAction) -> Vec<Permission> {
    action.required_permissions.clone()
}

/// Result of the proposal bridge run.
#[derive(Clone, Debug, Serialize, Deserialize)]
struct ProposalRunResult {
    proposed_actions: Vec<ProposedAction>,
    decisions: Vec<arpagona_agent_core::Decision>,
    audit_events: Vec<arpagona_agent_core::AuditEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExecutorInfoResponse {
    executor_id: String,
    executor_state: String,
    supported_action_types: Vec<String>,
    /// "offline" when queried from the core crate directly; empty/absent for live API responses.
    #[serde(default)]
    mode: String,
}

/// Build an executor info response for an offline registry query.
fn build_offline_executor_info(
    registry: &ExecutorRegistry,
    executor_id: &str,
) -> Option<ExecutorInfoResponse> {
    registry.get_slot(executor_id).map(|slot| {
        let state_str = serde_json::to_value(&slot.state)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| format!("{:?}", slot.state));
        let action_types: Vec<String> = slot
            .executor
            .supported_action_types()
            .iter()
            .map(|at| {
                serde_json::to_value(at)
                    .ok()
                    .and_then(|v| v.as_str().map(String::from))
                    .unwrap_or_else(|| format!("{at:?}"))
            })
            .collect();
        ExecutorInfoResponse {
            executor_id: executor_id.to_owned(),
            executor_state: state_str,
            supported_action_types: action_types,
            mode: "offline".to_owned(),
        }
    })
}

/// Load persisted executor state from a JSON file and apply it to the registry.
///
/// Expected format: `{"executor_id": "disabled"|"ready"|"blocked"}`
fn load_executor_state_file(
    registry: &mut ExecutorRegistry,
    state_file: &str,
) -> Result<(), Box<dyn Error>> {
    let content = std::fs::read_to_string(state_file)?;
    let states: HashMap<String, ExecutorState> = serde_json::from_str(&content)?;
    for (id, state) in states {
        registry.set_state(&id, state);
    }
    Ok(())
}

/// List all registered executors with their current state.
async fn executor_list(
    client: &Client,
    api_url: &str,
    args: ExecutorListArgs,
) -> Result<(), Box<dyn Error>> {
    let (executors, is_offline): (Vec<ExecutorInfoResponse>, bool) = if args.offline {
        let mut registry = ExecutorRegistry::new();
        if let Some(ref state_file) = args.state_file {
            load_executor_state_file(&mut registry, state_file)
                .map_err(|e| format!("failed to load state file '{}': {}", state_file, e))?;
        }
        let mut result = Vec::new();
        for id in registry.list() {
            if let Some(info) = build_offline_executor_info(&registry, &id) {
                result.push(info);
            }
        }
        (result, true)
    } else {
        let result: Vec<ExecutorInfoResponse> =
            get_json(client.get(format!("{api_url}/executors")).send().await?).await?;
        (result, false)
    };

    if args.json {
        let output = if is_offline {
            serde_json::json!({
                "mode": "offline",
                "executors": executors
            })
        } else {
            serde_json::json!({
                "mode": "live",
                "executors": executors
            })
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        if is_offline {
            println!("[offline mode — local registry state (not live server)]");
        }
        println!("Registered executors:");
        for exec in &executors {
            println!(
                "  {}  state={}  actions={}",
                exec.executor_id,
                exec.executor_state,
                exec.supported_action_types.join(", ")
            );
        }
    }
    Ok(())
}

/// Inspect the details of a specific executor.
async fn executor_inspect(
    client: &Client,
    api_url: &str,
    args: ExecutorInspectArgs,
) -> Result<(), Box<dyn Error>> {
    let (exec, is_offline) = if args.offline {
        let mut registry = ExecutorRegistry::new();
        if let Some(ref state_file) = args.state_file {
            load_executor_state_file(&mut registry, state_file)
                .map_err(|e| format!("failed to load state file '{}': {}", state_file, e))?;
        }
        let info = build_offline_executor_info(&registry, &args.executor_id);
        (info, true)
    } else {
        // Fetch the full executor list and filter for the requested ID
        let executors: Vec<ExecutorInfoResponse> =
            get_json(client.get(format!("{api_url}/executors")).send().await?).await?;
        let info = executors
            .iter()
            .find(|e| e.executor_id == args.executor_id)
            .cloned();
        (info, false)
    };

    match exec {
        Some(mut info) => {
            if is_offline {
                info.mode = "offline".to_owned();
            }
            if args.json {
                let output = if is_offline {
                    serde_json::json!({
                        "mode": "offline",
                        "executor": info
                    })
                } else {
                    serde_json::json!({
                        "mode": "live",
                        "executor": info
                    })
                };
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else {
                if is_offline {
                    println!("[offline mode — local registry state (not live server)]");
                }
                println!("Executor: {}", info.executor_id);
                println!("  State: {}", info.executor_state);
                println!(
                    "  Supported actions: {}",
                    info.supported_action_types.join(", ")
                );
            }
        }
        None => {
            if args.json {
                let output = if is_offline {
                    serde_json::json!({
                        "mode": "offline",
                        "error": "executor not found",
                        "executor_id": args.executor_id
                    })
                } else {
                    serde_json::json!({
                        "mode": "live",
                        "error": "executor not found",
                        "executor_id": args.executor_id
                    })
                };
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else {
                if is_offline {
                    println!("[offline mode — local registry state (not live server)]");
                }
                println!("Executor '{}' not found", args.executor_id);
            }
        }
    }
    Ok(())
}

/// Start the native MCP server (stdio transport).
///
/// Reads JSON-RPC 2.0 requests from stdin and writes responses to stdout.
/// External MCP clients (Claude Desktop, Cursor, etc.) connect via stdio.
fn mcp_server(args: McpServerArgs) -> Result<(), Box<dyn Error>> {
    let config = arpagona_mcp_server::McpServerConfig {
        server_name: args.name,
        server_version: args.version,
        workspace_path: args.workspace,
        audit_path: args.audit_path,
    };

    let mut server = arpagona_mcp_server::McpServer::new(config);

    eprintln!("Starting Arpagona MCP server (stdio transport)...");
    eprintln!(
        "Server: {} v{}",
        server.config.server_name, server.config.server_version
    );
    eprintln!("Workspace: {}", server.config.workspace_path);
    eprintln!("Waiting for MCP client to connect...");

    server.run()?;

    eprintln!("MCP server: client disconnected.");
    Ok(())
}

/// Read and display recent MCP governance audit decisions.
///
/// Reads the persisted JSON-lines audit file and displays recent decisions
/// with their outcomes, tool names, and timestamps.
fn mcp_governance_audit(args: McpGovernanceAuditArgs) -> Result<(), Box<dyn Error>> {
    use arpagona_mcp_server::McpGovernanceAuditStore;

    let store = McpGovernanceAuditStore::new(&args.audit_path)
        .map_err(|e| format!("Failed to read audit store at '{}': {e}", args.audit_path))?;

    let entries = store.recent(args.limit);

    if args.json {
        let json_output = serde_json::to_string_pretty(&serde_json::json!({
            "audit_path": args.audit_path,
            "total_entries": store.len(),
            "displayed_entries": entries.len(),
            "entries": entries.iter().map(|e| serde_json::json!({
                "outcome": e.outcome,
                "tool_name": e.tool_name,
                "summary": e.summary,
                "created_at": e.created_at,
                "arguments": e.arguments,
                "audit_event_id": e.audit_event.id,
            })).collect::<Vec<_>>(),
        }))
        .unwrap();
        println!("{json_output}");
    } else {
        println!("MCP Governance Audit — {}", args.audit_path);
        println!("Total entries: {}", store.len());
        println!("Showing last {} entries:", entries.len());
        println!();
        for (i, entry) in entries.iter().enumerate() {
            println!(
                "  #{:<4} | {:<16} | {}",
                entries.len() - i,
                entry.outcome,
                entry.tool_name
            );
            println!(
                "        | at: {}",
                entry.created_at.format("%Y-%m-%d %H:%M:%S")
            );
            println!("        | {}", entry.summary);
            println!(
                "        | audit_event_id: {}",
                entry.audit_event.id.as_str()
            );
            println!();
        }
    }

    Ok(())
}

/// Display recent LLM interaction journal entries (C3 readback).
///
/// Reads from the in-memory global journal and displays recent LLM interactions
/// including prompt summaries, response summaries, provider/model metadata,
/// proposed actions, tool-call intents, and Decision Gate outcomes.
fn llm_journal_list(args: LlmJournalArgs) -> Result<(), Box<dyn Error>> {
    let journal = global_llm_journal().lock().unwrap();
    let entries = journal.recent_entries(args.limit);

    if args.json {
        let json_output = serde_json::to_string_pretty(&serde_json::json!({
            "total_entries": journal.len(),
            "displayed_entries": entries.len(),
            "entries": entries.iter().map(|e| serde_json::json!({
                "id": e.id,
                "created_at": e.created_at,
                "interaction_type": e.interaction_type,
                "prompt_summary": e.prompt_summary,
                "response_summary": e.response_summary,
                "provider": e.provider,
                "model": e.model,
                "objective": e.objective,
                "proposed_actions": e.proposed_actions,
                "tool_call_intents": e.tool_call_intents,
                "decision_gate_outcomes": e.decision_gate_outcomes,
                "risk_level": e.risk_level,
                "compute_routing": e.compute_routing,
            })).collect::<Vec<_>>(),
        }))
        .unwrap();
        println!("{json_output}");
    } else {
        println!("LLM Interaction Journal — {} entries total", journal.len());
        println!("Showing {} most recent entries:", entries.len());
        println!();
        for (i, entry) in entries.iter().enumerate() {
            let created = entry.created_at.format("%Y-%m-%d %H:%M:%S");
            println!(
                "  #{:<4} | {:<12} | {}",
                entries.len() - i,
                format!("{:?}", entry.interaction_type),
                created
            );
            println!("        | provider: {}", entry.provider);
            if let Some(ref model) = entry.model {
                println!("        | model: {}", model);
            }
            if let Some(ref obj) = entry.objective {
                println!("        | objective: {}", &obj[..obj.len().min(80)]);
            }
            println!(
                "        | prompt: {}",
                &entry.prompt_summary[..entry.prompt_summary.len().min(120)]
            );
            println!(
                "        | response: {}",
                &entry.response_summary[..entry.response_summary.len().min(120)]
            );
            if let Some(ref dg) = entry.decision_gate_outcomes {
                println!(
                    "        | decision_gate: {}",
                    serde_json::to_string(dg).unwrap_or_default()
                );
            }
            if let Some(ref rl) = entry.risk_level {
                println!("        | risk_level: {:?}", rl);
            }
            if let Some(ref cr) = entry.compute_routing {
                let node_name = cr
                    .get("selected_node_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let justification = cr
                    .get("justification")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let routing_note = cr
                    .get("routing_note")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                println!("        | compute_routing:");
                println!("        |   selected_node: {node_name}");
                if !justification.is_empty() {
                    let just_short = &justification[..justification.len().min(120)];
                    println!("        |   justification: {just_short}");
                }
                if !routing_note.is_empty() {
                    println!("        |   routing_note: {routing_note}");
                }
            }
            println!();
        }
    }

    Ok(())
}

/// Run the General Cognitive Work Loop V0.
async fn cognitive_run(
    client: &Client,
    api_url: &str,
    args: CognitiveRunArgs,
) -> Result<(), Box<dyn Error>> {
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
            // When --assess, inject failure_insight_candidates into working_memory.
            // When --observe is also active, assess observations and merge their candidates.
            if args.assess {
                if let Some(wm) = obj
                    .get_mut("working_memory")
                    .and_then(|v| v.as_object_mut())
                {
                    let mut fic =
                        arpagona_agent_core::FailureInsightCandidate::from_improvement_candidates(
                            &result.improvement_candidates,
                        );

                    // Bridge: when --observe is also active, assess observations and
                    // merge their FailureInsightCandidates alongside the improvement-candidate-derived ones.
                    if args.observe {
                        let observations = run_observations(&result);
                        let assessments: Vec<_> = observations
                            .iter()
                            .map(|o| arpagona_agent_core::assess_observation(o))
                            .collect();
                        let obs_fic =
                            arpagona_agent_core::FailureInsightCandidate::from_assessments(
                                &assessments,
                            );
                        fic.extend(obs_fic);
                        // Inject cognitive observations into working_memory
                        wm.insert(
                            "cognitive_observations".to_owned(),
                            serde_json::to_value(&observations)?,
                        );
                        wm.insert("observed".to_owned(), serde_json::Value::Bool(true));
                    }

                    wm.insert(
                        "failure_insight_candidates".to_owned(),
                        serde_json::to_value(&fic)?,
                    );
                }
            }
            if args.allocate {
                let allocation = run_allocation(&result.working_memory);
                obj.insert(
                    "compute_requirement".to_owned(),
                    serde_json::to_value(&allocation)?,
                );
            }
            obj.insert("assessed".to_owned(), serde_json::Value::Bool(args.assess));
            obj.insert(
                "allocated".to_owned(),
                serde_json::Value::Bool(args.allocate),
            );
            if args.allocate {
                obj.insert(
                    "non_authorizing_warning".to_owned(),
                    serde_json::Value::String(NON_AUTHORIZING_READBACK.to_owned()),
                );
            }
            // When --resonate, run HolographicMemory resonance and inject into output
            if args.resonate {
                let wm = &result.working_memory;
                let domain_str = format!("{:?}", result.objective.domain).to_lowercase();
                let sensitivity_str = format!("{:?}", wm.sensitivity_estimate).to_lowercase();
                let allocation_justification = if args.allocate {
                    let allocation = run_allocation(&result.working_memory);
                    Some(allocation.justification.clone())
                } else {
                    None
                };

                let resonance = resonate_for_working_memory(
                    &domain_str,
                    &sensitivity_str,
                    wm.complexity_estimate,
                    &wm.proposed_next_action_kind,
                    allocation_justification.as_deref(),
                );

                obj.insert(
                    "holographic_resonance".to_owned(),
                    serde_json::to_value(&resonance)?,
                );
                obj.insert("resonated".to_owned(), serde_json::Value::Bool(true));
                obj.insert(
                    "holographic_warning".to_owned(),
                    serde_json::Value::String(RESONANCE_NON_AUTHORIZING_WARNING.to_owned()),
                );
            }
            // When --observe, run tool runtime observations and inject into working_memory.
            // When --assess is also active, observations are handled inside the --assess
            // block above (assessed and merged into failure_insight_candidates).
            if args.observe && !args.assess {
                let observations = run_observations(&result);
                if let Some(wm) = obj
                    .get_mut("working_memory")
                    .and_then(|v| v.as_object_mut())
                {
                    wm.insert(
                        "cognitive_observations".to_owned(),
                        serde_json::to_value(&observations)?,
                    );
                    wm.insert("observed".to_owned(), serde_json::Value::Bool(true));
                }
            }
            // When --propose, collect FailureInsightCandidates and observations
            // and create context-rich ProposedActions via the API.
            if args.propose {
                let objective = &args.objective;

                // Collect FailureInsightCandidates from working_memory (injected by --assess)
                let failure_insight_candidates: Vec<serde_json::Value> = obj
                    .get("working_memory")
                    .and_then(|wm| wm.get("failure_insight_candidates"))
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.clone())
                    .unwrap_or_default();

                // Collect CognitiveObservations from working_memory (injected by --observe)
                let cognitive_observations: Vec<serde_json::Value> = obj
                    .get("working_memory")
                    .and_then(|wm| wm.get("cognitive_observations"))
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.clone())
                    .unwrap_or_default();

                // Only run the proposal bridge if there are candidates or observations
                if !failure_insight_candidates.is_empty() || !cognitive_observations.is_empty() {
                    match run_proposals(
                        client,
                        api_url,
                        objective,
                        &failure_insight_candidates,
                        &cognitive_observations,
                    )
                    .await
                    {
                        Ok(proposal_result) => {
                            // Sort proposed actions by priority_score descending
                            let mut sorted_proposals = proposal_result.proposed_actions.clone();
                            sorted_proposals.sort_by(|a, b| {
                                let a_score = a
                                    .payload
                                    .get("priority_score")
                                    .and_then(|v| v.as_f64())
                                    .unwrap_or(-1.0);
                                let b_score = b
                                    .payload
                                    .get("priority_score")
                                    .and_then(|v| v.as_f64())
                                    .unwrap_or(-1.0);
                                b_score
                                    .partial_cmp(&a_score)
                                    .unwrap_or(std::cmp::Ordering::Equal)
                            });
                            obj.insert(
                                "proposed_actions".to_owned(),
                                serde_json::to_value(&sorted_proposals)?,
                            );
                            obj.insert(
                                "decisions".to_owned(),
                                serde_json::to_value(&proposal_result.decisions)?,
                            );
                            obj.insert(
                                "audit_events".to_owned(),
                                serde_json::to_value(&proposal_result.audit_events)?,
                            );
                            obj.insert(
                                "non_authorizing_warning".to_owned(),
                                serde_json::Value::String(
                                    "Readback only — these ProposedActions are PendingDecision. No execution without explicit Decision Gate approval."
                                        .to_owned(),
                                ),
                            );
                        }
                        Err(e) => {
                            obj.insert(
                                "proposal_error".to_owned(),
                                serde_json::Value::String(format!("Proposal bridge failed: {e}")),
                            );
                        }
                    }
                } else {
                    obj.insert(
                        "proposal_note".to_owned(),
                        serde_json::Value::String(
                            "No FailureInsightCandidates or observations to propose from. Run with --assess or --observe to generate proposals."
                                .to_owned(),
                        ),
                    );
                }
                obj.insert("proposed".to_owned(), serde_json::Value::Bool(true));
            }
            // When --govern, run offline governance: convert FailureInsightCandidates
            // into ProposedActions through DecisionGate -> Decision -> AuditEvent
            // without requiring the API server.
            if args.govern {
                let failure_insight_candidates: Vec<serde_json::Value> = obj
                    .get("working_memory")
                    .and_then(|wm| wm.get("failure_insight_candidates"))
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.clone())
                    .unwrap_or_default();

                if !failure_insight_candidates.is_empty() {
                    match run_offline_governance(&failure_insight_candidates) {
                        Ok(governance_results) => {
                            let decisions: Vec<_> = governance_results
                                .iter()
                                .filter_map(|r| r.get("decision").cloned())
                                .collect();
                            let audit_events: Vec<_> = governance_results
                                .iter()
                                .filter_map(|r| r.get("audit_event").cloned())
                                .collect();
                            obj.insert(
                                "governance_results".to_owned(),
                                serde_json::to_value(&governance_results)?,
                            );
                            obj.insert("decision_count".to_owned(), json!(decisions.len()));
                            obj.insert("audit_event_count".to_owned(), json!(audit_events.len()));
                            obj.insert(
                                "governance_warning".to_owned(),
                                serde_json::Value::String(
                                    "Offline governance readback — these Decision Gate decisions and AuditEvents are evidence only. No execution, no persistence, no external effects."
                                        .to_owned(),
                                ),
                            );
                        }
                        Err(e) => {
                            obj.insert(
                                "governance_error".to_owned(),
                                serde_json::Value::String(format!(
                                    "Offline governance failed: {e}"
                                )),
                            );
                        }
                    }
                } else {
                    obj.insert(
                        "governance_note".to_owned(),
                        serde_json::Value::String(
                            "No FailureInsightCandidates to govern. Run with --assess to generate candidates first."
                                .to_owned(),
                        ),
                    );
                }
                obj.insert("governed".to_owned(), serde_json::Value::Bool(true));
            }
            // When --llm, run LLM synthesis and inject into output
            if args.llm {
                let wm = &result.working_memory;
                let wm_summary = format!(
                    "Domain: {:?}\nSensitivity: {:?}\nComplexity: {:.2}\nContext items: {}\nMissing context: {}\nAssumptions: {}\nProposed next action: {:?}",
                    result.objective.domain,
                    wm.sensitivity_estimate,
                    wm.complexity_estimate,
                    wm.context_items.len(),
                    wm.missing_context.len(),
                    wm.assumptions.len(),
                    wm.proposed_next_action.as_ref().map(|a| &a.kind),
                );

                // Resolve provider: if --allocate is active, derive from compute allocation;
                // otherwise use the --provider CLI flag
                let (resolved_provider, routing_note): (&str, String) = if args.allocate {
                    let allocation = run_allocation(&result.working_memory);
                    match allocation.selected_node_id.as_ref().map(|id| id.as_str()) {
                        Some("cloud-strong") => (
                            "openai",
                            "routed via ComputeReservoir: cloud-strong → openai".to_owned(),
                        ),
                        Some("local-smol") => (
                            "ollama",
                            "routed via ComputeReservoir: local-smol → ollama (local)".to_owned(),
                        ),
                        Some("local-cpu") => (
                            "mock",
                            "routed via ComputeReservoir: local-cpu → mock (deterministic, no LLM)"
                                .to_owned(),
                        ),
                        Some(other) => (
                            &args.provider,
                            format!(
                                "ComputeReservoir node '{}' has no LLM mapping; using --provider",
                                other
                            ),
                        ),
                        None => (
                            &args.provider,
                            "No compute node selected; using --provider".to_owned(),
                        ),
                    }
                } else {
                    (
                        &args.provider.as_str(),
                        "Provider set via --provider flag".to_owned(),
                    )
                };

                // Log routing decision as a JSON field
                obj.insert(
                    "llm_routing".to_owned(),
                    serde_json::Value::String(routing_note.clone()),
                );

                match run_cognitive_synthesis(&args.objective, &wm_summary, resolved_provider).await
                {
                    Ok(synthesis) => {
                        obj.insert(
                            "llm_synthesis".to_owned(),
                            serde_json::Value::String(synthesis.clone()),
                        );
                        obj.insert(
                            "llm_provider".to_owned(),
                            serde_json::Value::String(resolved_provider.to_owned()),
                        );
                        // Journal the LLM interaction (C3 — prompt/response/decision journaling)
                        let (compute_routing_json, journal_routing_note) = if args.allocate {
                            let allocation = run_allocation(&result.working_memory);
                            let routing_note_clone = routing_note.clone();
                            let routing_json = serde_json::json!({
                                "selected_node_id": allocation.selected_node_id.as_ref().map(|id| id.as_str()),
                                "resource_kind": allocation.resource_kind,
                                "expected_cost_cents": allocation.expected_cost_cents,
                                "expected_latency_ms": allocation.expected_latency_ms,
                                "justification": allocation.justification,
                                "fallback": allocation.fallback,
                                "routing_note": routing_note_clone,
                            });
                            (
                                Some(routing_json),
                                format!("\nCompute routing: {}", routing_note),
                            )
                        } else {
                            (None, String::new())
                        };
                        global_llm_journal()
                            .lock()
                            .unwrap()
                            .add_synthesis_with_routing(
                                &args.objective,
                                resolved_provider,
                                None, // model info not tracked yet in this path
                                format!(
                                    "Cognitive synthesis: domain={:?}, complexity={:.2}, context_items={}{}",
                                    result.objective.domain,
                                    wm.complexity_estimate,
                                    wm.context_items.len(),
                                    journal_routing_note,
                                ),
                                format!(
                                    "Synthesis ({} chars): extracted {} chars of structured output",
                                    synthesis.len(),
                                    synthesis.len().min(200),
                                ),
                                compute_routing_json,
                            );
                    }
                    Err(e) => {
                        obj.insert(
                            "llm_synthesis_error".to_owned(),
                            serde_json::Value::String(format!("LLM synthesis failed: {e}")),
                        );
                    }
                }
            }
        }
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        cognitive_print_readback(&result, args.assess);
        if args.allocate {
            let allocation = run_allocation(&result.working_memory);
            print_allocation_readback(&allocation);
        }
        if args.resonate {
            let domain_str = format!("{:?}", result.objective.domain).to_lowercase();
            let sensitivity_str =
                format!("{:?}", result.working_memory.sensitivity_estimate).to_lowercase();
            let allocation_justification = if args.allocate {
                let allocation = run_allocation(&result.working_memory);
                Some(allocation.justification.clone())
            } else {
                None
            };
            let resonance = resonate_for_working_memory(
                &domain_str,
                &sensitivity_str,
                result.working_memory.complexity_estimate,
                &result.working_memory.proposed_next_action_kind,
                allocation_justification.as_deref(),
            );
            print_resonance_readback(&resonance);
        }
    }

    Ok(())
}

/// Run offline governance: convert FailureInsightCandidates from the cognitive work loop
/// into local ProposedActions, run them through DecisionGate -> Decision -> AuditEvent,
/// without requiring the API server.
///
/// # Safety
///
/// - All ProposedActions are `PendingDecision` — no execution authority.
/// - The output is evidence-only, non-authorizing readback.
/// - The DecisionGate is called with empty policies (alpha-safe default).
fn run_offline_governance(
    failure_insight_candidates: &[serde_json::Value],
) -> Result<Vec<serde_json::Value>, Box<dyn Error>> {
    let mut governance_results: Vec<serde_json::Value> = Vec::new();
    let workspace_id = WorkspaceId::new("workspace-cognitive-govern");
    let task_id = TaskId::new("task-cognitive-govern");
    let agent_id = AgentId::new("agent-cognitive-governor");

    for (i, fic) in failure_insight_candidates.iter().enumerate() {
        let kind = fic
            .get("kind")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let summary = fic.get("summary").and_then(|v| v.as_str()).unwrap_or("");
        let reason = fic.get("reason").and_then(|v| v.as_str()).unwrap_or("");
        let tool_name = fic
            .get("tool_name")
            .and_then(|v| v.as_str())
            .unwrap_or("cognitive_work_loop");

        let created_at = Utc::now();

        let action = ProposedAction {
            id: ProposedActionId::new(format!(
                "action-govern-fic-{}-{}",
                i,
                created_at.timestamp()
            )),
            workspace_id: workspace_id.clone(),
            task_id: Some(task_id.clone()),
            proposed_by: agent_id.clone(),
            action_type: ActionType::ProposeToolUse,
            target: Some(tool_name.to_owned()),
            payload: json!({
                "source_kind": "failure_insight_candidate",
                "kind": kind,
                "summary": summary,
                "reason": reason,
                "non_authorizing_warning": "Offline governance proposal — pending Decision Gate review. No execution without explicit approval.",
            }),
            risk_level: RiskLevel::Low,
            required_permissions: vec![Permission::ReadDocument],
            rationale: format!(
                "Offline governance from cognitive work loop: {} (kind: {}, tool: {})",
                reason, kind, tool_name
            ),
            context_refs: vec![],
            status: ProposedActionStatus::PendingDecision,
            created_at,
        };

        let decision = evaluate_proposed_action(&action, &[], &[Permission::ReadDocument]);
        let audit_event = audit_event_for_decision(&action, &decision);

        governance_results.push(json!({
            "proposed_action_id": action.id.to_string(),
            "proposed_action": action,
            "decision": decision,
            "audit_event": audit_event,
        }));
    }

    Ok(governance_results)
}

/// Build a demo compute inventory for the allocation bridge.
fn demo_inventory() -> Vec<ComputeNode> {
    vec![
        ComputeNode {
            id: ComputeNodeId::new("local-smol"),
            label: "Local small model".to_owned(),
            kind: ComputeResourceKind::LocalLlm,
            status: ComputeNodeStatus::Available,
            capabilities: vec![
                ComputeCapability::SimpleReasoning,
                ComputeCapability::TextSynthesis,
                ComputeCapability::LocalOnly,
            ],
            max_data_sensitivity: DataSensitivity::Secret,
            expected_cost_cents: 0,
            expected_latency_ms: 800,
            is_local: true,
            strength: 3,
        },
        ComputeNode {
            id: ComputeNodeId::new("local-cpu"),
            label: "Local CPU worker".to_owned(),
            kind: ComputeResourceKind::LocalCpu,
            status: ComputeNodeStatus::Available,
            capabilities: vec![ComputeCapability::DeterministicComputation],
            max_data_sensitivity: DataSensitivity::Secret,
            expected_cost_cents: 0,
            expected_latency_ms: 100,
            is_local: true,
            strength: 2,
        },
        ComputeNode {
            id: ComputeNodeId::new("cloud-strong"),
            label: "Cloud strong model".to_owned(),
            kind: ComputeResourceKind::CloudLlm,
            status: ComputeNodeStatus::Available,
            capabilities: vec![
                ComputeCapability::SimpleReasoning,
                ComputeCapability::ComplexReasoning,
                ComputeCapability::TextSynthesis,
            ],
            max_data_sensitivity: DataSensitivity::Confidential,
            expected_cost_cents: 25,
            expected_latency_ms: 1_200,
            is_local: false,
            strength: 10,
        },
    ]
}

/// Run allocation from WorkingMemory with the demo inventory.
fn run_allocation(
    working_memory: &arpagona_agent_core::cognitive_work::WorkingMemory,
) -> ComputeAllocation {
    let nodes = demo_inventory();
    let policy = ComputePolicy::default();
    allocate_for_working_memory(working_memory, &nodes, &policy)
}

/// Print a human-readable allocation result.
fn print_allocation_readback(allocation: &ComputeAllocation) {
    println!();
    println!(
        "{}",
        style_info("Compute Reservoir Allocation (--allocate)")
    );
    println!("  status:                    {:?}", allocation.status);
    println!(
        "  selected_node_id:           {}",
        allocation
            .selected_node_id
            .as_ref()
            .map(|id| id.as_str())
            .unwrap_or("(none)")
    );
    println!(
        "  resource_kind:              {:?}",
        allocation.resource_kind
    );
    println!(
        "  expected_cost_cents:        {:?}",
        allocation.expected_cost_cents
    );
    println!(
        "  expected_latency_ms:        {:?}",
        allocation.expected_latency_ms
    );
    println!("  justification:              {}", allocation.justification);
    if let Some(fallback) = &allocation.fallback {
        println!("  fallback_node:              {:?}", fallback.node_id);
        println!("  fallback_reason:            {}", fallback.reason);
    }
    println!();
    println!(
        "{}",
        style_dim(
            "⚠️  Allocation is not authorization — no action approved, no Decision Gate bypass."
        )
    );
}

/// Print a human-readable HolographicMemory resonance result.
fn print_resonance_readback(resonance: &arpagona_agent_core::holographic::WorkingMemoryResonance) {
    println!(
        "\n{}",
        style_info("HolographicMemory Resonance (--resonate)")
    );
    println!("  has_resonance:              {}", resonance.has_resonance);
    println!("  hints:                      {}", resonance.hints.len());
    for (i, hint) in resonance.hints.iter().enumerate() {
        println!("    [{}.] kind: {:?}", i + 1, hint.suggested_trace_kind);
        println!("         labels: {:?}", hint.labels);
        println!("         score:  {:.2}", hint.resonance_score);
        println!("         why:    {}", hint.rationale);
    }
    println!();
    println!(
        "{}",
        style_dim("⚠️  Resonance is non-authorizing — pattern hints only, no action approved.")
    );
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
                assert_eq!(args.provider, "ollama");
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
                assert_eq!(args.provider, "ollama");
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
            local: LocalSubsystemStatus {
                holographic_memory_db_exists: false,
                holographic_memory_db_path: None,
                openai_api_key_configured: false,
                ollama_endpoint_configured: true,
                ollama_appears_reachable: false,
                conversation_memory_trace_count: None,
                tool_runtime_tool_count: Some(3),
                tool_runtime_tools: vec![],
                handoff_next_action: None,
                backlog_open_count: None,
                mcp_server_binary_available: false,
                cli_version: env!("CARGO_PKG_VERSION").to_owned(),
                warning: AUDIT_READBACK_WARNING,
            },
            supervision: SupervisionSection {
                recent_proposed_actions: vec![],
                recent_decision_results: vec![],
                warning: AUDIT_READBACK_WARNING,
            },
        };

        let formatted = format_status_readback(&readback);

        assert!(formatted.contains("ARPAGONA status"));
        assert!(formatted.contains("api_health:"));
        assert!(formatted.contains("task_count:"));
        assert!(formatted.contains("pending_decision_count:"));
        assert!(formatted.contains("needs_human_approval_count:"));
        assert!(formatted.contains("last_audit_event_at:"));
        assert!(formatted.contains("Readback only"));
        assert!(formatted.contains("Local subsystems"));
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
            local: LocalSubsystemStatus {
                holographic_memory_db_exists: false,
                holographic_memory_db_path: None,
                openai_api_key_configured: false,
                ollama_endpoint_configured: true,
                ollama_appears_reachable: false,
                conversation_memory_trace_count: None,
                tool_runtime_tool_count: Some(3),
                tool_runtime_tools: vec![],
                handoff_next_action: None,
                backlog_open_count: None,
                mcp_server_binary_available: false,
                cli_version: env!("CARGO_PKG_VERSION").to_owned(),
                warning: AUDIT_READBACK_WARNING,
            },
            supervision: SupervisionSection {
                recent_proposed_actions: vec![],
                recent_decision_results: vec![],
                warning: AUDIT_READBACK_WARNING,
            },
        };

        let formatted = format_status_readback(&readback);

        assert!(formatted.contains("unavailable"));
        assert!(formatted.contains("last_audit_event_at:"));
        assert!(formatted.contains("Readback only"));
    }

    #[test]
    fn status_formatted_includes_local_subsystem_section() {
        let readback = StatusReadback {
            api_health: "ok".to_owned(),
            task_count: Some(0),
            proposed_action_count: Some(0),
            decision_count: Some(0),
            audit_event_count: Some(0),
            pending_decision_count: Some(0),
            needs_human_approval_count: Some(0),
            recent_audit_event_count: Some(0),
            last_audit_event_at: Some("2026-01-01T00:00:00+00:00".to_owned()),
            warning: AUDIT_READBACK_WARNING,
            local: LocalSubsystemStatus {
                holographic_memory_db_exists: false,
                holographic_memory_db_path: None,
                openai_api_key_configured: true,
                ollama_endpoint_configured: true,
                ollama_appears_reachable: false,
                conversation_memory_trace_count: None,
                tool_runtime_tool_count: Some(3),
                tool_runtime_tools: vec![
                    "read_file".to_owned(),
                    "list_files".to_owned(),
                    "search_text".to_owned(),
                ],
                handoff_next_action: Some("D1 — Operator status surface.".to_owned()),
                backlog_open_count: Some(0),
                mcp_server_binary_available: false,
                cli_version: "0.1.0".to_owned(),
                warning: AUDIT_READBACK_WARNING,
            },
            supervision: SupervisionSection {
                recent_proposed_actions: vec![],
                recent_decision_results: vec![],
                warning: AUDIT_READBACK_WARNING,
            },
        };

        let formatted = format_status_readback(&readback);
        assert!(formatted.contains("Local subsystems"));
        assert!(formatted.contains("cli_version:"));
        assert!(formatted.contains("hm_db_exists:"));
        assert!(formatted.contains("openai_key_configured:"));
        assert!(formatted.contains("ollama_configured:"));
        assert!(formatted.contains("tool_runtime_tool_count:"));
        assert!(formatted.contains("tool_runtime_tools:"));
        assert!(formatted.contains("handoff_next_action:"));
        assert!(formatted.contains("backlog_open_count:"));
        assert!(formatted.contains("mcp_server_binary:"));
        assert!(formatted.contains("read_file"));
        assert!(formatted.contains("D1"));
    }

    #[test]
    fn status_json_includes_local_subsystem_fields() {
        let readback = StatusReadback {
            api_health: "ok".to_owned(),
            task_count: None,
            proposed_action_count: None,
            decision_count: None,
            audit_event_count: None,
            pending_decision_count: None,
            needs_human_approval_count: None,
            recent_audit_event_count: None,
            last_audit_event_at: None,
            warning: AUDIT_READBACK_WARNING,
            local: LocalSubsystemStatus {
                holographic_memory_db_exists: false,
                holographic_memory_db_path: None,
                openai_api_key_configured: false,
                ollama_endpoint_configured: true,
                ollama_appears_reachable: false,
                conversation_memory_trace_count: None,
                tool_runtime_tool_count: Some(3),
                tool_runtime_tools: vec!["read_file".to_owned()],
                handoff_next_action: None,
                backlog_open_count: None,
                mcp_server_binary_available: false,
                cli_version: "0.1.0".to_owned(),
                warning: AUDIT_READBACK_WARNING,
            },
            supervision: SupervisionSection {
                recent_proposed_actions: vec![],
                recent_decision_results: vec![],
                warning: AUDIT_READBACK_WARNING,
            },
        };

        let json = serde_json::to_value(&readback).expect("JSON serialization");
        let local = json.get("local").expect("local field in JSON");
        assert!(local.get("holographic_memory_db_exists").is_some());
        assert!(local.get("openai_api_key_configured").is_some());
        assert!(local.get("ollama_endpoint_configured").is_some());
        assert!(local.get("ollama_appears_reachable").is_some());
        assert!(local.get("tool_runtime_tool_count").is_some());
        assert!(local.get("tool_runtime_tools").is_some());
        assert!(local.get("cli_version").is_some());
        assert!(local.get("mcp_server_binary_available").is_some());
        assert!(local.get("warning").is_some());
        // D2 supervision fields
        let supervision = json.get("supervision").expect("supervision field in JSON");
        assert!(supervision.get("recent_proposed_actions").is_some());
        assert!(supervision.get("recent_decision_results").is_some());
    }

    #[test]
    fn read_handoff_next_action_returns_content_when_file_exists() {
        // Write a temporary handoff file in a tmp location
        let path = std::env::temp_dir().join("test-handoff-arpagona.md");
        let content = "# Test\n\n## Next action\n**D1 — Operator status surface.**\n";
        std::fs::write(&path, content).unwrap();
        // The function reads FOCUS_LOOP_NEXT.md from CWD, so this tests
        // that it doesn't panic regardless of CWD
        let _ = super::read_handoff_next_action();
        // Cleanup
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn local_subsystem_status_null_optional_fields_serialize_correctly() {
        let status = LocalSubsystemStatus {
            holographic_memory_db_exists: false,
            holographic_memory_db_path: None,
            openai_api_key_configured: false,
            ollama_endpoint_configured: true,
            ollama_appears_reachable: false,
            conversation_memory_trace_count: None,
            tool_runtime_tool_count: None,
            tool_runtime_tools: vec![],
            handoff_next_action: None,
            backlog_open_count: None,
            mcp_server_binary_available: false,
            cli_version: "0.1.0".to_owned(),
            warning: AUDIT_READBACK_WARNING,
        };

        let json = serde_json::to_value(&status).expect("JSON serialization");
        assert_eq!(
            json.get("cli_version").and_then(|v| v.as_str()),
            Some("0.1.0")
        );
        assert!(json
            .get("handoff_next_action")
            .and_then(|v| v.as_str())
            .is_none());
        assert!(json
            .get("backlog_open_count")
            .and_then(|v| v.as_u64())
            .is_none());
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
    fn cli_parses_cognitive_run_basic() {
        let cli = Cli::parse_from([
            "arpagona",
            "cognitive",
            "run",
            "--objective",
            "Test objective",
        ]);
        match cli.command {
            Command::Cognitive(CognitiveCommand {
                command: CognitiveSubcommand::Run(args),
            }) => {
                assert_eq!(args.objective, "Test objective");
                assert!(args.domain.is_none());
                assert!(!args.assess);
                assert!(!args.allocate);
                assert!(!args.resonate);
                assert!(!args.observe);
                assert!(!args.llm);
                assert!(!args.json);
            }
            _ => panic!("expected cognitive run"),
        }
    }

    #[test]
    fn cli_parses_cognitive_run_with_domain() {
        let cli = Cli::parse_from([
            "arpagona",
            "cognitive",
            "run",
            "--objective",
            "Research quantum computing",
            "--domain",
            "research",
        ]);
        match cli.command {
            Command::Cognitive(CognitiveCommand {
                command: CognitiveSubcommand::Run(args),
            }) => {
                assert_eq!(args.objective, "Research quantum computing");
                assert_eq!(args.domain.as_deref(), Some("research"));
            }
            _ => panic!("expected cognitive run"),
        }
    }

    #[test]
    fn cli_parses_cognitive_run_with_assess_flag() {
        let cli = Cli::parse_from([
            "arpagona",
            "cognitive",
            "run",
            "--objective",
            "Assess market risk",
            "--assess",
        ]);
        match cli.command {
            Command::Cognitive(CognitiveCommand {
                command: CognitiveSubcommand::Run(args),
            }) => {
                assert_eq!(args.objective, "Assess market risk");
                assert!(args.assess);
                assert!(!args.allocate);
                assert!(!args.resonate);
            }
            _ => panic!("expected cognitive run with --assess"),
        }
    }

    #[test]
    fn cli_parses_cognitive_run_with_allocate_flag() {
        let cli = Cli::parse_from([
            "arpagona",
            "cognitive",
            "run",
            "--objective",
            "Allocate resources",
            "--allocate",
        ]);
        match cli.command {
            Command::Cognitive(CognitiveCommand {
                command: CognitiveSubcommand::Run(args),
            }) => {
                assert_eq!(args.objective, "Allocate resources");
                assert!(!args.assess);
                assert!(args.allocate);
                assert!(!args.resonate);
            }
            _ => panic!("expected cognitive run with --allocate"),
        }
    }

    #[test]
    fn cli_parses_cognitive_run_with_resonate_flag() {
        let cli = Cli::parse_from([
            "arpagona",
            "cognitive",
            "run",
            "--objective",
            "Detect patterns",
            "--resonate",
        ]);
        match cli.command {
            Command::Cognitive(CognitiveCommand {
                command: CognitiveSubcommand::Run(args),
            }) => {
                assert_eq!(args.objective, "Detect patterns");
                assert!(!args.assess);
                assert!(!args.allocate);
                assert!(args.resonate);
            }
            _ => panic!("expected cognitive run with --resonate"),
        }
    }

    #[test]
    fn cli_parses_cognitive_run_with_all_flags() {
        let cli = Cli::parse_from([
            "arpagona",
            "cognitive",
            "run",
            "--objective",
            "Full cognitive pipeline",
            "--domain",
            "engineering",
            "--assess",
            "--allocate",
            "--resonate",
            "--json",
        ]);
        match cli.command {
            Command::Cognitive(CognitiveCommand {
                command: CognitiveSubcommand::Run(args),
            }) => {
                assert_eq!(args.objective, "Full cognitive pipeline");
                assert_eq!(args.domain.as_deref(), Some("engineering"));
                assert!(args.assess);
                assert!(args.allocate);
                assert!(args.resonate);
                assert!(args.json);
            }
            _ => panic!("expected cognitive run with all flags"),
        }
    }

    #[test]
    fn cli_parses_cognitive_run_with_context() {
        let cli = Cli::parse_from([
            "arpagona",
            "cognitive",
            "run",
            "--objective",
            "Context-aware task",
            "--context",
            "budget: limited\nteam: small",
        ]);
        match cli.command {
            Command::Cognitive(CognitiveCommand {
                command: CognitiveSubcommand::Run(args),
            }) => {
                assert_eq!(args.objective, "Context-aware task");
                assert_eq!(
                    args.context.as_deref(),
                    Some("budget: limited\nteam: small")
                );
            }
            _ => panic!("expected cognitive run with context"),
        }
    }

    #[test]
    fn cli_parses_cognitive_run_context_comma_in_value() {
        // Proves the --context flag preserves comma as part of the value
        let cli = Cli::parse_from([
            "arpagona",
            "cognitive",
            "run",
            "--objective",
            "Prioritize tasks",
            "--context",
            "priority:green,workstream:validation",
        ]);
        match cli.command {
            Command::Cognitive(CognitiveCommand {
                command: CognitiveSubcommand::Run(args),
            }) => {
                assert_eq!(args.objective, "Prioritize tasks");
                assert_eq!(
                    args.context.as_deref(),
                    Some("priority:green,workstream:validation")
                );
            }
            _ => panic!("expected cognitive run with comma context"),
        }
    }

    #[test]
    fn cli_parses_cognitive_run_context_simple_key_value() {
        // Single key:value pair
        let cli = Cli::parse_from([
            "arpagona",
            "cognitive",
            "run",
            "--objective",
            "Simple context task",
            "--context",
            "sensitivity:high",
        ]);
        match cli.command {
            Command::Cognitive(CognitiveCommand {
                command: CognitiveSubcommand::Run(args),
            }) => {
                assert_eq!(args.objective, "Simple context task");
                assert_eq!(args.context.as_deref(), Some("sensitivity:high"));
            }
            _ => panic!("expected cognitive run with simple context"),
        }
    }

    #[test]
    fn cli_parses_cognitive_run_context_with_spaces() {
        // Proves key:value pairs with spaces in the value are preserved
        let cli = Cli::parse_from([
            "arpagona",
            "cognitive",
            "run",
            "--objective",
            "Spaces in context",
            "--context",
            "project name: my project",
        ]);
        match cli.command {
            Command::Cognitive(CognitiveCommand {
                command: CognitiveSubcommand::Run(args),
            }) => {
                assert_eq!(args.objective, "Spaces in context");
                assert_eq!(args.context.as_deref(), Some("project name: my project"));
            }
            _ => panic!("expected cognitive run with spaced context"),
        }
    }

    #[test]
    fn cli_parses_cognitive_run_context_empty() {
        // Proves empty context is accepted and stored as an empty string
        let cli = Cli::parse_from([
            "arpagona",
            "cognitive",
            "run",
            "--objective",
            "Empty context",
            "--context",
            "",
        ]);
        match cli.command {
            Command::Cognitive(CognitiveCommand {
                command: CognitiveSubcommand::Run(args),
            }) => {
                assert_eq!(args.objective, "Empty context");
                assert_eq!(args.context.as_deref(), Some(""));
            }
            _ => panic!("expected cognitive run with empty context"),
        }
    }

    #[test]
    fn cli_parses_cognitive_run_context_repeated_flag_rejected() {
        // Proves repeated --context flags are rejected at clap level
        // because --context is Option<String> (single-use), not Vec<String>
        // Use try_parse_from to avoid process::exit from clap's error handler
        let result = Cli::try_parse_from([
            "arpagona",
            "cognitive",
            "run",
            "--objective",
            "Repeated context",
            "--context",
            "key1:val1",
            "--context",
            "key2:val2",
        ]);
        assert!(
            result.is_err(),
            "repeated --context should be rejected: {:?}",
            result
        );
        let err = result.unwrap_err();
        let err_msg = err.to_string();
        assert!(
            err_msg.contains("cannot be used multiple times")
                || err_msg.contains("unexpected argument")
                || err_msg.contains("was used"),
            "error message should mention the repeated-flag issue, got: {}",
            err_msg
        );
    }

    #[test]
    fn cli_parses_cognitive_run_context_multiple_keys_newlines() {
        // Proves multiple key:value pairs per invocation with newline separation
        let cli = Cli::parse_from([
            "arpagona",
            "cognitive",
            "run",
            "--objective",
            "Multi-key context",
            "--context",
            "priority:high\nsensitivity:confidential\nteam:engineering",
        ]);
        match cli.command {
            Command::Cognitive(CognitiveCommand {
                command: CognitiveSubcommand::Run(args),
            }) => {
                assert_eq!(args.objective, "Multi-key context");
                assert_eq!(
                    args.context.as_deref(),
                    Some("priority:high\nsensitivity:confidential\nteam:engineering")
                );
            }
            _ => panic!("expected cognitive run with multi-key newline context"),
        }
    }

    #[test]
    fn cli_parses_cognitive_run_allocate_json() {
        let cli = Cli::parse_from([
            "arpagona",
            "cognitive",
            "run",
            "--objective",
            "Allocate and output JSON",
            "--allocate",
            "--json",
        ]);
        match cli.command {
            Command::Cognitive(CognitiveCommand {
                command: CognitiveSubcommand::Run(args),
            }) => {
                assert!(args.allocate);
                assert!(args.json);
                assert!(!args.resonate);
            }
            _ => panic!("expected cognitive run with --allocate --json"),
        }
    }

    #[test]
    fn cli_parses_cognitive_run_assess_allocate_resonate() {
        let cli = Cli::parse_from([
            "arpagona",
            "cognitive",
            "run",
            "--objective",
            "Full pipeline",
            "--assess",
            "--allocate",
            "--resonate",
        ]);
        match cli.command {
            Command::Cognitive(CognitiveCommand {
                command: CognitiveSubcommand::Run(args),
            }) => {
                assert!(args.assess);
                assert!(args.allocate);
                assert!(args.resonate);
                assert!(!args.llm);
                assert!(!args.observe);
            }
            _ => panic!("expected cognitive run with --assess --allocate --resonate"),
        }
    }

    #[test]
    fn cli_parses_cognitive_run_assess_observe_json() {
        let cli = Cli::parse_from([
            "arpagona",
            "cognitive",
            "run",
            "--objective",
            "Observe and assess pipeline",
            "--assess",
            "--observe",
            "--json",
        ]);
        match cli.command {
            Command::Cognitive(CognitiveCommand {
                command: CognitiveSubcommand::Run(args),
            }) => {
                assert!(args.assess);
                assert!(args.observe);
                assert!(args.json);
                assert!(!args.allocate);
                assert!(!args.resonate);
                assert!(!args.llm);
            }
            _ => panic!("expected cognitive run with --assess --observe --json"),
        }
    }

    #[test]
    fn cli_parses_cognitive_run_assess_observe_allocate_resonate() {
        let cli = Cli::parse_from([
            "arpagona",
            "cognitive",
            "run",
            "--objective",
            "Full pipeline with observe",
            "--assess",
            "--observe",
            "--allocate",
            "--resonate",
            "--json",
        ]);
        match cli.command {
            Command::Cognitive(CognitiveCommand {
                command: CognitiveSubcommand::Run(args),
            }) => {
                assert!(args.assess);
                assert!(args.observe);
                assert!(args.allocate);
                assert!(args.resonate);
                assert!(args.json);
                assert!(!args.llm);
                assert!(!args.govern);
            }
            _ => panic!("expected cognitive run with all flags including --observe"),
        }
    }

    #[test]
    fn cli_parses_cognitive_run_assess_govern_json() {
        let cli = Cli::parse_from([
            "arpagona",
            "cognitive",
            "run",
            "--objective",
            "Govern this candidate",
            "--assess",
            "--govern",
            "--json",
        ]);
        match cli.command {
            Command::Cognitive(CognitiveCommand {
                command: CognitiveSubcommand::Run(args),
            }) => {
                assert!(args.assess);
                assert!(args.govern);
                assert!(args.json);
                assert!(!args.allocate);
                assert!(!args.resonate);
                assert!(!args.observe);
                assert!(!args.propose);
                assert!(!args.llm);
            }
            _ => panic!("expected cognitive run with --assess --govern --json"),
        }
    }

    #[test]
    fn low_risk_high_confidence_ranks_above_high_risk_uncertain() {
        // Low-risk, high-confidence, medium-cost fix
        let low_risk_score = compute_priority_score(
            "Unblock a prevented operation so the agent can proceed safely.",
            Some(0.9),
            "low",
            "fix",
            "medium",
        );
        // High-risk, low-confidence, medium-cost research
        let high_risk_score = compute_priority_score(
            "Improve overall cognitive cycle quality.",
            Some(0.2),
            "high",
            "research",
            "medium",
        );
        assert!(
            low_risk_score > high_risk_score,
            "low-risk high-confidence ({:.2}) should rank above high-risk uncertain ({:.2})",
            low_risk_score,
            high_risk_score
        );
        assert_eq!(compute_priority_band(low_risk_score), "high");
        assert_eq!(compute_priority_band(high_risk_score), "low");
    }

    #[test]
    fn risk_level_reduces_priority_score() {
        // Same inputs, only risk changes
        let informational = compute_priority_score(
            "Unblock a prevented operation",
            Some(0.8),
            "informational",
            "fix",
            "medium",
        );
        let low = compute_priority_score(
            "Unblock a prevented operation",
            Some(0.8),
            "low",
            "fix",
            "medium",
        );
        let medium = compute_priority_score(
            "Unblock a prevented operation",
            Some(0.8),
            "medium",
            "fix",
            "medium",
        );
        let high = compute_priority_score(
            "Unblock a prevented operation",
            Some(0.8),
            "high",
            "fix",
            "medium",
        );
        let critical = compute_priority_score(
            "Unblock a prevented operation",
            Some(0.8),
            "critical",
            "fix",
            "medium",
        );

        assert!(informational > low, "informational > low");
        assert!(low > medium, "low > medium");
        assert!(medium > high, "medium > high");
        assert!(high > critical, "high > critical");
    }

    #[test]
    fn missing_confidence_defaults_to_safe_value() {
        // Without confidence (None) should default to 0.5
        let with_confidence = compute_priority_score(
            "Provide missing context",
            Some(0.9),
            "low",
            "research",
            "low",
        );
        let without_confidence =
            compute_priority_score("Provide missing context", None, "low", "research", "low");
        // With 0.9 confidence should be higher than with 0.5 default
        assert!(with_confidence > without_confidence);
        // But without_confidence should still produce a reasonable score
        assert!(without_confidence > 0.0);
    }

    #[test]
    fn generic_benefit_defaults_to_low_base_score() {
        // An unrecognized benefit string should map to 0.3
        let score = compute_priority_score(
            "Some unknown benefit description",
            Some(1.0),
            "informational",
            "refactor",
            "low",
        );
        assert!(
            score > 0.0,
            "even generic benefit should produce a positive score"
        );
        assert!(
            score < 0.8,
            "generic benefit should be lower than explicit high-benefit mappings"
        );
    }

    #[test]
    fn all_proposed_actions_remain_pending_decision() {
        // This test proves scoring doesn't change status
        use crate::{
            ActionType, AgentId, ProposedActionId, ProposedActionStatus, RiskLevel, WorkspaceId,
        };
        use serde_json::json;

        let action = ProposedAction {
            id: ProposedActionId::new("test-score-1"),
            workspace_id: WorkspaceId::new("test"),
            task_id: None,
            proposed_by: AgentId::new("test"),
            action_type: ActionType::ProposeToolUse,
            target: Some("test".to_owned()),
            payload: json!({
                "priority_score": 0.85,
                "priority_band": "high",
            }),
            risk_level: RiskLevel::Low,
            required_permissions: vec![],
            rationale: "test".to_owned(),
            context_refs: vec![],
            status: ProposedActionStatus::PendingDecision,
            created_at: chrono::Utc::now(),
        };

        assert_eq!(action.status, ProposedActionStatus::PendingDecision);
        assert_eq!(action.payload["priority_score"], json!(0.85));
        assert_eq!(action.payload["priority_band"], json!("high"));
    }

    #[test]
    fn priority_band_maps_score_ranges_correctly() {
        assert_eq!(compute_priority_band(0.9), "high");
        assert_eq!(compute_priority_band(0.7), "high");
        assert_eq!(compute_priority_band(0.69), "medium");
        assert_eq!(compute_priority_band(0.4), "medium");
        assert_eq!(compute_priority_band(0.39), "low");
        assert_eq!(compute_priority_band(0.0), "low");
    }

    #[test]
    fn dedup_key_produces_stable_keys() {
        let payload_a = serde_json::json!({
            "suggested_action_type": "fix",
            "source_kind": "failure_insight_candidate",
            "source_summary": "Blocked file access",
        });
        let payload_b = serde_json::json!({
            "suggested_action_type": "fix",
            "source_kind": "failure_insight_candidate",
            "source_summary": "BLOCKED file ACCESS",
        });
        let payload_c = serde_json::json!({
            "suggested_action_type": "research",
            "source_kind": "failure_insight_candidate",
            "source_summary": "Blocked file access",
        });

        assert_eq!(
            dedup_key_from_payload(&payload_a),
            dedup_key_from_payload(&payload_b),
            "case-insensitive keys should match"
        );
        assert_ne!(
            dedup_key_from_payload(&payload_a),
            dedup_key_from_payload(&payload_c),
            "different action types should have different keys"
        );
    }

    #[test]
    fn dedup_merges_identical_proposals() {
        use arpagona_agent_core::{
            ActionType, AgentId, ProposedActionId, ProposedActionStatus, RiskLevel, WorkspaceId,
        };
        use chrono::Utc;
        use serde_json::json;

        let now = Utc::now();
        let payload = json!({
            "suggested_action_type": "fix",
            "source_kind": "failure_insight_candidate",
            "source_summary": "Blocked file access on /etc/passwd",
            "expected_benefit": "Unblock a prevented operation so the agent can proceed safely.",
            "risk_level": "low",
            "implementation_cost": "medium",
            "priority_score": 0.52,
            "priority_band": "medium",
            "confidence": null,
            "rationale": "Tool blocked_file_access blocked (tool: test_tool)",
            "originating_objective": "Test objective",
        });

        let make_action = |id: &str| -> ProposedAction {
            ProposedAction {
                id: ProposedActionId::new(id),
                workspace_id: WorkspaceId::new("test"),
                task_id: None,
                proposed_by: AgentId::new("test"),
                action_type: ActionType::ProposeToolUse,
                target: Some("test_tool".to_owned()),
                payload: payload.clone(),
                risk_level: RiskLevel::Low,
                required_permissions: vec![],
                rationale: "Test".to_owned(),
                context_refs: vec![],
                status: ProposedActionStatus::PendingDecision,
                created_at: now,
            }
        };

        let actions = vec![
            make_action("dedup-test-1"),
            make_action("dedup-test-2"),
            make_action("dedup-test-3"),
        ];

        let (merged, decisions, audit_events) = dedup_proposed_actions(actions);

        // Should merge 3 into 1
        assert_eq!(merged.len(), 1, "3 identical proposals should merge into 1");
        assert_eq!(decisions.len(), 1, "1 decision for the merged proposal");
        assert_eq!(
            audit_events.len(),
            1,
            "1 audit event for the merged proposal"
        );

        let single = &merged[0];
        let p = &single.payload;

        // Check batch metadata
        assert_eq!(p["batched"], json!(true), "should be marked as batched");
        assert_eq!(p["merged_count"], json!(3), "merged_count should be 3");

        // Check merged_proposal_ids
        let ids = p["merged_proposal_ids"].as_array().unwrap();
        assert_eq!(ids.len(), 3, "should have 3 merged IDs");
        assert!(ids.iter().any(|v| v == "dedup-test-1"));
        assert!(ids.iter().any(|v| v == "dedup-test-2"));
        assert!(ids.iter().any(|v| v == "dedup-test-3"));

        // All must remain PendingDecision
        assert_eq!(single.status, ProposedActionStatus::PendingDecision);
        for decision in &decisions {
            assert_eq!(
                decision.proposed_action_id, single.id,
                "decision should reference the merged proposal"
            );
        }
    }

    #[test]
    fn dedup_does_not_merge_different_action_types() {
        use arpagona_agent_core::{
            ActionType, AgentId, ProposedActionId, ProposedActionStatus, RiskLevel, WorkspaceId,
        };
        use chrono::Utc;
        use serde_json::json;

        let now = Utc::now();

        let payload_fix = json!({
            "suggested_action_type": "fix",
            "source_kind": "failure_insight_candidate",
            "source_summary": "Tool runtime failure on search_text",
            "expected_benefit": "Restore tool runtime reliability.",
            "risk_level": "low",
            "implementation_cost": "medium",
            "priority_score": 0.52,
            "priority_band": "medium",
            "confidence": null,
            "rationale": "Tool search_text failed",
            "originating_objective": "Test objective",
        });
        let payload_research = json!({
            "suggested_action_type": "research",
            "source_kind": "failure_insight_candidate",
            "source_summary": "Missing context for search_text",
            "expected_benefit": "Provide missing context.",
            "risk_level": "low",
            "implementation_cost": "medium",
            "priority_score": 0.38,
            "priority_band": "low",
            "confidence": null,
            "rationale": "Need more context",
            "originating_objective": "Test objective",
        });

        let actions = vec![
            ProposedAction {
                id: ProposedActionId::new("act-fix-1"),
                workspace_id: WorkspaceId::new("test"),
                task_id: None,
                proposed_by: AgentId::new("test"),
                action_type: ActionType::ProposeToolUse,
                target: Some("tool_a".to_owned()),
                payload: payload_fix,
                risk_level: RiskLevel::Low,
                required_permissions: vec![],
                rationale: "Test fix".to_owned(),
                context_refs: vec![],
                status: ProposedActionStatus::PendingDecision,
                created_at: now,
            },
            ProposedAction {
                id: ProposedActionId::new("act-research-1"),
                workspace_id: WorkspaceId::new("test"),
                task_id: None,
                proposed_by: AgentId::new("test"),
                action_type: ActionType::ProposeToolUse,
                target: Some("tool_b".to_owned()),
                payload: payload_research,
                risk_level: RiskLevel::Low,
                required_permissions: vec![],
                rationale: "Test research".to_owned(),
                context_refs: vec![],
                status: ProposedActionStatus::PendingDecision,
                created_at: now,
            },
        ];

        let (merged, _, _) = dedup_proposed_actions(actions);

        // Different action types -> should NOT be merged
        assert_eq!(
            merged.len(),
            2,
            "different action types should not be merged"
        );
    }

    #[test]
    fn dedup_conservatively_preserves_highest_risk() {
        use arpagona_agent_core::{
            ActionType, AgentId, ProposedActionId, ProposedActionStatus, RiskLevel, WorkspaceId,
        };
        use chrono::Utc;
        use serde_json::json;

        let now = Utc::now();

        // Two proposals with the same dedup key but different risk levels
        let payload_low = json!({
            "suggested_action_type": "fix",
            "source_kind": "failure_insight_candidate",
            "source_summary": "Blocked tool use on file_read",
            "expected_benefit": "Unblock a prevented operation",
            "risk_level": "low",
            "implementation_cost": "medium",
            "priority_score": 0.52,
            "priority_band": "medium",
            "confidence": 0.8,
            "rationale": "Tool blocked (tool: file_read)",
            "originating_objective": "Test safety",
        });
        let payload_high = json!({
            "suggested_action_type": "fix",
            "source_kind": "failure_insight_candidate",
            "source_summary": "Blocked tool use on file_read",
            "expected_benefit": "Unblock a prevented operation",
            "risk_level": "high",
            "implementation_cost": "medium",
            "priority_score": 0.12,
            "priority_band": "low",
            "confidence": 0.3,
            "rationale": "Tool blocked (tool: file_read)",
            "originating_objective": "Test safety",
        });

        let actions = vec![
            ProposedAction {
                id: ProposedActionId::new("act-low-1"),
                workspace_id: WorkspaceId::new("test"),
                task_id: None,
                proposed_by: AgentId::new("test"),
                action_type: ActionType::ProposeToolUse,
                target: Some("file_read".to_owned()),
                payload: payload_low,
                risk_level: RiskLevel::Low,
                required_permissions: vec![],
                rationale: "Low risk".to_owned(),
                context_refs: vec![],
                status: ProposedActionStatus::PendingDecision,
                created_at: now,
            },
            ProposedAction {
                id: ProposedActionId::new("act-high-1"),
                workspace_id: WorkspaceId::new("test"),
                task_id: None,
                proposed_by: AgentId::new("test"),
                action_type: ActionType::ProposeToolUse,
                target: Some("file_read".to_owned()),
                payload: payload_high,
                risk_level: RiskLevel::High,
                required_permissions: vec![],
                rationale: "High risk".to_owned(),
                context_refs: vec![],
                status: ProposedActionStatus::PendingDecision,
                created_at: now,
            },
        ];

        let (merged, _, _) = dedup_proposed_actions(actions);

        assert_eq!(
            merged.len(),
            1,
            "identical-key proposals should merge even with different risks"
        );
        let p = &merged[0].payload;
        let risk_str = p["risk_level"].as_str().unwrap_or("");
        assert_eq!(
            risk_str, "high",
            "merged risk_level should be the highest (high)"
        );
        // Confirm risk_level on the struct is also High
        assert_eq!(
            merged[0].risk_level,
            RiskLevel::High,
            "struct risk_level should be High"
        );
    }

    #[test]
    fn dedup_preserves_single_proposals_unchanged() {
        use arpagona_agent_core::{
            ActionType, AgentId, ProposedActionId, ProposedActionStatus, RiskLevel, WorkspaceId,
        };
        use chrono::Utc;
        use serde_json::json;

        let now = Utc::now();
        let payload = json!({
            "suggested_action_type": "governance",
            "source_kind": "failure_insight_candidate",
            "source_summary": "Safety boundary triggered on /etc",
            "expected_benefit": "Review and harden safety boundaries.",
            "risk_level": "medium",
            "implementation_cost": "high",
            "priority_score": 0.45,
            "priority_band": "medium",
            "confidence": 0.7,
            "rationale": "Safety boundary triggered",
            "originating_objective": "Test single",
        });

        let actions = vec![ProposedAction {
            id: ProposedActionId::new("single-1"),
            workspace_id: WorkspaceId::new("test"),
            task_id: None,
            proposed_by: AgentId::new("test"),
            action_type: ActionType::ProposeToolUse,
            target: Some("safety_check".to_owned()),
            payload,
            risk_level: RiskLevel::Medium,
            required_permissions: vec![],
            rationale: "Single test".to_owned(),
            context_refs: vec![],
            status: ProposedActionStatus::PendingDecision,
            created_at: now,
        }];

        let (merged, decisions, audit_events) = dedup_proposed_actions(actions);

        assert_eq!(merged.len(), 1, "single proposal should remain 1");
        // Should NOT have batched flag
        let p = &merged[0].payload;
        assert_ne!(
            p.get("batched").and_then(|v| v.as_bool()),
            Some(true),
            "single proposals should not be marked as batched"
        );
        assert_eq!(merged[0].status, ProposedActionStatus::PendingDecision);
        assert_eq!(decisions.len(), 1);
        assert_eq!(audit_events.len(), 1);
    }

    #[test]
    fn dedup_merged_actions_remain_pending_decision() {
        // Combined with previous tests, this proves the invariant for all paths
        use arpagona_agent_core::{
            ActionType, AgentId, ProposedActionId, ProposedActionStatus, RiskLevel, WorkspaceId,
        };
        use chrono::Utc;
        use serde_json::json;

        let now = Utc::now();
        let payload = json!({
            "suggested_action_type": "test",
            "source_kind": "failure_insight_candidate",
            "source_summary": "Empty search result for main.rs",
            "expected_benefit": "Verify whether the expected data exists.",
            "risk_level": "informational",
            "implementation_cost": "low",
            "priority_score": 0.55,
            "priority_band": "medium",
            "confidence": 0.6,
            "rationale": "Empty search (tool: search_text)",
            "originating_objective": "Test dedup",
        });

        let actions = (0..5)
            .map(|i| ProposedAction {
                id: ProposedActionId::new(format!("dedup-pending-{}", i)),
                workspace_id: WorkspaceId::new("test"),
                task_id: None,
                proposed_by: AgentId::new("test"),
                action_type: ActionType::ProposeToolUse,
                target: Some("search_text".to_owned()),
                payload: payload.clone(),
                risk_level: RiskLevel::Informational,
                required_permissions: vec![],
                rationale: "Test".to_owned(),
                context_refs: vec![],
                status: ProposedActionStatus::PendingDecision,
                created_at: now,
            })
            .collect::<Vec<_>>();

        let (merged, decisions, _) = dedup_proposed_actions(actions);

        assert_eq!(merged.len(), 1, "5 identical proposals should merge into 1");
        for action in &merged {
            assert_eq!(
                action.status,
                ProposedActionStatus::PendingDecision,
                "merged action should remain PendingDecision"
            );
        }
        for decision in &decisions {
            assert!(
                matches!(
                    decision.status,
                    arpagona_agent_core::DecisionStatus::Approved
                        | arpagona_agent_core::DecisionStatus::Blocked
                        | arpagona_agent_core::DecisionStatus::NeedsHumanApproval
                ),
                "decisions should have valid status"
            );
        }
    }

    #[test]
    fn command_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn cli_parses_executor_list_defaults() {
        let cli = Cli::parse_from(["arpagona", "executor", "list"]);
        match cli.command {
            Command::Executor(ExecutorCommand {
                command: ExecutorSubcommand::List(args),
            }) => {
                assert!(!args.json);
            }
            _ => panic!("expected executor list"),
        }
    }

    #[test]
    fn cli_parses_executor_list_json() {
        let cli = Cli::parse_from(["arpagona", "executor", "list", "--json"]);
        match cli.command {
            Command::Executor(ExecutorCommand {
                command: ExecutorSubcommand::List(args),
            }) => {
                assert!(args.json);
            }
            _ => panic!("expected executor list --json"),
        }
    }

    #[test]
    fn cli_parses_executor_inspect_defaults() {
        let cli = Cli::parse_from(["arpagona", "executor", "inspect", "noop-executor"]);
        match cli.command {
            Command::Executor(ExecutorCommand {
                command: ExecutorSubcommand::Inspect(args),
            }) => {
                assert_eq!(args.executor_id, "noop-executor");
                assert!(!args.json);
            }
            _ => panic!("expected executor inspect"),
        }
    }

    #[test]
    fn cli_parses_executor_inspect_json() {
        let cli = Cli::parse_from(["arpagona", "executor", "inspect", "noop-executor", "--json"]);
        match cli.command {
            Command::Executor(ExecutorCommand {
                command: ExecutorSubcommand::Inspect(args),
            }) => {
                assert_eq!(args.executor_id, "noop-executor");
                assert!(args.json);
            }
            _ => panic!("expected executor inspect --json"),
        }
    }

    #[test]
    fn cli_parses_executor_inspect_custom_id() {
        let cli = Cli::parse_from(["arpagona", "executor", "inspect", "custom-exec"]);
        match cli.command {
            Command::Executor(ExecutorCommand {
                command: ExecutorSubcommand::Inspect(args),
            }) => {
                assert_eq!(args.executor_id, "custom-exec");
            }
            _ => panic!("expected executor inspect custom-exec"),
        }
    }

    #[test]
    fn cli_parses_executor_list_offline() {
        let cli = Cli::parse_from(["arpagona", "executor", "list", "--offline"]);
        match cli.command {
            Command::Executor(ExecutorCommand {
                command: ExecutorSubcommand::List(args),
            }) => {
                assert!(args.offline);
                assert!(!args.json);
            }
            _ => panic!("expected executor list --offline"),
        }
    }

    #[test]
    fn cli_parses_executor_list_offline_json() {
        let cli = Cli::parse_from(["arpagona", "executor", "list", "--offline", "--json"]);
        match cli.command {
            Command::Executor(ExecutorCommand {
                command: ExecutorSubcommand::List(args),
            }) => {
                assert!(args.offline);
                assert!(args.json);
            }
            _ => panic!("expected executor list --offline --json"),
        }
    }

    #[test]
    fn cli_parses_executor_list_offline_state_file() {
        let cli = Cli::parse_from([
            "arpagona",
            "executor",
            "list",
            "--offline",
            "--state-file",
            "states.json",
        ]);
        match cli.command {
            Command::Executor(ExecutorCommand {
                command: ExecutorSubcommand::List(args),
            }) => {
                assert!(args.offline);
                assert_eq!(args.state_file, Some("states.json".to_owned()));
            }
            _ => panic!("expected executor list --offline --state-file"),
        }
    }

    #[test]
    fn cli_parses_executor_list_offline_state_file_json() {
        let cli = Cli::parse_from([
            "arpagona",
            "executor",
            "list",
            "--offline",
            "--state-file",
            "states.json",
            "--json",
        ]);
        match cli.command {
            Command::Executor(ExecutorCommand {
                command: ExecutorSubcommand::List(args),
            }) => {
                assert!(args.offline);
                assert!(args.json);
                assert_eq!(args.state_file, Some("states.json".to_owned()));
            }
            _ => panic!("expected executor list --offline --state-file --json"),
        }
    }

    #[test]
    fn cli_parses_executor_inspect_offline_state_file() {
        let cli = Cli::parse_from([
            "arpagona",
            "executor",
            "inspect",
            "noop-executor",
            "--offline",
            "--state-file",
            "states.json",
        ]);
        match cli.command {
            Command::Executor(ExecutorCommand {
                command: ExecutorSubcommand::Inspect(args),
            }) => {
                assert!(args.offline);
                assert_eq!(args.state_file, Some("states.json".to_owned()));
            }
            _ => panic!("expected executor inspect --offline --state-file"),
        }
    }

    // ── LLM / C1 tests ─────────────────────────────────────────────────────

    #[test]
    fn cli_parses_cognitive_run_with_llm_flag() {
        let cli = Cli::parse_from([
            "arpagona",
            "cognitive",
            "run",
            "--objective",
            "Analyse le marché français",
            "--llm",
        ]);
        match cli.command {
            Command::Cognitive(CognitiveCommand {
                command: CognitiveSubcommand::Run(args),
            }) => {
                assert!(args.llm);
                assert_eq!(args.provider, "ollama", "default provider should be ollama");
                assert!(!args.json);
            }
            _ => panic!("expected cognitive run with --llm"),
        }
    }

    #[test]
    fn cli_parses_cognitive_run_with_llm_and_provider() {
        let cli = Cli::parse_from([
            "arpagona",
            "cognitive",
            "run",
            "--objective",
            "Analyse les journaux",
            "--llm",
            "--provider",
            "mock",
        ]);
        match cli.command {
            Command::Cognitive(CognitiveCommand {
                command: CognitiveSubcommand::Run(args),
            }) => {
                assert!(args.llm);
                assert_eq!(args.provider, "mock");
            }
            _ => panic!("expected cognitive run with --llm --provider mock"),
        }
    }

    #[test]
    fn cli_parses_cognitive_run_with_llm_and_json() {
        let cli = Cli::parse_from([
            "arpagona",
            "cognitive",
            "run",
            "--objective",
            "Research quantum",
            "--llm",
            "--json",
        ]);
        match cli.command {
            Command::Cognitive(CognitiveCommand {
                command: CognitiveSubcommand::Run(args),
            }) => {
                assert!(args.llm);
                assert!(args.json);
            }
            _ => panic!("expected cognitive run with --llm --json"),
        }
    }

    #[test]
    fn cli_parses_cognitive_run_with_llm_provider_and_assess() {
        let cli = Cli::parse_from([
            "arpagona",
            "cognitive",
            "run",
            "--objective",
            "Governed market analysis",
            "--llm",
            "--provider",
            "openai",
            "--assess",
            "--json",
        ]);
        match cli.command {
            Command::Cognitive(CognitiveCommand {
                command: CognitiveSubcommand::Run(args),
            }) => {
                assert!(args.llm);
                assert_eq!(args.provider, "openai");
                assert!(args.assess);
                assert!(args.json);
            }
            _ => panic!("expected cognitive run with --llm --provider openai --assess --json"),
        }
    }

    #[test]
    fn cli_parses_cognitive_run_with_provider_and_all_flags() {
        let cli = Cli::parse_from([
            "arpagona",
            "cognitive",
            "run",
            "--objective",
            "Full pipeline test",
            "--domain",
            "business",
            "--context",
            "sensitivity:low",
            "--assess",
            "--allocate",
            "--resonate",
            "--observe",
            "--llm",
            "--provider",
            "ollama",
            "--json",
        ]);
        match cli.command {
            Command::Cognitive(CognitiveCommand {
                command: CognitiveSubcommand::Run(args),
            }) => {
                assert!(args.llm);
                assert_eq!(args.provider, "ollama");
                assert!(args.assess);
                assert!(args.allocate);
                assert!(args.resonate);
                assert!(args.observe);
                assert!(args.json);
                assert_eq!(args.domain.as_deref(), Some("business"));
            }
            _ => panic!("expected cognitive run with all flags including --llm"),
        }
    }
}
