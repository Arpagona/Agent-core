use arpagona_agent_core::{
    AuditEvent, AuditTraceSummary, Decision, DecisionId, DecisionStatus, ProposedAction,
    ProposedActionStatus, Task, TaskId, WorkspaceId,
};
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
    Status,
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
    risk_level: Option<String>,
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

const AUDIT_READBACK_WARNING: &str =
    "Readback only: this summary is not approval, authorization, orchestration, or execution state.";

#[derive(Debug, PartialEq, Eq)]
enum ChatLine {
    Empty,
    Help,
    Quit,
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
        Command::Status => status(&client, &api_url).await?,
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

async fn status(client: &Client, api_url: &str) -> Result<(), Box<dyn Error>> {
    let readback = status_readback(client, api_url).await;
    print_status_readback(&readback);
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
    let permissions = normalize_permissions(args.permissions);
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
                "payload": default_payload(&args.action_type),
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
        risk_level: metadata.risk_level,
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
    risk_level: Option<String>,
    policies_applied: Vec<String>,
}

fn decision_readback_metadata(events: &[AuditEvent]) -> AuditDecisionMetadata {
    let mut metadata = AuditDecisionMetadata::default();

    for event in events {
        let trace = event.payload.get("causal_trace").unwrap_or(&event.payload);

        if metadata.decision_status.is_none() {
            metadata.decision_status = string_field(trace, "decision_status");
        }
        if metadata.risk_level.is_none() {
            metadata.risk_level = string_field(trace, "risk_level");
        }
        if metadata.policies_applied.is_empty() {
            metadata.policies_applied = string_array_field(trace, "policies_applied");
        }
    }

    metadata
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
        "risk_level:",
        readback.risk_level.as_deref().unwrap_or("-"),
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

fn default_payload(action_type: &str) -> Value {
    if action_type == "simulate_email" {
        json!({
            "to": "client@example.com",
            "subject": "Simulation alpha ARPAGONA",
            "body": "Préparer un brouillon sans l’envoyer"
        })
    } else {
        json!({})
    }
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
                        "risk_level": "high",
                        "policies_applied": ["policy-human-approval"]
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

        let formatted = format_audit_decision_readback(&readback);
        assert!(formatted.contains("decision_status:"));
        assert!(formatted.contains("needs_human_approval"));
        assert!(formatted.contains("risk_level:"));
        assert!(formatted.contains("high"));
        assert!(formatted.contains("policies_applied:"));
        assert!(formatted.contains("policy-human-approval"));
        assert!(formatted.contains("Readback only"));

        let json = serde_json::to_value(&readback).unwrap();
        assert_eq!(json["decision_status"], "needs_human_approval");
        assert_eq!(json["risk_level"], "high");
        assert_eq!(json["policies_applied"], json!(["policy-human-approval"]));
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
        let cli = Cli::parse_from(["arpagona", "status"]);
        assert!(matches!(cli.command, Command::Status));
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
