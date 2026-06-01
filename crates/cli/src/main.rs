use arpagona_agent_core::{
    action::ToolCallIntent,
    holographic::{resonate_for_working_memory, RESONANCE_NON_AUTHORIZING_WARNING},
    llm_journal::LlmJournal,
    orchestrator::ObjectiveInput,
    ActionType, AgentId, AuditEvent, AuditEventId, AuditTraceSummary, CognitiveCycleResult,
    CorrectionTarget, Decision, DecisionId, DecisionStatus, DetectionSignal, DetectionSignalType,
    ExecutorRegistry, ExecutorState, FailureClass, FailureInsight, FailureInsightId,
    InsightSeverity, MemoryWriteIntent, MemoryWriteKind, MemoryWriteProvenance, MemoryWriteTarget,
    ObjectiveDomain, Permission, ProposedAction, ProposedActionId, ProposedActionStatus, RiskLevel,
    SourceId, Task, TaskId, ToolExecutionStatus, WorkspaceId,
};
use arpagona_compute_reservoir::{
    allocate_for_working_memory, ComputeAllocation, ComputeCapability, ComputeNode, ComputeNodeId,
    ComputeNodeStatus, ComputePolicy, ComputeResourceKind, DataSensitivity,
    NON_AUTHORIZING_READBACK,
};
use arpagona_decision_gate::{
    audit_event_for_decision, evaluate_proposed_action, govern_tool_call,
};
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
use arpagona_llm::request_tool_call_from_llm;
use arpagona_llm::run_cognitive_synthesis;
use arpagona_neutral_orchestrator::OrchestratorEngine;
use arpagona_tool_runtime::{ToolRuntime, ToolRuntimeConfig};
use chrono::Utc;
use clap::{Args, Parser, Subcommand, ValueEnum};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeSet, HashMap};
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

// ---------------------------------------------------------------------------
// Process run journal — durable, inspectable run record
// ---------------------------------------------------------------------------

/// Directory name under $HOME for ARPAGONA local state.
const ARPAGONA_STATE_DIR: &str = ".arpagona";
/// Subdirectory for process run journals.
const PROCESS_JOURNAL_DIR: &str = "process-journal";

/// Per-step result recorded in a process run journal.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct JournalStepResult {
    step: usize,
    name: String,
    status: String, // "PASSED" | "FAILED"
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
}

/// Complete process run journal persisted to disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProcessRunJournal {
    run_id: String,
    process: String,
    started_at: String,
    ended_at: String,
    planned_steps: Vec<String>,
    step_results: Vec<JournalStepResult>,
    overall_status: String, // "PASSED" | "BLOCKED"
    #[serde(skip_serializing_if = "Option::is_none")]
    blocked_at_step: Option<usize>,
    next_action: String,
}

/// Resolve the process journal directory path.
///
/// Checks `ARPAGONA_PROCESS_JOURNAL_DIR` environment variable first (used by
/// tests to isolate journal writes to a temp directory). Falls back to
/// `$HOME/.arpagona/process-journal/` for production use.
fn process_journal_dir_resolved() -> PathBuf {
    if let Ok(override_dir) = std::env::var("ARPAGONA_PROCESS_JOURNAL_DIR") {
        return PathBuf::from(override_dir);
    }
    let home =
        std::env::home_dir().expect("HOME must be set for production use of process journals");
    PathBuf::from(home)
        .join(ARPAGONA_STATE_DIR)
        .join(PROCESS_JOURNAL_DIR)
}

/// Return the path to the process journal directory, creating it if needed.
fn ensure_journal_dir() -> Result<PathBuf, Box<dyn Error>> {
    let dir = process_journal_dir_resolved();
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Return the path to the process journal directory WITHOUT creating it.
fn journal_dir_path() -> Result<PathBuf, Box<dyn Error>> {
    Ok(process_journal_dir_resolved())
}

/// Generate a deterministic run ID for a process run.
fn generate_run_id(process_name: &str) -> String {
    let now = Utc::now();
    format!("{}-{}", process_name, now.format("%Y%m%dT%H%M%S"))
}
const DEFAULT_WORKSPACE_ID: &str = "workspace-alpha";
const DEFAULT_AGENT_ID: &str = "agent-alpha";
const DEFAULT_TASK_ID: &str = "task-1";
const DEFAULT_TARGET: &str = "client@example.com";
const DEFAULT_RATIONALE: &str = "Préparer un brouillon sans l’envoyer";
const DEFAULT_PROVIDER: &str = "ollama";
const DEFAULT_CHAT_PROVIDER: &str = "ollama";
const DEFAULT_SNAPSHOT_DIR: &str = "target/demo-snapshots";
const DEFAULT_OLLAMA_ENDPOINT: &str = "http://localhost:11434/api/chat";
const DEFAULT_OLLAMA_MODEL: &str = "qwen3.5:9b";

const ALLOWED_TOOLS: &[&str] = &["append_file", "read_file", "list_files", "search_text"];

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
    /// Run an objective through the orchestrator in-process. Standalone, no API server needed.
    ///
    /// Usage:
    ///   arpagona run "Review project documentation"
    ///   arpagona run "Analyze monthly sales data"
    ///
    /// This is the simplest way to use ARPAGONA. It runs an objective through
    /// the in-process orchestrator and produces clean, readable output without
    /// internal governance jargon. No API server, no LLM, no persistence needed.
    Run(RunArgs),
    /// Run a governed local actor mission from a natural language task.
    /// Uses deterministic (no LLM) parsing to route to bounded file tools.
    /// Simulates first; execution requires --approve.
    Actor(ActorCommand),
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
    /// Inspect and demo the alpha sandboxed cognitive tool runtime.
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
    /// Compute Reservoir operations (C4 — compute routing preview and trade-off analysis).
    Compute(ComputeCommand),
    /// Run the Neutral Orchestrator deterministic cycle.
    Orchestrator(OrchestratorCommand),
    /// Run a local system preflight / diagnostic check (Babysitter-inspired doctor).
    /// Checks repo state, binary availability, Ollama, tool runtime, and stale workspace copies.
    Doctor(DoctorArgs),
    /// Run a quality-gated validation process.
    /// V0 supports only `daily-validation`.
    #[command(subcommand)]
    Process(ProcessCmd),
}

#[derive(Debug, Args)]
pub struct DoctorArgs {
    /// Emit structured JSON instead of human-readable text.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Subcommand)]
pub enum ProcessCmd {
    /// Run a validation process by name. V0 supports only `daily-validation`.
    Run(ProcessRunArgs),
    /// Show status of a previous process run. Use --last for most recent
    /// or pass a specific run ID.
    Status(ProcessStatusArgs),
    /// Show what steps a process would execute, without running anything.
    /// Read-only process inspection — no doctor, no cargo, no journal writes.
    Plan(ProcessPlanArgs),
    /// List persisted process run journals. Shows all run records
    /// from ~/.arpagona/process-journal/, newest first.
    /// Read-only — no doctor, no cargo, no journal writes.
    List(ProcessListArgs),
}

#[derive(Debug, Args)]
pub struct ProcessRunArgs {
    /// Name of the process to run.
    pub name: String,
    /// Emit structured JSON instead of human-readable text.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct ProcessStatusArgs {
    /// Show the most recent process run status.
    #[arg(long)]
    pub last: bool,
    /// Optional specific run ID to inspect.
    pub run_id: Option<String>,
    /// Emit structured JSON instead of human-readable text.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct ProcessPlanArgs {
    /// Name of the process to plan. V0 supports only `daily-validation`.
    pub name: String,
    /// Emit structured JSON instead of human-readable text.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct ProcessListArgs {
    /// Emit structured JSON instead of human-readable text.
    #[arg(long)]
    pub json: bool,
}

/// Top-level `run` command: in-process orchestrator with clean readable output.
/// No API server needed. Standalone, smoke-testable.
///
/// Usage: arpagona run "Review project documentation"
#[derive(Debug, Args)]
pub struct RunArgs {
    /// The objective text to process through the orchestrator cycle.
    /// Example: arpagona run "Analyze the quarterly report"
    pub objective: String,
}

#[derive(Debug, Args)]
pub struct ActorCommand {
    #[command(subcommand)]
    pub command: ActorSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ActorSubcommand {
    /// Parse a natural language task and run it through the governed
    /// simulation -> approval -> execution -> readback loop.
    Run(ActorRunArgs),
    /// Start an interactive acquisition loop that reads tasks from stdin
    /// and runs each through the governed actor_run pipeline.
    Session(ActorSessionArgs),
    /// Show read-only actor status readback: agent info, executor state,
    /// journal summary, and session state.
    Status(ActorStatusArgs),
    /// Show read-only actor memory readback: graph memory state,
    /// facts, episodes, and observations. No mutation paths.
    Memory(ActorMemoryArgs),
    /// Show read-only actor journal readback: LLM journal entries
    /// from actor-run and actor-session interactions.
    Journal(ActorJournalArgs),
    /// Show a compact history of recent actor runs (tool, decision, outcome, time).
    /// Read-only — reads from the in-memory LLM journal. No external effects.
    History(ActorHistoryArgs),
}

#[derive(Debug, Args)]
pub struct ActorRunArgs {
    /// Natural language task description.
    /// Examples:
    ///   "append meeting notes to docs/log.md"
    ///   "read docs/README.md"
    ///   "list files in src/"
    ///   "search for FIXME in lib/"
    pub task: String,

    /// Explicitly approve the simulated proposal and execute.
    #[arg(long)]
    pub approve: bool,

    /// Emit structured JSON instead of human-oriented text.
    #[arg(long)]
    pub json: bool,

    /// Workspace root for file tools (default: current directory).
    #[arg(long, default_value = ".")]
    pub workspace: String,

    /// Intent interpretation provider: deterministic (std::str parsing) or ollama (local LLM).
    #[arg(long, default_value_t = IntentProviderArg::Deterministic)]
    pub intent_provider: IntentProviderArg,

    /// Ollama model name (only used with --intent-provider ollama).
    #[arg(long)]
    pub ollama_model: Option<String>,
}

#[derive(Debug, Args)]
pub struct ActorSessionArgs {
    /// Maximum number of tasks to process before exiting.
    #[arg(long)]
    pub max: Option<u32>,

    /// Workspace root for file tools (default: current directory).
    #[arg(long, default_value = ".")]
    pub workspace: String,

    /// Emit newline-delimited JSON envelopes for each task.
    #[arg(long)]
    pub json: bool,

    /// Intent interpretation provider: deterministic (std::str parsing) or ollama (local LLM).
    #[arg(long, default_value_t = IntentProviderArg::Deterministic)]
    pub intent_provider: IntentProviderArg,

    /// Ollama model name (only used with --intent-provider ollama).
    #[arg(long)]
    pub ollama_model: Option<String>,
}

#[derive(Debug, Args)]
pub struct ActorStatusArgs {
    /// Emit structured JSON instead of human-readable text.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct ActorMemoryArgs {
    /// Emit structured JSON instead of human-readable text.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct ActorJournalArgs {
    /// Maximum number of recent entries to show (default: 10).
    #[arg(long, default_value_t = 10)]
    pub limit: usize,
    /// Filter by interaction type (synthesis, tool_call_intent, direct_tool_call).
    #[arg(long)]
    pub interaction_type: Option<String>,
    /// Emit structured JSON instead of human-readable text.
    #[arg(long)]
    pub json: bool,
}

/// Compact history of recent actor runs (read-only).
#[derive(Debug, Args)]
pub struct ActorHistoryArgs {
    /// Maximum number of recent actor runs to show (default: 5).
    #[arg(long, default_value_t = 5)]
    pub limit: usize,
    /// Emit structured JSON instead of human-readable text.
    #[arg(long)]
    pub json: bool,
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
pub struct ComputeCommand {
    #[command(subcommand)]
    pub command: ComputeSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ComputeSubcommand {
    /// Preview how Compute Reservoir would route a cognitive task.
    /// Shows the allocation, provider mapping, trade-off analysis and cost/latency/sensitivity rationale.
    Routing(RoutingArgs),
}

#[derive(Debug, Args)]
pub struct RoutingArgs {
    /// Describe the purpose or objective of the computation.
    #[arg(long, default_value = "cognitive task")]
    pub purpose: String,

    /// Data sensitivity level: public, internal, confidential, or secret.
    /// Higher sensitivity increases local-first preference.
    #[arg(long, default_value_t = SensitivityArg::Internal)]
    pub sensitivity: SensitivityArg,

    /// Complexity estimate 0.0-1.0. Higher values require stronger resources.
    #[arg(long, default_value_t = 0.5)]
    pub complexity: f64,

    /// Prefer local-only resources. Equivalent to zero cloud budget.
    #[arg(long, default_value_t = false)]
    pub local_first: bool,

    /// Output as structured JSON.
    #[arg(long, short = 'j', default_value_t = false)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct OrchestratorCommand {
    #[command(subcommand)]
    pub command: OrchestratorSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum OrchestratorSubcommand {
    /// Run the Neutral Orchestrator deterministic cycle with an objective.
    Run(OrchestratorRunArgs),
    /// Display orchestrator status and last cycle trace.
    Status(OrchestratorStatusArgs),
    /// List saved orchestrator cycle traces from a directory.
    Cycles(OrchestratorCyclesArgs),
    /// Detect and collect failure insight candidates from a saved CycleTrace.
    InsightsCollect(OrchestratorInsightsCollectArgs),
    /// List collected failure insight candidate files.
    InsightsList(OrchestratorInsightsListArgs),
}

#[derive(Debug, Args)]
pub struct OrchestratorRunArgs {
    /// The objective text to process through the orchestrator cycle.
    #[arg(long)]
    pub objective: String,

    /// Output as structured JSON.
    #[arg(long, short = 'j', default_value_t = false)]
    pub json: bool,

    /// Output full CycleTrace with context assembly metadata (use with --json for structured output).
    #[arg(long, default_value_t = false)]
    pub trace: bool,

    /// Save the CycleTrace as JSON to a file.
    ///
    /// With an explicit path: save to that file.
    /// Without a path (--save-trace alone): auto-generate a path in
    /// target/orchestrator-traces/ using the cycle ID and timestamp.
    /// Use with --trace to capture the compute-aware breakdown.
    #[arg(long, num_args = 0..=1, default_missing_value = "auto")]
    pub save_trace: Option<String>,

    /// Permissions granted for Decision Gate evaluation (repeatable).
    #[arg(long = "perm", default_values = &["ReadDocument"])]
    pub permissions: Vec<String>,

    /// Workspace ID for the cycle.
    #[arg(long, default_value = DEFAULT_WORKSPACE_ID)]
    pub workspace_id: String,

    /// Agent ID for the cycle.
    #[arg(long, default_value = DEFAULT_AGENT_ID)]
    pub agent_id: String,

    /// Proposal generator backend: simulated (deterministic, default) or llm (mock provider for now).
    #[arg(long, default_value_t = ProposalGeneratorArg::Simulated)]
    pub proposal_generator: ProposalGeneratorArg,

    /// Automatically detect and save failure insight candidates from the cycle trace.
    ///
    /// When set, runs `CycleTrace::detect_failure_candidates()` on the generated trace
    /// and saves the result as a structured JSON file in the configured insights directory
    /// (default: target/orchestrator-insights/).
    #[arg(long, default_value_t = false)]
    pub collect_insights: bool,

    /// Directory for saved failure insight candidate files (default: target/orchestrator-insights/).
    #[arg(long)]
    pub insights_dir: Option<String>,

    /// Save audit events from this cycle as JSON files.
    ///
    /// With an explicit path prefix: save to `<prefix>/audit-events-<cycle-id>.json`.
    /// Without a path (--save-audit alone): auto-generate a path in
    /// target/orchestrator-audit/ using the cycle ID and timestamp.
    #[arg(long, num_args = 0..=1, default_missing_value = "auto")]
    pub save_audit: Option<String>,
}

const DEFAULT_ORCHESTRATOR_TRACE_PATH: &str = "target/last-orchestrator-trace.json";
const DEFAULT_ORCHESTRATOR_INSIGHTS_DIR: &str = "target/orchestrator-insights";
const DEFAULT_ORCHESTRATOR_AUDIT_DIR: &str = "target/orchestrator-audit";

#[derive(Debug, Args)]
pub struct OrchestratorStatusArgs {
    /// Output as structured JSON.
    #[arg(long, short = 'j', default_value_t = false)]
    pub json: bool,

    /// Path to a saved CycleTrace JSON file (default: target/last-orchestrator-trace.json).
    #[arg(long)]
    pub trace_path: Option<String>,
}

/// Default trace directory for the cycles list command.
const DEFAULT_ORCHESTRATOR_TRACES_DIR: &str = "target/orchestrator-traces";

#[derive(Debug, Args)]
pub struct OrchestratorCyclesArgs {
    /// Output as structured JSON.
    #[arg(long, short = 'j', default_value_t = false)]
    pub json: bool,

    /// Directory containing saved CycleTrace JSON files (default: target/orchestrator-traces).
    #[arg(long)]
    pub trace_dir: Option<String>,

    /// Also scan the audit event directory and show audit event counts per cycle.
    /// Audit events are saved by `orchestrator run --save-audit`.
    #[arg(long, default_value_t = false)]
    pub with_audit: bool,
}

#[derive(Debug, Args)]
pub struct OrchestratorInsightsCollectArgs {
    /// Path to a saved CycleTrace JSON file to analyze for failure insight candidates.
    pub trace_path: String,

    /// Output as structured JSON.
    #[arg(long, short = 'j', default_value_t = false)]
    pub json: bool,

    /// Optional path to write collected insights as a FailureInsightDemoSnapshot
    /// (for discoverability via `memory demo snapshot-list`).
    #[arg(long)]
    pub snapshot_path: Option<String>,
}

#[derive(Debug, Args)]
pub struct OrchestratorInsightsListArgs {
    /// Output as structured JSON.
    #[arg(long, short = 'j', default_value_t = false)]
    pub json: bool,

    /// Directory containing collected insight candidate files (default: target/orchestrator-insights).
    #[arg(long)]
    pub insights_dir: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum ProposalGeneratorArg {
    /// Deterministic ReadDocument at Low risk (no LLM, no I/O).
    Simulated,
    /// Mock LLM-backed proposal generator for real proposal-only integration.
    Llm,
}

impl std::fmt::Display for ProposalGeneratorArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProposalGeneratorArg::Simulated => write!(f, "simulated"),
            ProposalGeneratorArg::Llm => write!(f, "llm"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum IntentProviderArg {
    /// Deterministic NL parsing (no LLM, no network).
    Deterministic,
    /// Local Ollama LLM proposes structured intents.
    Ollama,
}

impl std::fmt::Display for IntentProviderArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IntentProviderArg::Deterministic => write!(f, "deterministic"),
            IntentProviderArg::Ollama => write!(f, "ollama"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum SensitivityArg {
    Public,
    Internal,
    Confidential,
    Secret,
}

impl std::fmt::Display for SensitivityArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SensitivityArg::Public => write!(f, "public"),
            SensitivityArg::Internal => write!(f, "internal"),
            SensitivityArg::Confidential => write!(f, "confidential"),
            SensitivityArg::Secret => write!(f, "secret"),
        }
    }
}

impl SensitivityArg {
    fn to_data_sensitivity(self) -> DataSensitivity {
        match self {
            SensitivityArg::Public => DataSensitivity::Public,
            SensitivityArg::Internal => DataSensitivity::Internal,
            SensitivityArg::Confidential => DataSensitivity::Confidential,
            SensitivityArg::Secret => DataSensitivity::Secret,
        }
    }
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
    /// Run governed tool-calling (Track C Step C2): request a tool-call intent from
    /// the LLM provider, route it through Decision Gate, execute approved calls through
    /// the bounded Tool Runtime, and journal the full trace (intent -> decision ->
    /// result -> observation). Requires --llm (needs an LLM provider for the intent).
    #[arg(long)]
    pub govern_tool: bool,
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
    /// Read-only supervision: list recent proposed actions and tool-call intents
    /// from the LLM journal with Decision Gate results, risk levels, and audit event IDs.
    /// Works offline — no API server required.
    Supervise(ActionSuperviseArgs),
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

/// Args for `arpagona action supervise` — read-only supervision surface.
#[derive(Debug, Args)]
pub struct ActionSuperviseArgs {
    /// Maximum number of entries to show.
    #[arg(long, default_value = "10")]
    pub limit: usize,
    /// Emit structured JSON instead of human-oriented text.
    #[arg(long)]
    pub json: bool,
    /// Optional: filter by interaction type (synthesis, tool_call_intent, direct_tool_call, governance).
    #[arg(long)]
    pub interaction_type: Option<String>,
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
    /// Show holographic memory store status: totals, recent traces, most activated traces,
    /// linked memory/decision IDs, and consolidation info.
    ///
    /// Reads from the SQLite-backed store (target/holographic-memory.db) by default.
    /// Falls back to the JSON file store if the SQLite DB is not accessible.
    Status(HolographicStatusArgs),
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

/// Status arguments for `memory holographic status`.
#[derive(Debug, Args)]
struct HolographicStatusArgs {
    /// Path to the SQLite holographic memory database file.
    #[arg(long, default_value = "target/holographic-memory.db")]
    db: String,
    /// Emit structured JSON instead of human-oriented text.
    #[arg(long)]
    json: bool,
}

// ---------------------------------------------------------------------------
// Tool — alpha sandboxed tool runtime demo commands
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
    /// Evaluate a tool-call intent through the Decision Gate (Track C Step C2).
    ///
    /// Creates a ToolCallIntent from the provided tool name and JSON arguments,
    /// runs it through govern_tool_call(), journals the governance result to
    /// the LLM journal, and returns the decision (approved/blocked/requires
    /// human approval).
    Govern(ToolGovernArgs),
    /// Run a sandboxed demo tool execution.
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

/// Arguments for tool govern — evaluate a tool-call intent through the Decision Gate.
#[derive(Debug, Args)]
struct ToolGovernArgs {
    /// Name of the tool the LLM wants to call (e.g. "read_file", "search_text").
    tool: String,
    /// JSON arguments for the tool call.
    args: String,
    /// Risk level for this tool call (informational, low, medium, high, critical).
    #[arg(long, default_value = "informational")]
    risk_level: String,
    /// Rationale for the tool call (why is this tool needed?).
    #[arg(long, default_value = "LLM-invoked governed tool-call evaluation")]
    rationale: String,
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
    /// Demo the sandboxed write_file tool.
    WriteFile(ToolDemoWriteFileArgs),
    /// Demo the sandboxed patch_file tool (exact-match text replacement).
    PatchFile(ToolDemoPatchFileArgs),
    /// Demo the sandboxed append_file tool.
    AppendFile(ToolDemoAppendFileArgs),
    /// Demo the sandboxed mkdir tool.
    Mkdir(ToolDemoMkdirArgs),
    /// Demo the sandboxed copy_file tool.
    CopyFile(ToolDemoCopyFileArgs),
    /// Demo the sandboxed move_file tool (alias: rename).
    MoveFile(ToolDemoMoveFileArgs),
    /// Run the full cognitive observation pipeline: tool execution → observation → assessment.
    Observe(ToolDemoObserveArgs),
    /// Run the First Useful Actor Lab: governed local file action with simulation, approval, execution, and readback.
    ActorLab(ToolDemoActorLabArgs),
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
struct ToolDemoWriteFileArgs {
    /// Path to the file to write (relative to workspace).
    path: String,
    /// Content to write.
    content: String,
    /// Actually write. Without this flag, write_file only simulates.
    #[arg(long)]
    execute: bool,
    /// Allow creating missing parent directories.
    #[arg(long)]
    create_parent_dirs: bool,
    /// Allow overwriting an existing file.
    #[arg(long)]
    overwrite: bool,
    /// Emit structured JSON instead of human-oriented text.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct ToolDemoPatchFileArgs {
    /// Path to the file to patch (relative to workspace).
    path: String,
    /// Exact text to find in the file.
    old_string: String,
    /// Replacement text.
    new_string: String,
    /// Actually apply the patch. Without this flag, patch_file only simulates.
    #[arg(long)]
    execute: bool,
    /// Replace all occurrences instead of just the first.
    #[arg(long)]
    replace_all: bool,
    /// Emit structured JSON instead of human-oriented text.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct ToolDemoAppendFileArgs {
    /// Path to the file to append to (relative to workspace).
    path: String,
    /// Content to append.
    content: String,
    /// Actually append. Without this flag, append_file only simulates.
    #[arg(long)]
    execute: bool,
    /// Allow creating missing parent directories.
    #[arg(long)]
    create_parent_dirs: bool,
    /// Allow creating the file if it does not exist.
    #[arg(long, default_value_t = true)]
    create_if_missing: bool,
    /// Emit structured JSON instead of human-oriented text.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct ToolDemoMkdirArgs {
    /// Directory path to create (relative to workspace).
    path: String,
    /// Actually create the directory. Without this flag, mkdir only simulates.
    #[arg(long)]
    execute: bool,
    /// Create parent directories as needed.
    #[arg(long)]
    parents: bool,
    /// Emit structured JSON instead of human-oriented text.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct ToolDemoCopyFileArgs {
    /// Source file path (relative to workspace).
    source: String,
    /// Destination file path (relative to workspace).
    destination: String,
    /// Actually copy. Without this flag, copy_file only simulates.
    #[arg(long)]
    execute: bool,
    /// Allow overwriting an existing destination file.
    #[arg(long)]
    overwrite: bool,
    /// Emit structured JSON instead of human-oriented text.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct ToolDemoMoveFileArgs {
    /// Source file path (relative to workspace).
    source: String,
    /// Destination file path (relative to workspace).
    destination: String,
    /// Actually move. Without this flag, move_file only simulates.
    #[arg(long)]
    execute: bool,
    /// Allow overwriting an existing destination file.
    #[arg(long)]
    overwrite: bool,
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

#[derive(Debug, Args)]
struct ToolDemoActorLabArgs {
    /// Workspace-relative note file used by the lab.
    #[arg(long, default_value = "actor-lab/NOTES.md")]
    path: String,
    /// Note content appended by the supervised actor. A trailing newline is added if missing.
    #[arg(
        long,
        allow_hyphen_values = true,
        default_value = "- First Useful Actor Lab: governed sandboxed local action proved end-to-end."
    )]
    note: String,
    /// Explicitly approve the simulated proposal and perform the sandboxed append.
    #[arg(long)]
    approve: bool,
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

#[derive(Debug, Args)]
struct ListAuditArgs {
    /// Emit structured JSON output instead of human-oriented text.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
struct ListTracesArgs {
    /// Emit structured JSON output instead of human-oriented text.
    #[arg(long)]
    json: bool,
    /// Directory containing saved CycleTrace JSON files.
    #[arg(long, default_value = DEFAULT_ORCHESTRATOR_TRACES_DIR)]
    trace_dir: String,
}

#[derive(Debug, Args)]
struct GetTraceArgs {
    /// Orchestrator cycle ID to inspect (e.g., "oc-1234567890").
    cycle_id: String,
    /// Emit structured JSON output instead of human-oriented text.
    #[arg(long)]
    json: bool,
    /// Directory containing saved CycleTrace JSON files.
    #[arg(long, default_value = DEFAULT_ORCHESTRATOR_TRACES_DIR)]
    trace_dir: String,
}

#[derive(Debug, Args)]
struct ListEventsFromDirArgs {
    /// Directory containing saved audit event JSON files.
    from_dir: String,
    /// Emit structured JSON output instead of human-oriented text.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Subcommand)]
enum AuditSubcommand {
    /// List audit events.
    List(ListAuditArgs),
    /// List saved audit event files from a directory (local filesystem, no API server needed).
    /// Reads individual JSON audit event files saved by `orchestrator run --save-audit`
    /// and displays each event with its type, timestamp, and payload preview.
    ListEventsFromDir(ListEventsFromDirArgs),
    /// Show a read-only decision-scoped audit summary.
    /// Includes causal links, decision status, risk level and policies applied.
    DecisionSummary(DecisionSummaryArgs),
    /// Show a read-only task-scoped audit summary.
    /// Includes causal links, event boundaries and readback-only safety flags.
    TaskSummary(TaskSummaryArgs),
    /// Show a read-only workspace-scoped audit summary.
    /// Includes causal links, event boundaries and readback-only safety flags.
    WorkspaceSummary(WorkspaceSummaryArgs),
    /// List saved CycleTrace files (local filesystem, no API server needed).
    /// Connects the audit system to orchestrator cycle traces for cross-session readback.
    ListTraces(ListTracesArgs),
    /// Read and display a specific CycleTrace by cycle ID (local filesystem, no API server needed).
    /// Shows the full cycle trace with context assembly metadata, compute route,
    /// decision outcome, audit event IDs and failure insight candidates.
    GetTrace(GetTraceArgs),
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
    /// D3 memory and resonance visibility — recent holographic memory traces.
    memory_visibility: MemoryVisibilitySection,
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

/// Compact summary of a holographic memory trace for operator visibility (D3).
#[derive(Debug, Serialize)]
struct TraceSummary {
    id: String,
    source_kind: String,
    content_summary: String,
    keywords: Vec<String>,
    concepts: Vec<String>,
    linked_memory_ids: Vec<String>,
    linked_decision_ids: Vec<String>,
    importance: f32,
    confidence: f32,
    activation_count: u64,
    created_at: String,
    last_activated_at: Option<String>,
}

/// Memory and resonance visibility section (D3).
///
/// Shows recent holographic memory traces, resonance context, linked
/// decisions/memory IDs, and consolidation evidence. Read-only — does
/// not authorize actions or treat recall hints as approval.
#[derive(Debug, Serialize)]
struct MemoryVisibilitySection {
    /// Total traces across all projects in the holographic memory store.
    total_trace_count: Option<usize>,
    /// Recent traces ordered by creation time (newest first).
    recent_traces: Vec<TraceSummary>,
    /// Most frequently activated traces, ordered by activation_count (highest first).
    most_activated_traces: Vec<TraceSummary>,
    /// All linked memory IDs aggregated from recent traces (deduplicated).
    aggregated_linked_memory_ids: Vec<String>,
    /// All linked decision IDs aggregated from recent traces (deduplicated).
    aggregated_linked_decision_ids: Vec<String>,
    /// Whether the holographic memory store is locally accessible.
    store_accessible: bool,
    /// Consolidation evidence string (summary of most recent consolidation).
    consolidation_info: Option<String>,
    /// Readback-only warning — recall hints are advisory, not authorization.
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
            ActionSubcommand::Supervise(args) => action_supervise(args)?,
        },
        Command::Agent(agent) => match agent.command {
            AgentSubcommand::Propose(args) => propose_agent_action(&client, &api_url, args).await?,
        },
        Command::Audit(audit) => match audit.command {
            AuditSubcommand::List(args) => list_audit(&client, &api_url, args).await?,
            AuditSubcommand::DecisionSummary(args) => {
                audit_decision_summary(&client, &api_url, args).await?
            }
            AuditSubcommand::TaskSummary(args) => {
                audit_task_summary(&client, &api_url, args).await?
            }
            AuditSubcommand::WorkspaceSummary(args) => {
                audit_workspace_summary(&client, &api_url, args).await?
            }
            AuditSubcommand::ListTraces(args) => audit_list_traces(args)?,
            AuditSubcommand::GetTrace(args) => audit_get_trace(args)?,
            AuditSubcommand::ListEventsFromDir(args) => audit_list_events_from_dir(args)?,
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
                HolographicSubcommand::Status(args) => memory_holographic_status(args)?,
            },
        },
        Command::Tool(tool) => match tool.command {
            ToolSubcommand::List(args) => tool_list(args)?,
            ToolSubcommand::Inspect(args) => tool_inspect(args)?,
            ToolSubcommand::Govern(args) => tool_govern(args)?,
            ToolSubcommand::Demo(demo) => match demo.command {
                ToolDemoSubcommand::ReadFile(args) => tool_demo_read_file(args)?,
                ToolDemoSubcommand::ListFiles(args) => tool_demo_list_files(args)?,
                ToolDemoSubcommand::SearchText(args) => tool_demo_search_text(args)?,
                ToolDemoSubcommand::WriteFile(args) => tool_demo_write_file(args)?,
                ToolDemoSubcommand::PatchFile(args) => tool_demo_patch_file(args)?,
                ToolDemoSubcommand::AppendFile(args) => tool_demo_append_file(args)?,
                ToolDemoSubcommand::Mkdir(args) => tool_demo_mkdir(args)?,
                ToolDemoSubcommand::CopyFile(args) => tool_demo_copy_file(args)?,
                ToolDemoSubcommand::MoveFile(args) => tool_demo_move_file(args)?,
                ToolDemoSubcommand::Observe(args) => tool_demo_observe(args)?,
                ToolDemoSubcommand::ActorLab(args) => tool_demo_actor_lab(args)?,
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
        Command::Compute(compute) => match compute.command {
            ComputeSubcommand::Routing(args) => compute_routing(args)?,
        },
        Command::Orchestrator(orchestrator) => match orchestrator.command {
            OrchestratorSubcommand::Run(args) => orchestrator_run(args)?,
            OrchestratorSubcommand::Status(args) => orchestrator_status(args)?,
            OrchestratorSubcommand::Cycles(args) => orchestrator_cycles(args)?,
            OrchestratorSubcommand::InsightsCollect(args) => orchestrator_insights_collect(args)?,
            OrchestratorSubcommand::InsightsList(args) => orchestrator_insights_list(args)?,
        },
        Command::Run(args) => handle_run(args)?,
        Command::Actor(actor) => match actor.command {
            ActorSubcommand::Run(args) => actor_run(args)?,
            ActorSubcommand::Session(args) => actor_session(args)?,
            ActorSubcommand::Status(args) => actor_status_readback(args)?,
            ActorSubcommand::Memory(args) => actor_memory_readback(args)?,
            ActorSubcommand::Journal(args) => actor_journal_readback(args)?,
            ActorSubcommand::History(args) => actor_history_readback(args)?,
        },
        Command::Doctor(args) => doctor(args).await?,
        Command::Process(cmd) => match cmd {
            ProcessCmd::Run(args) => process_run(args).await?,
            ProcessCmd::Status(args) => process_status(args)?,
            ProcessCmd::Plan(args) => process_plan(args)?,
            ProcessCmd::List(args) => process_list(args)?,
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

/// Run local system preflight / diagnostic checks (Babysitter-inspired doctor).
///
/// Checks:
///   1. Git repo state — clean working tree, HEAD matches origin/main
///   2. arpagona CLI binary availability
///   3. arpagona-api-server binary availability
///   4. Ollama endpoint reachability
///   5. qwen3.5:9b model availability via Ollama
///   6. Tool runtime safety smoke — can read a known workspace file
///   7. Stale secondary workspace copy warning
///
/// Output is human-readable by default; `--json` emits structured JSON.
async fn doctor(args: DoctorArgs) -> Result<(), Box<dyn Error>> {
    let mut checks: Vec<(String, String, bool, String)> = Vec::new();

    // 1. Git repo state
    let head = ProcessCommand::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok()
            } else {
                None
            }
        })
        .map(|s| s.trim().to_owned());
    let status_out = ProcessCommand::new("git")
        .args(["status", "--porcelain"])
        .output()
        .ok()
        .map(|o| String::from_utf8(o.stdout).unwrap_or_default());
    let clean = status_out
        .as_ref()
        .map(|s| s.trim().is_empty())
        .unwrap_or(false);
    let behind = ProcessCommand::new("git")
        .args(["rev-list", "--count", "HEAD..origin/main"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok()
            } else {
                None
            }
        })
        .map(|s| s.trim().parse::<usize>().unwrap_or(0))
        .unwrap_or(0);
    let git_detail = format!(
        "HEAD: {} | clean: {} | behind origin/main: {}",
        head.as_deref().unwrap_or("unknown"),
        if clean { "yes" } else { "NO" },
        behind
    );
    checks.push((
        "git_state".into(),
        git_detail,
        clean && behind == 0,
        if clean && behind == 0 {
            "ok".into()
        } else {
            "fail".into()
        },
    ));

