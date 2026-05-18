use arpagona_agent_core::{AuditEvent, ProposedAction, Task};
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
}

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
    proposed_action: ProposedAction,
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
                    println!("{}", style_error("Unsupported provider. Use mock or openai."));
                }
            }
            ChatLine::UnknownCommand(command) => {
                println!("{}", style_warning(&format!("Unknown command: {command}. Type /help.")));
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
                    Ok(response) => print_agent_proposal(&response.proposed_action)?,
                    Err(error) => println!("{}", format_provider_error(&provider, &error.to_string())),
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
            println!("{} {}", style_success("OPENAI_API_KEY:"), mask_openai_key(&key));
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
            println!("{} not set, provider default will be used", style_dim("OPENAI_MODEL:"));
        }
    }
}

fn auth_openai_instructions() {
    println!("{}", rainbow_text("Configure OpenAI for ARPAGONA"));
    println!("{}", style_dim("Alpha uses API key auth. Full OAuth is post-alpha / provider-dependent."));
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
    println!("{}", rainbow_text("╔════════════════════════════════════════════════════════════╗"));
    println!("{}", rainbow_text("║                 ARPAGONA AGENT CORE                      ║"));
    println!("{}", rainbow_text("║            Cognitive Runtime Alpha Terminal              ║"));
    println!("{}", rainbow_text("╚════════════════════════════════════════════════════════════╝"));
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
    println!("{}", style_dim("Type /help for commands. Nothing is executed directly."));
    println!();
}

fn print_chat_help() {
    println!("{}", style_info("Commands:"));
    println!("  {}                 Show this help", style_command("/help"));
    println!("  {} | {}         Leave chat", style_command("/quit"), style_command("/exit"));
    println!("  {}                List tasks", style_command("/tasks"));
    println!("  {}              List proposed actions", style_command("/actions"));
    println!("  {}    Evaluate a proposed action", style_command("/evaluate action-1"));
    println!("  {}                List audit events", style_command("/audit"));
    println!("  {}        Use mock provider", style_command("/provider mock"));
    println!("  {}      Use OpenAI provider", style_command("/provider openai"));
    println!("\n{}", style_dim("Any other text is sent to /agent/propose and returns a pending ProposedAction."));
}

async fn health(client: &Client, api_url: &str) -> Result<(), Box<dyn Error>> {
    let response: HealthResponse =
        get_json(client.get(format!("{api_url}/health")).send().await?).await?;
    println!("{} {}", style_success("ARPAGONA API:"), response.status);
    Ok(())
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
        println!("  {} {}", style_dim("status:"), to_api_string(&task.status)?);
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

    println!("{} {}", style_success("Created proposed action:"), response.id);
    println!("{} {}", style_dim("Status:"), to_api_string(&response.status)?);
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

    print_agent_proposal(&response.proposed_action)
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

fn print_agent_proposal(action: &ProposedAction) -> Result<(), Box<dyn Error>> {
    println!("{}", style_info("ProposedAction"));
    println!("{} {}", style_dim("id:"), action.id);
    println!("{} {}", style_dim("type:"), to_api_string(&action.action_type)?);
    println!("{} {}", style_dim("risk:"), style_risk(&to_api_string(&action.risk_level)?));
    println!("{} {}", style_dim("status:"), style_status(&to_api_string(&action.status)?));
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
    println!("{} {}", style_success("Decision:"), style_status(&response.decision.status));
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
        println!("  {} {}", style_dim("action_type:"), to_api_string(&action.action_type)?);
        println!("  {} {}", style_dim("risk_level:"), style_risk(&to_api_string(&action.risk_level)?));
        println!("  {} {}", style_dim("status:"), style_status(&to_api_string(&action.status)?));
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
        println!("  {} {}", style_dim("event_type:"), to_api_string(&event.event_type)?);
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
        "read_memory" | "write_memory" | "read_document" | "write_document"
        | "propose_tool_use" | "simulate_email" | "manage_task" => json!(action_type),
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

fn style_command(text: &str) -> String {
    format!("{ANSI_BOLD}{}{ANSI_RESET}", style_text(text, TermColor::Magenta))
}

fn style_prompt(text: &str) -> String {
    format!("{ANSI_BOLD}{}{ANSI_RESET}", style_text(text, TermColor::Cyan))
}

fn style_dim(text: &str) -> String {
    format!("{ANSI_DIM}{}{ANSI_RESET}", style_text(text, TermColor::Gray))
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
        output.push_str(&style_text(&character.to_string(), colors[color_index % colors.len()]));
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