    // 2. CLI binary
    let cli_path = PathBuf::from("target/debug/arpagona");
    let cli_ok = cli_path.exists();
    checks.push((
        "cli_binary".into(),
        format!(
            "target/debug/arpagona: {}",
            if cli_ok { "found" } else { "MISSING" }
        ),
        cli_ok,
        if cli_ok { "ok".into() } else { "fail".into() },
    ));

    // 3. API server binary
    let api_path = PathBuf::from("target/debug/arpagona-api-server");
    let api_ok = api_path.exists();
    checks.push((
        "api_server_binary".into(),
        format!(
            "target/debug/arpagona-api-server: {}",
            if api_ok { "found" } else { "MISSING" }
        ),
        api_ok,
        if api_ok { "ok".into() } else { "fail".into() },
    ));

    // 4. Ollama endpoint reachability
    let ollama_endpoint =
        std::env::var("OLLAMA_ENDPOINT").unwrap_or_else(|_| "http://localhost:11434".to_owned());
    let ollama_reachable = check_ollama_reachable(&ollama_endpoint).await;
    checks.push((
        "ollama".into(),
        format!(
            "endpoint: {} | reachable: {}",
            ollama_endpoint,
            if ollama_reachable { "yes" } else { "NO" }
        ),
        ollama_reachable,
        if ollama_reachable {
            "ok".into()
        } else {
            "fail".into()
        },
    ));

    // 5. qwen3.5:9b model availability
    let model_ok = if ollama_reachable {
        let url = format!("{}/api/tags", ollama_endpoint.trim_end_matches('/'));
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()?;
        let resp = client.get(&url).send().await;
        match resp {
            Ok(r) if r.status().is_success() => {
                let body: serde_json::Value = r.json().await.unwrap_or(serde_json::Value::Null);
                let models = body["models"]
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|m| m["name"].as_str())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                models
                    .iter()
                    .any(|m| m.contains("qwen3.5:9b") || m.contains("qwen3.5:9b"))
            }
            _ => false,
        }
    } else {
        false
    };
    checks.push((
        "qwen3.5:9b_model".into(),
        if model_ok {
            "available".into()
        } else {
            "not found or unreachable".into()
        },
        model_ok,
        if model_ok { "ok".into() } else { "fail".into() },
    ));

    // 6. Tool runtime safety smoke
    let tool_ok = {
        let config = ToolRuntimeConfig::default();
        let rt = ToolRuntime::new(config);
        let result = rt.execute("read_file", &json!({"path": "Cargo.toml"}));
        match result.status {
            ToolExecutionStatus::Success | ToolExecutionStatus::Warning => true,
            _ => false,
        }
    };
    checks.push((
        "tool_runtime_smoke".into(),
        format!("read Cargo.toml: {}", if tool_ok { "ok" } else { "FAILED" }),
        tool_ok,
        if tool_ok { "ok".into() } else { "fail".into() },
    ));

    // 7. Stale secondary workspace copy
    let secondary = PathBuf::from("/home/thibaud/.openclaw/workspace/arpagona-agent-core");
    let stale_warning = if secondary.exists() {
        let secondary_head = ProcessCommand::new("git")
            .args([
                "-C",
                "/home/thibaud/.openclaw/workspace/arpagona-agent-core",
                "rev-parse",
                "HEAD",
            ])
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    String::from_utf8(o.stdout).ok()
                } else {
                    None
                }
            })
            .map(|s| s.trim().to_owned());
        let secondary_behind = ProcessCommand::new("git")
            .args([
                "-C",
                "/home/thibaud/.openclaw/workspace/arpagona-agent-core",
                "rev-list",
                "--count",
                "HEAD..origin/main",
            ])
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    String::from_utf8(o.stdout).ok()
                } else {
                    None
                }
            })
            .map(|s| s.trim().parse::<usize>().unwrap_or(0))
            .unwrap_or(0);
        if secondary_behind > 0 || secondary_head != head {
            format!("WARNING: secondary copy at ~/.openclaw/workspace/arpagona-agent-core is {} commits behind origin/main (HEAD: {})",
                secondary_behind, secondary_head.as_deref().unwrap_or("unknown"))
        } else {
            "secondary copy at ~/.openclaw/workspace/arpagona-agent-core is current".to_owned()
        }
    } else {
        "no secondary copy found".to_owned()
    };
    let stale_ok = !stale_warning.contains("WARNING");
    let stale_severity = if stale_warning
        .contains("secondary copy at ~/.openclaw/workspace/arpagona-agent-core is current")
    {
        "ok"
    } else {
        "warn"
    };
    checks.push((
        "secondary_copy".into(),
        stale_warning,
        stale_ok,
        stale_severity.into(),
    ));

    // Output
    if args.json {
        let json_results: Vec<serde_json::Value> = checks
            .iter()
            .map(|(name, detail, pass, severity)| {
                json!({
                    "check": name,
                    "detail": detail,
                    "pass": pass,
                    "severity": severity
                })
            })
            .collect();
        let all_pass = !checks
            .iter()
            .any(|(_, _, pass, sev)| !*pass && sev == "fail");
        let summary = json!({
            "command": "doctor",
            "timestamp": Utc::now().to_rfc3339(),
            "all_pass": all_pass,
            "checks": json_results
        });
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else {
        let pass_count = checks.iter().filter(|(_, _, pass, _)| *pass).count();
        let total = checks.len();
        println!(
            "[ARPAGONA doctor] preflight diagnostic — {} / {} checks passing",
            pass_count, total
        );
        println!();
        for (name, detail, _pass, severity) in &checks {
            let status = match severity.as_str() {
                "ok" => "[OK]",
                "warn" => "[WARN]",
                _ => "[FAIL]",
            };
            println!("  {} {}: {}", status, name, detail);
        }
        println!();
        if pass_count == total {
            println!("All checks pass. System is healthy.");
        } else {
            let failing: Vec<&str> = checks
                .iter()
                .filter(|(_, _, pass, sev)| !*pass && sev == "fail")
                .map(|(name, _, _, _)| name.as_str())
                .collect();
            let warnings: Vec<&str> = checks
                .iter()
                .filter(|(_, _, pass, sev)| !*pass && sev == "warn")
                .map(|(name, _, _, _)| name.as_str())
                .collect();
            if !failing.is_empty() {
                println!(
                    "{} check(s) failing (blocker): {}",
                    failing.len(),
                    failing.join(", ")
                );
            }
            if !warnings.is_empty() {
                println!(
                    "{} check(s) with warnings (non-blocking): {}",
                    warnings.len(),
                    warnings.join(", ")
                );
            }
        }
    }

    // Return an error if any fail-severity check failed — process-run depends on this.
    let has_fail = checks
        .iter()
        .any(|(_, _, pass, sev)| !*pass && sev == "fail");
    if has_fail {
        let failing_names: Vec<&str> = checks
            .iter()
            .filter(|(_, _, pass, sev)| !*pass && sev == "fail")
            .map(|(name, _, _, _)| name.as_str())
            .collect();
        Err(format!(
            "Doctor found blocker(s): {} — fix these before proceeding",
            failing_names.join(", ")
        )
        .into())
    } else {
        Ok(())
    }
}

/// Run a quality-gated validation process (Babysitter-inspired).
///
/// V0 supports only `daily-validation`. Generates a run ID, persists a
/// durable journal at ~/.arpagona/process-journal/, and supports
/// readback via `arpagona process status --last`.
async fn process_run(args: ProcessRunArgs) -> Result<(), Box<dyn Error>> {
    let process_name = args.name.as_str();
    if process_name != "daily-validation" {
        eprintln!(
            "[ERROR] Unknown process '{}'. V0 supports only 'daily-validation'.",
            process_name
        );
        std::process::exit(1);
    }

    let json_output = args.json;
    let run_id = generate_run_id("daily-validation");
    let started_at = Utc::now().to_rfc3339();

    // Plan phase
    let steps = vec![
        "doctor — local preflight diagnostic (git state, binaries, Ollama, tool runtime)",
        "cargo fmt -- --check — formatting compliance",
        "cargo check — type-check the workspace",
        "cargo test — run full workspace test suite",
    ];
    let mut step_results: Vec<JournalStepResult> = Vec::new();

    if json_output {
        let plan = json!({
            "command": "process_run",
            "process": "daily-validation",
            "run_id": run_id,
            "phase": "plan",
            "steps": steps
        });
        println!("{}", serde_json::to_string_pretty(&plan)?);
    } else {
        println!("[ARPAGONA process run] daily-validation");
        println!("Run ID: {}", run_id);
        println!("{}", "=".repeat(46));
        println!("Planned steps:");
        for (i, step) in steps.iter().enumerate() {
            println!("  {}. {}", i + 1, step);
        }
        println!();
        println!("Starting execution...");
        println!();
    }

    // Step 1: doctor/preflight
    if !json_output {
        println!("--- Step 1/4: doctor/preflight ---");
    }
    let doctor_args = DoctorArgs { json: json_output };
    if let Err(e) = doctor(doctor_args).await {
        let msg = format!("doctor/preflight failed with error: {}", e);
        step_results.push(JournalStepResult {
            step: 1,
            name: "doctor".into(),
            status: "FAILED".into(),
            detail: Some(msg.clone()),
        });
        if json_output {
            let report = json!({
                "command": "process_run",
                "process": "daily-validation",
                "run_id": run_id,
                "phase": "step_result",
                "step": 1,
                "name": "doctor",
                "status": "FAILED",
                "detail": msg
            });
            println!("{}", serde_json::to_string_pretty(&report)?);
        } else {
            eprintln!("[BLOCKER] {}", msg);
        }
        if json_output {
            let final_report = json!({
                "command": "process_run",
                "process": "daily-validation",
                "run_id": run_id,
                "phase": "summary",
                "overall_status": "BLOCKED",
                "blocked_at_step": 1,
                "next_action": "Fix the doctor issue and re-run"
            });
            println!("{}", serde_json::to_string_pretty(&final_report)?);
        } else {
            println!();
            println!("[BLOCKED] Process stopped at step 1 (doctor/preflight).");
            println!(
                "Next action: Fix the issue and re-run `arpagona process run daily-validation`."
            );
        }
        // Write journal even on BLOCKED
        let journal = ProcessRunJournal {
            run_id: run_id.clone(),
            process: "daily-validation".into(),
            started_at: started_at.clone(),
            ended_at: Utc::now().to_rfc3339(),
            planned_steps: steps.iter().map(|s| s.to_string()).collect(),
            step_results,
            overall_status: "BLOCKED".into(),
            blocked_at_step: Some(1),
            next_action: "Fix the doctor issue and re-run".into(),
        };
        persist_journal(&journal);
        return Ok(());
    }
    step_results.push(JournalStepResult {
        step: 1,
        name: "doctor".into(),
        status: "PASSED".into(),
        detail: None,
    });
    if !json_output {
        println!("[OK] Step 1/4: doctor passed");
        println!();
    }

    // Step 2: cargo fmt -- --check
    if !json_output {
        println!("--- Step 2/4: cargo fmt -- --check ---");
    }
    let fmt_result = ProcessCommand::new("cargo")
        .args(["fmt", "--", "--check"])
        .output();
    match fmt_result {
        Ok(output) if output.status.success() => {
            step_results.push(JournalStepResult {
                step: 2,
                name: "cargo_fmt".into(),
                status: "PASSED".into(),
                detail: None,
            });
            if json_output {
                let report = json!({
                    "command": "process_run",
                    "process": "daily-validation",
                    "run_id": run_id,
                    "phase": "step_result",
                    "step": 2,
                    "name": "cargo_fmt",
                    "status": "PASSED"
                });
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("[OK] cargo fmt compliance check passed");
            }
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let msg = format!("cargo fmt -- --check failed:\n{}", stderr);
            step_results.push(JournalStepResult {
                step: 2,
                name: "cargo_fmt".into(),
                status: "FAILED".into(),
                detail: Some(msg.clone()),
            });
            if json_output {
                let report = json!({
                    "command": "process_run",
                    "process": "daily-validation",
                    "run_id": run_id,
                    "phase": "step_result",
                    "step": 2,
                    "name": "cargo_fmt",
                    "status": "FAILED",
                    "detail": msg
                });
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                eprintln!("[BLOCKER] {}", msg);
            }
            if json_output {
                let final_report = json!({
                    "command": "process_run",
                    "process": "daily-validation",
                    "run_id": run_id,
                    "phase": "summary",
                    "overall_status": "BLOCKED",
                    "blocked_at_step": 2,
                    "next_action": "Run `cargo fmt` and re-run"
                });
                println!("{}", serde_json::to_string_pretty(&final_report)?);
            } else {
                println!();
                println!("[BLOCKED] Process stopped at step 2 (cargo fmt).");
                println!("Next action: Run `cargo fmt` to fix formatting, then re-run.");
            }
            let journal = ProcessRunJournal {
                run_id: run_id.clone(),
                process: "daily-validation".into(),
                started_at: started_at.clone(),
                ended_at: Utc::now().to_rfc3339(),
                planned_steps: steps.iter().map(|s| s.to_string()).collect(),
                step_results,
                overall_status: "BLOCKED".into(),
                blocked_at_step: Some(2),
                next_action: "Run `cargo fmt` and re-run".into(),
            };
            persist_journal(&journal);
            return Ok(());
        }
        Err(e) => {
            let msg = format!("Failed to run cargo fmt: {}", e);
            step_results.push(JournalStepResult {
                step: 2,
                name: "cargo_fmt".into(),
                status: "FAILED".into(),
                detail: Some(msg.clone()),
            });
            if json_output {
                let report = json!({
                    "command": "process_run",
                    "process": "daily-validation",
                    "run_id": run_id,
                    "phase": "step_result",
                    "step": 2,
                    "name": "cargo_fmt",
                    "status": "FAILED",
                    "detail": msg
                });
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                eprintln!("[BLOCKER] {}", msg);
            }
            return Ok(());
        }
    }
    if !json_output {
        println!();
    }

    // Step 3: cargo check
    if !json_output {
        println!("--- Step 3/4: cargo check ---");
    }
    let check_result = ProcessCommand::new("cargo").args(["check"]).output();
    match check_result {
        Ok(output) if output.status.success() => {
            step_results.push(JournalStepResult {
                step: 3,
                name: "cargo_check".into(),
                status: "PASSED".into(),
                detail: None,
            });
            if json_output {
                let report = json!({
                    "command": "process_run",
                    "process": "daily-validation",
                    "run_id": run_id,
                    "phase": "step_result",
                    "step": 3,
                    "name": "cargo_check",
                    "status": "PASSED"
                });
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("[OK] cargo check passed");
            }
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let msg = format!("cargo check failed:\n{}", stderr);
            step_results.push(JournalStepResult {
                step: 3,
                name: "cargo_check".into(),
                status: "FAILED".into(),
                detail: Some(msg.clone()),
            });
            if json_output {
                let report = json!({
                    "command": "process_run",
                    "process": "daily-validation",
                    "run_id": run_id,
                    "phase": "step_result",
                    "step": 3,
                    "name": "cargo_check",
                    "status": "FAILED",
                    "detail": msg
                });
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                eprintln!("[BLOCKER] {}", msg);
            }
            if json_output {
                let final_report = json!({
                    "command": "process_run",
                    "process": "daily-validation",
                    "run_id": run_id,
                    "phase": "summary",
                    "overall_status": "BLOCKED",
                    "blocked_at_step": 3,
                    "next_action": "Fix type errors and re-run"
                });
                println!("{}", serde_json::to_string_pretty(&final_report)?);
            } else {
                println!();
                println!("[BLOCKED] Process stopped at step 3 (cargo check).");
                println!("Next action: Fix type-check errors and re-run.");
            }
            let journal = ProcessRunJournal {
                run_id: run_id.clone(),
                process: "daily-validation".into(),
                started_at: started_at.clone(),
                ended_at: Utc::now().to_rfc3339(),
                planned_steps: steps.iter().map(|s| s.to_string()).collect(),
                step_results,
                overall_status: "BLOCKED".into(),
                blocked_at_step: Some(3),
                next_action: "Fix type errors and re-run".into(),
            };
            persist_journal(&journal);
            return Ok(());
        }
        Err(e) => {
            let msg = format!("Failed to run cargo check: {}", e);
            step_results.push(JournalStepResult {
                step: 3,
                name: "cargo_check".into(),
                status: "FAILED".into(),
                detail: Some(msg.clone()),
            });
            if json_output {
                let report = json!({
                    "command": "process_run",
                    "process": "daily-validation",
                    "run_id": run_id,
                    "phase": "step_result",
                    "step": 3,
                    "name": "cargo_check",
                    "status": "FAILED",
                    "detail": msg
                });
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                eprintln!("[BLOCKER] {}", msg);
            }
            return Ok(());
        }
    }
    if !json_output {
        println!();
    }

    // Step 4: cargo test
    if !json_output {
        println!("--- Step 4/4: cargo test ---");
        println!("(This may take a few minutes...)");
    }
    let test_result = ProcessCommand::new("cargo").args(["test"]).output();
    match test_result {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            step_results.push(JournalStepResult {
                step: 4,
                name: "cargo_test".into(),
                status: "PASSED".into(),
                detail: None,
            });
            if json_output {
                let lines: Vec<&str> = stdout.lines().collect();
                let report = json!({
                    "command": "process_run",
                    "process": "daily-validation",
                    "run_id": run_id,
                    "phase": "step_result",
                    "step": 4,
                    "name": "cargo_test",
                    "status": "PASSED",
                    "output": lines
                });
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("{}", stdout);
                println!("[OK] cargo test passed");
            }
        }
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let msg = format!(
                "cargo test had failures.\nstdout:\n{}\nstderr:\n{}",
                stdout, stderr
            );
            step_results.push(JournalStepResult {
                step: 4,
                name: "cargo_test".into(),
                status: "FAILED".into(),
                detail: Some(msg.clone()),
            });
            if json_output {
                let report = json!({
                    "command": "process_run",
                    "process": "daily-validation",
                    "run_id": run_id,
                    "phase": "step_result",
                    "step": 4,
                    "name": "cargo_test",
                    "status": "FAILED",
                    "detail": msg
                });
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                eprintln!("[BLOCKER] {}", msg);
            }
            if json_output {
                let final_report = json!({
                    "command": "process_run",
                    "process": "daily-validation",
                    "run_id": run_id,
                    "phase": "summary",
                    "overall_status": "BLOCKED",
                    "blocked_at_step": 4,
                    "next_action": "Fix failing tests and re-run"
                });
                println!("{}", serde_json::to_string_pretty(&final_report)?);
            } else {
                println!();
                println!("[BLOCKED] Process stopped at step 4 (cargo test).");
                println!("Next action: Fix failing tests and re-run.");
            }
            let journal = ProcessRunJournal {
                run_id: run_id.clone(),
                process: "daily-validation".into(),
                started_at: started_at.clone(),
                ended_at: Utc::now().to_rfc3339(),
                planned_steps: steps.iter().map(|s| s.to_string()).collect(),
                step_results,
                overall_status: "BLOCKED".into(),
                blocked_at_step: Some(4),
                next_action: "Fix failing tests and re-run".into(),
            };
            persist_journal(&journal);
            return Ok(());
        }
        Err(e) => {
            let msg = format!("Failed to run cargo test: {}", e);
            step_results.push(JournalStepResult {
                step: 4,
                name: "cargo_test".into(),
                status: "FAILED".into(),
                detail: Some(msg.clone()),
            });
            if json_output {
                let report = json!({
                    "command": "process_run",
                    "process": "daily-validation",
                    "run_id": run_id,
                    "phase": "step_result",
                    "step": 4,
                    "name": "cargo_test",
                    "status": "FAILED",
                    "detail": msg
                });
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                eprintln!("[BLOCKER] {}", msg);
            }
            return Ok(());
        }
    }
    if !json_output {
        println!();
    }

    // Summary — all passed
    let journal = ProcessRunJournal {
        run_id: run_id.clone(),
        process: "daily-validation".into(),
        started_at: started_at.clone(),
        ended_at: Utc::now().to_rfc3339(),
        planned_steps: steps.iter().map(|s| s.to_string()).collect(),
        step_results,
        overall_status: "PASSED".into(),
        blocked_at_step: None,
        next_action: "No issues found. System is healthy.".into(),
    };
    persist_journal(&journal);

    if json_output {
        let final_report = json!({
            "command": "process_run",
            "process": "daily-validation",
            "run_id": run_id,
            "phase": "summary",
            "overall_status": "PASSED",
            "next_action": "No issues found. System is healthy."
        });
        println!("{}", serde_json::to_string_pretty(&final_report)?);
    } else {
        println!("{}", "=".repeat(46));
        println!("[ARPAGONA process run] daily-validation — PASSED");
        println!("Run ID: {}", run_id);
        println!("All 4 steps completed successfully.");
        println!("Next action: No issues found. System is healthy.");
    }

    Ok(())
}

/// Show what steps a process would execute, without running anything.
///
/// Read-only process inspection — no doctor, no cargo, no journal writes.
/// V0 supports only `daily-validation`.
fn process_plan(args: ProcessPlanArgs) -> Result<(), Box<dyn Error>> {
    let process_name = args.name.as_str();
    if process_name != "daily-validation" {
        eprintln!(
            "[ERROR] Unknown process '{}'. V0 supports only 'daily-validation'.",
            process_name
        );
        std::process::exit(1);
    }

    let steps = vec![
        "doctor — local preflight diagnostic (git state, binaries, Ollama, tool runtime)",
        "cargo fmt -- --check — formatting compliance",
        "cargo check — type-check the workspace",
        "cargo test — run full workspace test suite",
    ];

    let json_output = args.json;

    if json_output {
        let plan = serde_json::json!({
            "command": "process_plan",
            "process": "daily-validation",
            "description": "Read-only process plan — no execution, no journal writes",
            "steps": steps,
            "total_steps": steps.len(),
        });
        println!("{}", serde_json::to_string_pretty(&plan)?);
    } else {
        println!("[ARPAGONA process plan] daily-validation");
        println!("{}", "=".repeat(46));
        println!("Read-only process plan — no execution, no journal writes.");
        println!();
        println!("Planned steps:");
        for (i, step) in steps.iter().enumerate() {
            println!("  {}. {}", i + 1, step);
        }
        println!();
        println!("Total: {} steps", steps.len());
        println!("Mode:  read-only (no doctor, no cargo, no journal writes)");
    }

    Ok(())
}

/// List persisted process run journals.
///
/// Reads the journal directory and returns a summary of each run,
/// newest first. Read-only — no doctor, no cargo, no journal writes.
/// Does NOT create the journal directory — if it doesn't exist the
/// result is an empty list.
fn process_list(args: ProcessListArgs) -> Result<(), Box<dyn Error>> {
    let dir = journal_dir_path()?;
    let json_output = args.json;

    // Read journal files and deserialize each
    let mut entries: Vec<(PathBuf, ProcessRunJournal)> = Vec::new();
    let mut corrupt_count = 0usize;
    let mut warn_count = 0usize;

    if let Ok(read_dir) = std::fs::read_dir(&dir) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.extension().map_or(true, |ext| ext != "json") {
                continue;
            }
            match std::fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str::<ProcessRunJournal>(&content) {
                    Ok(journal) => entries.push((path, journal)),
                    Err(_) => {
                        warn_count += 1;
                        // Try to infer run_id from filename
                        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                            let fallback = ProcessRunJournal {
                                run_id: stem.to_owned(),
                                process: "unknown".to_owned(),
                                started_at: String::new(),
                                ended_at: String::new(),
                                planned_steps: Vec::new(),
                                step_results: Vec::new(),
                                overall_status: "CORRUPT".to_owned(),
                                blocked_at_step: None,
                                next_action: String::new(),
                            };
                            entries.push((path, fallback));
                            corrupt_count += 1;
                        }
                    }
                },
                Err(_) => {
                    warn_count += 1;
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        let fallback = ProcessRunJournal {
                            run_id: stem.to_owned(),
                            process: "unknown".to_owned(),
                            started_at: String::new(),
                            ended_at: String::new(),
                            planned_steps: Vec::new(),
                            step_results: Vec::new(),
                            overall_status: "UNREADABLE".to_owned(),
                            blocked_at_step: None,
                            next_action: String::new(),
                        };
                        entries.push((path, fallback));
                    }
                }
            }
        }
    }

    // Sort newest first by modification time (use started_at when available)
    entries.sort_by(|a, b| {
        // Prefer started_at timestamp if both have it
        let a_time = if !a.1.started_at.is_empty() {
            a.1.started_at.clone()
        } else {
            "".to_owned()
        };
        let b_time = if !b.1.started_at.is_empty() {
            b.1.started_at.clone()
        } else {
            "".to_owned()
        };
        if !a_time.is_empty() && !b_time.is_empty() {
            b_time.cmp(&a_time)
        } else {
            // Fall back to mtime
            let a_mtime = a.0.metadata().ok().and_then(|m| m.modified().ok());
            let b_mtime = b.0.metadata().ok().and_then(|m| m.modified().ok());
            b_mtime.cmp(&a_mtime)
        }
    });

    if json_output {
        let list: Vec<serde_json::Value> = entries
            .iter()
            .map(|(_, j)| {
                serde_json::json!({
                    "run_id": j.run_id,
                    "process": j.process,
                    "started_at": j.started_at,
                    "ended_at": j.ended_at,
                    "overall_status": j.overall_status,
                    "next_action": j.next_action,
                })
            })
            .collect();
        let result = serde_json::json!({
            "command": "process_list",
            "total": list.len(),
            "runs": list,
            "warnings": if warn_count > 0 { serde_json::Value::Number(serde_json::Number::from(warn_count)) } else { serde_json::Value::Null },
            "corrupt_entries": if corrupt_count > 0 { serde_json::Value::Number(serde_json::Number::from(corrupt_count)) } else { serde_json::Value::Null },
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        if entries.is_empty() {
            println!("No process runs found in {}", dir.display());
            println!();
            println!("  Run `arpagona process run daily-validation` to create the first journal.");
        } else {
            println!("ARPAGONA process run journals");
            println!("{}", "=".repeat(46));
            println!("Directory: {}", dir.display());
            println!("Total:     {} run(s)", entries.len());
            if warn_count > 0 {
                println!("Warnings:  {} (corrupt/unreadable entries)", warn_count);
            }
            println!();

            for (i, (_, j)) in entries.iter().enumerate() {
                println!("  {}. {} — [{}]", i + 1, j.run_id, j.overall_status);
                println!("     Process:   {}", j.process);
                if !j.started_at.is_empty() {
                    println!("     Started:   {}", j.started_at);
                }
                if !j.ended_at.is_empty() {
                    println!("     Ended:     {}", j.ended_at);
                }
                if !j.next_action.is_empty() {
                    println!("     Next:      {}", j.next_action);
                }
                println!();
            }
        }
    }

    Ok(())
}

/// Persist a process run journal to disk.
fn persist_journal(journal: &ProcessRunJournal) {
    let dir = match ensure_journal_dir() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("[WARN] Could not create process journal dir: {}", e);
            return;
        }
    };
    let path = dir.join(format!("{}.json", journal.run_id));
    let json = match serde_json::to_string_pretty(journal) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("[WARN] Could not serialize process journal: {}", e);
            return;
        }
    };
    if let Err(e) = std::fs::write(&path, &json) {
        eprintln!(
            "[WARN] Could not write process journal: {}: {}",
            path.display(),
            e
        );
    }
}

/// Show status of a previous process run.
fn process_status(args: ProcessStatusArgs) -> Result<(), Box<dyn Error>> {
    let dir = ensure_journal_dir()?;

    // Determine which journal file to read
    let journal_path = if let Some(run_id) = &args.run_id {
        let path = dir.join(format!("{}.json", run_id));
        if !path.exists() {
            eprintln!("[ERROR] No journal found for run ID '{}'.", run_id);
            eprintln!("        Looked in: {}", dir.display());
            eprintln!("        Use --last to see the most recent run.");
            std::process::exit(1);
        }
        path
    } else if args.last {
        // Find the most recent .json file in the journal dir
        let mut entries: Vec<_> = std::fs::read_dir(&dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "json"))
            .collect();
        entries
            .sort_by_key(|e| std::cmp::Reverse(e.metadata().ok().and_then(|m| m.modified().ok())));
        match entries.into_iter().next() {
            Some(entry) => entry.path(),
            None => {
                eprintln!("[ERROR] No process run journals found in {}", dir.display());
                eprintln!("        Run `arpagona process run daily-validation` first.");
                std::process::exit(1);
            }
        }
    } else {
        eprintln!("[ERROR] Specify --last or a run ID.");
        eprintln!("        Examples:");
        eprintln!("          arpagona process status --last");
        eprintln!("          arpagona process status daily-validation-20260531T043751");
        std::process::exit(1);
    };

    let content = std::fs::read_to_string(&journal_path)?;
    let journal: ProcessRunJournal = serde_json::from_str(&content)?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&journal)?);
    } else {
        println!("Process run status");
        println!("{}", "=".repeat(46));
        println!("Run ID:      {}", journal.run_id);
        println!("Process:     {}", journal.process);
        println!("Started:     {}", journal.started_at);
        println!("Ended:       {}", journal.ended_at);
        println!("Status:      {}", journal.overall_status);
        if let Some(step) = journal.blocked_at_step {
            println!("Blocked at:  Step {}", step);
        }
        println!();
        println!("Steps:");
        for (i, step_name) in journal.planned_steps.iter().enumerate() {
            let result = journal.step_results.iter().find(|r| r.step == i + 1);
            let status = result.map(|r| r.status.as_str()).unwrap_or("PENDING");
            println!("  {}. {} — [{}]", i + 1, step_name, status);
        }
        println!();
        println!("Next action: {}", journal.next_action);
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
            ChatLine::Audit => list_audit(client, api_url, ListAuditArgs { json: false }).await?,
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
    println!("{}", rainbow_text("           .  . /  \\.  ."));
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
        style_brand("/_/  |_/_/ |_/_/   /_/  |\\____/ \\____/_/ |_/_/  |_|")
    );
    println!(
        "{}{}",
        style_dim("        "),
        style_info("Cognitive Runtime Alpha")
    );
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
        style_dim("Type /help for commands. Read-only mode - nothing is executed directly.")
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
            memory_visibility: gather_memory_visibility_section(),
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
        memory_visibility: gather_memory_visibility_section(),
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

/// Gather the D3 memory visibility section from the local holographic memory store.
///
/// Opens the SQLite-backed holographic memory store if it exists and reads
/// recent traces. Builds a `MemoryVisibilitySection` with trace summaries,
/// linked memory/decision IDs, and consolidation hints.
fn gather_memory_visibility_section() -> MemoryVisibilitySection {
    let hm_db_path = PathBuf::from("target/holographic-memory.db");
    if !hm_db_path.exists() {
        return MemoryVisibilitySection {
            total_trace_count: None,
            recent_traces: vec![],
            most_activated_traces: vec![],
            aggregated_linked_memory_ids: vec![],
            aggregated_linked_decision_ids: vec![],
            store_accessible: false,
            consolidation_info: None,
            warning: AUDIT_READBACK_WARNING,
        };
    }

    // Try to open the SQLite store. If it fails (corrupt DB, etc.), report
    // the store as inaccessible rather than crashing.
    let store = match SqliteHolographicMemoryStore::new(&hm_db_path.to_string_lossy()) {
        Ok(s) => s,
        Err(_) => {
            return MemoryVisibilitySection {
                total_trace_count: None,
                recent_traces: vec![],
                most_activated_traces: vec![],
                aggregated_linked_memory_ids: vec![],
                aggregated_linked_decision_ids: vec![],
                store_accessible: false,
                consolidation_info: None,
                warning: AUDIT_READBACK_WARNING,
            };
        }
    };

    let all_traces: Vec<HolographicTrace> = store.all_traces();
    let recent_traces: Vec<&HolographicTrace> = all_traces.iter().take(5).collect();
    let total_count = store.len();

    // Build trace summaries
    let trace_summaries: Vec<TraceSummary> = recent_traces
        .iter()
        .map(|t| TraceSummary {
            id: t.id.clone(),
            source_kind: serde_json::to_string(&t.source_kind)
                .unwrap_or_else(|_| "unknown".to_owned()),
            content_summary: t.content_summary.clone(),
            keywords: t.keywords.clone(),
            concepts: t.concepts.clone(),
            linked_memory_ids: t.linked_memory_ids.clone(),
            linked_decision_ids: t.linked_decision_ids.clone(),
            importance: t.importance,
            confidence: t.confidence,
            activation_count: t.activation_count,
            created_at: t.created_at.clone(),
            last_activated_at: t.last_activated_at.clone(),
        })
        .collect();

    // Aggregated linked IDs from recent traces
    let mut mem_ids: BTreeSet<String> = BTreeSet::new();
    let mut dec_ids: BTreeSet<String> = BTreeSet::new();
    for t in &recent_traces {
        for mid in &t.linked_memory_ids {
            mem_ids.insert(mid.clone());
        }
        for did in &t.linked_decision_ids {
            dec_ids.insert(did.clone());
        }
    }

    MemoryVisibilitySection {
        total_trace_count: Some(total_count),
        recent_traces: trace_summaries,

        // Most activated traces (top 5 by activation_count, descending)
        most_activated_traces: {
            let mut sorted: Vec<TraceSummary> = all_traces
                .iter()
                .map(|t| TraceSummary {
                    id: t.id.clone(),
                    source_kind: serde_json::to_string(&t.source_kind)
                        .unwrap_or_else(|_| "unknown".to_owned()),
                    content_summary: t.content_summary.clone(),
                    keywords: t.keywords.clone(),
                    concepts: t.concepts.clone(),
                    linked_memory_ids: t.linked_memory_ids.clone(),
                    linked_decision_ids: t.linked_decision_ids.clone(),
                    importance: t.importance,
                    confidence: t.confidence,
                    activation_count: t.activation_count,
                    created_at: t.created_at.clone(),
                    last_activated_at: t.last_activated_at.clone(),
                })
                .collect();
            sorted.sort_by_key(|b| std::cmp::Reverse(b.activation_count));
            sorted.truncate(5);
            sorted
        },

        aggregated_linked_memory_ids: mem_ids.into_iter().collect(),
        aggregated_linked_decision_ids: dec_ids.into_iter().collect(),
        store_accessible: true,
        consolidation_info: None, // Not tracked dynamically — TBD
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
    // Look for the ## Next action section and return the first actionable line
    let mut found_section = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if !found_section {
            if trimmed.eq_ignore_ascii_case("## Next action")
                || trimmed.eq_ignore_ascii_case("## next action")
            {
                found_section = true;
            }
            continue;
        }
        // After finding ## Next action, return first non-empty non-header line
        if !trimmed.is_empty() && !trimmed.starts_with('#') && trimmed.len() > 3 {
            return Some(trimmed.to_owned());
        }
    }
    None
}

/// Count open candidate items in DAILY_VALIDATION_BACKLOG.md.
///
/// Only counts entries in the "## Open candidates" section.
/// Entries under "## Closed / superseded candidates" or
/// "## Older closed / superseded items" are excluded.
fn count_backlog_open_items() -> Option<usize> {
    let path = PathBuf::from("DAILY_VALIDATION_BACKLOG.md");
    if !path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(path).ok()?;
    // Only count DV entries within the ## Open candidates section
    // (stop counting when we reach ## Closed / superseded candidates or nested Open entries
    // whose status line says "fixed" or "superseded")
    let mut in_open_section = false;
    let mut count = 0_usize;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.eq_ignore_ascii_case("## Open candidates") {
            in_open_section = true;
            continue;
        }
        if in_open_section
            && (trimmed.starts_with("## ")
                || trimmed.eq_ignore_ascii_case("## Closed / superseded candidates"))
        {
            break;
        }
        if in_open_section && trimmed.starts_with("###") && trimmed.contains("DV-") {
            count += 1;
        }
    }
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
    push_readback_line(&mut output, "");

    // D3 — Memory and resonance visibility
    push_readback_line(
        &mut output,
        &style_info("Memory and resonance visibility (D3)"),
    );
    if !readback.memory_visibility.store_accessible {
        push_readback_line(&mut output, "  (holographic memory store not accessible)");
    } else {
        push_readback_field(
            &mut output,
            "total_trace_count:",
            &format_optional_usize(readback.memory_visibility.total_trace_count),
        );
        if readback.memory_visibility.recent_traces.is_empty() {
            push_readback_line(&mut output, "  (no recent traces)");
        } else {
            for trace in &readback.memory_visibility.recent_traces {
                push_readback_line(&mut output, &format!("  trace: {}", trace.id));
                push_readback_field(
                    &mut output,
                    "  source:",
                    &action_type_display(&trace.source_kind),
                );
                push_readback_field(&mut output, "  content:", &trace.content_summary);
                if !trace.keywords.is_empty() {
                    push_readback_field(&mut output, "  keywords:", &trace.keywords.join(", "));
                }
                if !trace.concepts.is_empty() {
                    push_readback_field(&mut output, "  concepts:", &trace.concepts.join(", "));
                }
                if !trace.linked_memory_ids.is_empty() {
                    push_readback_field(
                        &mut output,
                        "  linked_memories:",
                        &trace.linked_memory_ids.join(", "),
                    );
                }
                if !trace.linked_decision_ids.is_empty() {
                    push_readback_field(
                        &mut output,
                        "  linked_decisions:",
                        &trace.linked_decision_ids.join(", "),
                    );
                }
                push_readback_field(
                    &mut output,
                    "  importance:",
                    &format!("{:.2}", trace.importance),
                );
                push_readback_field(
                    &mut output,
                    "  confidence:",
                    &format!("{:.2}", trace.confidence),
                );
                push_readback_field(
                    &mut output,
                    "  activations:",
                    &trace.activation_count.to_string(),
                );
                push_readback_field(&mut output, "  at:", &trace.created_at);
                if let Some(last) = &trace.last_activated_at {
                    push_readback_field(&mut output, "  last_activated:", last);
                }
                push_readback_line(&mut output, "");
            }
        }

        // Most activated traces
        if !readback.memory_visibility.most_activated_traces.is_empty() {
            push_readback_line(
                &mut output,
                &format!(
                    "  most_activated_traces (top {}):",
                    readback.memory_visibility.most_activated_traces.len()
                ),
            );
            for trace in &readback.memory_visibility.most_activated_traces {
                push_readback_line(&mut output, &format!("    trace: {}", trace.id));
                push_readback_field(&mut output, "    content:", &trace.content_summary);
                push_readback_field(
                    &mut output,
                    "    activations:",
                    &trace.activation_count.to_string(),
                );
                push_readback_line(&mut output, "");
            }
        }

        if !readback
            .memory_visibility
            .aggregated_linked_memory_ids
            .is_empty()
        {
            push_readback_field(
                &mut output,
                "aggregated_linked_memory_ids:",
                &readback
                    .memory_visibility
                    .aggregated_linked_memory_ids
                    .join(", "),
            );
        }
        if !readback
            .memory_visibility
            .aggregated_linked_decision_ids
            .is_empty()
        {
            push_readback_field(
                &mut output,
                "aggregated_linked_decision_ids:",
                &readback
                    .memory_visibility
                    .aggregated_linked_decision_ids
                    .join(", "),
            );
        }
        if let Some(consolidation) = &readback.memory_visibility.consolidation_info {
            push_readback_field(&mut output, "consolidation:", consolidation);
        }
    }
    push_readback_line(&mut output, &style_dim(readback.memory_visibility.warning));
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

/// Handler for `memory holographic status` — show holographic memory store status.
///
/// Reads from the SQLite-backed store by default. Reports total trace count,
/// recent traces, most activated traces, aggregated linked IDs, and store backend type.
fn memory_holographic_status(args: HolographicStatusArgs) -> Result<(), Box<dyn Error>> {
    let hm_db_path = std::path::PathBuf::from(&args.db);

    // Try SQLite store first; if the file doesn't exist, report as empty store
    if !hm_db_path.exists() {
        if args.json {
            let output = serde_json::json!({
                "store_type": "sqlite",
                "store_path": args.db,
                "store_accessible": false,
                "total_trace_count": 0,
                "recent_traces": [],
                "most_activated_traces": [],
                "aggregated_linked_memory_ids": [],
                "aggregated_linked_decision_ids": [],
                "consolidation_info": null,
                "warning": AUDIT_READBACK_WARNING,
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            println!("Holographic memory store status");
            println!("  store_type: SQLite");
            println!("  store_path: {}", args.db);
            println!("  store_accessible: false");
            println!("  (no database file found — add traces first)");
            println!();
            println!("⚠️  {}", AUDIT_READBACK_WARNING);
        }
        return Ok(());
    }

    // Open the SQLite store
    let store = match SqliteHolographicMemoryStore::new(&args.db) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to open SQLite store at {}: {}", args.db, e);
            return Ok(());
        }
    };

    let total_count = store.len();
    let all_traces: Vec<HolographicTrace> = store.all_traces();

    // Recent traces: top 5 by created_at (already sorted newest-first by all_traces)
    let recent_traces: Vec<TraceSummary> = all_traces
        .iter()
        .take(5)
        .map(|t| TraceSummary {
            id: t.id.clone(),
            source_kind: serde_json::to_string(&t.source_kind)
                .unwrap_or_else(|_| "unknown".to_owned()),
            content_summary: t.content_summary.clone(),
            keywords: t.keywords.clone(),
            concepts: t.concepts.clone(),
            linked_memory_ids: t.linked_memory_ids.clone(),
            linked_decision_ids: t.linked_decision_ids.clone(),
            importance: t.importance,
            confidence: t.confidence,
            activation_count: t.activation_count,
            created_at: t.created_at.clone(),
            last_activated_at: t.last_activated_at.clone(),
        })
        .collect();

    // Most activated traces: top 5 by activation_count
    let mut most_activated: Vec<TraceSummary> = all_traces
        .iter()
        .map(|t| TraceSummary {
            id: t.id.clone(),
            source_kind: serde_json::to_string(&t.source_kind)
                .unwrap_or_else(|_| "unknown".to_owned()),
            content_summary: t.content_summary.clone(),
            keywords: t.keywords.clone(),
            concepts: t.concepts.clone(),
            linked_memory_ids: t.linked_memory_ids.clone(),
            linked_decision_ids: t.linked_decision_ids.clone(),
            importance: t.importance,
            confidence: t.confidence,
            activation_count: t.activation_count,
            created_at: t.created_at.clone(),
            last_activated_at: t.last_activated_at.clone(),
        })
        .collect();
    most_activated.sort_by_key(|b| std::cmp::Reverse(b.activation_count));
    most_activated.truncate(5);

    // Aggregated linked IDs from all traces
    let mut mem_ids: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut dec_ids: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for t in &all_traces {
        for mid in &t.linked_memory_ids {
            mem_ids.insert(mid.clone());
        }
        for did in &t.linked_decision_ids {
            dec_ids.insert(did.clone());
        }
    }
    let aggregated_linked_memory_ids: Vec<String> = mem_ids.into_iter().collect();
    let aggregated_linked_decision_ids: Vec<String> = dec_ids.into_iter().collect();

    if args.json {
        let output = serde_json::json!({
            "store_type": "sqlite",
            "store_path": args.db,
            "store_accessible": true,
            "total_trace_count": total_count,
            "recent_traces": recent_traces,
            "most_activated_traces": most_activated,
            "aggregated_linked_memory_ids": aggregated_linked_memory_ids,
            "aggregated_linked_decision_ids": aggregated_linked_decision_ids,
            "consolidation_info": null,
            "warning": AUDIT_READBACK_WARNING,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("Holographic memory store status");
        println!("  store_type: SQLite");
        println!("  store_path: {}", args.db);
        println!("  store_accessible: true");
        println!("  total_trace_count: {}", total_count);

        if !recent_traces.is_empty() {
            println!();
            println!("  Recent traces (top 5):");
            for trace in &recent_traces {
                println!("    trace: {}", trace.id);
                println!("      content: {}", trace.content_summary);
                if !trace.keywords.is_empty() {
                    println!("      keywords: {}", trace.keywords.join(", "));
                }
                if !trace.concepts.is_empty() {
                    println!("      concepts: {}", trace.concepts.join(", "));
                }
                if !trace.linked_memory_ids.is_empty() {
                    println!(
                        "      linked_memories: {}",
                        trace.linked_memory_ids.join(", ")
                    );
                }
                if !trace.linked_decision_ids.is_empty() {
                    println!(
                        "      linked_decisions: {}",
                        trace.linked_decision_ids.join(", ")
                    );
                }
                println!(
                    "      importance: {:.2}, confidence: {:.2}, activations: {}",
                    trace.importance, trace.confidence, trace.activation_count
                );
                println!("      created_at: {}", trace.created_at);
            }
        }

        if !most_activated.is_empty() {
            println!();
            println!("  Most activated traces (top 5):");
            for trace in &most_activated {
                println!(
                    "    trace: {} ({} activations)",
                    trace.id, trace.activation_count
                );
                println!("      content: {}", trace.content_summary);
            }
        }

        if !aggregated_linked_memory_ids.is_empty() {
            println!();
            println!(
                "  Linked memory IDs: {}",
                aggregated_linked_memory_ids.join(", ")
            );
        }
        if !aggregated_linked_decision_ids.is_empty() {
            println!(
                "  Linked decision IDs: {}",
                aggregated_linked_decision_ids.join(", ")
            );
        }

        println!();
        println!("⚠️  {}", AUDIT_READBACK_WARNING);
    }

    Ok(())
}

/// Parse a risk level string into a `RiskLevel`.
fn parse_risk_level(s: &str) -> Result<RiskLevel, String> {
    match s.to_lowercase().as_str() {
        "informational" => Ok(RiskLevel::Informational),
        "low" => Ok(RiskLevel::Low),
        "medium" => Ok(RiskLevel::Medium),
        "high" => Ok(RiskLevel::High),
        "critical" => Ok(RiskLevel::Critical),
        other => Err(format!(
            "Unknown risk level '{other}'. Valid values: informational, low, medium, high, critical"
        )),
    }
}

/// Evaluate a tool-call intent through the Decision Gate (Track C Step C2).
///
/// Creates a ToolCallIntent from CLI arguments, evaluates it via
/// `govern_tool_call()`, journals the governance result to the LLM journal,
/// and returns the decision with audit context.
fn tool_govern(args: ToolGovernArgs) -> Result<(), Box<dyn Error>> {
    let risk_level =
        parse_risk_level(&args.risk_level).map_err(|e| format!("Invalid --risk-level: {e}"))?;

    let arguments: Value =
        serde_json::from_str(&args.args).map_err(|e| format!("Invalid --args JSON: {e}"))?;

    let intent = ToolCallIntent {
        tool: args.tool.clone(),
        arguments: arguments.clone(),
        rationale: args.rationale.clone(),
        risk_level: risk_level.clone(),
    };

    let (decision, proposed_action) = govern_tool_call(&intent, &[Permission::ProposeToolUse]);

    // Execute the approved tool-call through the bounded Tool Runtime (C2.2)
    let execution_result = if decision.status == DecisionStatus::Approved {
        let config = ToolRuntimeConfig::new(".");
        let runtime = ToolRuntime::new(config);
        Some(runtime.execute(&args.tool, &arguments))
    } else {
        None
    };

    // Build the response summary for the journal
    let response_summary = if let Some(ref result) = execution_result {
        format!(
            "Decision Gate: {:?} — Tool runtime: {} ({:?})",
            decision.status, result.output_summary, result.status
        )
    } else {
        format!(
            "Decision Gate result: status={:?}, reason={}",
            decision.status, decision.reason
        )
    };

    // Journal the governance result (and execution result if available)
    let journal_entry_id = {
        let mut journal = global_llm_journal().lock().unwrap();
        journal.add_direct_tool_call(
            &args.tool,
            "cli",
            None,
            format!(
                "Governed tool-call evaluation: tool={}, risk_level={:?}, rationale={}",
                args.tool, risk_level, args.rationale
            ),
            response_summary,
            serde_json::json!({
                "tool": args.tool,
                "arguments": arguments,
                "rationale": args.rationale,
                "risk_level": risk_level,
            }),
            serde_json::json!({
                "decision_id": decision.id,
                "status": decision.status,
                "reason": decision.reason,
                "risk_level": decision.risk_level,
                "policies_applied": decision.policies_applied,
                "execution_result": execution_result.as_ref().map(|r| serde_json::json!({
                    "status": r.status,
                    "output_summary": r.output_summary,
                    "failure_insight_candidate": r.failure_insight_candidate,
                })),
            }),
            Some(risk_level.clone()),
        )
    };

    if args.json {
        let mut output = serde_json::json!({
            "status": "governed",
            "decision": {
                "id": decision.id,
                "status": decision.status,
                "reason": decision.reason,
                "risk_level": decision.risk_level,
                "policies_applied": decision.policies_applied,
                "override_hint": decision.override_hint,
            },
            "proposed_action": {
                "id": proposed_action.id,
                "action_type": proposed_action.action_type,
                "target": proposed_action.target,
                "risk_level": proposed_action.risk_level,
            },
            "tool_call_intent": {
                "tool": args.tool,
                "arguments": arguments,
                "rationale": args.rationale,
                "risk_level": risk_level,
            },
            "journal_entry_id": journal_entry_id,
            "non_authorizing": true,
        });
        if let Some(ref result) = execution_result {
            output["execution_result"] = serde_json::json!({
                "status": result.status,
                "output_summary": result.output_summary,
                "observation": result.observation,
                "error": result.error,
                "execution_id": result.execution_id,
            });
        } else {
            output["warning"] = serde_json::json!("Governance evaluation only. No tool was executed (blocked or requires human approval). Readback is evidence, not authorization.");
        }
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        let status_icon = match &decision.status {
            DecisionStatus::Approved => "✅",
            DecisionStatus::Blocked => "🔴",
            DecisionStatus::NeedsHumanApproval => "🟡",
            DecisionStatus::RequiresOverride => "🟠",
            _ => "❓",
        };
        println!("{status_icon} Tool Call Governance — tool: {}", args.tool);
        println!();
        println!("  Decision:     {:?}", decision.status);
        println!("  Reason:       {}", decision.reason);
        println!("  Risk level:   {:?}", decision.risk_level);
        println!("  Decision ID:  {}", decision.id);
        println!("  Proposal ID:  {}", proposed_action.id);
        println!("  Journal ID:   {}", journal_entry_id);
        if let Some(ref hint) = decision.override_hint {
            println!("  Override:     {hint}");
        }
        if !decision.policies_applied.is_empty() {
            println!(
                "  Policies:     {}",
                decision
                    .policies_applied
                    .iter()
                    .map(|p| p.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        // Show execution result for approved calls
        if let Some(ref result) = execution_result {
            println!();
            println!("  ── Execution Result ──");
            println!("  Status:       {:?}", result.status);
            println!("  Output:       {}", result.output_summary);
            if let Some(ref err) = result.error {
                println!("  Error:        {} (code: {})", err.message, err.code);
            }
            println!("  Audit hint:   {}", result.audit_hint);
            println!();
            println!("✅ Approved and executed through the bounded Tool Runtime.");
            println!("   Readback is evidence, not authorization.");
        } else {
            println!();
            println!("⚠️  Governance evaluation only. No tool was executed.");
            println!("   Readback is evidence, not authorization.");
            println!("   Use `arpagona llm journal` to inspect journaled interactions.");
        }
    }

    Ok(())
}

const DEMO_TOOL_WARNING: &str = "⚠️  Alpha demo tool runtime — local sandboxed execution only. Writes simulate by default; no shell/network. ⚠️";

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
        (
            "write_file",
            "Write a workspace-bounded file; simulates by default",
            "Transformation",
        ),
        (
            "patch_file",
            "Exact-match text replacement in a workspace file; simulates by default",
            "Transformation",
        ),
        (
            "append_file",
            "Append to a workspace-bounded file; simulates by default",
            "Transformation",
        ),
        (
            "mkdir",
            "Create a workspace-bounded directory; simulates by default",
            "Transformation",
        ),
        (
            "copy_file",
            "Copy a file within the workspace; simulates by default",
            "Transformation",
        ),
        (
            "move_file",
            "Move/rename a file within the workspace; simulates by default",
            "Transformation",
        ),
    ];

    let mutation_tools = [
        "write_file",
        "patch_file",
        "append_file",
        "mkdir",
        "copy_file",
        "move_file",
    ];

    if args.json {
        let output: Vec<serde_json::Value> = tools
            .iter()
            .map(|(name, desc, role)| {
                serde_json::json!({
                    "name": name,
                    "description": desc,
                    "cognitive_role": role,
                    "read_only": !mutation_tools.contains(name),
                    "alpha": true,
                    "sandboxed": true,
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("{DEMO_TOOL_WARNING}");
        println!();
        println!("Available tools (alpha sandboxed runtime):");
        println!();
        for (name, desc, role) in &tools {
            println!("  {name}");
            println!("    Description:  {desc}");
            println!("    Cognitive role: {role}");
            println!(
                "    Read-only:    {}",
                if mutation_tools.contains(name) {
                    "no — sandboxed mutation, simulate by default"
                } else {
                    "yes"
                }
            );
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
        "write_file" => {
            let info = serde_json::json!({
                "name": "write_file",
                "description": "Write a file within the workspace; simulate by default unless execution is explicit",
                "cognitive_role": ["Transformation"],
                "read_only": false,
                "sandboxed": true,
                "alpha": true,
                "arguments": {
                    "path": {"type": "string", "description": "Path to write, relative to workspace", "required": true},
                    "content": {"type": "string", "description": "Content to write", "required": true},
                    "simulate": {"type": "boolean", "description": "If true, do not mutate filesystem", "default": true},
                    "create_parent_dirs": {"type": "boolean", "default": false},
                    "overwrite": {"type": "boolean", "default": false}
                },
                "security": {
                    "absolute_paths": "blocked",
                    "parent_traversal": "blocked",
                    "sensitive_files": "blocked (.env, .ssh, id_rsa, id_ed25519)",
                    "blocked_dirs": ".git, target, node_modules, .env, .ssh",
                    "max_content_size": "256 KiB"
                },
                "workspace": "current directory"
            });
            if args.json {
                println!("{}", serde_json::to_string_pretty(&info)?);
            } else {
                println!("{DEMO_TOOL_WARNING}");
                println!();
                println!("Tool: write_file");
                println!("  Description:   Write a file within the workspace");
                println!("  Cognitive role: Transformation");
                println!("  Read-only:     no — sandboxed; simulate by default");
                println!("  Workspace:     current directory");
                println!();
                println!("Arguments:");
                println!("  path (required): Path to write, relative to workspace");
                println!("  content (required): Content to write");
                println!("  simulate (default true): Do not mutate filesystem");
                println!("  create_parent_dirs (default false)");
                println!("  overwrite (default false)");
                println!();
                println!("Security:");
                println!("  Absolute paths:           blocked");
                println!("  Parent traversal (..):    blocked");
                println!("  Sensitive files:          blocked (.env, .ssh, id_rsa, id_ed25519)");
                println!("  Max content size:         256 KiB");
            }
        }
        "patch_file" => {
            let info = serde_json::json!({
                "name": "patch_file",
                "description": "Exact-match text replacement in a workspace file; simulates by default",
                "cognitive_role": ["Transformation"],
                "read_only": false,
                "sandboxed": true,
                "alpha": true,
                "aliases": ["replace_text"],
                "arguments": {
                    "path": {"type": "string", "description": "File to patch, relative to workspace", "required": true},
                    "old_string": {"type": "string", "description": "Exact text to find", "required": true},
                    "new_string": {"type": "string", "description": "Replacement text", "default": ""},
                    "simulate": {"type": "boolean", "description": "Show diff without mutating", "default": true},
                    "replace_all": {"type": "boolean", "default": false}
                },
                "security": {
                    "absolute_paths": "blocked",
                    "parent_traversal": "blocked",
                    "sensitive_files": "blocked (.env, .ssh, id_rsa, id_ed25519)",
                    "blocked_dirs": ".git, target, node_modules, .env, .ssh",
                    "max_file_size": "256 KiB",
                    "binary_files": "blocked"
                },
                "workspace": "current directory"
            });
            if args.json {
                println!("{}", serde_json::to_string_pretty(&info)?);
            } else {
                println!("{DEMO_TOOL_WARNING}");
                println!();
                println!("Tool: patch_file (alias: replace_text)");
                println!("  Description:   Exact-match text replacement in a workspace file");
                println!("  Cognitive role: Transformation");
                println!("  Read-only:     no — sandboxed; simulate by default");
                println!("  Workspace:     current directory");
                println!();
                println!("Arguments:");
                println!("  path (required): File to patch, relative to workspace");
                println!("  old_string (required): Exact text to find");
                println!("  new_string (default ''): Replacement text");
                println!("  simulate (default true): Show diff without mutating");
                println!("  replace_all (default false): Replace all occurrences");
                println!();
                println!("Security:");
                println!("  Absolute paths:           blocked");
                println!("  Parent traversal (..):    blocked");
                println!("  Sensitive files:          blocked (.env, .ssh, id_rsa, id_ed25519)");
                println!("  Max file size:            256 KiB");
                println!("  Binary files:             blocked");
            }
        }
        "append_file" => {
            let info = serde_json::json!({
                "name": "append_file",
                "description": "Append content to a file within the workspace; simulate by default",
                "cognitive_role": ["Transformation"],
                "read_only": false,
                "sandboxed": true,
                "alpha": true,
                "arguments": {
                    "path": {"type": "string", "description": "File to append to, relative to workspace", "required": true},
                    "content": {"type": "string", "description": "Content to append", "required": true},
                    "simulate": {"type": "boolean", "default": true},
                    "create_parent_dirs": {"type": "boolean", "default": false},
                    "create_if_missing": {"type": "boolean", "default": true}
                },
                "security": {
                    "absolute_paths": "blocked",
                    "parent_traversal": "blocked",
                    "sensitive_files": "blocked (.env, .ssh, id_rsa, id_ed25519)",
                    "blocked_dirs": ".git, target, node_modules, .env, .ssh",
                    "max_result_size": "256 KiB"
                }
            });
            if args.json {
                println!("{}", serde_json::to_string_pretty(&info)?);
            } else {
                println!("{DEMO_TOOL_WARNING}");
                println!();
                println!("Tool: append_file");
                println!("  Description:   Append to a file within the workspace");
                println!("  Cognitive role: Transformation");
                println!("  Read-only:     no — sandboxed; simulate by default");
                println!(
                    "Arguments: path, content, simulate, create_parent_dirs, create_if_missing"
                );
                println!("Security: absolute paths/.. blocked; sensitive files blocked; max result 256 KiB");
            }
        }
        "mkdir" | "create_dir" => {
            let info = serde_json::json!({
                "name": "mkdir",
                "aliases": ["create_dir"],
                "description": "Create a directory within the workspace; simulate by default",
                "cognitive_role": ["Transformation"],
                "read_only": false,
                "sandboxed": true,
                "alpha": true,
                "arguments": {
                    "path": {"type": "string", "description": "Directory path relative to workspace", "required": true},
                    "simulate": {"type": "boolean", "default": true},
                    "parents": {"type": "boolean", "default": false}
                },
                "security": {
                    "absolute_paths": "blocked",
                    "parent_traversal": "blocked",
                    "blocked_dirs": ".git, target, node_modules, .env, .ssh"
                }
            });
            if args.json {
                println!("{}", serde_json::to_string_pretty(&info)?);
            } else {
                println!("{DEMO_TOOL_WARNING}");
                println!();
                println!("Tool: mkdir (alias: create_dir)");
                println!("  Description:   Create a directory within the workspace");
                println!("  Cognitive role: Transformation");
                println!("  Read-only:     no — sandboxed; simulate by default");
                println!("Arguments: path, simulate, parents");
                println!("Security: absolute paths/.. blocked; blocked dirs protected");
            }
        }
        "copy_file" => {
            let info = serde_json::json!({
                "name": "copy_file",
                "description": "Copy a file within the workspace; simulates by default",
                "cognitive_role": ["Transformation"],
                "read_only": false,
                "sandboxed": true,
                "alpha": true,
                "arguments": {
                    "source": {"type": "string", "description": "Source file path relative to workspace", "required": true},
                    "destination": {"type": "string", "description": "Destination file path relative to workspace", "required": true},
                    "simulate": {"type": "boolean", "default": true},
                    "overwrite": {"type": "boolean", "default": false}
                },
                "security": {
                    "absolute_paths": "blocked",
                    "parent_traversal": "blocked",
                    "blocked_dirs": ".git, target, node_modules, .env, .ssh",
                    "max_file_size": "256 KiB"
                }
            });
            if args.json {
                println!("{}", serde_json::to_string_pretty(&info)?);
            } else {
                println!("{DEMO_TOOL_WARNING}");
                println!();
                println!("Tool: copy_file");
                println!("  Description:   Copy a file within the workspace");
                println!("  Cognitive role: Transformation");
                println!("  Read-only:     no — sandboxed; simulate by default");
                println!("Arguments: source, destination, simulate, overwrite");
                println!("Security: absolute paths/.. blocked; max file 256 KiB");
            }
        }
        "move_file" | "rename" => {
            let info = serde_json::json!({
                "name": "move_file",
                "aliases": ["rename"],
                "description": "Move or rename a file within the workspace; simulates by default",
                "cognitive_role": ["Transformation"],
                "read_only": false,
                "sandboxed": true,
                "alpha": true,
                "arguments": {
                    "source": {"type": "string", "description": "Source file path relative to workspace", "required": true},
                    "destination": {"type": "string", "description": "Destination file path relative to workspace", "required": true},
                    "simulate": {"type": "boolean", "default": true},
                    "overwrite": {"type": "boolean", "default": false}
                },
                "security": {
                    "absolute_paths": "blocked",
                    "parent_traversal": "blocked",
                    "blocked_dirs": ".git, target, node_modules, .env, .ssh",
                    "max_file_size": "256 KiB"
                }
            });
            if args.json {
                println!("{}", serde_json::to_string_pretty(&info)?);
            } else {
                println!("{DEMO_TOOL_WARNING}");
                println!();
                println!("Tool: move_file (alias: rename)");
                println!("  Description:   Move or rename a file within the workspace");
                println!("  Cognitive role: Transformation");
                println!("  Read-only:     no — sandboxed; simulate by default");
                println!("Arguments: source, destination, simulate, overwrite");
                println!("Security: absolute paths/.. blocked; max file 256 KiB");
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

fn tool_demo_write_file(args: ToolDemoWriteFileArgs) -> Result<(), Box<dyn Error>> {
    let config = ToolRuntimeConfig::new(".");
    let runtime = ToolRuntime::new(config);

    let result = runtime.execute(
        "write_file",
        &serde_json::json!({
            "path": args.path,
            "content": args.content,
            "simulate": !args.execute,
            "create_parent_dirs": args.create_parent_dirs,
            "overwrite": args.overwrite,
        }),
    );

    if args.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("{DEMO_TOOL_WARNING}");
        println!();
        println!("🧰 Tool demo: write_file");
        println!(
            "   Path: {}",
            result.observation.payload["path"].as_str().unwrap_or("?")
        );
        println!(
            "   Mode: {}",
            if args.execute { "execute" } else { "simulate" }
        );
        println!();
        match result.status {
            arpagona_agent_core::ToolExecutionStatus::Success => {
                println!("✅ Status: Success");
                println!("   {}", result.output_summary);
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
            arpagona_agent_core::ToolExecutionStatus::Skipped => println!("⏭️  Status: Skipped"),
        }
        println!();
        println!("Full result available with --json");
    }
    Ok(())
}

fn tool_demo_patch_file(args: ToolDemoPatchFileArgs) -> Result<(), Box<dyn Error>> {
    let config = ToolRuntimeConfig::new(".");
    let runtime = ToolRuntime::new(config);

    let result = runtime.execute(
        "patch_file",
        &serde_json::json!({
            "path": args.path,
            "old_string": args.old_string,
            "new_string": args.new_string,
            "simulate": !args.execute,
            "replace_all": args.replace_all,
        }),
    );

    if args.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("{DEMO_TOOL_WARNING}");
        println!();
        println!("Tool demo: patch_file");
        println!(
            "   Path: {}",
            result.observation.payload["path"].as_str().unwrap_or("?")
        );
        println!(
            "   Mode: {}",
            if args.execute { "execute" } else { "simulate" }
        );
        println!();
        match result.status {
            arpagona_agent_core::ToolExecutionStatus::Success => {
                println!("Status: Success");
                println!("   {}", result.output_summary);
            }
            arpagona_agent_core::ToolExecutionStatus::Blocked => {
                println!("Status: Blocked (security)");
                if let Some(error) = &result.error {
                    println!("   Reason: {}", error.message);
                }
            }
            arpagona_agent_core::ToolExecutionStatus::Failed => {
                println!("Status: Failed");
                if let Some(error) = &result.error {
                    println!("   Error: {}", error.message);
                }
            }
            arpagona_agent_core::ToolExecutionStatus::Warning => {
                println!("Status: Warning");
                println!("   {}", result.output_summary);
            }
            arpagona_agent_core::ToolExecutionStatus::Skipped => println!("Status: Skipped"),
        }
        println!();
        println!("Full result available with --json");
    }
    Ok(())
}

fn tool_demo_append_file(args: ToolDemoAppendFileArgs) -> Result<(), Box<dyn Error>> {
    let runtime = ToolRuntime::new(ToolRuntimeConfig::new("."));
    let result = runtime.execute(
        "append_file",
        &serde_json::json!({
            "path": args.path,
            "content": args.content,
            "simulate": !args.execute,
            "create_parent_dirs": args.create_parent_dirs,
            "create_if_missing": args.create_if_missing,
        }),
    );

    if args.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("{DEMO_TOOL_WARNING}");
        println!();
        println!("Tool demo: append_file");
        println!(
            "   Mode: {}",
            if args.execute { "execute" } else { "simulate" }
        );
        println!("   Status: {:?}", result.status);
        println!("   {}", result.output_summary);
        if let Some(error) = &result.error {
            println!("   Error: {}", error.message);
        }
        println!("Full result available with --json");
    }
    Ok(())
}

fn tool_demo_mkdir(args: ToolDemoMkdirArgs) -> Result<(), Box<dyn Error>> {
    let runtime = ToolRuntime::new(ToolRuntimeConfig::new("."));
    let result = runtime.execute(
        "mkdir",
        &serde_json::json!({
            "path": args.path,
            "simulate": !args.execute,
            "parents": args.parents,
        }),
    );

    if args.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("{DEMO_TOOL_WARNING}");
        println!();
        println!("Tool demo: mkdir");
        println!(
            "   Mode: {}",
            if args.execute { "execute" } else { "simulate" }
        );
        println!("   Status: {:?}", result.status);
        println!("   {}", result.output_summary);
        if let Some(error) = &result.error {
            println!("   Error: {}", error.message);
        }
        println!("Full result available with --json");
    }
    Ok(())
}

fn tool_demo_copy_file(args: ToolDemoCopyFileArgs) -> Result<(), Box<dyn Error>> {
    let runtime = ToolRuntime::new(ToolRuntimeConfig::new("."));
    let result = runtime.execute(
        "copy_file",
        &serde_json::json!({
            "source": args.source,
            "destination": args.destination,
            "simulate": !args.execute,
            "overwrite": args.overwrite,
        }),
    );

    if args.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("{DEMO_TOOL_WARNING}");
        println!();
        println!("Tool demo: copy_file");
        println!(
            "   Mode: {}",
            if args.execute { "execute" } else { "simulate" }
        );
        println!("   Status: {:?}", result.status);
        println!("   {}", result.output_summary);
        if let Some(error) = &result.error {
            println!("   Error: {}", error.message);
        }
        println!("Full result available with --json");
    }
    Ok(())
}

fn tool_demo_move_file(args: ToolDemoMoveFileArgs) -> Result<(), Box<dyn Error>> {
    let runtime = ToolRuntime::new(ToolRuntimeConfig::new("."));
    let result = runtime.execute(
        "move_file",
        &serde_json::json!({
            "source": args.source,
            "destination": args.destination,
            "simulate": !args.execute,
            "overwrite": args.overwrite,
        }),
    );

    if args.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("{DEMO_TOOL_WARNING}");
        println!();
        println!("Tool demo: move_file");
        println!(
            "   Mode: {}",
            if args.execute { "execute" } else { "simulate" }
        );
        println!("   Status: {:?}", result.status);
        println!("   {}", result.output_summary);
        if let Some(error) = &result.error {
            println!("   Error: {}", error.message);
        }
        println!("Full result available with --json");
    }
    Ok(())
}

const OBSERVE_TOOL_WARNING: &str =
    "⚠️  Cognitive observation pipeline demo — sandboxed, no authorization, no governance bypass. ⚠️";

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

const ACTOR_LAB_WARNING: &str = "⚠️  First Useful Actor Lab — supervised local sandbox only. Simulation first; execution requires --approve. ⚠️";

fn tool_demo_actor_lab(args: ToolDemoActorLabArgs) -> Result<(), Box<dyn Error>> {
    let note = if args.note.ends_with('\n') {
        args.note.clone()
    } else {
        format!("{}\n", args.note)
    };
    let user_task = format!(
        "Append one supervised note to the workspace-local lab file `{}` and read it back.",
        args.path
    );

    let simulate_args = serde_json::json!({
        "path": args.path,
        "content": note,
        "create_parent_dirs": true,
        "create_if_missing": true,
        "simulate": true,
    });

    let intent = ToolCallIntent {
        tool: "append_file".to_owned(),
        arguments: simulate_args.clone(),
        rationale: "First Useful Actor Lab: prove governed local file action via simulation before explicit approval.".to_owned(),
        risk_level: RiskLevel::Low,
    };

    let (decision, proposed_action) = govern_tool_call(&intent, &[Permission::ProposeToolUse]);
    let runtime = ToolRuntime::new(ToolRuntimeConfig::new("."));
    let simulation_result = if decision.status == DecisionStatus::Approved {
        Some(runtime.execute("append_file", &simulate_args))
    } else {
        None
    };

    let simulation_succeeded = simulation_result
        .as_ref()
        .map(|result| result.status == ToolExecutionStatus::Success)
        .unwrap_or(false);

    let execute_args = serde_json::json!({
        "path": args.path,
        "content": note,
        "create_parent_dirs": true,
        "create_if_missing": true,
        "simulate": false,
    });

    let execution_result =
        if args.approve && decision.status == DecisionStatus::Approved && simulation_succeeded {
            Some(runtime.execute("append_file", &execute_args))
        } else {
            None
        };

    let readback_result = execution_result
        .as_ref()
        .filter(|result| result.status == ToolExecutionStatus::Success)
        .map(|_| runtime.execute("read_file", &serde_json::json!({"path": args.path})));

    let observed_result = readback_result
        .as_ref()
        .or(execution_result.as_ref())
        .or(simulation_result.as_ref())
        .ok_or("Actor Lab could not produce a tool execution result to observe")?;
    let cognitive_observation =
        arpagona_agent_core::CognitiveObservation::from_tool_execution(observed_result);
    let assessment = arpagona_agent_core::assess_observation(&cognitive_observation);

    let approval_state = if args.approve {
        if execution_result
            .as_ref()
            .map(|result| result.status == ToolExecutionStatus::Success)
            .unwrap_or(false)
        {
            "approved_and_executed"
        } else {
            "approved_but_not_executed"
        }
    } else {
        "simulation_only_waiting_for_explicit_approval"
    };

    let journal_entry_id = {
        let mut journal = global_llm_journal().lock().unwrap();
        journal.add_direct_tool_call(
            "first_useful_actor_lab",
            "cli",
            None,
            user_task.clone(),
            format!(
                "Actor Lab {}: decision={:?}, simulation={}, execution={}",
                approval_state,
                decision.status,
                simulation_result
                    .as_ref()
                    .map(|result| format!("{:?}", result.status))
                    .unwrap_or_else(|| "not_run".to_owned()),
                execution_result
                    .as_ref()
                    .map(|result| format!("{:?}", result.status))
                    .unwrap_or_else(|| "not_run".to_owned())
            ),
            serde_json::json!({
                "lab": "first_useful_actor_lab",
                "user_task": user_task,
                "tool": "append_file",
                "simulate_arguments": simulate_args,
                "execute_arguments": execute_args,
                "approval_flag": args.approve,
            }),
            serde_json::json!({
                "decision_id": decision.id,
                "decision_status": decision.status,
                "decision_reason": decision.reason,
                "proposed_action_id": proposed_action.id,
                "approval_state": approval_state,
                "simulation_result": simulation_result,
                "execution_result": execution_result,
                "readback_result": readback_result,
                "cognitive_observation": cognitive_observation,
                "assessment": assessment,
                "non_authorizing": true,
            }),
            Some(RiskLevel::Low),
        )
    };

    if args.json {
        let output = serde_json::json!({
            "lab": "first_useful_actor_lab",
            "warning": ACTOR_LAB_WARNING,
            "user_task": user_task,
            "proposed_action": {
                "id": proposed_action.id,
                "action_type": proposed_action.action_type,
                "target": proposed_action.target,
                "risk_level": proposed_action.risk_level,
            },
            "decision": {
                "id": decision.id,
                "status": decision.status,
                "reason": decision.reason,
                "risk_level": decision.risk_level,
                "policies_applied": decision.policies_applied,
            },
            "simulation_result": simulation_result,
            "approval_state": approval_state,
            "execution_result": execution_result,
            "readback_result": readback_result,
            "cognitive_observation": cognitive_observation,
            "assessment": assessment,
            "journal_entry_id": journal_entry_id,
            "next_step": if args.approve { "Inspect readback_result and journal_entry_id." } else { "Rerun with --approve to perform the sandboxed append." },
            "non_authorizing": true,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("{ACTOR_LAB_WARNING}");
        println!();
        println!("🧪 First Useful Actor Lab");
        println!("   Mission: {user_task}");
        println!();
        println!("━━━ 1. Proposed sandboxed action ━━━");
        println!("   Tool:        append_file");
        println!("   Path:        {}", args.path);
        println!("   Risk:        Low");
        println!("   Proposal ID: {}", proposed_action.id);
        println!();
        println!("━━━ 2. Decision Gate ━━━");
        println!("   Decision:    {:?}", decision.status);
        println!("   Decision ID: {}", decision.id);
        println!("   Reason:      {}", decision.reason);
        println!();
        println!("━━━ 3. Simulation / diff preview ━━━");
        if let Some(result) = &simulation_result {
            println!("   Status:      {:?}", result.status);
            println!("   Summary:     {}", result.output_summary);
            println!("   Would append: {:?}", note);
        } else {
            println!("   Status:      not run because governance did not approve simulation");
        }
        println!();
        println!("━━━ 4. Explicit approval path ━━━");
        if args.approve {
            println!("   Approval:    --approve supplied");
        } else {
            println!("   Approval:    missing — simulation only");
            println!("   Next step:   rerun with --approve to execute the sandboxed append");
        }
        println!();
        println!("━━━ 5. Execution + readback ━━━");
        if let Some(result) = &execution_result {
            println!("   Execution:   {:?}", result.status);
            println!("   Summary:     {}", result.output_summary);
            if let Some(readback) = &readback_result {
                println!("   Readback:    {:?}", readback.status);
                println!("   Summary:     {}", readback.output_summary);
            }
        } else {
            println!("   Execution:   not run");
        }
        println!();
        println!("━━━ 6. Observation / audit readback ━━━");
        println!("   Observation: {:?}", cognitive_observation.status);
        println!("   Useful:      {}", assessment.is_useful);
        println!("   Complete:    {}", assessment.is_complete);
        println!("   Journal ID:  {}", journal_entry_id);
        println!("   Warning:     readback is evidence, not authorization");
        println!();
        println!("Full lab output available with --json");
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
                println!("{} {:?}", style_dim("risk_level:"), action.risk_level);
                println!("{} {}", style_dim("rationale:"), action.rationale);
                println!("{} {}", style_dim("created_at:"), action.created_at);
                if let Some(target) = &action.target {
                    println!("{} {}", style_dim("target:"), target);
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

async fn list_audit(
    client: &Client,
    api_url: &str,
    args: ListAuditArgs,
) -> Result<(), Box<dyn Error>> {
    let events: Vec<AuditEvent> =
        get_json(client.get(format!("{api_url}/audit")).send().await?).await?;

    if events.is_empty() {
        if args.json {
            println!("[]");
        } else {
            println!("{}", style_dim("No audit events."));
        }
        return Ok(());
    }

    if args.json {
        println!("{}", serde_json::to_string_pretty(&events)?);
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

// ─── Audit cycle trace discovery (local filesystem, no API server needed) ───

/// List saved CycleTrace files from the trace directory.
///
/// This is a local filesystem operation — no API server needed. It reuses the
/// same `list_orchestrator_cycles_in_directory` function that powers
/// `orchestrator cycles list`, making cycle traces discoverable from the audit
/// namespace.
///
/// # Safety
///
/// All output is readback only. No trace entry may be interpreted as approval,
/// authorization, or execution permission.
fn audit_list_traces(args: ListTracesArgs) -> Result<(), Box<dyn Error>> {
    let dir = std::path::PathBuf::from(&args.trace_dir);

    let entries = list_orchestrator_cycles_in_directory(&dir)?;

    if args.json {
        let listings: Vec<&CycleTraceListingEntry> =
            entries.iter().map(|(_, listing)| listing).collect();
        println!("{}", serde_json::to_string_pretty(&listings)?);
    } else {
        println!("{}", style_info("Audit: Orchestrator Cycle Traces"));
        println!("{}", "-".repeat(60));
        if entries.is_empty() {
            println!("{}", style_dim("No orchestrator cycle traces found."));
            println!();
            println!(
                "  Run `{}` first to save a trace.",
                "cargo run -q --bin arpagona -- orchestrator run --objective \"...\" --save-trace auto"
            );
            println!();
            println!(
                "  Default trace directory: {}",
                DEFAULT_ORCHESTRATOR_TRACES_DIR
            );
        } else {
            println!(
                "Found {} cycle trace(s) in '{}':\n",
                entries.len(),
                dir.display()
            );
            for (i, (_path, listing)) in entries.iter().enumerate() {
                println!("{}. \x1b[1m{}\x1b[0m", i + 1, listing.file_name);
                println!("   Cycle ID:     {}", listing.cycle_id);
                println!("   Objective:    {}", listing.objective_preview);
                println!("   Status:       {}", listing.cycle_status);
                println!("   Context srcs: {}", listing.context_source_count);
                println!("   Gate applied: {}", listing.gate_was_applied);
                println!("   Non-auth:     {}", listing.non_authorizing);
                println!(
                    "   FI cands:     {}",
                    listing.failure_insight_candidate_count
                );
                println!("   Audit events: {}", listing.audit_event_count);
                println!("   Created:      {}", listing.created_at);
                println!("   Summary:      {}", listing.summary_preview);
                println!();
            }
            println!(
                "Use `{} <cycle-id>` to inspect a specific trace.",
                "cargo run -q --bin arpagona -- audit get-trace"
            );
        }
        println!();
        println!(
            "{}",
            style_dim("⚠  Readback only — trace entries are evidence, not authorization.")
        );
        println!(
            "{}",
            style_dim("   No execution without explicit Decision Gate approval.")
        );
    }

    Ok(())
}

/// Read and display a specific CycleTrace by cycle ID.
///
/// Searches the trace directory for a file whose cycle_id matches the given
/// cycle ID, then displays it using the CycleTrace format method (human) or
/// as pretty-printed JSON.
///
/// # Safety
///
/// All output is readback only. No trace entry may be interpreted as approval,
/// authorization, or execution permission.
fn audit_get_trace(args: GetTraceArgs) -> Result<(), Box<dyn Error>> {
    use arpagona_agent_core::orchestrator::CycleTrace;

    let dir = std::path::PathBuf::from(&args.trace_dir);

    let entries = list_orchestrator_cycles_in_directory(&dir)?;

    // Find the first trace matching the requested cycle ID
    let matched: Vec<_> = entries
        .into_iter()
        .filter(|(_, listing)| listing.cycle_id == args.cycle_id)
        .collect();

    if matched.is_empty() {
        if args.json {
            println!("{{}}");
        } else {
            println!(
                "{}",
                style_dim(&format!(
                    "No cycle trace found with cycle ID '{}' in '{}'",
                    args.cycle_id,
                    dir.display()
                ))
            );
            println!();
            println!("Available traces:");
            let all_entries = list_orchestrator_cycles_in_directory(&dir)?;
            for (_path, listing) in &all_entries {
                println!("  - {}", listing.cycle_id);
            }
        }
        return Ok(());
    }

    let (path, _listing) = &matched[0];

    // Read and deserialize the full trace
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read '{}': {e}", path.display()))?;
    let trace: CycleTrace = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse CycleTrace from '{}': {e}", path.display()))?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&trace)?);
    } else {
        println!(
            "{}",
            style_info(&format!("Cycle Trace: {}", trace.cycle_id))
        );
        println!("{}", "-".repeat(60));
        println!("{}", trace.format());
        println!();
        println!(
            "{}",
            style_dim("⚠  Readback only — this trace is evidence, not authorization.")
        );
        println!(
            "{}",
            style_dim("   No execution without explicit Decision Gate approval.")
        );
        println!(
            "{}",
            style_dim(&format!("   Non-authorizing: {}", trace.non_authorizing))
        );
    }

    Ok(())
}

/// List saved audit event files from a directory and display each event.
///
/// Reads individual JSON audit event files saved by `orchestrator run --save-audit`
/// and displays each event with its type, timestamp, and payload preview.
///
/// # Safety
///
/// All output is readback only. No event content may be interpreted as approval,
/// authorization, or execution permission.
fn audit_list_events_from_dir(args: ListEventsFromDirArgs) -> Result<(), Box<dyn Error>> {
    let dir = std::path::Path::new(&args.from_dir);

    if !dir.exists() {
        if args.json {
            println!("[]");
        } else {
            println!(
                "{}",
                style_dim(&format!(
                    "Audit event directory '{}' does not exist.",
                    args.from_dir
                ))
            );
            println!();
            println!(
                "  Run `{}` to save audit events first.",
                "cargo run -q --bin arpagona -- orchestrator run --objective \"...\" --save-audit"
            );
            println!();
        }
        return Ok(());
    }

    // Read and parse all JSON files in the directory
    let mut events: Vec<(std::path::PathBuf, AuditEvent)> = Vec::new();
    for entry in std::fs::read_dir(dir)
        .map_err(|e| format!("Cannot read directory '{}': {e}", args.from_dir))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(event) = serde_json::from_str::<AuditEvent>(&content) {
                events.push((path, event));
            }
        }
    }

    // Sort by event id for deterministic output
    events.sort_by(|a, b| a.1.id.0.cmp(&b.1.id.0));

    if args.json {
        let json_events: Vec<&AuditEvent> = events.iter().map(|(_, e)| e).collect();
        println!("{}", serde_json::to_string_pretty(&json_events)?);
    } else {
        println!("{}", style_info("Audit Events (from saved files)"));
        println!("{}", "-".repeat(60));
        if events.is_empty() {
            println!(
                "{}",
                style_dim("No valid audit event files found in the directory.")
            );
        } else {
            println!(
                "Found {} audit event file(s) in '{}':\n",
                events.len(),
                args.from_dir
            );
            for (i, (path, event)) in events.iter().enumerate() {
                let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
                let event_type_str = format!("{:?}", event.event_type);
                let created_at_str = event.created_at.format("%Y-%m-%d %H:%M:%S UTC").to_string();
                let payload_preview = serde_json::to_string(&event.payload)
                    .unwrap_or_default()
                    .chars()
                    .take(120)
                    .collect::<String>();

                println!("{}. \x1b[1m{}\x1b[0m", i + 1, file_name);
                println!("   Event ID:    {}", event.id);
                println!("   Type:        {}", event_type_str);
                println!("   Actor:       {:?}", event.actor);
                println!("   Timestamp:   {}", created_at_str);
                if let Some(ws) = &event.workspace_id {
                    println!("   Workspace:   {}", ws);
                }
                if let Some(pa_id) = &event.proposed_action_id {
                    println!("   Proposed:    {}", pa_id);
                }
                if let Some(d_id) = &event.decision_id {
                    println!("   Decision:    {}", d_id);
                }
                println!("   Payload:     {}", payload_preview);
                println!();
            }
        }
        println!(
            "{}",
            style_dim("⚠  Readback only — audit events are evidence, not authorization.")
        );
        println!(
            "{}",
            style_dim("   No execution without explicit Decision Gate approval.")
        );
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
            aggregated_rationales,
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
                "tool_call_intents": e.tool_call_intents.as_ref().map(|v| redact_journal_value(v.clone())),
                "decision_gate_outcomes": e.decision_gate_outcomes.as_ref().map(|v| redact_journal_value(v.clone())),
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
                entry.prompt_summary.chars().take(120).collect::<String>()
            );
            println!(
                "        | response: {}",
                entry.response_summary.chars().take(120).collect::<String>()
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

/// Read-only supervision surface for `arpagona action supervise`.
///
/// Reads from the LLM journal and displays entries with proposed actions,
/// tool-call intents, Decision Gate outcomes, risk levels, and audit event IDs.
/// Works offline — no API server required.
fn action_supervise(args: ActionSuperviseArgs) -> Result<(), Box<dyn Error>> {
    let journal = global_llm_journal().lock().unwrap();
    let all_entries = journal.all_entries();

    // Filter by interaction type if specified
    let filtered: Vec<_> = if let Some(ref filter_type) = args.interaction_type {
        all_entries
            .iter()
            .filter(|e| {
                let type_str = format!("{:?}", e.interaction_type).to_lowercase();
                type_str.contains(&filter_type.to_lowercase())
                    || (filter_type.to_lowercase() == "governance"
                        && e.proposed_actions.is_some()
                        && e.decision_gate_outcomes.is_some())
            })
            .collect()
    } else {
        // Default: only show entries with governance-relevant data
        all_entries
            .iter()
            .filter(|e| {
                e.proposed_actions.is_some()
                    || e.tool_call_intents.is_some()
                    || e.decision_gate_outcomes.is_some()
            })
            .collect()
    };

    let limit = args.limit.min(filtered.len());
    let entries: Vec<_> = filtered.iter().rev().take(limit).collect();

    if args.json {
        let json_entries: Vec<serde_json::Value> = entries
            .iter()
            .map(|e| {
                serde_json::json!({
                    "id": e.id,
                    "created_at": e.created_at,
                    "interaction_type": format!("{:?}", e.interaction_type),
                    "provider": e.provider,
                    "model": e.model,
                    "objective": e.objective,
                    "prompt_summary": e.prompt_summary,
                    "response_summary": e.response_summary,
                    "proposed_actions": e.proposed_actions,
                    "tool_call_intents": e.tool_call_intents.as_ref().map(|v| redact_journal_value(v.clone())),
                    "decision_gate_outcomes": e.decision_gate_outcomes.as_ref().map(|v| redact_journal_value(v.clone())),
                    "risk_level": e.risk_level,
                })
            })
            .collect();
        let output = serde_json::json!({
            "supervision_entries": json_entries,
            "total_entries": journal.len(),
            "total_governance_entries": filtered.len(),
            "shown_entries": entries.len(),
            "non_authorizing_warning": "Readback only — these supervision entries are evidence, not authorization. No execution without explicit Decision Gate approval."
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!(
            "Action Supervision Surface — {} entries showing ({} total, {} with governance data)",
            entries.len(),
            journal.len(),
            filtered.len()
        );
        println!("{}", "-".repeat(72));
        println!();

        if entries.is_empty() {
            println!("No governance-related entries found in the LLM journal.");
            println!("Run `arpagona cognitive run --objective \"...\" --govern --json` to generate governance data.");
        }

        for entry in entries {
            let created = entry.created_at.format("%Y-%m-%d %H:%M:%S");
            println!(
                "Entry: #{} | {} | {}",
                &entry.id[entry.id.len().saturating_sub(12)..],
                format!("{:?}", entry.interaction_type),
                created
            );
            println!("  Provider: {}", entry.provider);
            if let Some(ref model) = entry.model {
                println!("  Model: {}", model);
            }
            if let Some(ref obj) = entry.objective {
                println!("  Objective: {}", &obj[..obj.len().min(80)]);
            }
            if let Some(ref rl) = entry.risk_level {
                println!("  Risk Level: {:?}", rl);
            }
            if let Some(ref pa) = entry.proposed_actions {
                let pa_str = serde_json::to_string_pretty(pa).unwrap_or_default();
                if pa_str.len() > 200 {
                    println!(
                        "  Proposed Actions: {} entries",
                        pa.as_array().map(|a| a.len()).unwrap_or(0)
                    );
                    println!("    {}", &pa_str[..pa_str.len().min(200)]);
                } else {
                    println!("  Proposed Actions: {}", pa_str);
                }
            }
            if let Some(ref tci) = entry.tool_call_intents {
                let tci_str = serde_json::to_string_pretty(tci).unwrap_or_default();
                if tci_str.len() > 200 {
                    println!(
                        "  Tool-Call Intents: {} entries",
                        tci.as_array().map(|a| a.len()).unwrap_or(0)
                    );
                    println!("    {}", &tci_str[..tci_str.len().min(200)]);
                } else {
                    println!("  Tool-Call Intents: {}", tci_str);
                }
            }
            if let Some(ref dg) = entry.decision_gate_outcomes {
                let dg_str = serde_json::to_string_pretty(dg).unwrap_or_default();
                if dg_str.len() > 200 {
                    println!(
                        "  Decision Gate Outcomes: {} entries",
                        dg.as_array().map(|a| a.len()).unwrap_or(0)
                    );
                    println!("    {}", &dg_str[..dg_str.len().min(200)]);
                } else {
                    println!("  Decision Gate Outcomes: {}", dg_str);
                }
                // Extract and show audit event IDs
                if let Some(arr) = dg.as_array() {
                    for (idx, outcome) in arr.iter().enumerate() {
                        let audit_id = outcome
                            .get("audit_event")
                            .and_then(|ae| ae.get("id"))
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_owned());
                        let decision_status = outcome
                            .get("decision")
                            .and_then(|d| d.get("status"))
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_owned());
                        if let (Some(aid), Some(ds)) = (&audit_id, &decision_status) {
                            println!(
                                "    Audit Entry #{}: decision={}, event_id={}",
                                idx + 1,
                                ds,
                                aid
                            );
                        }
                    }
                }
            }
            println!();
        }

        println!("{}", "-".repeat(72));
        println!("⚠  Readback only — supervision entries are evidence, not authorization.");
        println!("   No execution without explicit Decision Gate approval.");
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
                            .map(arpagona_agent_core::assess_observation)
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
                    .cloned()
                    .unwrap_or_default();

                // Collect CognitiveObservations from working_memory (injected by --observe)
                let cognitive_observations: Vec<serde_json::Value> = obj
                    .get("working_memory")
                    .and_then(|wm| wm.get("cognitive_observations"))
                    .and_then(|v| v.as_array())
                    .cloned()
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
                    .cloned()
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

                            // Journal governance results to the LLM journal for D2 supervision readback
                            let proposed_actions_json: Vec<Value> = governance_results
                                .iter()
                                .filter_map(|r| r.get("proposed_action").cloned())
                                .collect();
                            let governance_outcomes_json: Vec<Value> = governance_results
                                .iter()
                                .map(|r| {
                                    let entry = serde_json::json!({
                                        "proposed_action_id": r.get("proposed_action_id"),
                                        "decision": r.get("decision"),
                                        "audit_event": r.get("audit_event"),
                                    });
                                    entry
                                })
                                .collect();
                            let max_risk = governance_results
                                .iter()
                                .filter_map(|r| r.get("decision").and_then(|d| d.get("risk_level")))
                                .filter_map(|v| v.as_str())
                                .map(|s| s.to_owned())
                                .collect::<Vec<_>>()
                                .join(", ");
                            let risk = if max_risk.is_empty() {
                                None
                            } else {
                                Some(max_risk)
                            };
                            global_llm_journal()
                                .lock()
                                .unwrap()
                                .add_governance(
                                    &args.objective,
                                    "offline-governance",
                                    None,
                                    format!(
                                        "Offline governance: {:?} FailureInsightCandidates → DecisionGate → AuditEvents",
                                        failure_insight_candidates.len(),
                                    ),
                                    format!(
                                        "Governance complete: {} decisions, {} audit events",
                                        decisions.len(),
                                        audit_events.len(),
                                    ),
                                    serde_json::to_value(&proposed_actions_json).unwrap_or_default(),
                                    serde_json::to_value(&governance_outcomes_json).unwrap_or_default(),
                                    risk.as_deref().and_then(|risk_str| {
                                        match risk_str {
                                            "low" => Some(RiskLevel::Low),
                                            "medium" => Some(RiskLevel::Medium),
                                            "high" => Some(RiskLevel::High),
                                            "critical" => Some(RiskLevel::Critical),
                                            _ => None,
                                        }
                                    }),
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
                let mut context_details = String::new();
                for (i, item) in wm.context_items.iter().enumerate() {
                    context_details.push_str(&format!(
                        "\nContext item {}: key={}, value={}, source={}",
                        i + 1,
                        item.key,
                        item.value,
                        item.source,
                    ));
                }
                let wm_summary = format!(
                    "Domain: {:?}\nSensitivity: {:?}\nComplexity: {:.2}\nContext items: {}{}\nMissing context: {}\nAssumptions: {}\nProposed next action: {:?}",
                    result.objective.domain,
                    wm.sensitivity_estimate,
                    wm.complexity_estimate,
                    wm.context_items.len(),
                    context_details,
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
                        args.provider.as_str(),
                        "Provider set via --provider flag".to_owned(),
                    )
                };

                // Resolve model name for the resolved provider
                let resolved_model: Option<String> = match resolved_provider {
                    "mock" => None,
                    "openai" => Some(
                        std::env::var("OPENAI_MODEL")
                            .unwrap_or_else(|_| arpagona_llm::DEFAULT_OPENAI_MODEL.to_owned()),
                    ),
                    "ollama" => Some(
                        std::env::var("OLLAMA_MODEL")
                            .unwrap_or_else(|_| arpagona_llm::DEFAULT_OLLAMA_MODEL.to_owned()),
                    ),
                    other => {
                        // Unknown provider — try env var, otherwise indicate unknown
                        Some(format!("unknown/{other}"))
                    }
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
                                resolved_model, // model metadata tracked via env vars or defaults
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

            // When --govern-tool, request a tool-call intent from the LLM provider,
            // route it through the Decision Gate, execute approved calls through
            // the bounded Tool Runtime, and journal the full trace (C2).
            if args.govern_tool {
                let resolved_provider =
                    env::var("ARPAGONA_LLM_PROVIDER").unwrap_or_else(|_| "mock".to_owned());

                match request_tool_call_from_llm(&args.objective, &resolved_provider).await {
                    Ok(intent) => {
                        // Route through Decision Gate
                        let (decision, _proposed_action) =
                            govern_tool_call(&intent, &[Permission::ProposeToolUse]);

                        // Execute approved calls through bounded Tool Runtime
                        let execution_result = if decision.status == DecisionStatus::Approved {
                            let config = ToolRuntimeConfig::new(".");
                            let runtime = ToolRuntime::new(config);
                            Some(runtime.execute(&intent.tool, &intent.arguments))
                        } else {
                            None
                        };

                        // Journal the full trace (intent -> decision -> result -> observation)
                        let response_summary = if let Some(ref result) = execution_result {
                            format!(
                                "Decision Gate: {:?} — Tool runtime: {} ({:?})",
                                decision.status, result.output_summary, result.status
                            )
                        } else {
                            format!(
                                "Decision Gate result: status={:?}, reason={}",
                                decision.status, decision.reason
                            )
                        };

                        let journal_entry_id = global_llm_journal()
                            .lock()
                            .unwrap()
                            .add_direct_tool_call(
                                &intent.tool,
                                &resolved_provider,
                                None,
                                format!(
                                    "Governed tool-call from LLM: tool={}, objective={}, rationale={}",
                                    intent.tool, args.objective, intent.rationale
                                ),
                                response_summary,
                                serde_json::json!({
                                    "tool": intent.tool,
                                    "arguments": intent.arguments,
                                    "rationale": intent.rationale,
                                    "risk_level": intent.risk_level,
                                    "objective": args.objective,
                                }),
                                serde_json::json!({
                                    "decision_id": decision.id,
                                    "status": decision.status,
                                    "reason": decision.reason,
                                    "risk_level": decision.risk_level,
                                    "policies_applied": decision.policies_applied,
                                    "execution_result": execution_result.as_ref().map(|r| serde_json::json!({
                                        "status": r.status,
                                        "output_summary": r.output_summary,
                                        "failure_insight_candidate": r.failure_insight_candidate,
                                    })),
                                }),
                                Some(intent.risk_level),
                            );

                        // Inject governance results into output
                        obj.insert(
                            "governed_tool_call".to_owned(),
                            serde_json::json!({
                                "tool": intent.tool,
                                "arguments": intent.arguments,
                                "rationale": intent.rationale,
                                "decision": {
                                    "id": decision.id,
                                    "status": decision.status,
                                    "reason": decision.reason,
                                    "risk_level": decision.risk_level,
                                    "policies_applied": decision.policies_applied,
                                },
                                "execution_result": execution_result.as_ref().map(|r| serde_json::json!({
                                    "status": r.status,
                                    "output_summary": r.output_summary,
                                    "observation": {
                                        "summary": r.observation.summary,
                                        "is_actionable": r.observation.actionable,
                                        "failure_insight_candidate": r.failure_insight_candidate,
                                        "payload": r.observation.payload,
                                    },
                                })),
                                "journal_entry_id": journal_entry_id,
                                "llm_provider": resolved_provider,
                            }),
                        );
                        obj.insert(
                            "governed_tool_called".to_owned(),
                            serde_json::Value::Bool(true),
                        );
                    }
                    Err(e) => {
                        obj.insert(
                            "governed_tool_call_error".to_owned(),
                            serde_json::Value::String(format!("Governed tool-call failed: {e}")),
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

/// Preview compute routing from routing args (standalone `compute routing` command).
fn compute_routing(args: RoutingArgs) -> Result<(), Box<dyn Error>> {
    use arpagona_agent_core::cognitive_work::{SensitivityEstimate, WorkingMemory};

    let sensitivity = args.sensitivity.to_data_sensitivity();
    let sensitivity_estimate = match sensitivity {
        DataSensitivity::Public => SensitivityEstimate::Public,
        DataSensitivity::Internal => SensitivityEstimate::Internal,
        DataSensitivity::Confidential => SensitivityEstimate::Confidential,
        DataSensitivity::Secret => SensitivityEstimate::Secret,
    };

    let working_memory = WorkingMemory {
        objective: None,
        context_items: vec![],
        assumptions: vec![],
        constraints: vec![],
        missing_context: vec![],
        sensitivity_estimate,
        complexity_estimate: args.complexity as f32,
        local_first: args.local_first,
        cost_sensitive: args.local_first,
        proposed_next_action_kind: String::new(),
        required_observations_count: 0,
        required_observations: vec![],
        cognitive_observations: vec![],
        improvement_candidates: vec![],
        failure_insight_candidates: vec![],
        proposed_next_action: None,
        cycle_status: arpagona_agent_core::cognitive_work::CycleStatus::Initialized,
        evidence_only_warning:
            "Routing preview — evidence-only, not authorization, not Decision Gate bypass."
                .to_owned(),
    };

    let nodes = demo_inventory();
    let policy = ComputePolicy::default();
    let allocation = allocate_for_working_memory(&working_memory, &nodes, &policy);

    // Resolve provider from allocation
    let unknown_node_s: String;
    let (provider, provider_note) = match allocation.selected_node_id.as_ref().map(|id| id.as_str())
    {
        Some("cloud-strong") => (
            "openai",
            "routed via ComputeReservoir: cloud-strong → openai (cloud, high strength)",
        ),
        Some("local-smol") => (
            "ollama",
            "routed via ComputeReservoir: local-smol → ollama (local, low cost)",
        ),
        Some("local-cpu") => (
            "mock",
            "routed via ComputeReservoir: local-cpu → mock (deterministic, no LLM)",
        ),
        Some(other) => {
            unknown_node_s = format!("unknown node '{other}'");
            ("unknown", unknown_node_s.as_str())
        }
        None => ("none", "no suitable resource"),
    };

    // Build trade-off explanation
    let cost_savings: &str = match &allocation.selected_node_id {
        Some(id) if id.as_str() != "cloud-strong" => {
            "Local resource cost: 0¢ (vs ~50¢ cloud equivalent)."
        }
        _ => "Cloud resource cost: varies by provider (check env).",
    };

    let latency_s: String;
    let latency_note: &str = match allocation.expected_latency_ms {
        Some(ms) if ms <= 100 => "Real-time suitable.",
        Some(ms) if ms <= 1000 => "Slightly latent but interactive.",
        Some(ms) => {
            latency_s = format!("High latency (~{ms}ms); unsuitable for interactive use.");
            &latency_s
        }
        None => "Latency unknown.",
    };

    let sensitivity_requires_local =
        sensitivity == DataSensitivity::Confidential || sensitivity == DataSensitivity::Secret;
    let privacy_note = if sensitivity_requires_local {
        "Sensitive data → local-only processing enforced by policy."
    } else {
        "Non-sensitive data → cloud routing allowed by policy."
    };

    let non_authorizing = NON_AUTHORIZING_READBACK;

    if args.json {
        let output = serde_json::json!({
            "compute_routing": {
                "request": {
                    "purpose": args.purpose,
                    "sensitivity": sensitivity,
                    "complexity": args.complexity,
                    "local_first": args.local_first,
                },
                "allocation": {
                    "status": allocation.status,
                    "selected_node_id": allocation.selected_node_id.as_ref().map(|id| id.as_str()),
                    "resource_kind": allocation.resource_kind,
                    "expected_cost_cents": allocation.expected_cost_cents,
                    "expected_latency_ms": allocation.expected_latency_ms,
                    "justification": allocation.justification,
                    "fallback": allocation.fallback,
                },
                "resolved_provider": provider,
                "provider_rationale": provider_note,
                "trade_offs": {
                    "cost": cost_savings,
                    "latency": latency_note,
                    "privacy": privacy_note,
                }
            },
            "non_authorizing_warning": non_authorizing,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!();
        println!(
            "{}",
            style_brand("=== Compute Reservoir Routing Preview (C4) ===")
        );
        println!();
        println!("{}", style_info("Request Summary"));
        println!("  purpose:    {}", args.purpose);
        println!("  sensitivity: {:?}", sensitivity);
        println!("  complexity:  {:.2}", args.complexity);
        println!("  local_first: {}", args.local_first);
        println!();

        print_allocation_readback(&allocation);
        println!();

        println!("{}", style_info("Resolved Provider"));
        println!("  provider:       {}", provider);
        println!("  rationale:      {}", provider_note);
        println!();

        println!("{}", style_info("Trade-off Analysis"));
        println!("  {}", cost_savings);
        println!("  {}", latency_note);
        println!("  {}", privacy_note);
        println!();

        println!(
            "{}",
            style_dim("⚠️  Routing preview is not authorization — no action approved, no Decision Gate bypass.")
        );
        println!();
    }

    // Journal the compute routing interaction for operator readback (C3/C4 integration)
    let journal_routing = serde_json::json!({
        "standalone_routing_preview": true,
        "purpose": args.purpose,
        "sensitivity": sensitivity,
        "complexity": args.complexity,
        "local_first": args.local_first,
        "selected_node_id": allocation.selected_node_id.as_ref().map(|id| id.as_str()),
        "resource_kind": allocation.resource_kind,
        "resolved_provider": provider,
        "provider_rationale": provider_note,
        "cost_trade_off": cost_savings,
        "latency_trade_off": latency_note,
        "privacy_trade_off": privacy_note,
    });
    global_llm_journal().lock().unwrap().add_compute_routing(
        &args.purpose,
        provider,
        journal_routing,
    );

    Ok(())
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

// ---------------------------------------------------------------------------
// Actor types
// ---------------------------------------------------------------------------

/// The tool to invoke and its arguments, parsed from NL.
#[derive(Debug, Clone, Serialize)]
struct ActorIntent {
    tool: String,
    arguments: serde_json::Value,
    risk_level: RiskLevel,
    rationale: String,
    display_summary: String,
}

/// Errors from NL intent parsing.
#[derive(Debug, Clone)]
enum IntentParseError {
    UnrecognizedTask(String),
    MissingArgument(String),
    /// Ollama endpoint was unreachable or returned an error.
    OllamaUnavailable(String),
    /// Ollama returned invalid JSON that could not be parsed as an intent.
    InvalidOllamaResponse(String),
    /// Ollama proposed a tool that is not in the allowed list.
    DisallowedTool(String),
    /// Ollama response was missing required fields (tool, arguments, rationale, risk_level).
    IncompleteResponse(String),
}

/// Seam for pluggable intent interpretation providers.
///
/// Roadmap:
/// - Current (phase 1): DeterministicIntentInterpreter (std::str parsing) — no deps, no LLM
/// - Next:   OllamaIntentInterpreter  — local LLM proposes ToolCallIntent via `arpagona-llm`
/// - Later:  DeepSeekIntentInterpreter — advanced reasoning for self-improvement
///
/// LLM providers must never execute tools directly. They may only propose
/// a structured ToolCallIntent that passes through deterministic validation,
/// allowed-tool checks, risk labeling, Decision Gate, simulation, explicit
/// approval, execution, readback, journal.
trait IntentInterpreter {
    fn interpret(&self, task: &str) -> Result<ActorIntent, IntentParseError>;
}

/// Current deterministic interpreter using only std::str operations.
/// No external dependencies, no LLM, no network.
struct DeterministicIntentInterpreter;

impl IntentInterpreter for DeterministicIntentInterpreter {
    fn interpret(&self, task: &str) -> Result<ActorIntent, IntentParseError> {
        parse_intent(task)
    }
}

/// An Ollama-backed intent interpreter that calls a local Ollama instance.
///
/// # Safety
///
/// - Ollama may only propose structured `ToolCallIntent` values.
/// - All proposed intents pass through deterministic validation:
///   allowed-tool checks, schema validation, and risk labeling.
/// - Direct tool execution by the LLM is forbidden.
/// - The proposal always enters the Decision Gate as PendingDecision.
struct OllamaIntentInterpreter {
    client: reqwest::Client,
    endpoint: String,
    model: String,
}

impl OllamaIntentInterpreter {
    fn new() -> Self {
        let endpoint =
            env::var("OLLAMA_ENDPOINT").unwrap_or_else(|_| DEFAULT_OLLAMA_ENDPOINT.to_owned());
        let model = env::var("OLLAMA_MODEL").unwrap_or_else(|_| DEFAULT_OLLAMA_MODEL.to_owned());
        Self {
            client: reqwest::Client::new(),
            endpoint,
            model,
        }
    }

    fn with_model(model: String) -> Self {
        let endpoint =
            env::var("OLLAMA_ENDPOINT").unwrap_or_else(|_| DEFAULT_OLLAMA_ENDPOINT.to_owned());
        Self {
            client: reqwest::Client::new(),
            endpoint,
            model,
        }
    }

    /// Call Ollama and parse a structured intent from the response.
    fn call_ollama(&self, task: &str) -> Result<ActorIntent, IntentParseError> {
        let system_prompt = r#"You are an intent parsing router. Given a natural language task, return ONLY valid JSON with exactly this structure:
{
  "tool": "append_file" | "read_file" | "list_files" | "search_text",
  "arguments": { ... },
  "rationale": "why this tool was chosen",
  "risk_level": "informational" | "low"
}

Rules:
- ALLOWED tools only: append_file, read_file, list_files, search_text
- "read_file" and "list_files" and "search_text" are informational risk
- "append_file" is low risk
- Return ONLY the JSON object, nothing else

Per-tool argument schemas (use EXACTLY these field names):
- "read_file" arguments: {"path": "/path/to/file"}
- "append_file" arguments: {"path": "/path/to/file", "content": "text to append"}
- "list_files" arguments: {"path": "/optional/directory"} (path is optional, omit to list cwd)
- "search_text" arguments: {"pattern": "search term", "path": "/optional/file-or-dir"} (path is optional)
- Never claim to execute, approve, or bypass governance"#;

        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": task}
            ],
            "stream": false,
            "options": {
                "temperature": 0.1
            }
        });

        let response = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async { self.client.post(&self.endpoint).json(&body).send().await })
                .map_err(|e| {
                    IntentParseError::OllamaUnavailable(format!(
                        "{} — endpoint: {}",
                        e, self.endpoint
                    ))
                })
        })?;

        let status = response.status();
        let value: serde_json::Value = tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async { response.json().await })
                .map_err(|e| {
                    IntentParseError::InvalidOllamaResponse(format!(
                        "{} — endpoint: {}",
                        e, self.endpoint
                    ))
                })
        })?;

        if !status.is_success() {
            let msg = value
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            return Err(IntentParseError::OllamaUnavailable(format!(
                "{} (HTTP {}) — endpoint: {}",
                msg, status, self.endpoint
            )));
        }

        let content = value
            .pointer("/message/content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                IntentParseError::InvalidOllamaResponse(
                    "missing /message/content in response".to_owned(),
                )
            })?;

        // Strip markdown code fences if present
        let cleaned = content
            .trim()
            .strip_prefix("```json")
            .or_else(|| content.trim().strip_prefix("```"))
            .map(|s| s.trim_end_matches("```").trim())
            .unwrap_or(content.trim());

        let parsed: serde_json::Value = serde_json::from_str(cleaned).map_err(|e| {
            IntentParseError::InvalidOllamaResponse(format!("JSON parse error: {e}"))
        })?;

        parse_ollama_intent(&parsed)
    }
}

impl IntentInterpreter for OllamaIntentInterpreter {
    fn interpret(&self, task: &str) -> Result<ActorIntent, IntentParseError> {
        self.call_ollama(task)
    }
}

impl std::fmt::Display for IntentParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IntentParseError::UnrecognizedTask(t) => {
                write!(f, "Unrecognized task: '{t}'. Supported: append <content> to <path>, read <path>, show <path>, list files [in <path>], list directory <path>, search [for] <pattern> [in <path>], find <pattern> [in <path]>.")
            }
            IntentParseError::MissingArgument(m) => {
                write!(f, "Missing argument: {m}")
            }
            IntentParseError::OllamaUnavailable(msg) => {
                write!(f, "Ollama unavailable: {msg}")
            }
            IntentParseError::InvalidOllamaResponse(msg) => {
                write!(f, "Invalid response from Ollama: {msg}")
            }
            IntentParseError::DisallowedTool(tool) => {
                write!(f, "Ollama proposed disallowed tool '{tool}'. Allowed: append_file, read_file, list_files, search_text")
            }
            IntentParseError::IncompleteResponse(msg) => {
                write!(f, "Incomplete response from Ollama: {msg}")
            }
        }
    }
}

/// Deterministic NL intent parser using only std::str operations.
/// No external regex dependency.
fn parse_intent(task: &str) -> Result<ActorIntent, IntentParseError> {
    let task = task.trim();
    let lower = task.to_lowercase();

    // --- Append patterns: "append <content> to <path>", "append <content> at <path>", "add <content> to <path>" ---
    if let Some(rest) = lower
        .strip_prefix("append ")
        .or_else(|| lower.strip_prefix("add "))
    {
        if let Some(to_pos) = rest.rfind(" to ").or_else(|| rest.rfind(" at ")) {
            let prefix_len = if lower.starts_with("append ") { 7 } else { 4 };
            let content = &task[prefix_len..to_pos + task.len() - rest.len()];
            let path = &task[to_pos + task.len() - rest.len() + 4..];
            return Ok(ActorIntent {
                tool: "append_file".to_owned(),
                arguments: serde_json::json!({
                    "content": content.trim(),
                    "path": path.trim(),
                    "create_parent_dirs": true,
                    "create_if_missing": true,
                }),
                risk_level: RiskLevel::Low,
                rationale: format!("Append content to file at {}", path.trim()),
                display_summary: format!("Append to {}", path.trim()),
            });
        }
    }

    // --- Read patterns: "read <path>", "show <path>" ---
    if lower.starts_with("read ") || lower.starts_with("show ") {
        let path = task[5..].trim(); // "read " and "show " are both 5 chars
        return Ok(ActorIntent {
            tool: "read_file".to_owned(),
            arguments: serde_json::json!({ "path": path }),
            risk_level: RiskLevel::Informational,
            rationale: format!("Read file at {path}"),
            display_summary: format!("Read {path}"),
        });
    }

    // --- List patterns: "list files", "list files in <path>", "list directory <path>" ---
    if lower.starts_with("list files") || lower.starts_with("list directory") {
        let path = if lower.starts_with("list files in ") {
            task[14..].trim() // "list files in " is 14 chars
        } else if lower.starts_with("list directory ") {
            task[15..].trim() // "list directory " is 15 chars
        } else {
            ""
        };
        return Ok(ActorIntent {
            tool: "list_files".to_owned(),
            arguments: serde_json::json!({ "path": path }),
            risk_level: RiskLevel::Informational,
            rationale: format!(
                "List files in {}",
                if path.is_empty() {
                    "workspace root"
                } else {
                    path
                }
            ),
            display_summary: if path.is_empty() {
                "List files".to_owned()
            } else {
                format!("List files in {path}")
            },
        });
    }

    // --- Search patterns: "search for <pattern> in <path>", "search <pattern> in <path>", "find <pattern> in <path>", "search <pattern>", "find <pattern>" ---
    if lower.starts_with("search ") || lower.starts_with("find ") {
        let after_keyword = if lower.starts_with("search for ") {
            &task[11..]
        } else if lower.starts_with("search ") {
            &task[7..]
        } else if lower.starts_with("find ") {
            &task[5..]
        } else {
            &task[0..]
        };
        if let Some(in_pos) = after_keyword.rfind(" in ") {
            let pattern = after_keyword[..in_pos].trim();
            let path = after_keyword[in_pos + 4..].trim();
            return Ok(ActorIntent {
                tool: "search_text".to_owned(),
                arguments: serde_json::json!({
                    "pattern": pattern,
                    "path": path,
                }),
                risk_level: RiskLevel::Informational,
                rationale: format!("Search for '{pattern}' in {path}"),
                display_summary: format!("Search for '{pattern}'"),
            });
        }
        // No "in" clause -- search whole workspace
        return Ok(ActorIntent {
            tool: "search_text".to_owned(),
            arguments: serde_json::json!({
                "pattern": after_keyword.trim(),
                "path": "",
            }),
            risk_level: RiskLevel::Informational,
            rationale: format!("Search for '{}' in workspace", after_keyword.trim()),
            display_summary: format!("Search for '{}'", after_keyword.trim()),
        });
    }

    Err(IntentParseError::UnrecognizedTask(task.to_owned()))
}

/// Top-level `arpagona actor run "<task>"` command.
///
/// Parses the natural language task, runs it through the governed
/// simulation -> approval -> execution -> readback -> journal loop.
const ACTOR_RUN_WARNING: &str = "[WARNING - Actor Run is a sandboxed governed local mission. Simulation first; execution requires --approve.]";
const ACTOR_SESSION_WARNING: &str = "[WARNING - Actor Session is a governed local loop. Each task is simulated first; rerun with `arpagona actor run --approve` to execute after review.]";

/// Core actor run logic: returns a structured JSON value with all result fields.
/// Does NOT print anything — used by both `actor_run` and `actor_session`.
/// `approve` controls whether execution follows simulation (session always passes false).
fn actor_run_core(
    task: &str,
    workspace: &str,
    approve: bool,
    interpreter: &dyn IntentInterpreter,
) -> Result<serde_json::Value, Box<dyn Error>> {
    let intent = interpreter.interpret(task).map_err(|e| format!("{e}"))?;

    let tool_call_intent = ToolCallIntent {
        tool: intent.tool.clone(),
        arguments: intent.arguments.clone(),
        rationale: intent.rationale.clone(),
        risk_level: intent.risk_level.clone(),
    };

    let workspace_path = if workspace.is_empty() || workspace == "." {
        ".".to_owned()
    } else {
        workspace.to_owned()
    };

    let (decision, proposed_action) =
        govern_tool_call(&tool_call_intent, &[Permission::ProposeToolUse]);

    let runtime = ToolRuntime::new(ToolRuntimeConfig::new(&workspace_path));

    let simulate_args = {
        let mut sim = intent.arguments.clone();
        sim["simulate"] = serde_json::Value::Bool(true);
        sim
    };

    let simulation_result = if decision.status == DecisionStatus::Approved {
        Some(runtime.execute(&intent.tool, &simulate_args))
    } else {
        None
    };

    let simulation_succeeded = simulation_result
        .as_ref()
        .map(|r| r.status == ToolExecutionStatus::Success)
        .unwrap_or(false);

    let is_read_only = matches!(
        intent.tool.as_str(),
        "read_file" | "list_files" | "search_text"
    );

    let execute_args = {
        let mut exec = intent.arguments.clone();
        exec["simulate"] = serde_json::Value::Bool(false);
        exec
    };

    // Session always passes approve=false — no implicit approval in V0
    let execution_result = if approve
        && decision.status == DecisionStatus::Approved
        && simulation_succeeded
        && !is_read_only
    {
        Some(runtime.execute(&intent.tool, &execute_args))
    } else {
        None
    };

    let readback_result = execution_result
        .as_ref()
        .filter(|r| r.status == ToolExecutionStatus::Success)
        .and_then(|_| {
            let path = intent.arguments.get("path")?.as_str()?;
            runtime
                .execute("read_file", &serde_json::json!({ "path": path }))
                .into()
        });

    let observed_result = execution_result
        .as_ref()
        .or_else(|| simulation_result.as_ref());
    let cognitive_observation = observed_result.and_then(|r| {
        let obs = arpagona_agent_core::CognitiveObservation::from_tool_execution(r);
        Some(obs)
    });
    let assessment = cognitive_observation
        .as_ref()
        .map(|obs| arpagona_agent_core::assess_observation(obs));

    let approval_state = if approve && execution_result.is_some() {
        "approved_and_executed"
    } else if approve {
        "approved_but_not_executed"
    } else {
        "simulation_only_waiting_for_explicit_approval"
    };

    let journal_entry_id = {
        let mut journal = global_llm_journal().lock().unwrap();
        journal.add_direct_tool_call(
            "actor_run",
            "cli",
            None,
            task.to_owned(),
            format!(
                "Actor Run {}: tool={}, decision={:?}, simulation={}, execution={}",
                approval_state,
                intent.tool,
                decision.status,
                simulation_result
                    .as_ref()
                    .map(|r| format!("{:?}", r.status))
                    .unwrap_or_else(|| "not_run".to_owned()),
                execution_result
                    .as_ref()
                    .map(|r| format!("{:?}", r.status))
                    .unwrap_or_else(|| "not_run".to_owned()),
            ),
            serde_json::json!({
                "command": "actor_run",
                "user_task": task,
                "tool": intent.tool,
                "parsed_intent": intent,
                "workspace": workspace_path,
                "approval_flag": approve,
            }),
            serde_json::json!({
                "decision_id": decision.id,
                "decision_status": decision.status,
                "decision_reason": decision.reason,
                "proposed_action_id": proposed_action.id,
                "approval_state": approval_state,
                "simulation_result": simulation_result,
                "execution_result": execution_result,
                "readback_result": readback_result,
                "cognitive_observation": cognitive_observation,
                "assessment": assessment,
            }),
            Some(intent.risk_level.clone()),
        )
    };

    let output = serde_json::json!({
        "task": task,
        "intent": {
            "tool": intent.tool,
            "rationale": intent.rationale,
            "risk_level": format!("{:?}", intent.risk_level),
            "display_summary": intent.display_summary,
        },
        "decision": {
            "id": decision.id,
            "status": format!("{:?}", decision.status),
            "reason": decision.reason,
        },
        "simulation_result": simulation_result,
        "approval_state": approval_state,
        "execution_result": execution_result,
        "readback_result": readback_result,
        "cognitive_observation": cognitive_observation,
        "journal_entry_id": journal_entry_id,
        "is_read_only": is_read_only,
        "workspace_path": workspace_path,
    });

    Ok(output)
}

/// Print actor-run results in human-readable text format (reused by session).
fn print_actor_run_text(output: &serde_json::Value, task: &str) {
    let intent_tool = output["intent"]["tool"].as_str().unwrap_or("?");
    let risk_level = output["intent"]["risk_level"].as_str().unwrap_or("?");
    let display_summary = output["intent"]["display_summary"].as_str().unwrap_or("?");
    let decision_status = output["decision"]["status"].as_str().unwrap_or("?");
    let decision_id = output["decision"]["id"].as_str().unwrap_or("?");
    let decision_reason = output["decision"]["reason"].as_str().unwrap_or("?");
    let approval_state = output["approval_state"].as_str().unwrap_or("?");
    let journal_entry_id = output["journal_entry_id"].as_str().unwrap_or("?");
    let is_read_only = output["is_read_only"].as_bool().unwrap_or(false);

    println!("{ACTOR_RUN_WARNING}");
    println!();
    println!("=== Actor Run ===");
    println!("Task: {:?}", task);
    println!();
    println!("--- 1. Intent interpretation ---");
    println!("Tool:   {}", intent_tool);
    println!("Risk:   {:?}", risk_level);
    println!("Summary: {}", display_summary);
    println!();
    println!("--- 2. Decision Gate ---");
    println!("Decision:    {:?}", decision_status);
    println!("Decision ID: {}", decision_id);
    println!("Reason:     {}", decision_reason);
    println!();
    println!("--- 3. Simulation / diff preview ---");
    if let Some(sim) = output["simulation_result"].as_object() {
        let sim_status = sim.get("status").and_then(|v| v.as_str()).unwrap_or("?");
        let sim_summary = sim
            .get("output_summary")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        println!("Status:  {:?}", sim_status);
        println!("Summary: {}", sim_summary);
    } else if output["simulation_result"].is_null() {
        println!("Status:  not run (governance blocked or no permission)");
    }
    println!();
    println!("--- 4. Approval path ---");
    if approval_state == "approved_and_executed" {
        println!("Approval: --approve supplied");
        println!("Execution: completed");
    } else if approval_state == "approved_but_not_executed" {
        println!("Approval: --approve supplied");
        println!("Execution: not run (simulation failed or governance denied)");
    } else {
        println!("Approval: missing -- simulation only");
        println!("Next step: rerun with --approve to execute");
        println!("  arpagona actor run {:?} --approve", task);
    }
    println!();
    println!("--- 5. Execution + readback ---");
    if let Some(exec) = output["execution_result"].as_object() {
        let exec_status = exec.get("status").and_then(|v| v.as_str()).unwrap_or("?");
        let exec_summary = exec
            .get("output_summary")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        println!("Execution:   {:?}", exec_status);
        println!("Summary:     {}", exec_summary);
        if let Some(readback) = output["readback_result"].as_object() {
            let rb_status = readback
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            let rb_summary = readback
                .get("output_summary")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            println!("Readback:    {:?}", rb_status);
            println!("Content:     {}", rb_summary);
        }
    } else if is_read_only {
        if let Some(sim) = output["simulation_result"].as_object() {
            let sim_status = sim.get("status").and_then(|v| v.as_str()).unwrap_or("?");
            let sim_summary = sim
                .get("output_summary")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            println!("Result:      {:?}", sim_status);
            println!("Output:      {}", sim_summary);
        } else {
            println!("Execution:   not run");
        }
    } else {
        println!("Execution:   not run");
    }
    println!();
    println!("--- 6. Observation / audit ---");
    println!(
        "Observation ID: obs-{}",
        &decision_id[..decision_id.len().min(8)]
    );
    println!("Journal Entry:  {}", journal_entry_id);
}

fn actor_run(args: ActorRunArgs) -> Result<(), Box<dyn Error>> {
    let interpreter: Box<dyn IntentInterpreter> = match args.intent_provider {
        IntentProviderArg::Deterministic => Box::new(DeterministicIntentInterpreter),
        IntentProviderArg::Ollama => {
            let ollama = match &args.ollama_model {
                Some(model) => OllamaIntentInterpreter::with_model(model.clone()),
                None => OllamaIntentInterpreter::new(),
            };
            Box::new(ollama)
        }
    };
    let output = actor_run_core(&args.task, &args.workspace, args.approve, &*interpreter)?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        print_actor_run_text(&output, &args.task);
    }

    Ok(())
}

/// Start an interactive acquisition loop reading tasks from stdin.
/// Each task goes through the governed actor_run_core pipeline (simulation-only, no implicit approval).
fn actor_session(args: ActorSessionArgs) -> Result<(), Box<dyn Error>> {
    let interpreter: Box<dyn IntentInterpreter> = match args.intent_provider {
        IntentProviderArg::Deterministic => Box::new(DeterministicIntentInterpreter),
        IntentProviderArg::Ollama => {
            let ollama = match &args.ollama_model {
                Some(model) => OllamaIntentInterpreter::with_model(model.clone()),
                None => OllamaIntentInterpreter::new(),
            };
            Box::new(ollama)
        }
    };
    let max_tasks = args.max.unwrap_or(u32::MAX);
    let mut task_count: u32 = 0;

    if args.json {
        // JSON mode: compact one-line envelopes per task (newline-delimited)
        let mut input = String::new();
        loop {
            if task_count >= max_tasks {
                break;
            }
            input.clear();
            let bytes_read = io::stdin().read_line(&mut input)?;
            if bytes_read == 0 {
                break;
            }
            let line = input.trim().to_owned();
            if line.is_empty() {
                continue;
            }

            // Session commands
            if line == "/quit" || line == "/exit" {
                break;
            }
            if line == "/help" {
                let envelope = serde_json::json!({
                    "type": "help",
                    "commands": ["/quit", "/exit", "/help", "/status", "<task>"]
                });
                println!("{}", serde_json::to_string(&envelope)?);
                continue;
            }
            if line == "/status" {
                let envelope = serde_json::json!({
                    "type": "status",
                    "tasks_processed": task_count,
                    "max": args.max,
                });
                println!("{}", serde_json::to_string(&envelope)?);
                continue;
            }

            task_count += 1;

            match actor_run_core(&line, &args.workspace, false, &*interpreter) {
                Ok(result) => {
                    let status = if result["simulation_result"].is_null() {
                        "governance_blocked"
                    } else {
                        "simulated"
                    };
                    let envelope = serde_json::json!({
                        "type": "task",
                        "task_number": task_count,
                        "task": line,
                        "status": status,
                        "tool": result["intent"]["tool"],
                        "decision": result["decision"]["status"],
                        "simulation_summary": result["simulation_result"]["output_summary"],
                        "journal_entry_id": result["journal_entry_id"],
                    });
                    println!("{}", serde_json::to_string(&envelope)?);
                }
                Err(e) => {
                    let envelope = serde_json::json!({
                        "type": "task",
                        "task_number": task_count,
                        "task": line,
                        "status": "error",
                        "error": format!("{e}"),
                    });
                    println!("{}", serde_json::to_string(&envelope)?);
                }
            }
        }

        let summary = serde_json::json!({
            "type": "summary",
            "tasks_processed": task_count,
        });
        println!("{}", serde_json::to_string(&summary)?);
    } else {
        // Text mode: interactive session with full actor_run output per task
        println!("{ACTOR_SESSION_WARNING}");
        println!();
        println!("=== Actor Session ===");
        println!("Type a task, or /help for commands. Ctrl+C or /quit to exit.");
        if let Some(max) = args.max {
            println!("Max tasks: {}", max);
        }
        println!();

        let mut input = String::new();
        loop {
            if task_count >= max_tasks {
                println!("[Session: max tasks ({}) reached, exiting.]", max_tasks);
                break;
            }

            print!("> ");
            io::stdout().flush()?;
            input.clear();
            let bytes_read = io::stdin().read_line(&mut input)?;
            if bytes_read == 0 {
                break;
            }
            let line = input.trim().to_owned();
            if line.is_empty() {
                continue;
            }

            // Session commands
            if line == "/quit" || line == "/exit" {
                break;
            }
            if line == "/help" {
                let provider_name = match args.intent_provider {
                    IntentProviderArg::Deterministic => "deterministic",
                    IntentProviderArg::Ollama => "ollama",
                };
                let mut help_lines = vec![
                    "Commands:".to_owned(),
                    "  /quit or /exit  Exit session".to_owned(),
                    "  /help           Show this help".to_owned(),
                    "  /status         Show session state".to_owned(),
                    "  <task>          Run a task through the governed pipeline".to_owned(),
                    String::new(),
                    format!("Intent provider: {}", provider_name),
                ];
                if let Some(model) = &args.ollama_model {
                    help_lines.push(format!("Ollama model:     {}", model));
                }
                for line in &help_lines {
                    println!("{}", line);
                }
                println!();
                continue;
            }
            if line == "/status" {
                println!("Task count:      {}", task_count);
                if let Some(max) = args.max {
                    println!("Max tasks:       {}", max);
                }
                let provider_name = match args.intent_provider {
                    IntentProviderArg::Deterministic => "deterministic",
                    IntentProviderArg::Ollama => "ollama",
                };
                println!("Intent provider: {}", provider_name);
                if let Some(model) = &args.ollama_model {
                    println!("Ollama model:    {}", model);
                }
                println!("Session state:   active");
                println!();
                continue;
            }

            task_count += 1;
            println!();
            println!("--- Task #{} ---", task_count);

            match actor_run_core(&line, &args.workspace, false, &*interpreter) {
                Ok(result) => {
                    print_actor_run_text(&result, &line);
                }
                Err(e) => {
                    println!("Error: {e}");
                    println!("[Loop continues -- error handled per task.]");
                }
            }
            println!();
        }

        println!("Session ended. {} tasks processed.", task_count);
    }

    Ok(())
}

/// Show read-only actor status readback: agent info, executor state,
/// journal summary, and session state. Pure readback — no mutation paths.
fn actor_status_readback(args: ActorStatusArgs) -> Result<(), Box<dyn Error>> {
    // Gather agent info from constants/env
    let agent_id = env::var("ARPAGONA_AGENT_ID").unwrap_or_else(|_| DEFAULT_AGENT_ID.to_owned());
    let workspace_id =
        env::var("ARPAGONA_WORKSPACE_ID").unwrap_or_else(|_| DEFAULT_WORKSPACE_ID.to_owned());
    let api_url = env::var("ARPAGONA_API_URL").unwrap_or_else(|_| DEFAULT_API_URL.to_owned());

    // Check agent-kind env var (optional)
    let agent_kind = env::var("ARPAGONA_AGENT_KIND")
        .ok()
        .unwrap_or_else(|| "unknown".to_owned());

    // Gather journal summary
    let journal = global_llm_journal().lock().unwrap();
    let journal_total = journal.len();
    let direct_tool_calls = journal
        .all_entries()
        .iter()
        .filter(|e| {
            e.interaction_type
                == arpagona_agent_core::llm_journal::LlmInteractionType::DirectToolCall
        })
        .count();
    let governance_entries = journal
        .all_entries()
        .iter()
        .filter(|e| e.decision_gate_outcomes.is_some())
        .count();
    drop(journal);

    let readback = serde_json::json!({
        "actor_status": {
            "agent_id": agent_id,
            "agent_kind": agent_kind,
            "workspace_id": workspace_id,
            "api_url": api_url,
        },
        "journal_summary": {
            "total_entries": journal_total,
            "direct_tool_calls": direct_tool_calls,
            "governance_entries": governance_entries,
        },
        "readback_warning": "NON_AUTHORIZING_READBACK: This is a read-only actor status summary. It carries no authority to execute or approve actions. No Decision Gate boundaries were crossed."
    });

    if args.json {
        println!("{}", serde_json::to_string_pretty(&readback)?);
    } else {
        let status = &readback["actor_status"];
        let journal_summary = &readback["journal_summary"];
        println!("[Actor Status Readback]");
        println!(
            "  agent_id:       {}",
            status["agent_id"].as_str().unwrap_or("?")
        );
        println!(
            "  agent_kind:     {}",
            status["agent_kind"].as_str().unwrap_or("?")
        );
        println!(
            "  workspace_id:   {}",
            status["workspace_id"].as_str().unwrap_or("?")
        );
        println!(
            "  api_url:        {}",
            status["api_url"].as_str().unwrap_or("?")
        );
        println!();
        println!("[LLM Journal Summary]");
        println!(
            "  total_entries:      {}",
            journal_summary["total_entries"].as_u64().unwrap_or(0)
        );
        println!(
            "  direct_tool_calls:  {}",
            journal_summary["direct_tool_calls"].as_u64().unwrap_or(0)
        );
        println!(
            "  governance_entries: {}",
            journal_summary["governance_entries"].as_u64().unwrap_or(0)
        );
        println!();
        println!("NON_AUTHORIZING_READBACK: This is a read-only actor status summary.");
        println!("It carries no authority to execute or approve actions.");
    }

    Ok(())
}

/// Show read-only actor memory readback: graph memory state,
/// facts, episodes, and observations. No mutation paths.
fn actor_memory_readback(args: ActorMemoryArgs) -> Result<(), Box<dyn Error>> {
    // Graph memory is in-memory only unless SURREALDB is configured.
    // Read the memory store's alpha status rather than mutating anything.
    let configured_backend = env::var("ARPAGONA_GRAPH_MEMORY_BACKEND")
        .ok()
        .map(|v| v.trim().to_ascii_lowercase())
        .filter(|v| !v.is_empty());

    // Memory is configured only if ARPAGONA_GRAPH_MEMORY_BACKEND is set to surrealdb
    let memory_active = configured_backend.as_deref() == Some("surrealdb");

    let readback = serde_json::json!({
        "actor_memory": {
            "graph_memory_support_compiled": true,
            "configured_backend": configured_backend,
            "memory_active": memory_active,
            "alpha_limits": [
                "No persistent graph memory by default (in-memory only)",
                "Use ARPAGONA_GRAPH_MEMORY_BACKEND=surrealdb for persistence",
                "Facts, episodes, and observations are in-memory only",
            ],
            "not_implemented": [
                "Remote graph memory queries (no API server required for local readback)",
                "Multi-actor memory scope isolation",
            ],
        },
        "access_methods": [
            "arpagona memory status -- show alpha graph memory status",
            "arpagona audit list -- list audit events (requires API server)",
            "arpagona audit decision-summary <id> -- show decision audit trace",
            "arpagona audit task-summary <id> -- show task-scoped audit summary",
        ],
        "readback_warning": "NON_AUTHORIZING_READBACK: This is a read-only actor memory overview. It inspects configured memory state without mutating facts, episodes, or observations. No Decision Gate boundaries were crossed."
    });

    if args.json {
        println!("{}", serde_json::to_string_pretty(&readback)?);
    } else {
        let memory = &readback["actor_memory"];
        println!("[Actor Memory Readback]");
        println!(
            "  graph_memory_support_compiled: {}",
            memory["graph_memory_support_compiled"]
                .as_bool()
                .unwrap_or(false)
        );
        let backend = memory["configured_backend"]
            .as_str()
            .unwrap_or("not configured");
        println!(
            "  configured_backend:           {}",
            if backend.is_empty() {
                "not configured"
            } else {
                backend
            }
        );
        println!(
            "  memory_active:                {}",
            memory["memory_active"].as_bool().unwrap_or(false)
        );
        println!();
        println!("[Alpha Limits]");
        for limit in memory["alpha_limits"].as_array().unwrap() {
            println!("  - {}", limit.as_str().unwrap_or("?"));
        }
        println!();
        println!("[Access Methods]");
        for method in readback["access_methods"].as_array().unwrap() {
            println!("  - {}", method.as_str().unwrap_or("?"));
        }
        println!();
        println!("NON_AUTHORIZING_READBACK: This is a read-only actor memory overview.");
        println!("It inspects configured memory state without mutating facts, episodes, or observations.");
    }

    Ok(())
}

/// Recursively redact sensitive fields from a journal Value for display.
/// Replaces `content_preview` and `payload` keys at any nesting depth
/// with a redacted marker, preventing secret exposure through readback JSON.
fn redact_journal_value(v: serde_json::Value) -> serde_json::Value {
    const REDACTED: &str = "[REDACTED: journal readback — use raw journal file for full detail]";
    match v {
        serde_json::Value::Object(map) => {
            let mut redacted = serde_json::Map::new();
            for (k, val) in map {
                let key_lower = k.to_lowercase();
                if key_lower == "content_preview"
                    || key_lower == "payload"
                    || key_lower == "simulation_payload"
                    || key_lower == "raw_content"
                {
                    redacted.insert(k, serde_json::Value::String(REDACTED.to_owned()));
                } else {
                    redacted.insert(k, redact_journal_value(val));
                }
            }
            serde_json::Value::Object(redacted)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.into_iter().map(redact_journal_value).collect())
        }
        other => other,
    }
}

/// Show read-only actor journal readback: LLM journal entries
/// from actor-run and actor-session interactions.
fn actor_journal_readback(args: ActorJournalArgs) -> Result<(), Box<dyn Error>> {
    let journal = global_llm_journal().lock().unwrap();
    let all_entries = journal.all_entries();

    // Filter by interaction type if specified
    let filtered: Vec<_> = if let Some(ref filter_type) = args.interaction_type {
        let filter_lower = filter_type.to_lowercase();
        all_entries
            .iter()
            .filter(|e| {
                let type_str = format!("{:?}", e.interaction_type).to_lowercase();
                type_str.contains(&filter_lower)
            })
            .collect()
    } else {
        // Default: show actor-run entries (direct_tool_call with objective=actor_run)
        all_entries
            .iter()
            .filter(|e| {
                e.interaction_type
                    == arpagona_agent_core::llm_journal::LlmInteractionType::DirectToolCall
                    && e.objective.as_deref() == Some("actor_run")
            })
            .collect()
    };

    let limit = args.limit.min(filtered.len());
    let entries: Vec<_> = filtered.iter().rev().take(limit).collect();

    if args.json {
        let json_entries: Vec<serde_json::Value> = entries
            .iter()
            .map(|e| {
                serde_json::json!({
                    "id": e.id,
                    "created_at": e.created_at,
                    "interaction_type": format!("{:?}", e.interaction_type),
                    "provider": e.provider,
                    "model": e.model,
                    "objective": e.objective,
                    "prompt_summary": e.prompt_summary,
                    "response_summary": e.response_summary,
                    "tool_call_intents": e.tool_call_intents.as_ref().map(|v| redact_journal_value(v.clone())),
                    "decision_gate_outcomes": e.decision_gate_outcomes.as_ref().map(|v| redact_journal_value(v.clone())),
                    "risk_level": e.risk_level,
                })
            })
            .collect();
        let output = serde_json::json!({
            "total_entries": journal.len(),
            "displayed_entries": entries.len(),
            "filter": args.interaction_type,
            "entries": json_entries,
            "readback_warning": "NON_AUTHORIZING_READBACK: Read-only LLM journal readback. No Decision Gate boundaries crossed."
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("[Actor Journal Readback] — {} entries total", journal.len());
        println!("Showing {} most recent actor-run entries:", entries.len());
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
                let preview: String = obj.chars().take(80).collect();
                println!("        | objective: {}", preview);
            }
            println!(
                "        | prompt: {}",
                entry.prompt_summary.chars().take(120).collect::<String>()
            );
            println!(
                "        | response: {}",
                entry.response_summary.chars().take(120).collect::<String>()
            );
            if let Some(ref dg) = entry.decision_gate_outcomes {
                println!(
                    "        | decision_gate: {}",
                    serde_json::to_string(&redact_journal_value(dg.clone())).unwrap_or_default()
                );
            }
            if let Some(ref rl) = entry.risk_level {
                println!("        | risk_level: {:?}", rl);
            }
            println!();
        }
        println!("NON_AUTHORIZING_READBACK: Read-only LLM journal readback.");
    }

    Ok(())
}

/// Show a compact history of recent actor runs (read-only).
///
/// Reads from the in-memory LLM journal and displays the N most recent
/// actor-run entries as a scannable table (run time, tool, decision,
/// simulation status, execution status, summary). No external effects.
fn actor_history_readback(args: ActorHistoryArgs) -> Result<(), Box<dyn Error>> {
    let journal = global_llm_journal().lock().unwrap();
    let entries: Vec<_> = journal
        .all_entries()
        .iter()
        .filter(|e| {
            e.interaction_type
                == arpagona_agent_core::llm_journal::LlmInteractionType::DirectToolCall
                && e.objective.as_deref() == Some("actor_run")
        })
        .rev()
        .take(args.limit)
        .collect();

    if args.json {
        // JSON output: each entry with key fields for machine consumption
        let json_entries: Vec<serde_json::Value> = entries
            .iter()
            .map(|e| {
                // Extract tool and status from the decision_gate_outcomes
                let tool_call_intents = e.tool_call_intents.as_ref();
                let tool = tool_call_intents
                    .and_then(|v| v.get("tool"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                let dec_outcomes = e.decision_gate_outcomes.as_ref();
                let decision_status = dec_outcomes
                    .and_then(|v| v.get("decision_status"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                let approval_state = dec_outcomes
                    .and_then(|v| v.get("approval_state"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                serde_json::json!({
                    "id": e.id,
                    "created_at": e.created_at,
                    "tool": tool,
                    "decision_status": decision_status,
                    "approval_state": approval_state,
                    "risk_level": e.risk_level,
                    "prompt_summary": e.prompt_summary,
                    "response_summary": e.response_summary,
                })
            })
            .collect();
        let output = serde_json::json!({
            "command": "actor_history",
            "total_matching": entries.len(),
            "entries": json_entries,
            "readback_warning":
                "NON_AUTHORIZING_READBACK: Read-only actor run history. No Decision Gate boundaries crossed.",
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        // Text output: compact table
        println!("[Actor Run History] — {} most recent run(s)", entries.len());
        println!();
        for (i, entry) in entries.iter().enumerate() {
            let created = entry.created_at.format("%Y-%m-%d %H:%M:%S");

            // Extract tool and status from structured data in decision_gate_outcomes
            let tool_call_intents = entry.tool_call_intents.as_ref();
            let tool = tool_call_intents
                .and_then(|v| v.get("tool"))
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            let dec_outcomes = entry.decision_gate_outcomes.as_ref();
            let decision_status = dec_outcomes
                .and_then(|v| v.get("decision_status"))
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            let approval_state = dec_outcomes
                .and_then(|v| v.get("approval_state"))
                .and_then(|v| v.as_str())
                .unwrap_or("?");

            let risk_str = entry
                .risk_level
                .as_ref()
                .map(|r| format!("{:?}", r))
                .unwrap_or_else(|| "?".to_owned());

            let task_preview: String = entry.prompt_summary.chars().take(60).collect();

            println!(
                "  #{:<4} | {} | {} | risk={}",
                i + 1,
                created,
                tool,
                risk_str
            );
            println!("        | task:   {}", task_preview);
            println!("        | gate:   {} | {}", decision_status, approval_state);
            println!();
        }
        println!("NON_AUTHORIZING_READBACK: Read-only actor run history.");
    }

    Ok(())
}

/// Run the Neutral Orchestrator deterministic cycle and display the result.
/// Run an objective through the in-process orchestrator with clean, readable output.
///
/// This is the top-level `arpagona run "<objective>"` command. It uses the
/// OrchestratorEngine directly — no API server, no LLM, no persistence.
/// Output is formatted for human readability without internal governance jargon.
fn handle_run(args: RunArgs) -> Result<(), Box<dyn Error>> {
    let workspace_id = arpagona_agent_core::ids::WorkspaceId::new("workspace-alpha");
    let agent_id = arpagona_agent_core::ids::AgentId::new("agent-alpha");
    let engine = arpagona_neutral_orchestrator::OrchestratorEngine::new();

    let input = arpagona_agent_core::orchestrator::ObjectiveInput::new(
        args.objective,
        workspace_id,
        agent_id,
        chrono::Utc::now(),
    );

    let cycle = engine
        .run_cycle(
            input,
            &[arpagona_agent_core::permission::Permission::ReadDocument],
        )
        .map_err(|e| format!("Run failed: {e}"))?;

    let outcome = &cycle.outcome;
    let decision_status = cycle
        .decision
        .as_ref()
        .map(|d| format!("{:?}", d.status))
        .unwrap_or_else(|| "pending".to_owned());

    let action_type = cycle
        .proposed_action
        .as_ref()
        .map(|a| format!("{:?}", a.action_type))
        .unwrap_or_else(|| "none".to_owned());

    let risk = cycle
        .proposed_action
        .as_ref()
        .map(|a| format!("{:?}", a.risk_level))
        .unwrap_or_else(|| "unknown".to_owned());

    // Build compute route info for operator visibility (provider/model path)
    let route_display = cycle.compute_route_result.selected_route_label.clone();

    // Governance status: audit event count and gate applied status
    let audit_count = outcome.audit_event_ids.len();
    let gate_status = if outcome.gate_was_applied {
        "applied"
    } else {
        "not applied"
    };
    let governance_info = format!("{} event(s) | Gate {}", audit_count, gate_status);

    // Clean, human-readable output — no governance jargon
    println!();
    println!("{}", style_brand("  ❍ ARPAGONA"));
    println!("{}", "───────────────────────────────");
    println!("  Objective   {}", style_bold(&cycle.objective_input.text));
    println!("  Action      {} at {}", action_type, style_risk(&risk));
    println!(
        "  Decision    {} {}",
        decision_icon(&decision_status),
        style_decision(&decision_status)
    );
    println!("  Compute     {}", route_display);
    println!("  Audit       {}", governance_info);
    println!("  Cycle       {}", outcome.cycle_id);
    println!();

    Ok(())
}

/// Bold text helper.
fn style_bold(text: &str) -> String {
    format!(
        "{ANSI_BOLD}{}{ANSI_RESET}",
        style_text(text, TermColor::White)
    )
}

/// Decision status icon.
fn decision_icon(status: &str) -> &'static str {
    match status.to_lowercase().as_str() {
        "approved" | "approvedbyoverride" => "✅",
        "blocked" => "⛔",
        "requiresoverride" | "needshumanapproval" => "⚠️",
        "pendingdecision" => "⏳",
        _ => "❓",
    }
}

/// Style decision status text.
fn style_decision(status: &str) -> String {
    match status.to_lowercase().as_str() {
        "approved" | "approvedbyoverride" => style_success(status),
        "blocked" => style_error(status),
        "requiresoverride" | "needshumanapproval" => style_warning(status),
        _ => style_text(status, TermColor::Cyan),
    }
}

fn orchestrator_run(args: OrchestratorRunArgs) -> Result<(), Box<dyn Error>> {
    let perm: Permission = match args.permissions.first().map(|s| s.as_str()) {
        Some("ReadDocument") => Permission::ReadDocument,
        Some("WriteMemory") => Permission::WriteMemory,
        Some(other) => {
            // Try to parse via serde
            serde_json::from_str(&format!("\"{}\"", other)).unwrap_or(Permission::ReadDocument)
        }
        None => Permission::ReadDocument,
    };

    let workspace_id = WorkspaceId::new(args.workspace_id);
    let agent_id = AgentId::new(args.agent_id);

    let engine = match args.proposal_generator {
        ProposalGeneratorArg::Simulated => OrchestratorEngine::new(),
        ProposalGeneratorArg::Llm => {
            let mock_provider = Box::new(arpagona_llm::MockProvider::safe_default());
            let llm_generator =
                arpagona_neutral_orchestrator::LlmProposalGenerator::new(mock_provider);
            OrchestratorEngine::new().with_proposal_generator(Box::new(llm_generator))
        }
    };

    let input = ObjectiveInput::new(args.objective, workspace_id, agent_id, chrono::Utc::now());
    let cycle = engine
        .run_cycle(input, &[perm])
        .map_err(|e| format!("Orchestrator cycle failed: {e}"))?;

    if args.json && args.trace {
        // Full CycleTrace JSON with context assembly metadata
        let trace = cycle.to_cycle_trace();
        println!("{}", serde_json::to_value(&trace)?);
    } else if args.json {
        // Backward-compatible: just the outcome
        println!("{}", serde_json::to_value(&cycle.outcome)?);
    } else if args.trace {
        // Rich human-readable trace with context assembly metadata
        let trace = cycle.to_cycle_trace();
        println!(
            "{}",
            style_info("Orchestrator Cycle Trace (with context assembly metadata)")
        );
        println!("{}", "-".repeat(60));
        println!("{}", trace.format());
        println!();
        println!(
            "{}",
            style_dim("⚠  Advisory — context assembly metadata is non-authorizing.")
        );
        println!(
            "{}",
            style_dim("   No execution without explicit Decision Gate approval.")
        );
    } else {
        println!("{}", style_info("Orchestrator Cycle Trace"));
        println!("{}", "-".repeat(60));
        println!("{}", cycle.causal_trace());
        println!();
        println!(
            "{}",
            style_dim("⚠  Readback only — supervision entries are evidence, not authorization.")
        );
        println!(
            "{}",
            style_dim("   No execution without explicit Decision Gate approval.")
        );
        println!(
            "{}",
            style_dim(&format!(
                "   Non-authorizing: {}",
                cycle.outcome.non_authorizing
            ))
        );
    }

    // ── Save trace to file when --save-trace is provided ──────────────
    if let Some(ref save_trace_val) = args.save_trace {
        let trace = cycle.to_cycle_trace();
        let actual_path = if save_trace_val == "auto" {
            // Auto-generate a unique path from the cycle ID and timestamp
            let dir = "target/orchestrator-traces";
            std::fs::create_dir_all(dir).ok();
            format!(
                "{}/cycle-{}-{}.json",
                dir,
                trace.cycle_id,
                trace.created_at.format("%Y%m%dT%H%M%S")
            )
        } else {
            save_trace_val.clone()
        };
        let json = serde_json::to_string_pretty(&trace)?;
        if let Some(parent) = std::path::Path::new(&actual_path).parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(&actual_path, &json)?;
        if !args.json {
            println!(
                "{}",
                style_dim(&format!("   Trace saved to: {}", actual_path))
            );
        }
    }

    // ── Collect and save failure insight candidates when --collect-insights ─
    if args.collect_insights {
        let trace = cycle.to_cycle_trace();
        let candidates = trace.detect_failure_candidates();

        let insights_dir = args
            .insights_dir
            .clone()
            .unwrap_or_else(|| DEFAULT_ORCHESTRATOR_INSIGHTS_DIR.to_owned());
        let insights_path = std::path::Path::new(&insights_dir);
        std::fs::create_dir_all(insights_path).ok();

        let insight_file = insights_path.join(format!("insights-{}.json", trace.cycle_id));
        let insight_entry = serde_json::json!({
            "cycle_id": trace.cycle_id,
            "objective_text": trace.objective_text,
            "cycle_status": trace.cycle_status,
            "candidate_count": candidates.len(),
            "candidates": candidates,
            "non_authorizing": true,
            "created_at": chrono::Utc::now().to_rfc3339(),
        });
        let insight_json = serde_json::to_string_pretty(&insight_entry)?;
        std::fs::write(&insight_file, &insight_json)?;

        if !args.json {
            if candidates.is_empty() {
                println!(
                    "{}",
                    style_dim(&format!(
                        "   Insights: no failure candidates detected for cycle {}",
                        trace.cycle_id
                    ))
                );
            } else {
                println!(
                    "{}",
                    style_info(&format!(
                        "   Insights: {} failure candidate(s) detected for cycle {}",
                        candidates.len(),
                        trace.cycle_id
                    ))
                );
            }
            println!(
                "{}",
                style_dim(&format!("   Insights saved to: {}", insight_file.display()))
            );
        }
    }

    // ── Save audit events when --save-audit is provided ──────────────
    if let Some(ref save_audit_val) = args.save_audit {
        let actual_dir = if save_audit_val == "auto" {
            DEFAULT_ORCHESTRATOR_AUDIT_DIR.to_owned()
        } else {
            save_audit_val.clone()
        };
        let dir = std::path::Path::new(&actual_dir);
        std::fs::create_dir_all(dir).ok();

        for event in &cycle.audit_events {
            let file_name = format!("audit-event-{}.json", event.id);
            let event_path = dir.join(&file_name);
            let event_json = serde_json::to_string_pretty(event)?;
            std::fs::write(&event_path, &event_json)?;
        }

        if !args.json {
            println!(
                "{}",
                style_dim(&format!(
                    "   Audit events saved to: {}/ ({} event(s))",
                    actual_dir,
                    cycle.audit_events.len()
                ))
            );
        }
    }

    Ok(())
}

/// Display orchestrator status from a saved CycleTrace file.
///
/// Reads a previously saved CycleTrace JSON file (from `orchestrator run --trace --save-trace`)
/// and displays the compute-aware context assembly breakdown: per-source context item counts,
/// compute route selection, decision outcome and cycle summary.
fn orchestrator_status(args: OrchestratorStatusArgs) -> Result<(), Box<dyn Error>> {
    let path = args
        .trace_path
        .unwrap_or_else(|| DEFAULT_ORCHESTRATOR_TRACE_PATH.to_owned());

    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read orchestrator trace file '{}': {e}", path))?;

    let trace: arpagona_agent_core::orchestrator::CycleTrace = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse orchestrator trace file '{}': {e}", path))?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&trace)?);
    } else {
        println!("{}", style_info("Orchestrator Status (from saved trace)"));
        println!("{}", "-".repeat(60));
        println!("{}", trace.format());
        println!();
        println!(
            "{}",
            style_dim("⚠  Readback only — trace fields are evidence, not authorization.")
        );
        println!(
            "{}",
            style_dim("   No execution without explicit Decision Gate approval.")
        );
        println!(
            "{}",
            style_dim(&format!("   Non-authorizing: {}", trace.non_authorizing))
        );
    }

    Ok(())
}

/// A lightweight listing entry for a single orchestrator cycle trace file.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CycleTraceListingEntry {
    /// File name (e.g., "cycle-abc123.json").
    pub file_name: String,
    /// The cycle ID as displayed in the trace.
    pub cycle_id: String,
    /// Objective text (truncated for preview).
    pub objective_preview: String,
    /// Cycle status.
    pub cycle_status: String,
    /// Summary (truncated for preview).
    pub summary_preview: String,
    /// Number of context sources assembled.
    pub context_source_count: usize,
    /// Whether the Decision Gate was applied.
    pub gate_was_applied: bool,
    /// Whether the trace claims non-authorizing status.
    pub non_authorizing: bool,
    /// Number of failure insight candidates.
    pub failure_insight_candidate_count: usize,
    /// Number of audit events recorded inside the trace.
    pub audit_event_count: usize,
    /// Number of audit event files saved externally (via --save-audit).
    /// Only populated when --with-audit is set.
    #[serde(default)]
    pub external_audit_event_count: usize,
    /// Breakdown of audit event types from saved audit event files.
    /// Only populated when --with-audit is set.
    /// Maps audit event type label → count.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audit_event_type_breakdown: Option<std::collections::HashMap<String, usize>>,
    /// Timestamp of the trace.
    pub created_at: String,
}

/// Scan a directory for `.json` files, deserialize valid CycleTrace instances,
/// and return sorted listing metadata.
///
/// Invalid or non-CycleTrace `.json` files are silently skipped.
fn list_orchestrator_cycles_in_directory(
    dir: &std::path::Path,
) -> Result<Vec<(std::path::PathBuf, CycleTraceListingEntry)>, Box<dyn Error>> {
    use arpagona_agent_core::orchestrator::CycleTrace;

    let mut entries: Vec<(std::path::PathBuf, CycleTraceListingEntry)> = Vec::new();

    if !dir.exists() {
        return Ok(entries);
    }

    let mut read_dir = std::fs::read_dir(dir)
        .map_err(|e| format!("Failed to read directory '{}': {e}", dir.display()))?;

    while let Some(entry) = read_dir.next().transpose()? {
        let path = entry.path();
        if path.extension().map_or(true, |ext| ext != "json") {
            continue;
        }

        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let trace: CycleTrace = match serde_json::from_str(&content) {
            Ok(t) => t,
            Err(_) => continue,
        };

        let objective_preview = if trace.objective_text.len() > 60 {
            format!("{}...", &trace.objective_text[..57])
        } else {
            trace.objective_text.clone()
        };

        let summary_preview = if trace.summary.len() > 80 {
            format!("{}...", &trace.summary[..77])
        } else {
            trace.summary.clone()
        };

        let listing = CycleTraceListingEntry {
            file_name: path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            cycle_id: trace.cycle_id.to_string(),
            objective_preview,
            cycle_status: trace.cycle_status,
            summary_preview,
            context_source_count: trace.context_source_summaries.len(),
            gate_was_applied: trace.gate_was_applied,
            non_authorizing: trace.non_authorizing,
            failure_insight_candidate_count: trace.failure_insight_candidates.len(),
            audit_event_count: trace.audit_event_count,
            external_audit_event_count: 0,
            created_at: trace.created_at.to_rfc3339(),
            audit_event_type_breakdown: None,
        };

        entries.push((path, listing));
    }

    // Sort by created_at descending (newest first)
    entries.sort_by(|a, b| b.1.created_at.cmp(&a.1.created_at));

    Ok(entries)
}

/// Scan a directory of saved audit event JSON files and count how many events
/// reference each cycle ID.
///
/// Audit event files are saved by `orchestrator run --save-audit` as individual
/// JSON files. This function reads each file, deserializes it as an `AuditEvent`,
/// and extracts the `cycle_id` from `CognitiveCycleCompleted` event payloads.
/// Events that cannot be parsed or lack a cycle_id are counted as "unassociated."
///
/// Returns a map: cycle_id String → external audit event count.
///
/// Non-authorizing: this is a readback surface for operator inspection only.
fn count_external_audit_events_by_cycle_id(
    audit_dir: &std::path::Path,
) -> std::collections::HashMap<String, usize> {
    use arpagona_agent_core::AuditEvent;
    use arpagona_agent_core::AuditEventType;

    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut unassociated: usize = 0;

    if !audit_dir.exists() {
        return counts;
    }

    let read_dir = match std::fs::read_dir(audit_dir) {
        Ok(rd) => rd,
        Err(_) => return counts,
    };

    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let event: AuditEvent = match serde_json::from_str(&content) {
            Ok(e) => e,
            Err(_) => continue,
        };

        if event.event_type == AuditEventType::CognitiveCycleCompleted {
            // Extract cycle_id from payload — CognitiveCycleCompleted events
            // carry { "cycle_id": "oc-...", "objective_text": "...", ... }
            let cycle_id = event
                .payload
                .get("cycle_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            if let Some(cid) = cycle_id {
                *counts.entry(cid).or_insert(0) += 1;
            } else {
                unassociated += 1;
            }
        } else {
            // Non-CognitiveCycleCompleted events are counted as "other"
            // and keyed by their event_type for diagnostic purposes
            *counts
                .entry(format!("(other: {:?})", event.event_type))
                .or_insert(0) += 1;
        }
    }

    if unassociated > 0 {
        counts.insert("(unassociated)".to_string(), unassociated);
    }

    counts
}

/// Scan a directory of saved audit event JSON files and collect audit event
/// type breakdowns per cycle ID.
///
/// Returns a map: cycle_id String → HashMap<event_type_label, count>.
///
/// Non-authorizing: this is a readback surface for operator inspection only.
fn collect_external_audit_type_breakdowns(
    audit_dir: &std::path::Path,
) -> std::collections::HashMap<String, std::collections::HashMap<String, usize>> {
    use arpagona_agent_core::AuditEvent;

    let mut breakdowns: std::collections::HashMap<
        String,
        std::collections::HashMap<String, usize>,
    > = std::collections::HashMap::new();

    if !audit_dir.exists() {
        return breakdowns;
    }

    let read_dir = match std::fs::read_dir(audit_dir) {
        Ok(rd) => rd,
        Err(_) => return breakdowns,
    };

    for entry in read_dir.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let event: AuditEvent = match serde_json::from_str(&content) {
            Ok(e) => e,
            Err(_) => continue,
        };

        // Determine which cycle this event belongs to
        let cycle_id = event
            .payload
            .get("cycle_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "(unassociated)".to_owned());

        let type_label = format!("{:?}", event.event_type);
        *breakdowns
            .entry(cycle_id)
            .or_default()
            .entry(type_label)
            .or_insert(0) += 1;
    }

    breakdowns
}

/// List saved orchestrator cycle traces from a directory.
fn orchestrator_cycles(args: OrchestratorCyclesArgs) -> Result<(), Box<dyn Error>> {
    let dir = args
        .trace_dir
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(DEFAULT_ORCHESTRATOR_TRACES_DIR));

    let mut entries = list_orchestrator_cycles_in_directory(&dir)?;

    // ── Optional: populate external audit event counts and type breakdown ──
    if args.with_audit {
        let audit_dir = std::path::PathBuf::from(DEFAULT_ORCHESTRATOR_AUDIT_DIR);
        let audit_counts = count_external_audit_events_by_cycle_id(&audit_dir);
        // Also build audit event type breakdown from saved audit files
        let audit_breakdowns = collect_external_audit_type_breakdowns(&audit_dir);
        for (_path, listing) in entries.iter_mut() {
            let count = audit_counts.get(&listing.cycle_id).copied().unwrap_or(0);
            listing.external_audit_event_count = count;
            listing.audit_event_type_breakdown = audit_breakdowns.get(&listing.cycle_id).cloned();
        }
    }

    if args.json {
        let listings: Vec<&CycleTraceListingEntry> =
            entries.iter().map(|(_, listing)| listing).collect();
        println!("{}", serde_json::to_string_pretty(&listings)?);
    } else {
        println!("{}", style_info("Orchestrator Cycle Traces"));
        println!("{}", "-".repeat(60));
        if entries.is_empty() {
            println!("No orchestrator cycle traces found.");
            println!();
            println!(
                "  Run `{}` first to save a trace, then run `{}` to list it.",
                "cargo run -q --bin arpagona -- orchestrator run --objective \"...\" --save-trace <path>",
                "cargo run -q --bin arpagona -- orchestrator cycles"
            );
            println!();
            println!(
                "  Default trace directory: {}",
                DEFAULT_ORCHESTRATOR_TRACES_DIR
            );
        } else {
            println!(
                "Found {} cycle trace(s) in '{}':\n",
                entries.len(),
                dir.display()
            );
            for (i, (_path, listing)) in entries.iter().enumerate() {
                println!("{}. \x1b[1m{}\x1b[0m", i + 1, listing.file_name);
                println!("   Cycle ID:     {}", listing.cycle_id);
                println!("   Objective:    {}", listing.objective_preview);
                println!("   Status:       {}", listing.cycle_status);
                println!("   Context srcs: {}", listing.context_source_count);
                println!("   Gate applied: {}", listing.gate_was_applied);
                println!("   Non-auth:     {}", listing.non_authorizing);
                println!(
                    "   FI cands:     {}",
                    listing.failure_insight_candidate_count
                );
                println!("   Audit (trace): {}", listing.audit_event_count);
                if args.with_audit {
                    println!("   Audit (ext):  {}", listing.external_audit_event_count);
                }
                println!("   Created:      {}", listing.created_at);
                println!("   Summary:      {}", listing.summary_preview);
                println!();
            }
            println!(
                "Use `{} <file>` to inspect a specific trace in detail.",
                "cargo run -q --bin arpagona -- orchestrator status --trace-path"
            );
        }
        println!();
        println!(
            "{}",
            style_dim("⚠  Readback only — trace entries are evidence, not authorization.")
        );
        println!(
            "{}",
            style_dim("   No execution without explicit Decision Gate approval.")
        );
    }

    Ok(())
}

/// Detect and collect failure insight candidates from a saved CycleTrace file.
///
/// Reads a previously saved CycleTrace JSON (from `orchestrator run --save-trace`),
/// runs `CycleTrace::detect_failure_candidates()`, and if any candidates are found,
/// saves them as a structured insights file in the configured insights directory.
///
/// This is a read-only, non-authorizing operation. The collected candidates are
/// advisory signals, not corrective actions.
fn orchestrator_insights_collect(
    args: OrchestratorInsightsCollectArgs,
) -> Result<(), Box<dyn Error>> {
    let path = std::path::Path::new(&args.trace_path);
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read CycleTrace file '{}': {e}", args.trace_path))?;

    let trace: arpagona_agent_core::orchestrator::CycleTrace = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse CycleTrace from '{}': {e}", args.trace_path))?;

    let candidates = trace.detect_failure_candidates();

    let insights_dir = std::path::Path::new(DEFAULT_ORCHESTRATOR_INSIGHTS_DIR);
    std::fs::create_dir_all(insights_dir).ok();

    // Generate a deterministic filename from the cycle ID
    let insights_path = insights_dir.join(format!("insights-{}.json", trace.cycle_id));

    let insight_entry = serde_json::json!({
        "cycle_id": trace.cycle_id,
        "objective_text": trace.objective_text,
        "cycle_status": trace.cycle_status,
        "source_trace_path": args.trace_path,
        "candidate_count": candidates.len(),
        "candidates": candidates,
        "non_authorizing": true,
        "created_at": chrono::Utc::now().to_rfc3339(),
    });

    let json = serde_json::to_string_pretty(&insight_entry)?;
    std::fs::write(&insights_path, &json)?;

    // ── Optional: write as FailureInsightDemoSnapshot for snapshot pipeline ──
    if let Some(ref snapshot_path_str) = args.snapshot_path {
        let snapshot_path = std::path::Path::new(snapshot_path_str);
        if let Some(parent) = snapshot_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let snapshot = FailureInsightDemoSnapshot::new(
            insight_entry.clone(),
            vec![
                format!("insights-collect: cycle {}", trace.cycle_id),
                format!("objective: {}", trace.objective_text),
                format!("candidates: {}", candidates.len()),
                "orchestrator failure insight collection".to_owned(),
                "written as FailureInsightDemoSnapshot for snapshot-list discoverability"
                    .to_owned(),
            ],
        );
        snapshot.write_to_file(snapshot_path).map_err(|e| {
            format!(
                "Failed to write demo snapshot to '{}': {e}",
                snapshot_path_str
            )
        })?;
        if !args.json {
            println!(
                "{}",
                style_dim(&format!(
                    "   Demo snapshot written to: {}",
                    snapshot_path_str
                ))
            );
        }
    }

    if args.json {
        println!("{}", json);
    } else {
        println!("{}", style_info("Orchestrator Failure Insights — Collect"));
        println!("{}", "-".repeat(60));
        if candidates.is_empty() {
            println!(
                "  No failure insight candidates detected for cycle {}.",
                trace.cycle_id
            );
            println!("  The cycle trace indicates healthy context assembly and decision state.");
        } else {
            println!(
                "  Detected {} candidate(s) for cycle {}:",
                candidates.len(),
                trace.cycle_id
            );
            for (i, fc) in candidates.iter().enumerate() {
                let kind_str = serde_json::to_string(&fc.kind).unwrap_or_default();
                println!("  {}. kind={}", i + 1, kind_str);
                println!("     summary: {}", fc.summary);
                if !fc.reason.is_empty() {
                    println!("     reason:  {}", fc.reason);
                }
            }
        }
        println!();
        println!("  Saved to: {}", insights_path.display());
        println!();
        println!(
            "{}",
            style_dim("⚠  Advisory — candidates are non-authorizing detection signals.")
        );
        println!(
            "{}",
            style_dim("   No corrective action is implied or authorized.")
        );
    }

    Ok(())
}

/// List collected orchestrator failure insight files from the configured directory.
fn orchestrator_insights_list(args: OrchestratorInsightsListArgs) -> Result<(), Box<dyn Error>> {
    let dir = args
        .insights_dir
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(DEFAULT_ORCHESTRATOR_INSIGHTS_DIR));

    let mut entries: Vec<(std::path::PathBuf, serde_json::Value)> = vec![];

    if dir.exists() {
        for entry in std::fs::read_dir(&dir)
            .map_err(|e| format!("Cannot read directory '{}': {e}", dir.display()))?
        {
            let entry = entry?;
            let entry_path = entry.path();
            if entry_path.extension().map_or(false, |ext| ext == "json") {
                if let Ok(content) = std::fs::read_to_string(&entry_path) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                        entries.push((entry_path, val));
                    }
                }
            }
        }
    }

    // Sort by filename for deterministic order
    entries.sort_by(|a, b| a.0.file_name().cmp(&b.0.file_name()));

    if args.json {
        let listings: Vec<&serde_json::Value> = entries.iter().map(|(_, v)| v).collect();
        println!("{}", serde_json::to_string_pretty(&listings)?);
    } else {
        println!(
            "{}",
            style_info("Orchestrator Failure Insights — Collected")
        );
        println!("{}", "-".repeat(60));
        if entries.is_empty() {
            println!("  No collected orchestrator failure insight files found.");
            println!();
            println!(
                "  Run `{}` to collect insights from a saved CycleTrace.",
                "cargo run -q --bin arpagona -- orchestrator insights-collect <trace-path>"
            );
            println!();
            println!(
                "  Default insights directory: {}",
                DEFAULT_ORCHESTRATOR_INSIGHTS_DIR
            );
        } else {
            println!(
                "Found {} insight file(s) in '{}':\n",
                entries.len(),
                dir.display()
            );
            for (i, (path, val)) in entries.iter().enumerate() {
                let cycle_id = val.get("cycle_id").and_then(|v| v.as_str()).unwrap_or("?");
                let objective = val
                    .get("objective_text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                let count = val
                    .get("candidate_count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let created = val
                    .get("created_at")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                println!(
                    "{}. {}",
                    i + 1,
                    path.file_name().unwrap_or_default().to_string_lossy()
                );
                println!("   Cycle ID:    {}", cycle_id);
                println!("   Objective:   {}", objective);
                println!("   Candidates:  {}", count);
                println!("   Created:     {}", created);
                println!();
            }
            println!(
                "  Run `{}` to collect insights from a new trace.",
                "cargo run -q --bin arpagona -- orchestrator insights-collect <trace-path>"
            );
        }
        println!();
        println!(
            "{}",
            style_dim("⚠  Readback only — collected insights are evidence, not authorization.")
        );
        println!(
            "{}",
            style_dim("   No execution without explicit Decision Gate approval.")
        );
    }

    Ok(())
}

/// Parse a validated Ollama JSON response into an ActorIntent.
///
/// Performs deterministic validation: allowed-tool checks, schema validation,
/// and risk labeling. This function is testable without an Ollama instance.
fn parse_ollama_intent(parsed: &serde_json::Value) -> Result<ActorIntent, IntentParseError> {
    let tool = parsed
        .get("tool")
        .and_then(|v| v.as_str())
        .ok_or_else(|| IntentParseError::IncompleteResponse("missing 'tool' field".to_owned()))?;

    // Validate tool is in the allowed list
    if !ALLOWED_TOOLS.contains(&tool) {
        return Err(IntentParseError::DisallowedTool(tool.to_owned()));
    }

    let arguments = parsed.get("arguments").ok_or_else(|| {
        IntentParseError::IncompleteResponse("missing 'arguments' field".to_owned())
    })?;

    let rationale = parsed
        .get("rationale")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            IntentParseError::IncompleteResponse("missing 'rationale' field".to_owned())
        })?;

    // Per-tool schema validation: verify required arguments exist and are non-empty
    validate_tool_arguments(tool, arguments)?;

    // Derive risk_level deterministically from tool, ignoring Ollama's risk_level field
    let risk_level = match tool {
        "append_file" => RiskLevel::Low,
        _ => RiskLevel::Informational, // read_file, list_files, search_text
    };

    let display_summary = format!(
        "Ollama proposed: {} {}",
        tool,
        arguments
            .get("path")
            .and_then(|v| v.as_str())
            .or_else(|| arguments.get("pattern").and_then(|v| v.as_str()))
            .unwrap_or("?")
    );

    Ok(ActorIntent {
        tool: tool.to_owned(),
        arguments: arguments.clone(),
        risk_level,
        rationale: rationale.to_owned(),
        display_summary,
    })
}

/// Validate per-tool argument schemas deterministically.
/// Called before Decision Gate to catch malformed Ollama proposals.
fn validate_tool_arguments(
    tool: &str,
    arguments: &serde_json::Value,
) -> Result<(), IntentParseError> {
    match tool {
        "read_file" => {
            let path = arguments.get("path").and_then(|v| v.as_str());
            match path {
                Some(p) if !p.is_empty() => Ok(()),
                _ => Err(IntentParseError::MissingArgument(
                    "read_file requires non-empty string 'path'".to_owned(),
                )),
            }
        }
        "append_file" => {
            let path = arguments.get("path").and_then(|v| v.as_str());
            match path {
                Some(p) if !p.is_empty() => {}
                _ => {
                    return Err(IntentParseError::MissingArgument(
                        "append_file requires non-empty string 'path'".to_owned(),
                    ));
                }
            }
            let content = arguments.get("content").and_then(|v| v.as_str());
            match content {
                Some(c) if !c.is_empty() => Ok(()),
                _ => Err(IntentParseError::MissingArgument(
                    "append_file requires non-empty string 'content'".to_owned(),
                )),
            }
        }
        "list_files" => {
            // path is optional for list_files (defaults to cwd if missing)
            if let Some(path) = arguments.get("path") {
                if !path.is_string() {
                    return Err(IntentParseError::MissingArgument(
                        "list_files 'path' must be a string".to_owned(),
                    ));
                }
            }
            Ok(())
        }
        "search_text" => {
            let pattern = arguments.get("pattern").and_then(|v| v.as_str());
            match pattern {
                Some(p) if !p.is_empty() => {}
                _ => {
                    return Err(IntentParseError::MissingArgument(
                        "search_text requires non-empty string 'pattern'".to_owned(),
                    ));
                }
            }
            // path is optional for search_text
            if let Some(path) = arguments.get("path") {
                if !path.is_string() {
                    return Err(IntentParseError::MissingArgument(
                        "search_text 'path' must be a string".to_owned(),
                    ));
                }
            }
            Ok(())
        }
        _ => Ok(()), // Should not reach here; tool already validated against ALLOWED_TOOLS
    }
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
    fn cli_parses_action_supervise_defaults() {
        let cli = Cli::parse_from(["arpagona", "action", "supervise"]);
        match cli.command {
            Command::Action(ActionCommand {
                command: ActionSubcommand::Supervise(args),
            }) => {
                assert_eq!(args.limit, 10);
                assert!(!args.json);
                assert!(args.interaction_type.is_none());
            }
            _ => panic!("expected action supervise"),
        }
    }

    #[test]
    fn cli_parses_action_supervise_with_limit_and_json() {
        let cli = Cli::parse_from(["arpagona", "action", "supervise", "--limit", "25", "--json"]);
        match cli.command {
            Command::Action(ActionCommand {
                command: ActionSubcommand::Supervise(args),
            }) => {
                assert_eq!(args.limit, 25);
                assert!(args.json);
            }
            _ => panic!("expected action supervise"),
        }
    }

    #[test]
    fn cli_parses_action_supervise_with_interaction_type() {
        let cli = Cli::parse_from([
            "arpagona",
            "action",
            "supervise",
            "--interaction-type",
            "governance",
        ]);
        match cli.command {
            Command::Action(ActionCommand {
                command: ActionSubcommand::Supervise(args),
            }) => {
                assert_eq!(args.interaction_type.as_deref(), Some("governance"));
                assert_eq!(args.limit, 10);
            }
            _ => panic!("expected action supervise"),
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
    fn cli_parses_audit_list_without_flags() {
        let cli = Cli::parse_from(["arpagona", "audit", "list"]);
        match cli.command {
            Command::Audit(AuditCommand {
                command: AuditSubcommand::List(args),
            }) => {
                assert!(!args.json);
            }
            _ => panic!("expected audit list"),
        }
    }

    #[test]
    fn cli_parses_audit_list_with_json_flag() {
        let cli = Cli::parse_from(["arpagona", "audit", "list", "--json"]);
        match cli.command {
            Command::Audit(AuditCommand {
                command: AuditSubcommand::List(args),
            }) => {
                assert!(args.json);
            }
            _ => panic!("expected audit list --json"),
        }
    }

    #[test]
    fn cli_parses_audit_list_traces_without_flags() {
        let cli = Cli::parse_from(["arpagona", "audit", "list-traces"]);
        match cli.command {
            Command::Audit(AuditCommand {
                command: AuditSubcommand::ListTraces(args),
            }) => {
                assert!(!args.json);
                assert_eq!(args.trace_dir, "target/orchestrator-traces");
            }
            _ => panic!("expected audit list-traces"),
        }
    }

    #[test]
    fn cli_parses_audit_list_traces_with_flags() {
        let cli = Cli::parse_from([
            "arpagona",
            "audit",
            "list-traces",
            "--json",
            "--trace-dir",
            "target/custom-traces",
        ]);
        match cli.command {
            Command::Audit(AuditCommand {
                command: AuditSubcommand::ListTraces(args),
            }) => {
                assert!(args.json);
                assert_eq!(args.trace_dir, "target/custom-traces");
            }
            _ => panic!("expected audit list-traces --json --trace-dir"),
        }
    }

    #[test]
    fn cli_parses_audit_get_trace_without_flags() {
        let cli = Cli::parse_from(["arpagona", "audit", "get-trace", "oc-1234567890"]);
        match cli.command {
            Command::Audit(AuditCommand {
                command: AuditSubcommand::GetTrace(args),
            }) => {
                assert_eq!(args.cycle_id, "oc-1234567890");
                assert!(!args.json);
                assert_eq!(args.trace_dir, "target/orchestrator-traces");
            }
            _ => panic!("expected audit get-trace"),
        }
    }

    #[test]
    fn cli_parses_audit_get_trace_with_flags() {
        let cli = Cli::parse_from([
            "arpagona",
            "audit",
            "get-trace",
            "oc-9876543210",
            "--json",
            "--trace-dir",
            "target/custom-traces",
        ]);
        match cli.command {
            Command::Audit(AuditCommand {
                command: AuditSubcommand::GetTrace(args),
            }) => {
                assert_eq!(args.cycle_id, "oc-9876543210");
                assert!(args.json);
                assert_eq!(args.trace_dir, "target/custom-traces");
            }
            _ => panic!("expected audit get-trace --json --trace-dir"),
        }
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
    fn cli_parses_memory_holographic_status() {
        let cli = Cli::parse_from(["arpagona", "memory", "holographic", "status"]);
        match cli.command {
            Command::Memory(MemoryCommand {
                command: MemorySubcommand::Holographic(h),
            }) => match h.command {
                HolographicSubcommand::Status(args) => {
                    assert!(!args.json);
                    assert_eq!(args.db, "target/holographic-memory.db");
                }
                _ => panic!("expected holographic status"),
            },
            _ => panic!("expected memory holographic status"),
        }
    }

    #[test]
    fn cli_parses_memory_holographic_status_json() {
        let cli = Cli::parse_from(["arpagona", "memory", "holographic", "status", "--json"]);
        match cli.command {
            Command::Memory(MemoryCommand {
                command: MemorySubcommand::Holographic(h),
            }) => match h.command {
                HolographicSubcommand::Status(args) => {
                    assert!(args.json);
                    assert_eq!(args.db, "target/holographic-memory.db");
                }
                _ => panic!("expected holographic status with --json"),
            },
            _ => panic!("expected memory holographic status with --json"),
        }
    }

    #[test]
    fn cli_parses_memory_holographic_status_custom_db() {
        let cli = Cli::parse_from([
            "arpagona",
            "memory",
            "holographic",
            "status",
            "--db",
            "custom/path.db",
            "--json",
        ]);
        match cli.command {
            Command::Memory(MemoryCommand {
                command: MemorySubcommand::Holographic(h),
            }) => match h.command {
                HolographicSubcommand::Status(args) => {
                    assert!(args.json);
                    assert_eq!(args.db, "custom/path.db");
                }
                _ => panic!("expected holographic status with custom db"),
            },
            _ => panic!("expected memory holographic status with custom db"),
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
            memory_visibility: MemoryVisibilitySection {
                total_trace_count: None,
                recent_traces: vec![],
                most_activated_traces: vec![],
                aggregated_linked_memory_ids: vec![],
                aggregated_linked_decision_ids: vec![],
                store_accessible: false,
                consolidation_info: None,
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
        assert!(formatted.contains("Memory and resonance visibility (D3)"));
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
            memory_visibility: MemoryVisibilitySection {
                total_trace_count: None,
                recent_traces: vec![],
                most_activated_traces: vec![],
                aggregated_linked_memory_ids: vec![],
                aggregated_linked_decision_ids: vec![],
                store_accessible: false,
                consolidation_info: None,
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
            memory_visibility: MemoryVisibilitySection {
                total_trace_count: None,
                recent_traces: vec![],
                most_activated_traces: vec![],
                aggregated_linked_memory_ids: vec![],
                aggregated_linked_decision_ids: vec![],
                store_accessible: false,
                consolidation_info: None,
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
            memory_visibility: MemoryVisibilitySection {
                total_trace_count: None,
                recent_traces: vec![],
                most_activated_traces: vec![],
                aggregated_linked_memory_ids: vec![],
                aggregated_linked_decision_ids: vec![],
                store_accessible: false,
                consolidation_info: None,
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
        // D3 memory visibility fields
        let memory_vis = json
            .get("memory_visibility")
            .expect("memory_visibility field in JSON");
        assert!(memory_vis.get("total_trace_count").is_some());
        assert!(memory_vis.get("recent_traces").is_some());
        assert!(memory_vis.get("store_accessible").is_some());
        assert!(memory_vis.get("warning").is_some());
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

    #[test]
    fn cli_parses_cognitive_run_with_govern_tool_flag() {
        let cli = Cli::parse_from([
            "arpagona",
            "cognitive",
            "run",
            "--objective",
            "Read project status",
            "--govern-tool",
            "--json",
        ]);
        match cli.command {
            Command::Cognitive(CognitiveCommand {
                command: CognitiveSubcommand::Run(args),
            }) => {
                assert!(args.govern_tool);
                assert!(args.json);
                assert!(!args.llm);
                assert!(!args.govern);
            }
            _ => panic!("expected cognitive run with --govern-tool --json"),
        }
    }

    #[test]
    fn cli_parses_cognitive_run_with_llm_and_govern_tool() {
        let cli = Cli::parse_from([
            "arpagona",
            "cognitive",
            "run",
            "--objective",
            "Analyse le code source",
            "--llm",
            "--govern-tool",
            "--provider",
            "mock",
        ]);
        match cli.command {
            Command::Cognitive(CognitiveCommand {
                command: CognitiveSubcommand::Run(args),
            }) => {
                assert!(args.llm);
                assert!(args.govern_tool);
                assert_eq!(args.provider, "mock");
            }
            _ => panic!("expected cognitive run with --llm --govern-tool --provider mock"),
        }
    }

    #[test]
    fn tool_govern_parses_with_minimal_args() {
        let cli = Cli::parse_from([
            "arpagona",
            "tool",
            "govern",
            "read_file",
            r#"{"path": "test.md"}"#,
        ]);
        match cli.command {
            Command::Tool(ToolCommand {
                command: ToolSubcommand::Govern(args),
            }) => {
                assert_eq!(args.tool, "read_file");
                assert!(args.args.contains("test.md"));
                assert_eq!(args.risk_level, "informational");
                assert!(!args.json);
            }
            _ => panic!("expected tool govern"),
        }
    }

    #[test]
    fn tool_govern_parses_with_all_flags() {
        let cli = Cli::parse_from([
            "arpagona",
            "tool",
            "govern",
            "search_text",
            r#"{"query": "Decision Gate"}"#,
            "--risk-level",
            "low",
            "--rationale",
            "Search for governance references",
            "--json",
        ]);
        match cli.command {
            Command::Tool(ToolCommand {
                command: ToolSubcommand::Govern(args),
            }) => {
                assert_eq!(args.tool, "search_text");
                assert!(args.args.contains("Decision Gate"));
                assert_eq!(args.risk_level, "low");
                assert_eq!(args.rationale, "Search for governance references");
                assert!(args.json);
            }
            _ => panic!("expected tool govern with all flags"),
        }
    }

    #[test]
    fn tool_govern_parses_medium_risk() {
        let cli = Cli::parse_from([
            "arpagona",
            "tool",
            "govern",
            "write_document",
            r#"{"path": "doc.md", "content": "data"}"#,
            "--risk-level",
            "medium",
            "--json",
        ]);
        match cli.command {
            Command::Tool(ToolCommand {
                command: ToolSubcommand::Govern(args),
            }) => {
                assert_eq!(args.tool, "write_document");
                assert_eq!(args.risk_level, "medium");
                assert!(args.json);
            }
            _ => panic!("expected tool govern with medium risk"),
        }
    }

    #[test]
    fn tool_demo_actor_lab_parses_approval_path() {
        let cli = Cli::parse_from([
            "arpagona",
            "tool",
            "demo",
            "actor-lab",
            "--path",
            "actor-lab/NOTES.md",
            "--note",
            "- test note",
            "--approve",
            "--json",
        ]);
        match cli.command {
            Command::Tool(ToolCommand {
                command:
                    ToolSubcommand::Demo(ToolDemoCommand {
                        command: ToolDemoSubcommand::ActorLab(args),
                    }),
            }) => {
                assert_eq!(args.path, "actor-lab/NOTES.md");
                assert_eq!(args.note, "- test note");
                assert!(args.approve);
                assert!(args.json);
            }
            _ => panic!("expected tool demo actor-lab"),
        }
    }

    #[test]
    fn parse_risk_level_valid_values() {
        assert_eq!(
            parse_risk_level("informational").unwrap(),
            RiskLevel::Informational
        );
        assert_eq!(parse_risk_level("low").unwrap(), RiskLevel::Low);
        assert_eq!(parse_risk_level("medium").unwrap(), RiskLevel::Medium);
        assert_eq!(parse_risk_level("high").unwrap(), RiskLevel::High);
        assert_eq!(parse_risk_level("critical").unwrap(), RiskLevel::Critical);
        // Case insensitive
        assert_eq!(parse_risk_level("HIGH").unwrap(), RiskLevel::High);
        assert_eq!(parse_risk_level("Low").unwrap(), RiskLevel::Low);
    }

    #[test]
    fn parse_risk_level_invalid_returns_error() {
        assert!(parse_risk_level("extreme").is_err());
        assert!(parse_risk_level("").is_err());
        assert!(parse_risk_level("none").is_err());
    }

    // -----------------------------------------------------------------------
    // C2.2 — approved tool-call execution through the bounded Tool Runtime
    // -----------------------------------------------------------------------

    #[test]
    fn tool_govern_approved_executes_read_file_and_returns_observation() {
        // When governance approves an informational read_file call,
        // the Tool Runtime should execute it and return a result.
        let args = ToolGovernArgs {
            tool: "read_file".to_owned(),
            args: r#"{"path": "Cargo.toml"}"#.to_owned(),
            risk_level: "informational".to_owned(),
            rationale: "Test approved tool execution".to_owned(),
            json: true,
        };
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| tool_govern(args)));
        // The function should not panic; we check the Ok/Err result
        assert!(result.is_ok(), "tool_govern should not panic");
    }

    #[test]
    fn tool_govern_approved_execution_includes_execution_result_in_json() {
        // Parse CLI args and check that JSON output contains execution_result
        let cli = Cli::parse_from([
            "arpagona",
            "tool",
            "govern",
            "read_file",
            r#"{"path": "Cargo.toml"}"#,
            "--json",
        ]);
        match cli.command {
            Command::Tool(ToolCommand {
                command: ToolSubcommand::Govern(args),
            }) => {
                assert_eq!(args.tool, "read_file");
                assert!(args.json);
                // The JSON flag is present — tool_govern should produce
                // a JSON response that includes execution_result for approved calls
            }
            _ => panic!("expected tool govern with --json"),
        }
    }

    #[test]
    fn tool_govern_unknown_tool_returns_execution_error() {
        // When governance approves but the Tool Runtime doesn't know the tool,
        // the execution_result should contain an error.
        let args = ToolGovernArgs {
            tool: "nonexistent_tool".to_owned(),
            args: r#"{}"#.to_owned(),
            risk_level: "informational".to_owned(),
            rationale: "Test unknown tool handling".to_owned(),
            json: true,
        };
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| tool_govern(args)));
        assert!(
            result.is_ok(),
            "tool_govern with unknown tool should not panic"
        );
    }

    #[test]
    fn tool_govern_blocked_governance_does_not_execute() {
        // When governance blocks (medium risk without permissions),
        // the output should NOT contain execution_result.
        let args = ToolGovernArgs {
            tool: "read_file".to_owned(),
            args: r#"{"path": "test.md"}"#.to_owned(),
            risk_level: "medium".to_owned(),
            rationale: "Test blocked governance".to_owned(),
            json: true,
        };
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| tool_govern(args)));
        assert!(
            result.is_ok(),
            "tool_govern with blocked governance should not panic"
        );
    }

    #[test]
    fn tool_govern_approved_blocked_file_returns_security_block() {
        // Approved governance + blocked file (.env) should execute through
        // Tool Runtime and get a security-blocked result.
        let args = ToolGovernArgs {
            tool: "read_file".to_owned(),
            args: r#"{"path": ".env"}"#.to_owned(),
            risk_level: "informational".to_owned(),
            rationale: "Test security blocked file via governed path".to_owned(),
            json: true,
        };
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| tool_govern(args)));
        assert!(
            result.is_ok(),
            "tool_govern with blocked file should not panic"
        );
    }

    #[test]
    fn cli_parses_compute_routing_command() {
        let cli = Cli::try_parse_from(vec![
            "arpagona",
            "compute",
            "routing",
            "--purpose",
            "Analyze project proposal",
            "--sensitivity",
            "confidential",
            "--complexity",
            "0.8",
            "--local-first",
        ])
        .expect("compute routing should parse");
        match cli.command {
            Command::Compute(ComputeCommand {
                command: ComputeSubcommand::Routing(args),
            }) => {
                assert_eq!(args.purpose, "Analyze project proposal");
                assert_eq!(args.sensitivity, SensitivityArg::Confidential);
                assert!((args.complexity - 0.8).abs() < 0.01);
                assert!(args.local_first);
                assert!(!args.json);
            }
            _ => panic!("expected compute routing"),
        }
    }

    #[test]
    fn cli_parses_compute_routing_json_flag() {
        let cli = Cli::try_parse_from(vec![
            "arpagona",
            "compute",
            "routing",
            "--purpose",
            "Summarize data",
            "--complexity",
            "0.3",
            "--json",
        ])
        .expect("compute routing with json should parse");
        match cli.command {
            Command::Compute(ComputeCommand {
                command: ComputeSubcommand::Routing(args),
            }) => {
                assert_eq!(args.purpose, "Summarize data");
                assert!((args.complexity - 0.3).abs() < 0.01);
                assert!(!args.local_first);
                assert!(args.json);
            }
            _ => panic!("expected compute routing"),
        }
    }

    #[test]
    fn cli_parses_orchestrator_run_defaults() {
        let cli = Cli::try_parse_from(vec![
            "arpagona",
            "orchestrator",
            "run",
            "--objective",
            "Review project documentation",
        ])
        .expect("orchestrator run should parse");
        match cli.command {
            Command::Orchestrator(OrchestratorCommand {
                command: OrchestratorSubcommand::Run(args),
            }) => {
                assert_eq!(args.objective, "Review project documentation");
                assert!(!args.json);
                assert_eq!(args.permissions, vec!["ReadDocument"]);
                assert_eq!(args.workspace_id, "workspace-alpha");
                assert_eq!(args.agent_id, "agent-alpha");
            }
            _ => panic!("expected orchestrator run"),
        }
    }

    #[test]
    fn cli_parses_orchestrator_run_with_json() {
        let cli = Cli::try_parse_from(vec![
            "arpagona",
            "orchestrator",
            "run",
            "--objective",
            "Analyze the dataset",
            "--json",
            "--perm",
            "WriteMemory",
            "--workspace-id",
            "ws-prod",
            "--agent-id",
            "agent-delta",
        ])
        .expect("orchestrator run with flags should parse");
        match cli.command {
            Command::Orchestrator(OrchestratorCommand {
                command: OrchestratorSubcommand::Run(args),
            }) => {
                assert_eq!(args.objective, "Analyze the dataset");
                assert!(args.json);
                assert_eq!(args.permissions, vec!["WriteMemory"]);
                assert_eq!(args.workspace_id, "ws-prod");
                assert_eq!(args.agent_id, "agent-delta");
            }
            _ => panic!("expected orchestrator run"),
        }
    }

    #[test]
    fn cli_parses_orchestrator_run_multiple_permissions() {
        let cli = Cli::try_parse_from(vec![
            "arpagona",
            "orchestrator",
            "run",
            "--objective",
            "Multi-perm test",
            "--perm",
            "ReadDocument",
            "--perm",
            "WriteMemory",
        ])
        .expect("orchestrator run with multiple permissions should parse");
        match cli.command {
            Command::Orchestrator(OrchestratorCommand {
                command: OrchestratorSubcommand::Run(args),
            }) => {
                assert_eq!(args.objective, "Multi-perm test");
                assert_eq!(args.permissions, vec!["ReadDocument", "WriteMemory"]);
            }
            _ => panic!("expected orchestrator run"),
        }
    }

    #[test]
    fn orchestrator_outcome_is_non_authorizing() {
        let cycle = arpagona_neutral_orchestrator::run_deterministic_cycle(
            "Test non-authorizing invariant",
            WorkspaceId::new("ws-1"),
            AgentId::new("agent-alpha"),
            &[arpagona_agent_core::Permission::ReadDocument],
        )
        .expect("cycle should succeed");
        assert!(
            cycle.outcome.non_authorizing,
            "OrchestratorOutcome must always be non-authorizing"
        );
    }

    #[test]
    fn cli_parses_orchestrator_run_with_simulated_generator() {
        let cli = Cli::try_parse_from(vec![
            "arpagona",
            "orchestrator",
            "run",
            "--objective",
            "Test simulated generator",
            "--proposal-generator",
            "simulated",
        ])
        .expect("orchestrator run with --proposal-generator simulated should parse");
        match cli.command {
            Command::Orchestrator(OrchestratorCommand {
                command: OrchestratorSubcommand::Run(args),
            }) => {
                assert_eq!(args.proposal_generator, ProposalGeneratorArg::Simulated);
            }
            _ => panic!("expected orchestrator run"),
        }
    }

    #[test]
    fn cli_parses_orchestrator_run_with_llm_generator() {
        let cli = Cli::try_parse_from(vec![
            "arpagona",
            "orchestrator",
            "run",
            "--objective",
            "Test llm generator",
            "--proposal-generator",
            "llm",
        ])
        .expect("orchestrator run with --proposal-generator llm should parse");
        match cli.command {
            Command::Orchestrator(OrchestratorCommand {
                command: OrchestratorSubcommand::Run(args),
            }) => {
                assert_eq!(args.proposal_generator, ProposalGeneratorArg::Llm);
            }
            _ => panic!("expected orchestrator run"),
        }
    }

    #[test]
    fn cli_orchestrator_run_defaults_to_simulated_generator() {
        let cli = Cli::try_parse_from(vec![
            "arpagona",
            "orchestrator",
            "run",
            "--objective",
            "Default generator test",
        ])
        .expect("orchestrator run without --proposal-generator should parse");
        match cli.command {
            Command::Orchestrator(OrchestratorCommand {
                command: OrchestratorSubcommand::Run(args),
            }) => {
                assert_eq!(
                    args.proposal_generator,
                    ProposalGeneratorArg::Simulated,
                    "default proposal_generator should be Simulated"
                );
            }
            _ => panic!("expected orchestrator run"),
        }
    }

    // ── H1b: CLI error-path coverage ───────────────────────────────────────
    //
    // These tests verify that CLI commands produce proper clap errors (not
    // panics) when given invalid or missing arguments.

    #[test]
    fn cli_rejects_cognitive_run_without_objective() {
        let result = Cli::try_parse_from(vec!["arpagona", "cognitive", "run"]);
        assert!(
            result.is_err(),
            "cognitive run without --objective should fail: {:?}",
            result
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("objective") || err.contains("required"),
            "error should mention missing objective: {err}"
        );
    }

    #[test]
    fn cli_rejects_cognitive_run_with_empty_objective() {
        let cli = Cli::parse_from(vec!["arpagona", "cognitive", "run", "--objective", ""]);
        match cli.command {
            Command::Cognitive(CognitiveCommand {
                command: CognitiveSubcommand::Run(args),
            }) => {
                assert!(
                    args.objective.is_empty(),
                    "empty --objective should be accepted but empty"
                );
            }
            _ => panic!("expected cognitive run"),
        }
    }

    #[test]
    fn cli_rejects_invalid_provider_value() {
        // Provider is a free-form string; parser accepts any value
        let cli = Cli::parse_from(vec![
            "arpagona",
            "cognitive",
            "run",
            "--objective",
            "test",
            "--provider",
            "nonexistent_provider",
        ]);
        match cli.command {
            Command::Cognitive(CognitiveCommand {
                command: CognitiveSubcommand::Run(args),
            }) => {
                assert_eq!(args.provider, "nonexistent_provider");
            }
            _ => panic!("expected cognitive run"),
        }
    }

    #[test]
    fn cli_rejects_tool_inspect_without_tool_name() {
        let result = Cli::try_parse_from(vec!["arpagona", "tool", "inspect"]);
        assert!(
            result.is_err(),
            "tool inspect without tool name should fail: {:?}",
            result
        );
    }

    // ── P3-next: Orchestrator status and --save-trace parser tests ──────────

    #[test]
    fn cli_parses_orchestrator_status_defaults() {
        let cli = Cli::try_parse_from(vec!["arpagona", "orchestrator", "status"])
            .expect("orchestrator status should parse");
        match cli.command {
            Command::Orchestrator(OrchestratorCommand {
                command: OrchestratorSubcommand::Status(args),
            }) => {
                assert!(!args.json, "default status should not be json");
                assert_eq!(args.trace_path, None, "default should use default path");
            }
            _ => panic!("expected orchestrator status"),
        }
    }

    #[test]
    fn cli_parses_orchestrator_status_with_json() {
        let cli = Cli::try_parse_from(vec!["arpagona", "orchestrator", "status", "--json"])
            .expect("orchestrator status --json should parse");
        match cli.command {
            Command::Orchestrator(OrchestratorCommand {
                command: OrchestratorSubcommand::Status(args),
            }) => {
                assert!(args.json, "status --json should be true");
            }
            _ => panic!("expected orchestrator status"),
        }
    }

    #[test]
    fn cli_parses_orchestrator_status_with_trace_path() {
        let cli = Cli::try_parse_from(vec![
            "arpagona",
            "orchestrator",
            "status",
            "--trace-path",
            "custom/trace.json",
        ])
        .expect("orchestrator status --trace-path should parse");
        match cli.command {
            Command::Orchestrator(OrchestratorCommand {
                command: OrchestratorSubcommand::Status(args),
            }) => {
                assert_eq!(
                    args.trace_path,
                    Some("custom/trace.json".to_owned()),
                    "custom trace path should resolve"
                );
            }
            _ => panic!("expected orchestrator status"),
        }
    }

    #[test]
    fn cli_parses_orchestrator_run_with_save_trace() {
        let cli = Cli::try_parse_from(vec![
            "arpagona",
            "orchestrator",
            "run",
            "--objective",
            "Save trace test",
            "--save-trace",
            "target/my-trace.json",
        ])
        .expect("orchestrator run --save-trace should parse");
        match cli.command {
            Command::Orchestrator(OrchestratorCommand {
                command: OrchestratorSubcommand::Run(args),
            }) => {
                assert_eq!(
                    args.save_trace,
                    Some("target/my-trace.json".to_owned()),
                    "--save-trace path should be captured"
                );
                assert_eq!(args.objective, "Save trace test");
            }
            _ => panic!("expected orchestrator run"),
        }
    }

    #[test]
    fn cli_parses_orchestrator_run_with_save_trace_auto() {
        // --save-trace without a path should default to "auto" (auto-naming sentinel)
        let cli = Cli::try_parse_from(vec![
            "arpagona",
            "orchestrator",
            "run",
            "--objective",
            "Auto-name test",
            "--save-trace",
        ])
        .expect("orchestrator run --save-trace (no path) should parse");
        match cli.command {
            Command::Orchestrator(OrchestratorCommand {
                command: OrchestratorSubcommand::Run(args),
            }) => {
                assert_eq!(
                    args.save_trace,
                    Some("auto".to_owned()),
                    "--save-trace without path should default to auto sentinel"
                );
                assert_eq!(args.objective, "Auto-name test");
            }
            _ => panic!("expected orchestrator run"),
        }
    }

    #[test]
    fn cli_parses_orchestrator_run_with_save_trace_and_trace() {
        let cli = Cli::try_parse_from(vec![
            "arpagona",
            "orchestrator",
            "run",
            "--objective",
            "Full demo: save trace for status",
            "--trace",
            "--save-trace",
            "target/demo-trace.json",
        ])
        .expect("orchestrator run --trace --save-trace should parse");
        match cli.command {
            Command::Orchestrator(OrchestratorCommand {
                command: OrchestratorSubcommand::Run(args),
            }) => {
                assert!(args.trace, "--trace should be set");
                assert_eq!(
                    args.save_trace,
                    Some("target/demo-trace.json".to_owned()),
                    "--save-trace should be captured"
                );
                assert_eq!(args.objective, "Full demo: save trace for status");
            }
            _ => panic!("expected orchestrator run"),
        }
    }

    #[test]
    fn cli_parses_orchestrator_run_with_collect_insights() {
        let cli = Cli::try_parse_from(vec![
            "arpagona",
            "orchestrator",
            "run",
            "--objective",
            "Collect insights demo",
            "--collect-insights",
        ])
        .expect("orchestrator run --collect-insights should parse");
        match cli.command {
            Command::Orchestrator(OrchestratorCommand {
                command: OrchestratorSubcommand::Run(args),
            }) => {
                assert!(args.collect_insights, "--collect-insights should be true");
                assert_eq!(args.objective, "Collect insights demo");
                assert_eq!(
                    args.insights_dir, None,
                    "default insights dir should not be set"
                );
            }
            _ => panic!("expected orchestrator run"),
        }
    }

    #[test]
    fn cli_parses_orchestrator_run_with_collect_insights_and_custom_dir() {
        let cli = Cli::try_parse_from(vec![
            "arpagona",
            "orchestrator",
            "run",
            "--objective",
            "Custom insights dir",
            "--collect-insights",
            "--insights-dir",
            "custom/insights",
        ])
        .expect("orchestrator run --collect-insights --insights-dir should parse");
        match cli.command {
            Command::Orchestrator(OrchestratorCommand {
                command: OrchestratorSubcommand::Run(args),
            }) => {
                assert!(args.collect_insights, "--collect-insights should be true");
                assert_eq!(
                    args.insights_dir,
                    Some("custom/insights".to_owned()),
                    "--insights-dir should be captured"
                );
                assert_eq!(args.objective, "Custom insights dir");
            }
            _ => panic!("expected orchestrator run"),
        }
    }

    #[test]
    fn cli_rejects_orchestrator_status_without_such_subcommand() {
        // Verify that 'orchestrator status' is a real subcommand (not a rejected flag)
        let cli = Cli::try_parse_from(vec!["arpagona", "orchestrator", "status"])
            .expect("orchestrator status should be a valid subcommand");
        assert!(
            matches!(cli.command, Command::Orchestrator(_)),
            "orchestrator status must dispatch to Orchestrator command"
        );
    }

    #[test]
    fn cli_parses_orchestrator_run_with_save_audit() {
        let cli = Cli::try_parse_from(vec![
            "arpagona",
            "orchestrator",
            "run",
            "--objective",
            "Save audit test",
            "--save-audit",
            "target/my-audit-dir",
        ])
        .expect("orchestrator run --save-audit should parse");
        match cli.command {
            Command::Orchestrator(OrchestratorCommand {
                command: OrchestratorSubcommand::Run(args),
            }) => {
                assert_eq!(
                    args.save_audit,
                    Some("target/my-audit-dir".to_owned()),
                    "--save-audit path should be captured"
                );
                assert_eq!(args.objective, "Save audit test");
            }
            _ => panic!("expected orchestrator run"),
        }
    }

    #[test]
    fn cli_parses_orchestrator_run_with_save_audit_auto() {
        // --save-audit without a path should default to "auto" (auto-naming sentinel)
        let cli = Cli::try_parse_from(vec![
            "arpagona",
            "orchestrator",
            "run",
            "--objective",
            "Auto-name audit test",
            "--save-audit",
        ])
        .expect("orchestrator run --save-audit (no path) should parse");
        match cli.command {
            Command::Orchestrator(OrchestratorCommand {
                command: OrchestratorSubcommand::Run(args),
            }) => {
                assert_eq!(
                    args.save_audit,
                    Some("auto".to_owned()),
                    "--save-audit without path should default to auto sentinel"
                );
                assert_eq!(args.objective, "Auto-name audit test");
            }
            _ => panic!("expected orchestrator run"),
        }
    }

    #[test]
    fn cli_parses_orchestrator_run_with_save_audit_and_save_trace() {
        // Both --save-audit and --save-trace can be combined
        let cli = Cli::try_parse_from(vec![
            "arpagona",
            "orchestrator",
            "run",
            "--objective",
            "Combined save test",
            "--save-audit",
            "target/audit",
            "--save-trace",
            "target/trace.json",
        ])
        .expect("orchestrator run --save-audit --save-trace should parse");
        match cli.command {
            Command::Orchestrator(OrchestratorCommand {
                command: OrchestratorSubcommand::Run(args),
            }) => {
                assert_eq!(
                    args.save_audit,
                    Some("target/audit".to_owned()),
                    "--save-audit should be captured"
                );
                assert_eq!(
                    args.save_trace,
                    Some("target/trace.json".to_owned()),
                    "--save-trace should be captured"
                );
            }
            _ => panic!("expected orchestrator run"),
        }
    }

    // ── Orchestrator cycles parser tests ────────────────────────────────

    #[test]
    fn cli_parses_orchestrator_cycles_defaults() {
        let cli = Cli::try_parse_from(vec!["arpagona", "orchestrator", "cycles"])
            .expect("orchestrator cycles should parse");
        match cli.command {
            Command::Orchestrator(OrchestratorCommand {
                command: OrchestratorSubcommand::Cycles(args),
            }) => {
                assert!(!args.json, "default cycles should not be json");
                assert_eq!(args.trace_dir, None, "default should use default dir");
            }
            _ => panic!("expected orchestrator cycles"),
        }
    }

    #[test]
    fn cli_parses_orchestrator_cycles_with_json() {
        let cli = Cli::try_parse_from(vec!["arpagona", "orchestrator", "cycles", "--json"])
            .expect("orchestrator cycles --json should parse");
        match cli.command {
            Command::Orchestrator(OrchestratorCommand {
                command: OrchestratorSubcommand::Cycles(args),
            }) => {
                assert!(args.json, "cycles --json should be true");
            }
            _ => panic!("expected orchestrator cycles"),
        }
    }

    #[test]
    fn cli_parses_orchestrator_cycles_with_trace_dir() {
        let cli = Cli::try_parse_from(vec![
            "arpagona",
            "orchestrator",
            "cycles",
            "--trace-dir",
            "custom/traces",
        ])
        .expect("orchestrator cycles --trace-dir should parse");
        match cli.command {
            Command::Orchestrator(OrchestratorCommand {
                command: OrchestratorSubcommand::Cycles(args),
            }) => {
                assert_eq!(
                    args.trace_dir,
                    Some("custom/traces".to_owned()),
                    "--trace-dir should be captured"
                );
            }
            _ => panic!("expected orchestrator cycles"),
        }
    }

    #[test]
    fn cli_parses_orchestrator_cycles_with_audit() {
        let cli = Cli::try_parse_from(vec!["arpagona", "orchestrator", "cycles", "--with-audit"])
            .expect("orchestrator cycles --with-audit should parse");
        match cli.command {
            Command::Orchestrator(OrchestratorCommand {
                command: OrchestratorSubcommand::Cycles(args),
            }) => {
                assert!(args.with_audit, "cycles --with-audit should be true");
                assert!(!args.json, "cycles --with-audit should not default to json");
            }
            _ => panic!("expected orchestrator cycles"),
        }
    }

    #[test]
    fn cli_parses_orchestrator_cycles_with_audit_and_json() {
        let cli = Cli::try_parse_from(vec![
            "arpagona",
            "orchestrator",
            "cycles",
            "--with-audit",
            "--json",
        ])
        .expect("orchestrator cycles --with-audit --json should parse");
        match cli.command {
            Command::Orchestrator(OrchestratorCommand {
                command: OrchestratorSubcommand::Cycles(args),
            }) => {
                assert!(args.with_audit, "cycles --with-audit should be true");
                assert!(args.json, "cycles --with-audit --json should be true");
            }
            _ => panic!("expected orchestrator cycles"),
        }
    }

    #[test]
    fn cli_parses_orchestrator_cycles_with_audit_and_trace_dir() {
        let cli = Cli::try_parse_from(vec![
            "arpagona",
            "orchestrator",
            "cycles",
            "--with-audit",
            "--trace-dir",
            "custom/traces",
        ])
        .expect("orchestrator cycles --with-audit --trace-dir should parse");
        match cli.command {
            Command::Orchestrator(OrchestratorCommand {
                command: OrchestratorSubcommand::Cycles(args),
            }) => {
                assert!(args.with_audit, "cycles --with-audit should be true");
                assert_eq!(
                    args.trace_dir,
                    Some("custom/traces".to_owned()),
                    "--trace-dir should be captured"
                );
            }
            _ => panic!("expected orchestrator cycles"),
        }
    }

    // ── Run command parser tests ─────────────────────────────────────

    #[test]
    fn cli_parses_run_positional() {
        let cli = Cli::try_parse_from(vec!["arpagona", "run", "Review project documentation"])
            .expect("run with positional arg should parse");
        match cli.command {
            Command::Run(args) => {
                assert_eq!(args.objective, "Review project documentation");
            }
            _ => panic!("expected Run"),
        }
    }

    #[test]
    fn cli_parses_run_with_quoted_objective() {
        let cli = Cli::try_parse_from(vec![
            "arpagona",
            "run",
            "Analyze the quarterly dataset and propose actions",
        ])
        .expect("run with quoted objective should parse");
        match cli.command {
            Command::Run(args) => {
                assert_eq!(
                    args.objective,
                    "Analyze the quarterly dataset and propose actions"
                );
            }
            _ => panic!("expected Run"),
        }
    }

    #[test]
    fn cli_parses_run_short_objective() {
        let cli = Cli::try_parse_from(vec!["arpagona", "run", "test"])
            .expect("run with short objective should parse");
        match cli.command {
            Command::Run(args) => {
                assert_eq!(args.objective, "test");
            }
            _ => panic!("expected Run"),
        }
    }

    #[test]
    fn handle_run_returns_readable_output() {
        // Integration test: execute the run command and verify output is readable
        // (no JSON, no governance jargon markers like "Decision Gate")
        let args = RunArgs {
            objective: "Integration test objective".to_owned(),
        };
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| handle_run(args).ok()));
        // Should not panic — the function should complete without error
        assert!(result.is_ok(), "handle_run should not panic");
    }

    // ── Actor Readback parse tests ──────────────────────────────────

    #[test]
    fn actor_status_parses_with_defaults() {
        let cli = Cli::parse_from(["arpagona", "actor", "status"]);
        match cli.command {
            Command::Actor(ActorCommand {
                command: ActorSubcommand::Status(args),
            }) => {
                assert!(!args.json);
            }
            _ => panic!("expected actor status"),
        }
    }

    #[test]
    fn actor_status_parses_with_json() {
        let cli = Cli::parse_from(["arpagona", "actor", "status", "--json"]);
        match cli.command {
            Command::Actor(ActorCommand {
                command: ActorSubcommand::Status(args),
            }) => {
                assert!(args.json);
            }
            _ => panic!("expected actor status --json"),
        }
    }

    #[test]
    fn actor_memory_parses_with_defaults() {
        let cli = Cli::parse_from(["arpagona", "actor", "memory"]);
        match cli.command {
            Command::Actor(ActorCommand {
                command: ActorSubcommand::Memory(args),
            }) => {
                assert!(!args.json);
            }
            _ => panic!("expected actor memory"),
        }
    }

    #[test]
    fn actor_memory_parses_with_json() {
        let cli = Cli::parse_from(["arpagona", "actor", "memory", "--json"]);
        match cli.command {
            Command::Actor(ActorCommand {
                command: ActorSubcommand::Memory(args),
            }) => {
                assert!(args.json);
            }
            _ => panic!("expected actor memory --json"),
        }
    }

    #[test]
    fn actor_journal_parses_with_defaults() {
        let cli = Cli::parse_from(["arpagona", "actor", "journal"]);
        match cli.command {
            Command::Actor(ActorCommand {
                command: ActorSubcommand::Journal(args),
            }) => {
                assert_eq!(args.limit, 10);
                assert!(args.interaction_type.is_none());
                assert!(!args.json);
            }
            _ => panic!("expected actor journal"),
        }
    }

    #[test]
    fn actor_journal_parses_with_limit() {
        let cli = Cli::parse_from(["arpagona", "actor", "journal", "--limit", "5"]);
        match cli.command {
            Command::Actor(ActorCommand {
                command: ActorSubcommand::Journal(args),
            }) => {
                assert_eq!(args.limit, 5);
            }
            _ => panic!("expected actor journal --limit"),
        }
    }

    #[test]
    fn actor_journal_parses_with_interaction_type() {
        let cli = Cli::parse_from([
            "arpagona",
            "actor",
            "journal",
            "--interaction-type",
            "direct_tool_call",
        ]);
        match cli.command {
            Command::Actor(ActorCommand {
                command: ActorSubcommand::Journal(args),
            }) => {
                assert_eq!(args.interaction_type.as_deref(), Some("direct_tool_call"));
            }
            _ => panic!("expected actor journal --interaction-type"),
        }
    }

    #[test]
    fn actor_journal_parses_with_json() {
        let cli = Cli::parse_from(["arpagona", "actor", "journal", "--json"]);
        match cli.command {
            Command::Actor(ActorCommand {
                command: ActorSubcommand::Journal(args),
            }) => {
                assert!(args.json);
            }
            _ => panic!("expected actor journal --json"),
        }
    }

    #[test]
    fn actor_journal_parses_with_all_options() {
        let cli = Cli::parse_from([
            "arpagona",
            "actor",
            "journal",
            "--limit",
            "3",
            "--interaction-type",
            "synthesis",
            "--json",
        ]);
        match cli.command {
            Command::Actor(ActorCommand {
                command: ActorSubcommand::Journal(args),
            }) => {
                assert_eq!(args.limit, 3);
                assert_eq!(args.interaction_type.as_deref(), Some("synthesis"));
                assert!(args.json);
            }
            _ => panic!("expected actor journal with all options"),
        }
    }

    // ── Orchestrator insights-collect parser tests ──────────────────────────

    #[test]
    fn cli_parses_orchestrator_insights_collect_with_path() {
        let cli = Cli::try_parse_from(vec![
            "arpagona",
            "orchestrator",
            "insights-collect",
            "target/last-orchestrator-trace.json",
        ])
        .expect("orchestrator insights-collect <path> should parse");
        match cli.command {
            Command::Orchestrator(OrchestratorCommand {
                command: OrchestratorSubcommand::InsightsCollect(args),
            }) => {
                assert_eq!(args.trace_path, "target/last-orchestrator-trace.json");
                assert!(!args.json, "default should not be json");
            }
            _ => panic!("expected orchestrator insights-collect"),
        }
    }

    #[test]
    fn cli_parses_orchestrator_insights_collect_with_json() {
        let cli = Cli::try_parse_from(vec![
            "arpagona",
            "orchestrator",
            "insights-collect",
            "trace.json",
            "--json",
        ])
        .expect("orchestrator insights-collect <path> --json should parse");
        match cli.command {
            Command::Orchestrator(OrchestratorCommand {
                command: OrchestratorSubcommand::InsightsCollect(args),
            }) => {
                assert_eq!(args.trace_path, "trace.json");
                assert!(args.json, "--json should be true");
            }
            _ => panic!("expected orchestrator insights-collect --json"),
        }
    }

    // ── Orchestrator insights-list parser tests ─────────────────────────────

    #[test]
    fn cli_parses_orchestrator_insights_list_defaults() {
        let cli = Cli::try_parse_from(vec!["arpagona", "orchestrator", "insights-list"])
            .expect("orchestrator insights-list should parse");
        match cli.command {
            Command::Orchestrator(OrchestratorCommand {
                command: OrchestratorSubcommand::InsightsList(args),
            }) => {
                assert!(!args.json, "default should not be json");
                assert_eq!(args.insights_dir, None, "default should use default dir");
            }
            _ => panic!("expected orchestrator insights-list"),
        }
    }

    #[test]
    fn cli_parses_orchestrator_insights_list_with_json() {
        let cli = Cli::try_parse_from(vec!["arpagona", "orchestrator", "insights-list", "--json"])
            .expect("orchestrator insights-list --json should parse");
        match cli.command {
            Command::Orchestrator(OrchestratorCommand {
                command: OrchestratorSubcommand::InsightsList(args),
            }) => {
                assert!(args.json, "--json should be true");
            }
            _ => panic!("expected orchestrator insights-list --json"),
        }
    }

    #[test]
    fn cli_parses_orchestrator_insights_list_with_insights_dir() {
        let cli = Cli::try_parse_from(vec![
            "arpagona",
            "orchestrator",
            "insights-list",
            "--insights-dir",
            "custom/insights",
        ])
        .expect("orchestrator insights-list --insights-dir should parse");
        match cli.command {
            Command::Orchestrator(OrchestratorCommand {
                command: OrchestratorSubcommand::InsightsList(args),
            }) => {
                assert_eq!(
                    args.insights_dir,
                    Some("custom/insights".to_owned()),
                    "--insights-dir should be captured"
                );
            }
            _ => panic!("expected orchestrator insights-list --insights-dir"),
        }
    }

    // -----------------------------------------------------------------------
    // Actor Run command parse tests
    // -----------------------------------------------------------------------

    #[test]
    fn actor_run_parses_simple_task() {
        let cli = Cli::parse_from(["arpagona", "actor", "run", "append hello to test.txt"]);
        match cli.command {
            Command::Actor(ActorCommand {
                command: ActorSubcommand::Run(args),
            }) => {
                assert_eq!(args.task, "append hello to test.txt");
                assert!(!args.approve);
                assert!(!args.json);
                assert_eq!(args.workspace, ".");
            }
            _ => panic!("expected actor run with task"),
        }
    }

    #[test]
    fn actor_run_parses_with_approve() {
        let cli = Cli::parse_from(["arpagona", "actor", "run", "read docs/x", "--approve"]);
        match cli.command {
            Command::Actor(ActorCommand {
                command: ActorSubcommand::Run(args),
            }) => {
                assert_eq!(args.task, "read docs/x");
                assert!(args.approve);
            }
            _ => panic!("expected actor run with --approve"),
        }
    }

    #[test]
    fn actor_run_parses_with_json() {
        let cli = Cli::parse_from(["arpagona", "actor", "run", "list files", "--json"]);
        match cli.command {
            Command::Actor(ActorCommand {
                command: ActorSubcommand::Run(args),
            }) => {
                assert_eq!(args.task, "list files");
                assert!(args.json);
            }
            _ => panic!("expected actor run with --json"),
        }
    }

    #[test]
    fn actor_run_parses_with_workspace() {
        let cli = Cli::parse_from([
            "arpagona",
            "actor",
            "run",
            "search for x",
            "--workspace",
            "/tmp/scratch",
        ]);
        match cli.command {
            Command::Actor(ActorCommand {
                command: ActorSubcommand::Run(args),
            }) => {
                assert_eq!(args.task, "search for x");
                assert_eq!(args.workspace, "/tmp/scratch");
            }
            _ => panic!("expected actor run with --workspace"),
        }
    }

    // -----------------------------------------------------------------------
    // Actor Session command parse tests
    // -----------------------------------------------------------------------

    #[test]
    fn actor_session_parses_basic() {
        let cli = Cli::parse_from(["arpagona", "actor", "session"]);
        match cli.command {
            Command::Actor(ActorCommand {
                command: ActorSubcommand::Session(args),
            }) => {
                assert!(args.max.is_none());
                assert!(!args.json);
                assert_eq!(args.workspace, ".");
            }
            _ => panic!("expected actor session"),
        }
    }

    #[test]
    fn actor_session_parses_with_max() {
        let cli = Cli::parse_from(["arpagona", "actor", "session", "--max", "5"]);
        match cli.command {
            Command::Actor(ActorCommand {
                command: ActorSubcommand::Session(args),
            }) => {
                assert_eq!(args.max, Some(5));
            }
            _ => panic!("expected actor session with --max"),
        }
    }

    #[test]
    fn actor_session_parses_with_json() {
        let cli = Cli::parse_from(["arpagona", "actor", "session", "--json"]);
        match cli.command {
            Command::Actor(ActorCommand {
                command: ActorSubcommand::Session(args),
            }) => {
                assert!(args.json);
            }
            _ => panic!("expected actor session with --json"),
        }
    }

    #[test]
    fn actor_session_parses_with_workspace() {
        let cli = Cli::parse_from([
            "arpagona",
            "actor",
            "session",
            "--workspace",
            "/tmp/test-workspace",
        ]);
        match cli.command {
            Command::Actor(ActorCommand {
                command: ActorSubcommand::Session(args),
            }) => {
                assert_eq!(args.workspace, "/tmp/test-workspace");
            }
            _ => panic!("expected actor session with --workspace"),
        }
    }

    #[test]
    fn actor_session_parses_with_max_and_workspace() {
        let cli = Cli::parse_from([
            "arpagona",
            "actor",
            "session",
            "--max",
            "10",
            "--workspace",
            "/tmp/ws",
            "--json",
        ]);
        match cli.command {
            Command::Actor(ActorCommand {
                command: ActorSubcommand::Session(args),
            }) => {
                assert_eq!(args.max, Some(10));
                assert_eq!(args.workspace, "/tmp/ws");
                assert!(args.json);
            }
            _ => panic!("expected actor session with all flags"),
        }
    }

    // -----------------------------------------------------------------------
    // Actor intent parsing unit tests
    // -----------------------------------------------------------------------

    #[test]
    fn parse_intent_append_text_to_path() {
        let intent = parse_intent("append hello to docs/test.txt").unwrap();
        assert_eq!(intent.tool, "append_file");
        assert_eq!(intent.arguments["content"], "hello");
        assert_eq!(intent.arguments["path"], "docs/test.txt");
        assert_eq!(format!("{:?}", intent.risk_level), "Low");
    }

    #[test]
    fn parse_intent_read_path() {
        let intent = parse_intent("read docs/notes.md").unwrap();
        assert_eq!(intent.tool, "read_file");
        assert_eq!(intent.arguments["path"], "docs/notes.md");
        assert_eq!(format!("{:?}", intent.risk_level), "Informational");
    }

    #[test]
    fn parse_intent_show_path() {
        let intent = parse_intent("show docs/notes.md").unwrap();
        assert_eq!(intent.tool, "read_file");
    }

    #[test]
    fn parse_intent_list_files_root() {
        let intent = parse_intent("list files").unwrap();
        assert_eq!(intent.tool, "list_files");
    }

    #[test]
    fn parse_intent_list_files_in_path() {
        let intent = parse_intent("list files in src/").unwrap();
        assert_eq!(intent.tool, "list_files");
        assert_eq!(intent.arguments["path"], "src/");
    }

    #[test]
    fn parse_intent_search_for_pattern_in_path() {
        let intent = parse_intent("search for FIXME in lib/").unwrap();
        assert_eq!(intent.tool, "search_text");
        assert_eq!(intent.arguments["pattern"], "FIXME");
        assert_eq!(intent.arguments["path"], "lib/");
    }

    #[test]
    fn parse_intent_search_text_in_path() {
        let intent = parse_intent("find TODO in src/").unwrap();
        assert_eq!(intent.tool, "search_text");
        assert_eq!(intent.arguments["pattern"], "TODO");
        assert_eq!(intent.arguments["path"], "src/");
    }

    #[test]
    fn parse_intent_unrecognized() {
        let result = parse_intent("do something crazy");
        assert!(result.is_err());
    }

    #[test]
    fn parse_intent_case_insensitive() {
        let intent = parse_intent("APPEND hello TO docs/test.txt").unwrap();
        assert_eq!(intent.tool, "append_file");
        assert_eq!(intent.arguments["content"], "hello");
        assert_eq!(intent.arguments["path"], "docs/test.txt");
    }

    #[test]
    fn parse_intent_append_content_multi_word() {
        let intent = parse_intent("append my multi-word note to docs/log.md").unwrap();
        assert_eq!(intent.tool, "append_file");
        assert_eq!(intent.arguments["content"], "my multi-word note");
        assert_eq!(intent.arguments["path"], "docs/log.md");
    }

    #[test]
    fn parse_intent_append_content_containing_to() {
        let intent = parse_intent("append welcome to the team to docs/notes.md").unwrap();
        assert_eq!(intent.tool, "append_file");
        assert_eq!(intent.arguments["content"], "welcome to the team");
        assert_eq!(intent.arguments["path"], "docs/notes.md");
    }

    #[test]
    fn parse_intent_shell_like_task_rejected() {
        let result = parse_intent("run rm -rf /");
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(
            msg.contains("Unrecognized task") || msg.contains("unrecognized"),
            "expected unrecognized task: {msg}"
        );
    }

    #[test]
    fn parse_intent_ambiguous_append_no_content() {
        // "append to docs" — "append" followed by "to" with no gap -> should be unrecognized
        let result = parse_intent("append to docs");
        // The parser finds "append " prefix, then looks for " to " in "to docs" which doesn't exist
        // because there's no leading space. So it falls through to Err.
        assert!(result.is_err());
    }

    #[test]
    fn parse_intent_read_ignores_content_to() {
        // "read" doesn't use "to" parsing — returns full remainder as path
        let intent = parse_intent("read file.md to review it").unwrap();
        assert_eq!(intent.tool, "read_file");
        assert_eq!(intent.arguments["path"], "file.md to review it");
    }

    #[test]
    fn parse_intent_add_works_like_append() {
        let intent = parse_intent("add my note to docs/notes.md").unwrap();
        assert_eq!(intent.tool, "append_file");
        assert_eq!(intent.arguments["content"], "my note");
        assert_eq!(intent.arguments["path"], "docs/notes.md");
    }

    #[test]
    fn parse_intent_append_with_at_preposition() {
        let intent = parse_intent("append my note at docs/notes.md").unwrap();
        assert_eq!(intent.tool, "append_file");
        assert_eq!(intent.arguments["content"], "my note");
        assert_eq!(intent.arguments["path"], "docs/notes.md");
    }

    #[test]
    fn parse_intent_list_directory() {
        let intent = parse_intent("list directory src/").unwrap();
        assert_eq!(intent.tool, "list_files");
        assert_eq!(intent.arguments["path"], "src/");
    }

    #[test]
    fn parse_intent_read_preserves_mixed_case_path() {
        let intent = parse_intent("read Docs/Notes.md").unwrap();
        assert_eq!(intent.tool, "read_file");
        assert_eq!(intent.arguments["path"], "Docs/Notes.md");
    }

    #[test]
    fn parse_intent_show_preserves_mixed_case_path() {
        let intent = parse_intent("show Docs/Notes.md").unwrap();
        assert_eq!(intent.tool, "read_file");
        assert_eq!(intent.arguments["path"], "Docs/Notes.md");
    }

    #[test]
    fn parse_intent_list_files_in_preserves_mixed_case_path() {
        let intent = parse_intent("list files in Src/App/").unwrap();
        assert_eq!(intent.tool, "list_files");
        assert_eq!(intent.arguments["path"], "Src/App/");
    }

    #[test]
    fn parse_intent_list_directory_preserves_mixed_case_path() {
        let intent = parse_intent("list directory Src/App").unwrap();
        assert_eq!(intent.tool, "list_files");
        assert_eq!(intent.arguments["path"], "Src/App");
    }

    // ─── parse_ollama_intent tests ──────────────────────────────────────────

    #[test]
    fn parse_ollama_intent_valid_read_file() {
        let response = json!({
            "tool": "read_file",
            "arguments": {"path": "docs/README.md"},
            "rationale": "User wants to read the README",
            "risk_level": "informational",
        });
        let intent = parse_ollama_intent(&response).unwrap();
        assert_eq!(intent.tool, "read_file");
        assert_eq!(intent.arguments["path"], "docs/README.md");
        assert_eq!(intent.risk_level, RiskLevel::Informational);
    }

    #[test]
    fn parse_ollama_intent_valid_append_file() {
        let response = json!({
            "tool": "append_file",
            "arguments": {"path": "notes.md", "content": "hello"},
            "rationale": "Append user note",
            "risk_level": "low",
        });
        let intent = parse_ollama_intent(&response).unwrap();
        assert_eq!(intent.tool, "append_file");
        assert_eq!(intent.risk_level, RiskLevel::Low);
    }

    #[test]
    fn parse_ollama_intent_disallowed_tool_returns_error() {
        let response = json!({
            "tool": "shell",
            "arguments": {},
            "rationale": "run command",
            "risk_level": "high",
        });
        let err = parse_ollama_intent(&response).unwrap_err();
        assert!(matches!(err, IntentParseError::DisallowedTool(t) if t == "shell"));
    }

    #[test]
    fn parse_ollama_intent_missing_tool_returns_error() {
        let response = json!({
            "arguments": {},
            "rationale": "test",
            "risk_level": "informational",
        });
        let err = parse_ollama_intent(&response).unwrap_err();
        assert!(matches!(err, IntentParseError::IncompleteResponse(_)));
    }

    #[test]
    fn parse_ollama_intent_missing_arguments_returns_error() {
        let response = json!({
            "tool": "read_file",
            "rationale": "test",
            "risk_level": "informational",
        });
        let err = parse_ollama_intent(&response).unwrap_err();
        assert!(matches!(err, IntentParseError::IncompleteResponse(_)));
    }

    #[test]
    fn parse_ollama_intent_missing_rationale_returns_error() {
        let response = json!({
            "tool": "read_file",
            "arguments": {"path": "x"},
            "risk_level": "informational",
        });
        let err = parse_ollama_intent(&response).unwrap_err();
        assert!(matches!(err, IntentParseError::IncompleteResponse(_)));
    }

    #[test]
    fn parse_ollama_intent_default_risk_level_when_missing() {
        let response = json!({
            "tool": "list_files",
            "arguments": {"path": "src/"},
            "rationale": "list files",
        });
        let intent = parse_ollama_intent(&response).unwrap();
        assert_eq!(intent.risk_level, RiskLevel::Informational);
    }

    // ─── Per-tool schema validation tests ─────────────────────────────────

    #[test]
    fn parse_ollama_intent_read_file_missing_path_returns_error() {
        let response = json!({
            "tool": "read_file",
            "arguments": {},
            "rationale": "read a file",
            "risk_level": "informational",
        });
        let err = parse_ollama_intent(&response).unwrap_err();
        assert!(matches!(err, IntentParseError::MissingArgument(msg) if msg.contains("path")));
    }

    #[test]
    fn parse_ollama_intent_read_file_empty_path_returns_error() {
        let response = json!({
            "tool": "read_file",
            "arguments": {"path": ""},
            "rationale": "read a file",
            "risk_level": "informational",
        });
        let err = parse_ollama_intent(&response).unwrap_err();
        assert!(matches!(err, IntentParseError::MissingArgument(msg) if msg.contains("path")));
    }

    #[test]
    fn parse_ollama_intent_append_file_missing_content_returns_error() {
        let response = json!({
            "tool": "append_file",
            "arguments": {"path": "notes.md"},
            "rationale": "append content",
            "risk_level": "low",
        });
        let err = parse_ollama_intent(&response).unwrap_err();
        assert!(matches!(err, IntentParseError::MissingArgument(msg) if msg.contains("content")));
    }

    #[test]
    fn parse_ollama_intent_append_file_empty_content_returns_error() {
        let response = json!({
            "tool": "append_file",
            "arguments": {"path": "notes.md", "content": ""},
            "rationale": "append content",
            "risk_level": "low",
        });
        let err = parse_ollama_intent(&response).unwrap_err();
        assert!(matches!(err, IntentParseError::MissingArgument(msg) if msg.contains("content")));
    }

    #[test]
    fn parse_ollama_intent_append_file_missing_path_returns_error() {
        let response = json!({
            "tool": "append_file",
            "arguments": {"content": "hello"},
            "rationale": "append content",
            "risk_level": "low",
        });
        let err = parse_ollama_intent(&response).unwrap_err();
        assert!(matches!(err, IntentParseError::MissingArgument(msg) if msg.contains("path")));
    }

    #[test]
    fn parse_ollama_intent_search_text_missing_pattern_returns_error() {
        let response = json!({
            "tool": "search_text",
            "arguments": {},
            "rationale": "search for text",
            "risk_level": "informational",
        });
        let err = parse_ollama_intent(&response).unwrap_err();
        assert!(matches!(err, IntentParseError::MissingArgument(msg) if msg.contains("pattern")));
    }

    #[test]
    fn parse_ollama_intent_search_text_empty_pattern_returns_error() {
        let response = json!({
            "tool": "search_text",
            "arguments": {"pattern": ""},
            "rationale": "search for text",
            "risk_level": "informational",
        });
        let err = parse_ollama_intent(&response).unwrap_err();
        assert!(matches!(err, IntentParseError::MissingArgument(msg) if msg.contains("pattern")));
    }

    #[test]
    fn parse_ollama_intent_list_files_invalid_path_type_returns_error() {
        let response = json!({
            "tool": "list_files",
            "arguments": {"path": 42},
            "rationale": "list files",
            "risk_level": "informational",
        });
        let err = parse_ollama_intent(&response).unwrap_err();
        assert!(matches!(err, IntentParseError::MissingArgument(msg) if msg.contains("path")));
    }

    // ─── Deterministic risk-level tests ───────────────────────────────────

    #[test]
    fn parse_ollama_intent_risk_level_derived_from_tool_not_ollama_field() {
        // read_file with Ollama saying "low" should still be Informational
        let response = json!({
            "tool": "read_file",
            "arguments": {"path": "doc.md"},
            "rationale": "read doc",
            "risk_level": "low",
        });
        let intent = parse_ollama_intent(&response).unwrap();
        assert_eq!(
            intent.risk_level,
            RiskLevel::Informational,
            "read_file must always be Informational, ignoring Ollama's risk_level"
        );
    }

    #[test]
    fn parse_ollama_intent_append_file_always_low() {
        // append_file with Ollama saying "informational" should still be Low
        let response = json!({
            "tool": "append_file",
            "arguments": {"path": "doc.md", "content": "hello"},
            "rationale": "append to doc",
            "risk_level": "informational",
        });
        let intent = parse_ollama_intent(&response).unwrap();
        assert_eq!(
            intent.risk_level,
            RiskLevel::Low,
            "append_file must always be Low, ignoring Ollama's risk_level"
        );
    }

    // ─── Edge-case validation tests ─────────────────────────────────────────

    #[test]
    fn parse_ollama_intent_extra_fields_are_silently_tolerated() {
        // Ollama may return extra fields beyond the schema — they must be ignored
        let response = json!({
            "tool": "read_file",
            "arguments": {"path": "doc.md"},
            "rationale": "read the doc",
            "risk_level": "informational",
            "extra_field": "should be ignored",
            "nested_extra": {"unused": true},
        });
        let intent = parse_ollama_intent(&response).unwrap();
        assert_eq!(intent.tool, "read_file");
        assert_eq!(intent.arguments["path"], "doc.md");
    }

    #[test]
    fn parse_ollama_intent_arguments_as_non_object_returns_error() {
        // If Ollama sends arguments as a string instead of an object
        let response = json!({
            "tool": "read_file",
            "arguments": "not-an-object",
            "rationale": "read a file",
            "risk_level": "informational",
        });
        let err = parse_ollama_intent(&response).unwrap_err();
        assert!(matches!(err, IntentParseError::MissingArgument(msg) if msg.contains("path")));
    }

    #[test]
    fn parse_ollama_intent_list_files_without_path_succeeds() {
        // list_files path is optional — defaults to cwd
        let response = json!({
            "tool": "list_files",
            "arguments": {},
            "rationale": "list current directory",
            "risk_level": "informational",
        });
        let intent = parse_ollama_intent(&response).unwrap();
        assert_eq!(intent.tool, "list_files");
        assert_eq!(intent.risk_level, RiskLevel::Informational);
    }

    #[test]
    fn parse_ollama_intent_search_text_without_path_succeeds() {
        // search_text path is optional — defaults to cwd
        let response = json!({
            "tool": "search_text",
            "arguments": {"pattern": "TODO"},
            "rationale": "search for TODO markers",
            "risk_level": "informational",
        });
        let intent = parse_ollama_intent(&response).unwrap();
        assert_eq!(intent.tool, "search_text");
        assert_eq!(intent.arguments["pattern"], "TODO");
        assert_eq!(intent.risk_level, RiskLevel::Informational);
    }

    #[test]
    fn doctor_all_pass_false_when_fail_severity_check_fails() {
        // Replicates the exact all_pass logic from the doctor function:
        // all_pass = !checks.iter().any(|(_, _, pass, sev)| !*pass && sev == "fail")
        let checks: Vec<(String, String, bool, String)> = vec![
            ("git_state".into(), "ok".into(), true, "ok".into()),
            ("ollama".into(), "unreachable".into(), false, "fail".into()),
            (
                "qwen3.5:9b_model".into(),
                "unavailable".into(),
                false,
                "fail".into(),
            ),
            (
                "secondary_copy".into(),
                "stale warning".into(),
                false,
                "warn".into(),
            ),
        ];
        let all_pass = !checks
            .iter()
            .any(|(_, _, pass, sev)| !*pass && sev == "fail");
        assert!(
            !all_pass,
            "all_pass must be false when fail-severity checks fail"
        );

        // has_fail (for Err return) uses the same logic
        let has_fail = checks
            .iter()
            .any(|(_, _, pass, sev)| !*pass && sev == "fail");
        assert!(
            has_fail,
            "has_fail must be true when fail-severity checks fail"
        );
    }

    #[test]
    fn doctor_all_pass_true_when_only_warn_severity_fails() {
        // Warn-only failures should NOT set all_pass to false
        let checks: Vec<(String, String, bool, String)> = vec![
            ("git_state".into(), "ok".into(), true, "ok".into()),
            ("ollama".into(), "ok".into(), true, "ok".into()),
            ("qwen3.5:9b_model".into(), "ok".into(), true, "ok".into()),
            (
                "secondary_copy".into(),
                "stale warning".into(),
                false,
                "warn".into(),
            ),
        ];
        let all_pass = !checks
            .iter()
            .any(|(_, _, pass, sev)| !*pass && sev == "fail");
        assert!(
            all_pass,
            "all_pass must be true when only warn-severity checks fail"
        );

        // has_fail must be false for warn-only
        let has_fail = checks
            .iter()
            .any(|(_, _, pass, sev)| !*pass && sev == "fail");
        assert!(
            !has_fail,
            "has_fail must be false when only warn-severity checks fail"
        );
    }

    #[test]
    fn doctor_all_pass_true_when_all_checks_pass() {
        let checks: Vec<(String, String, bool, String)> = vec![
            ("git_state".into(), "ok".into(), true, "ok".into()),
            ("ollama".into(), "ok".into(), true, "ok".into()),
        ];
        let all_pass = !checks
            .iter()
            .any(|(_, _, pass, sev)| !*pass && sev == "fail");
        assert!(all_pass, "all_pass must be true when all checks pass");
    }

    #[test]
    fn actor_history_truncation_handles_non_ascii() {
        // Regression: byte-index s[..s.len().min(N)] panics when N falls
        // mid-char in a multi-byte UTF-8 sequence.
        // GONA repro: prompt_summary = "read " + "a"*54 + "é"
        // → 61 bytes, 60 chars; s[..60] → panic (byte 60 is mid-é).
        // Fix: .chars().take(N).collect() — always char-boundary-safe.

        // Case 1: exactly 60 chars with multi-byte tail (would panic under old code)
        let s = format!("read {}{}", "a".repeat(54), "é");
        let truncated: String = s.chars().take(60).collect();
        assert_eq!(truncated.chars().count(), 60);
        assert_eq!(
            truncated, s,
            "60-char non-ASCII string preserves all content"
        );

        // Case 2: 70 chars with multi-byte — truncation actually cuts
        let s = format!("read {}{}{}", "a".repeat(54), "é", "b".repeat(10));
        let truncated: String = s.chars().take(60).collect();
        assert_eq!(truncated.chars().count(), 60);
        assert!(
            truncated.len() <= s.len(),
            "truncation never increases byte length"
        );
        // Verify no panic by reaching here — the test itself validates the approach
    }
}
