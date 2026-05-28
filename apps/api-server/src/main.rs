use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use arpagona_agent_core::{
    execution_capability, list_execution_capabilities, ActionType, ActorRef, AgentId, AuditEvent,
    AuditEventId, AuditEventType, Decision, DecisionActor, DecisionId, DecisionStatus,
    DryRunResult, DryRunStatus, ExecutionCapability, ExecutionRequest, ExecutionResult,
    ExecutorRegistry, ExecutorState, Permission, PolicyDecision, PolicyEngine, PolicyEngineResult,
    PolicyInput, ProposedAction, ProposedActionId, ProposedActionStatus, RiskLevel, Task, TaskId,
    TaskPriority, TaskStatus, WorkspaceId,
};
use arpagona_decision_gate::{
    audit_event_for_decision, evaluate_proposed_action, override_engine::Argon2PasswordVerifier,
    override_engine::DefaultHasherVerifier, override_engine::OverrideEngine,
    override_engine::OverrideOutcome, override_engine::PasswordVerifier,
};
use arpagona_llm::{
    deterministic_turn_for_prompt, AgentTurnDraft, LlmActionRequest, LlmProvider, MockProvider,
    OllamaProvider, OpenAiProvider,
};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::str::FromStr;

#[derive(Clone, Default)]
pub struct AppState {
    store: Arc<Mutex<InMemoryStore>>,
}

struct InMemoryStore {
    tasks: Vec<Task>,
    proposed_actions: Vec<ProposedAction>,
    decisions: Vec<Decision>,
    audit_events: Vec<AuditEvent>,
    sandbox_runs: Vec<SandboxRun>,
    dry_run_results: Vec<DryRunResult>,
    executor_registry: ExecutorRegistry,
    override_engine: Option<OverrideEngine<Box<dyn PasswordVerifier>>>,
}

impl Default for InMemoryStore {
    fn default() -> Self {
        // Priority 1: ARPAGONA_OVERRIDE_PASSWORD_HASH → Argon2 (production)
        // Priority 2: ARPAGONA_OVERRIDE_PASSWORD + ARPAGONA_ALLOW_DEV_OVERRIDE=true → DefaultHasher (dev only)
        // Otherwise: override disabled
        let override_engine = match Argon2PasswordVerifier::from_env_hash() {
            Some(verifier) => Some(OverrideEngine::new(
                Box::new(verifier) as Box<dyn PasswordVerifier>
            )),
            None => {
                let override_password = std::env::var("ARPAGONA_OVERRIDE_PASSWORD");
                let allow_dev =
                    std::env::var("ARPAGONA_ALLOW_DEV_OVERRIDE").is_ok_and(|v| v == "true");
                match &override_password {
                    Ok(password) if !password.is_empty() && allow_dev => Some(OverrideEngine::new(
                        Box::new(DefaultHasherVerifier::new(password, "arpagona-alpha-salt"))
                            as Box<dyn PasswordVerifier>,
                    )),
                    _ if allow_dev => Some(OverrideEngine::new(
                        Box::new(DefaultHasherVerifier::new(
                            "alpha-override-password",
                            "arpagona-alpha-salt",
                        )) as Box<dyn PasswordVerifier>,
                    )),
                    _ => None,
                }
            }
        };

        Self {
            tasks: Vec::new(),
            proposed_actions: Vec::new(),
            decisions: Vec::new(),
            audit_events: Vec::new(),
            sandbox_runs: Vec::new(),
            dry_run_results: Vec::new(),
            executor_registry: ExecutorRegistry::default(),
            override_engine,
        }
    }
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
}

#[derive(Deserialize)]
struct CreateTaskRequest {
    workspace_id: String,
    title: String,
    description: Option<String>,
}

#[derive(Deserialize)]
struct CreateProposedActionRequest {
    workspace_id: String,
    task_id: Option<String>,
    proposed_by: String,
    action_type: ActionType,
    target: Option<String>,
    risk_level: RiskLevel,
    required_permissions: Vec<Permission>,
    rationale: String,
    #[serde(default = "empty_payload")]
    payload: Value,
}

#[derive(Deserialize)]
struct EvaluateDecisionGateRequest {
    proposed_action_id: String,
    granted_permissions: Vec<Permission>,
}

#[derive(Serialize)]
struct EvaluateDecisionGateResponse {
    decision: Decision,
    audit_event: AuditEvent,
}

#[derive(Deserialize)]
struct AgentProposeRequest {
    workspace_id: String,
    task_id: Option<String>,
    prompt: String,
    provider: String,
}

#[derive(Deserialize)]
struct ReviewActionRequest {
    /// "approve", "reject", or "defer"
    action: String,
    /// Optional human-readable reason for the review decision
    reason: Option<String>,
    /// Actor identifier (e.g. "human-thibaud")
    actor: String,
}

#[derive(Serialize)]
struct ReviewActionResponse {
    proposed_action: ProposedAction,
    audit_event: AuditEvent,
}

#[derive(Deserialize)]
struct OverrideActionRequest {
    /// The override password.
    password: String,
    /// Actor identifier (e.g. "admin-thibaud").
    actor: String,
}

#[derive(Serialize)]
struct OverrideActionResponse {
    decision: Decision,
    audit_event: AuditEvent,
    outcome: String,
}

#[derive(Debug, Serialize)]
struct AgentProposeResponse {
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    proposed_action: Option<ProposedAction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    question: Option<String>,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

/// Response for a single executor in list/get-own-state endpoints.
#[derive(Serialize)]
struct ExecutorInfoResponse {
    executor_id: String,
    executor_state: ExecutorState,
    supported_action_types: Vec<ActionType>,
}

/// Request body for changing an executor's state.
#[derive(Deserialize)]
struct SetExecutorStateRequest {
    state: ExecutorState,
}

/// Response after setting an executor's state.
#[derive(Debug, Serialize)]
struct SetExecutorStateResponse {
    executor_id: String,
    executor_state: ExecutorState,
}

#[tokio::main]
async fn main() {
    let app = app(AppState::default());
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("api server should bind to 127.0.0.1:3000");

    println!("arpagona-api-server listening on http://{}", addr);
    axum::serve(listener, app)
        .await
        .expect("api server should run");
}

pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/tasks", post(create_task).get(list_tasks))
        .route(
            "/proposed-actions",
            post(create_proposed_action).get(list_proposed_actions),
        )
        .route("/proposed-actions/:id/review", post(review_proposed_action))
        .route("/agent/propose", post(agent_propose))
        .route("/decision-gate/evaluate", post(evaluate_decision_gate))
        .route("/decisions", get(list_decisions))
        .route("/audit", get(list_audit))
        .route("/proposed-actions/:id/sandbox", post(sandbox_run_proposal))
        .route("/sandbox-runs", get(list_sandbox_runs))
        .route("/proposed-actions/:id/dry-run", post(dry_run_proposal))
        .route("/dry-run-results", get(list_dry_run_results))
        .route(
            "/execution-capabilities",
            get(list_execution_capabilities_handler),
        )
        .route(
            "/execution-capabilities/:action_type",
            get(get_execution_capability_handler),
        )
        .route(
            "/proposed-actions/:id/policy-check",
            post(policy_check_proposal),
        )
        .route(
            "/proposed-actions/:id/override",
            post(override_proposed_action),
        )
        .route("/proposed-actions/:id/execute", post(execute_proposal))
        .route("/executors", get(list_executors))
        .route("/executors/:id", get(get_executor_handler))
        .route("/executors/:id/state", post(set_executor_state_handler))
        .with_state(state)
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "arpagona-api-server",
    })
}

async fn create_task(
    State(state): State<AppState>,
    Json(request): Json<CreateTaskRequest>,
) -> Result<Json<Task>, ApiError> {
    let mut store = state.lock()?;
    let now = Utc::now();
    let task = Task {
        id: TaskId::new(format!("task-{}", store.tasks.len() + 1)),
        workspace_id: WorkspaceId::new(request.workspace_id),
        title: request.title,
        description: request.description,
        status: TaskStatus::Open,
        priority: TaskPriority::Normal,
        created_at: now,
        updated_at: now,
    };

    store.tasks.push(task.clone());
    Ok(Json(task))
}

async fn list_tasks(State(state): State<AppState>) -> Result<Json<Vec<Task>>, ApiError> {
    Ok(Json(state.lock()?.tasks.clone()))
}

async fn create_proposed_action(
    State(state): State<AppState>,
    Json(request): Json<CreateProposedActionRequest>,
) -> Result<Json<ProposedAction>, ApiError> {
    let mut store = state.lock()?;
    let action = ProposedAction {
        id: ProposedActionId::new(format!("action-{}", store.proposed_actions.len() + 1)),
        workspace_id: WorkspaceId::new(request.workspace_id),
        task_id: request.task_id.map(TaskId::new),
        proposed_by: AgentId::new(request.proposed_by),
        action_type: request.action_type,
        target: request.target,
        payload: request.payload,
        risk_level: request.risk_level,
        required_permissions: request.required_permissions,
        rationale: request.rationale,
        context_refs: vec![],
        status: ProposedActionStatus::PendingDecision,
        created_at: Utc::now(),
    };

    store.proposed_actions.push(action.clone());
    Ok(Json(action))
}

async fn list_proposed_actions(
    State(state): State<AppState>,
) -> Result<Json<Vec<ProposedAction>>, ApiError> {
    Ok(Json(state.lock()?.proposed_actions.clone()))
}

/// Valid state transitions for human review of proposed actions.
fn valid_review_transition(current: &ProposedActionStatus, target: &ProposedActionStatus) -> bool {
    matches!(
        (current, target),
        (
            ProposedActionStatus::PendingDecision,
            ProposedActionStatus::Approved
        ) | (
            ProposedActionStatus::PendingDecision,
            ProposedActionStatus::Rejected
        ) | (
            ProposedActionStatus::PendingDecision,
            ProposedActionStatus::Deferred
        ) | (
            ProposedActionStatus::Deferred,
            ProposedActionStatus::PendingDecision
        ) | (
            ProposedActionStatus::Deferred,
            ProposedActionStatus::Approved
        ) | (
            ProposedActionStatus::Deferred,
            ProposedActionStatus::Rejected
        ) | (
            ProposedActionStatus::Approved,
            ProposedActionStatus::Superseded
        )
    )
}

async fn review_proposed_action(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(request): Json<ReviewActionRequest>,
) -> Result<Json<ReviewActionResponse>, ApiError> {
    let action_id = ProposedActionId::new(id);
    let mut store = state.lock()?;

    let idx = store
        .proposed_actions
        .iter()
        .position(|a| a.id == action_id)
        .ok_or_else(|| ApiError::not_found(format!("proposed action {} not found", action_id)))?;

    let target_status = match request.action.as_str() {
        "approve" => ProposedActionStatus::Approved,
        "reject" => ProposedActionStatus::Rejected,
        "defer" => ProposedActionStatus::Deferred,
        "supersede" => ProposedActionStatus::Superseded,
        _ => {
            return Err(ApiError::bad_request(format!(
            "invalid review action '{}' (expected 'approve', 'reject', 'defer', or 'supersede')",
            request.action
        )))
        }
    };

    let current_status = store.proposed_actions[idx].status.clone();
    if !valid_review_transition(&current_status, &target_status) {
        return Err(ApiError::bad_request(format!(
            "invalid transition from {:?} to {:?}",
            current_status, target_status
        )));
    }

    // Update the action status
    store.proposed_actions[idx].status = target_status.clone();

    // Map review action to audit event type
    let event_type = match target_status {
        ProposedActionStatus::Approved => AuditEventType::HumanApproved,
        ProposedActionStatus::Rejected => AuditEventType::HumanRejected,
        ProposedActionStatus::Deferred => AuditEventType::HumanDeferred,
        ProposedActionStatus::Superseded => AuditEventType::HumanApproved, // supersede is still recorded as human action
        _ => AuditEventType::DecisionCreated,
    };

    let audit_event = AuditEvent {
        id: AuditEventId::new(format!(
            "audit-review-{}-{}",
            store.proposed_actions[idx].id.as_str(),
            store.audit_events.len() + 1
        )),
        event_type,
        actor: ActorRef::Human(request.actor),
        workspace_id: Some(store.proposed_actions[idx].workspace_id.clone()),
        task_id: store.proposed_actions[idx].task_id.clone(),
        proposed_action_id: Some(store.proposed_actions[idx].id.clone()),
        decision_id: None,
        payload: json!({
            "review_action": request.action,
            "reason": request.reason,
            "previous_status": current_status,
            "new_status": target_status,
        }),
        created_at: Utc::now(),
    };

    store.audit_events.push(audit_event.clone());

    Ok(Json(ReviewActionResponse {
        proposed_action: store.proposed_actions[idx].clone(),
        audit_event,
    }))
}

async fn agent_propose(
    State(state): State<AppState>,
    Json(request): Json<AgentProposeRequest>,
) -> Result<Json<AgentProposeResponse>, ApiError> {
    let prompt = request.prompt;
    let deterministic_turn = deterministic_turn_for_prompt(&prompt);
    let llm_request = LlmActionRequest { prompt };

    let turn = match request.provider.as_str() {
        "openai" => match deterministic_turn {
            Some(turn) => turn,
            None => {
                OpenAiProvider::from_env()?
                    .propose_turn(llm_request)
                    .await?
            }
        },
        "ollama" => match deterministic_turn {
            Some(turn) => turn,
            None => {
                OllamaProvider::from_env()
                    .propose_turn(llm_request)
                    .await?
            }
        },
        "mock" => AgentTurnDraft::ProposedAction {
            action: MockProvider::safe_default()
                .propose_action(llm_request)
                .await?,
        },
        other => {
            return Err(ApiError::bad_request(format!(
                "unsupported agent proposer provider '{other}' (expected 'openai', 'ollama', or 'mock')"
            )))
        }
    };

    match turn {
        AgentTurnDraft::DirectReply { message } => Ok(Json(AgentProposeResponse {
            kind: "direct_reply",
            proposed_action: None,
            message: Some(message),
            question: None,
        })),
        AgentTurnDraft::ClarifyingQuestion { question } => Ok(Json(AgentProposeResponse {
            kind: "clarifying_question",
            proposed_action: None,
            message: None,
            question: Some(question),
        })),
        AgentTurnDraft::ProposedAction { action: draft } => {
            let mut store = state.lock()?;
            let generated_action_id =
                ProposedActionId::new(format!("action-{}", store.proposed_actions.len() + 1));
            let action = draft.into_proposed_action(
                WorkspaceId::new(request.workspace_id),
                request.task_id.map(TaskId::new),
                AgentId::new("agent-proposer-v0"),
                generated_action_id,
            );

            store.proposed_actions.push(action.clone());

            Ok(Json(AgentProposeResponse {
                kind: "proposed_action",
                proposed_action: Some(action),
                message: None,
                question: None,
            }))
        }
    }
}

async fn evaluate_decision_gate(
    State(state): State<AppState>,
    Json(request): Json<EvaluateDecisionGateRequest>,
) -> Result<Json<EvaluateDecisionGateResponse>, ApiError> {
    let mut store = state.lock()?;
    let action_id = ProposedActionId::new(request.proposed_action_id);
    let action_index = store
        .proposed_actions
        .iter()
        .position(|action| action.id == action_id)
        .ok_or_else(|| ApiError::not_found(format!("proposed action {} not found", action_id)))?;

    let action = store.proposed_actions[action_index].clone();
    let mut decision = evaluate_proposed_action(&action, &[], &request.granted_permissions);

    // Add action fingerprint if RequiresOverride
    if matches!(decision.status, DecisionStatus::RequiresOverride) {
        decision.action_fingerprint = Some(compute_action_fingerprint(&action));
    }

    let audit_event = audit_event_for_decision(&action, &decision);

    // If the decision requires override, also produce an OverrideRequested audit event
    if matches!(decision.status, DecisionStatus::RequiresOverride) {
        let override_requested = AuditEvent::override_event(
            AuditEventId::new(format!("audit-override-requested-{}", action.id.as_str())),
            AuditEventType::OverrideRequested,
            Some(ActorRef::System),
            &action,
            Some(&decision),
            "requires_override",
            &decision.reason,
            Utc::now(),
        );
        store.audit_events.push(override_requested);
    }

    store.proposed_actions[action_index].status = status_from_decision(&decision.status);
    store.decisions.push(decision.clone());
    store.audit_events.push(audit_event.clone());

    Ok(Json(EvaluateDecisionGateResponse {
        decision,
        audit_event,
    }))
}

/// Compute a stable fingerprint for an action at decision time.
///
/// Used to detect if the action has changed between RequiresOverride
/// decision and override attempt.
fn compute_action_fingerprint(action: &ProposedAction) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    action.id.hash(&mut hasher);
    format!("{:?}", action.action_type).hash(&mut hasher);
    format!("{:?}", action.risk_level).hash(&mut hasher);
    action.payload.to_string().hash(&mut hasher);
    format!("{:?}", action.required_permissions).hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

/// Override a proposed action that has RequiresOverride status.
///
/// Validates the password against the configured override engine.
/// If successful, the decision is changed to ApprovedByOverride.
///
/// # Single-action scoping
///
/// Override is strictly limited to the action identified by `:id`:
/// - An ApprovedByOverride decision is created ONLY for this action_id
/// - No other action's status or decision is modified
/// - No global "admin session" is created
/// - Every override attempt independently validates the password
/// - Anti-mutation: action fingerprint is verified at override time
///
/// The password is NEVER included in audit events or logs.
async fn override_proposed_action(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(request): Json<OverrideActionRequest>,
) -> Result<Json<OverrideActionResponse>, ApiError> {
    let action_id = ProposedActionId::new(id);
    let mut store = state.lock()?;

    // Check if override is configured
    let override_engine = store.override_engine.as_mut().ok_or_else(|| {
        ApiError::bad_request(
            "override_not_configured: ARPAGONA_OVERRIDE_PASSWORD is not set. \
                 Set the environment variable to enable override, or set \
                 ARPAGONA_ALLOW_DEV_OVERRIDE=true for development.",
        )
    })?;

    let action_idx = store
        .proposed_actions
        .iter()
        .position(|a| a.id == action_id)
        .ok_or_else(|| ApiError::not_found(format!("proposed action {} not found", action_id)))?;

    let action = &store.proposed_actions[action_idx];

    // Find the latest RequiresOverride decision for this action
    // Must do this BEFORE accessing override_engine to avoid borrow conflicts
    let requires_override_decision = store
        .decisions
        .iter()
        .rfind(|d: &&Decision| {
            d.proposed_action_id == action_id
                && matches!(d.status, DecisionStatus::RequiresOverride)
        })
        .cloned();

    // Check if action is already ApprovedByOverride (idempotent)
    let already_approved_decision = store
        .decisions
        .iter()
        .rfind(|d: &&Decision| {
            d.proposed_action_id == action_id
                && matches!(d.status, DecisionStatus::ApprovedByOverride)
        })
        .cloned();

    if let Some(existing_decision) = already_approved_decision {
        let audit_event = AuditEvent::override_event(
            AuditEventId::new(format!(
                "audit-override-already-approved-{}",
                action_id.as_str()
            )),
            AuditEventType::OverrideApproved,
            Some(ActorRef::Human(request.actor)),
            action,
            Some(&existing_decision),
            "already_approved",
            "Action was already approved by a prior override; no change made.",
            Utc::now(),
        );
        store.audit_events.push(audit_event.clone());

        return Ok(Json(OverrideActionResponse {
            decision: existing_decision,
            audit_event,
            outcome: "already_approved".to_owned(),
        }));
    }

    let requires_override_decision = requires_override_decision.ok_or_else(|| {
        ApiError::bad_request(format!(
            "action {} is not in RequiresOverride state; current status: {:?}",
            action_id, store.proposed_actions[action_idx].status
        ))
    })?;

    // Anti-mutation: verify action fingerprint matches the decision
    if let Some(stored_fingerprint) = &requires_override_decision.action_fingerprint {
        let current_fingerprint = compute_action_fingerprint(action);
        if stored_fingerprint != &current_fingerprint {
            let audit_event = AuditEvent::override_event(
                AuditEventId::new(format!(
                    "audit-override-fingerprint-mismatch-{}",
                    action_id.as_str()
                )),
                AuditEventType::OverrideFailed,
                Some(ActorRef::Human(request.actor)),
                action,
                Some(&requires_override_decision),
                "fingerprint_mismatch",
                "Override refused: action has changed since the RequiresOverride decision was made.",
                Utc::now(),
            );
            store.audit_events.push(audit_event.clone());
            return Err(ApiError::bad_request(format!(
                "Override refused: action {} has changed since RequiresOverride decision.",
                action_id
            )));
        }
    }

    // Clone the action for use before the mutable borrow
    let action_clone = action.clone();

    // Attempt override in a scoped block to release mutable borrow
    // on store before subsequent store access.
    let (outcome_status, outcome_reason, new_status) = {
        let override_engine = store.override_engine.as_mut().ok_or_else(|| {
            ApiError::bad_request(
                "override_not_configured: ARPAGONA_OVERRIDE_PASSWORD is not set. \
                     Set the environment variable to enable override, or set \
                     ARPAGONA_ALLOW_DEV_OVERRIDE=true for development.",
            )
        })?;

        match override_engine.attempt_override(&request.password) {
            OverrideOutcome::Approved => {
                let new_decision = Decision {
                    id: DecisionId::new(format!(
                        "decision-{}-override-approved",
                        action_id.as_str()
                    )),
                    proposed_action_id: action_id.clone(),
                    status: DecisionStatus::ApprovedByOverride,
                    reason: "Approved by administrative override.".to_owned(),
                    risk_level: action_clone.risk_level.clone(),
                    policies_applied: requires_override_decision.policies_applied.clone(),
                    decided_by: Some(DecisionActor::Human(request.actor.clone())),
                    created_at: Utc::now(),
                    override_hint: None,
                    action_fingerprint: Some(compute_action_fingerprint(&action_clone)),
                };

                store.proposed_actions[action_idx].status = ProposedActionStatus::Approved;
                store.decisions.push(new_decision.clone());

                (
                    "approved",
                    "Override approved by administrator.",
                    new_decision,
                )
            }
            OverrideOutcome::Failed => {
                let failed_decision = requires_override_decision.clone();
                (
                    "failed",
                    "Override failed: incorrect password.",
                    failed_decision,
                )
            }
            OverrideOutcome::Locked => {
                let locked_decision = requires_override_decision.clone();
                (
                    "locked",
                    "Override is temporarily locked due to too many failed attempts.",
                    locked_decision,
                )
            }
            OverrideOutcome::Expired => {
                // Expired cannot occur since TTL was removed from the engine.
                // This arm is kept for exhaustive match on OverrideOutcome.
                let expired_decision = requires_override_decision.clone();
                (
                    "expired",
                    "Override authorization has expired.",
                    expired_decision,
                )
            }
            OverrideOutcome::NotOverridable => {
                let blocked_decision = requires_override_decision.clone();
                (
                    "not_overridable",
                    "This action cannot be overridden.",
                    blocked_decision,
                )
            }
        }
    };

    let audit_event_type = match outcome_status {
        "approved" => AuditEventType::OverrideApproved,
        "failed" => AuditEventType::OverrideFailed,
        _ => AuditEventType::OverrideRequested,
    };

    // Build audit event — NEVER include the password
    let audit_event = AuditEvent::override_event(
        AuditEventId::new(format!(
            "audit-override-{}-{}",
            outcome_status,
            action_id.as_str()
        )),
        audit_event_type,
        Some(ActorRef::Human(request.actor)),
        &action_clone,
        Some(&new_status),
        outcome_status,
        outcome_reason,
        Utc::now(),
    );
    store.audit_events.push(audit_event.clone());

    Ok(Json(OverrideActionResponse {
        decision: new_status,
        audit_event,
        outcome: outcome_status.to_owned(),
    }))
}

async fn list_decisions(State(state): State<AppState>) -> Result<Json<Vec<Decision>>, ApiError> {
    Ok(Json(state.lock()?.decisions.clone()))
}

async fn list_audit(State(state): State<AppState>) -> Result<Json<Vec<AuditEvent>>, ApiError> {
    Ok(Json(state.lock()?.audit_events.clone()))
}

/// Run a dry-run sandbox simulation for an approved low-risk proposed action.
async fn sandbox_run_proposal(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<SandboxRun>, ApiError> {
    let action_id = ProposedActionId::new(id);
    let mut store = state.lock()?;

    let action = store
        .proposed_actions
        .iter()
        .find(|a| a.id == action_id)
        .ok_or_else(|| ApiError::not_found(format!("proposed action {} not found", action_id)))?
        .clone();

    // Validate: only Approved proposals can be sandboxed
    if action.status != ProposedActionStatus::Approved {
        let audit_event = AuditEvent {
            id: AuditEventId::new(format!(
                "audit-sandbox-blocked-{}-{}",
                action.id.as_str(),
                store.audit_events.len() + 1
            )),
            event_type: AuditEventType::ExecutionBlocked,
            actor: ActorRef::System,
            workspace_id: Some(action.workspace_id.clone()),
            task_id: action.task_id.clone(),
            proposed_action_id: Some(action.id.clone()),
            decision_id: None,
            payload: serde_json::json!({
                "sandbox_run_status": "blocked",
                "reason": format!("Proposal status is {:?}, must be Approved", action.status),
            }),
            created_at: Utc::now(),
        };
        store.audit_events.push(audit_event);
        return Err(ApiError::bad_request(format!(
            "Proposed action must be 'approved' to run sandbox (current: {:?})",
            action.status
        )));
    }

    // Validate: only low-risk proposals allowed
    if action.risk_level != RiskLevel::Informational && action.risk_level != RiskLevel::Low {
        return Err(ApiError::bad_request(format!(
            "Sandbox requires Informational or Low risk level (current: {:?})",
            action.risk_level
        )));
    }

    let (simulated_output, warnings, status) = simulate_action(&action);

    let run_id = format!("sandbox-{}", store.sandbox_runs.len() + 1);

    let run = SandboxRun {
        id: run_id.clone(),
        proposed_action_id: action.id.as_str().to_owned(),
        status,
        action_type: format!("{:?}", action.action_type),
        risk_level: format!("{:?}", action.risk_level),
        simulated_output,
        warnings,
        created_at: Utc::now(),
        simulation_warning: "⚠ DRY-RUN SIMULATION — No real side effects were executed.".to_owned(),
    };

    store.sandbox_runs.push(run.clone());

    // Create an audit event for the sandbox run
    let audit_event = AuditEvent {
        id: AuditEventId::new(format!("audit-sandbox-{}", store.audit_events.len() + 1)),
        event_type: AuditEventType::SandboxCompleted,
        actor: ActorRef::System,
        workspace_id: Some(action.workspace_id),
        task_id: action.task_id,
        proposed_action_id: Some(action.id),
        decision_id: None,
        payload: json!({
            "sandbox_run_id": run_id,
            "simulation_mode": true,
            "non_authorizing_warning": "This is a dry-run simulation only. No real execution occurred.",
        }),
        created_at: Utc::now(),
    };
    store.audit_events.push(audit_event);

    Ok(Json(run))
}

/// List all sandbox runs.
async fn list_sandbox_runs(
    State(state): State<AppState>,
) -> Result<Json<Vec<SandboxRun>>, ApiError> {
    Ok(Json(state.lock()?.sandbox_runs.clone()))
}

/// List all execution capabilities.
async fn list_execution_capabilities_handler() -> Json<Vec<ExecutionCapability>> {
    Json(list_execution_capabilities())
}

/// Get execution capability for a specific action type.
async fn get_execution_capability_handler(
    Path(action_type): Path<String>,
) -> Result<Json<ExecutionCapability>, ApiError> {
    let at = ActionType::from_str(&action_type)
        .map_err(|_| ApiError::bad_request(format!("unknown action type: '{}'", action_type)))?;
    let cap = execution_capability(&at);
    Ok(Json(cap))
}

/// Describe expected effects of an action type without executing anything.
fn describe_action_effects(action: &ProposedAction) -> (Vec<String>, Vec<String>, String, String) {
    match &action.action_type {
        ActionType::ReadMemory => (
            vec!["In-memory inspection only".to_owned()],
            vec!["memory graph (read)".to_owned()],
            "Fully reversible — no state mutation.".to_owned(),
            "Would read memory from the graph store.".to_owned(),
        ),
        ActionType::ProposeToolUse => {
            let target = action.target.as_deref().unwrap_or("unknown tool");
            (
                vec![format!("Would propose using tool: {}", target)],
                vec![format!("tool:{}", target)],
                "Fully reversible — proposal only, no execution.".to_owned(),
                format!(
                    "Would create a new ProposedAction for tool '{}' through the Decision Gate.",
                    target
                ),
            )
        }
        ActionType::DirectToolCall => {
            let target = action.target.as_deref().unwrap_or("unknown tool");
            (
                vec![format!(
                    "Would evaluate LLM tool-call intent for: {}",
                    target
                )],
                vec![format!("llm_tool_call:{}", target)],
                "Fully reversible — governance evaluation only, no execution.".to_owned(),
                format!(
                    "Would route LLM tool-call intent for '{}' through the Decision Gate.",
                    target
                ),
            )
        }
        ActionType::SimulateEmail => (
            vec!["Would simulate an email draft.".to_owned()],
            vec!["email draft (memory)".to_owned()],
            "Fully reversible — no email is sent.".to_owned(),
            "Would generate an email draft in memory without sending.".to_owned(),
        ),
        ActionType::SystemCheck => (
            vec!["Would check system health.".to_owned()],
            vec!["system status".to_owned()],
            "Fully reversible — no state change.".to_owned(),
            "Would perform a read-only system health check.".to_owned(),
        ),
        _ => (
            vec![format!("Would perform {:?} action.", action.action_type)],
            vec![format!(
                "resource:{}",
                action.target.as_deref().unwrap_or("unknown")
            )],
            "Reversibility depends on action type.".to_owned(),
            format!(
                "Would execute a {:?} action on target '{}'.",
                action.action_type,
                action.target.as_deref().unwrap_or("none")
            ),
        ),
    }
}

async fn dry_run_proposal(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<DryRunResult>, ApiError> {
    let action_id = ProposedActionId::new(id);
    let mut store = state.lock()?;

    let idx = store
        .proposed_actions
        .iter()
        .position(|a| a.id == action_id)
        .ok_or_else(|| ApiError::not_found(format!("proposed action {} not found", action_id)))?;

    let action = store.proposed_actions[idx].clone();

    // Build policy input from the action
    let policy_input = PolicyInput {
        action_type: action.action_type.clone(),
        proposal_status: action.status.clone(),
        risk_level: action.risk_level.clone(),
        required_permissions: action
            .required_permissions
            .iter()
            .map(|p| format!("{:?}", p))
            .collect(),
        touched_resource_kinds: vec![], // filled below by describe_action_effects
        actor: Some(format!("{:?}", action.proposed_by)),
        workspace: Some(action.workspace_id.as_str().to_owned()),
        dry_run_requested: true,
        real_execution_requested: false,
    };

    // Run policy check
    let policy_result = PolicyEngine::evaluate_dry_run(&policy_input);

    let (expected_effects, touched_resources, reversibility, summary) =
        describe_action_effects(&action);

    let capability = execution_capability(&action.action_type);

    let (dry_run_status, policy_blocked_reason): (DryRunStatus, Option<String>) =
        match &policy_result.decision {
            PolicyDecision::Allowed => (DryRunStatus::DryRunCompleted, None),
            PolicyDecision::NeedsDryRun => (DryRunStatus::DryRunCompleted, None),
            PolicyDecision::NeedsHumanApproval => (
                DryRunStatus::DryRunCompleted,
                Some(
                    "NeedsHumanApproval: action requires human approval before proceeding."
                        .to_owned(),
                ),
            ),
            PolicyDecision::Blocked => (
                DryRunStatus::DryRunBlocked,
                Some(policy_result.reason.clone()),
            ),
            PolicyDecision::UnsupportedCapability => (
                DryRunStatus::DryRunBlocked,
                Some(policy_result.reason.clone()),
            ),
        };

    let is_blocked = dry_run_status == DryRunStatus::DryRunBlocked;

    let (expected_effects, touched_resources, reversibility, summary) =
        describe_action_effects(&action);

    let capability = execution_capability(&action.action_type);
    let result = DryRunResult {
        proposal_id: action.id.clone(),
        action_type: action.action_type.clone(),
        expected_effects,
        required_permissions: action.required_permissions.clone(),
        touched_resources,
        risk_level: action.risk_level.clone(),
        reversibility,
        human_readable_summary: summary,
        status: dry_run_status,
        execution_capability: Some(serde_json::to_value(&capability).unwrap_or_default()),
        policy_decision: Some(serde_json::to_value(&policy_result).unwrap_or_default()),
        created_at: Utc::now(),
    };

    // Create audit event
    let audit_event = AuditEvent {
        id: AuditEventId::new(format!(
            "audit-dry-run-{}-{}",
            result.proposal_id.as_str(),
            store.audit_events.len() + 1
        )),
        event_type: if is_blocked {
            AuditEventType::DryRunBlocked
        } else {
            AuditEventType::DryRunCompleted
        },
        actor: ActorRef::System,
        workspace_id: Some(action.workspace_id.clone()),
        task_id: action.task_id.clone(),
        proposed_action_id: Some(action.id.clone()),
        decision_id: None,
        payload: serde_json::json!({
            "dry_run_status": if is_blocked { "blocked" } else { "completed" },
            "action_type": result.action_type,
            "policy_decision": policy_result.decision,
            "policy_reason": policy_result.reason,
            "policy_matched_rules": policy_result.matched_rules,
            "expected_effects": result.expected_effects,
            "touched_resources": result.touched_resources,
            "block_reason": policy_blocked_reason,
        }),
        created_at: Utc::now(),
    };

    store.dry_run_results.push(result.clone());
    store.audit_events.push(audit_event);

    if is_blocked {
        return Err(ApiError::bad_request(format!(
            "Dry-run blocked by policy: {}",
            policy_blocked_reason.unwrap_or_else(|| "Unknown policy block.".to_owned())
        )));
    }

    Ok(Json(result))
}

async fn list_dry_run_results(
    State(state): State<AppState>,
) -> Result<Json<Vec<DryRunResult>>, ApiError> {
    Ok(Json(state.lock()?.dry_run_results.clone()))
}

/// Run a policy check on a proposed action without executing anything.
async fn policy_check_proposal(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<PolicyEngineResult>, ApiError> {
    let action_id = ProposedActionId::new(id);
    let store = state.lock()?;

    let action = store
        .proposed_actions
        .iter()
        .find(|a| a.id == action_id)
        .ok_or_else(|| ApiError::not_found(format!("proposed action {} not found", action_id)))?
        .clone();

    let policy_input = PolicyInput {
        action_type: action.action_type.clone(),
        proposal_status: action.status.clone(),
        risk_level: action.risk_level.clone(),
        required_permissions: action
            .required_permissions
            .iter()
            .map(|p| format!("{:?}", p))
            .collect(),
        touched_resource_kinds: vec![],
        actor: Some(format!("{:?}", action.proposed_by)),
        workspace: Some(action.workspace_id.as_str().to_owned()),
        dry_run_requested: true,
        real_execution_requested: false,
    };

    let result = PolicyEngine::evaluate_dry_run(&policy_input);

    Ok(Json(result))
}

/// Execute an approved proposal through the NoopExecutor (always disabled).
async fn execute_proposal(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ExecutionResult>, ApiError> {
    let action_id = ProposedActionId::new(id);
    let mut store = state.lock()?;

    let action = store
        .proposed_actions
        .iter()
        .find(|a| a.id == action_id)
        .ok_or_else(|| ApiError::not_found(format!("proposed action {} not found", action_id)))?
        .clone();

    // Step 1: Run policy check first
    let policy_input = PolicyInput {
        action_type: action.action_type.clone(),
        proposal_status: action.status.clone(),
        risk_level: action.risk_level.clone(),
        required_permissions: action
            .required_permissions
            .iter()
            .map(|p| format!("{:?}", p))
            .collect(),
        touched_resource_kinds: vec![],
        actor: Some(format!("{:?}", action.proposed_by)),
        workspace: Some(action.workspace_id.as_str().to_owned()),
        dry_run_requested: false,
        real_execution_requested: true,
    };

    let policy_result = PolicyEngine::evaluate(&policy_input);

    if policy_result.decision != PolicyDecision::Allowed {
        // Blocked by policy — create audit event
        let audit_event = AuditEvent {
            id: AuditEventId::new(format!(
                "audit-execute-blocked-{}-{}",
                action.id.as_str(),
                store.audit_events.len() + 1
            )),
            event_type: AuditEventType::ExecutionBlocked,
            actor: ActorRef::System,
            workspace_id: Some(action.workspace_id.clone()),
            task_id: action.task_id.clone(),
            proposed_action_id: Some(action.id.clone()),
            decision_id: None,
            payload: serde_json::json!({
                "execution_status": "blocked_by_policy",
                "policy_decision": policy_result.decision,
                "policy_reason": policy_result.reason,
                "policy_matched_rules": policy_result.matched_rules,
            }),
            created_at: Utc::now(),
        };
        store.audit_events.push(audit_event.clone());

        return Err(ApiError::bad_request(format!(
            "Execution blocked by policy: {}",
            policy_result.reason
        )));
    }

    // Step 2: Build execution request
    let capability = execution_capability(&action.action_type);
    let execution_request = ExecutionRequest {
        proposal_id: action.id.clone(),
        action_type: action.action_type.clone(),
        actor: format!("{:?}", action.proposed_by),
        workspace_scope: action.workspace_id.as_str().to_owned(),
        policy_decision: Some(policy_result),
        capability: Some(capability),
        dry_run_result: None,
        risk_level: action.risk_level.clone(),
        required_permissions: action
            .required_permissions
            .iter()
            .map(|p| format!("{:?}", p))
            .collect(),
    };

    // Step 3: Execute through executor registry
    let audit_id = AuditEventId::new(format!(
        "audit-execute-{}-{}",
        action.id.as_str(),
        store.audit_events.len() + 1
    ));
    let result = store
        .executor_registry
        .execute(&execution_request, Some(audit_id.clone()));

    // Step 4: Create audit event
    let audit_event = AuditEvent {
        id: audit_id.clone(),
        event_type: AuditEventType::ExecutionDisabled,
        actor: ActorRef::System,
        workspace_id: Some(action.workspace_id.clone()),
        task_id: action.task_id.clone(),
        proposed_action_id: Some(action.id.clone()),
        decision_id: None,
        payload: serde_json::json!({
            "execution_status": result.status,
            "executor": store.executor_registry.resolve(&result.action_type).map(|e| e.executor_id().to_owned()),
            "action_type": result.action_type,
            "reason": result.reason,
            "touched_resources": result.touched_resources,
        }),
        created_at: Utc::now(),
    };

    store.audit_events.push(audit_event);

    Ok(Json(result))
}

fn status_from_decision(status: &DecisionStatus) -> ProposedActionStatus {
    match status {
        DecisionStatus::Approved => ProposedActionStatus::Approved,
        DecisionStatus::ApprovedByOverride => ProposedActionStatus::Approved,
        DecisionStatus::Blocked => ProposedActionStatus::Blocked,
        DecisionStatus::RequiresOverride => ProposedActionStatus::NeedsHumanApproval,
        DecisionStatus::NeedsHumanApproval => ProposedActionStatus::NeedsHumanApproval,
    }
}

/// List all registered executors with their current state and supported action types.
async fn list_executors(
    State(state): State<AppState>,
) -> Result<Json<Vec<ExecutorInfoResponse>>, ApiError> {
    let store = state.lock()?;
    let ids = store.executor_registry.list();
    let executors: Vec<ExecutorInfoResponse> = ids
        .into_iter()
        .map(|id| {
            let executor_state = store
                .executor_registry
                .get_state(&id)
                .unwrap_or(ExecutorState::Disabled);
            let supported_action_types = store
                .executor_registry
                .get(&id)
                .map(|e| e.supported_action_types())
                .unwrap_or_default();
            ExecutorInfoResponse {
                executor_id: id,
                executor_state,
                supported_action_types,
            }
        })
        .collect();
    Ok(Json(executors))
}

/// Get a specific executor by ID.
async fn get_executor_handler(
    State(state): State<AppState>,
    Path(executor_id): Path<String>,
) -> Result<Json<ExecutorInfoResponse>, ApiError> {
    let store = state.lock()?;
    let slot = store
        .executor_registry
        .get_slot(&executor_id)
        .ok_or_else(|| ApiError::not_found(format!("executor '{}' not found", executor_id)))?;
    Ok(Json(ExecutorInfoResponse {
        executor_id: executor_id.clone(),
        executor_state: slot.state.clone(),
        supported_action_types: slot.executor.supported_action_types().to_vec(),
    }))
}

/// Set an executor's readiness state.
async fn set_executor_state_handler(
    State(state): State<AppState>,
    Path(executor_id): Path<String>,
    Json(request): Json<SetExecutorStateRequest>,
) -> Result<Json<SetExecutorStateResponse>, ApiError> {
    let mut store = state.lock()?;
    let result = store
        .executor_registry
        .set_state(&executor_id, request.state);
    match result {
        Some(()) => {
            let new_state = store
                .executor_registry
                .get_state(&executor_id)
                .unwrap_or(ExecutorState::Disabled);
            Ok(Json(SetExecutorStateResponse {
                executor_id,
                executor_state: new_state,
            }))
        }
        None => Err(ApiError::not_found(format!(
            "executor '{}' not found in registry",
            executor_id
        ))),
    }
}

fn empty_payload() -> Value {
    json!({})
}

// --- Sandbox types ---

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SandboxRunStatus {
    Pending,
    Completed,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct SandboxRun {
    id: String,
    proposed_action_id: String,
    status: SandboxRunStatus,
    action_type: String,
    risk_level: String,
    simulated_output: Value,
    warnings: Vec<String>,
    created_at: DateTime<Utc>,
    simulation_warning: String,
}

/// Generate a deterministic simulated output for a proposed action without side effects.
fn simulate_action(action: &ProposedAction) -> (Value, Vec<String>, SandboxRunStatus) {
    let mut warnings = Vec::new();
    let mut effects = Vec::new();

    match action.action_type {
        ActionType::ReadMemory => {
            effects.push(json!({
                "description": "Read memory matching the specified criteria",
                "target": action.target,
                "effect_type": "read",
            }));
        }
        ActionType::ReadTasks => {
            effects.push(json!({
                "description": "List tasks for the workspace",
                "target": action.workspace_id,
                "effect_type": "read",
            }));
        }
        ActionType::ReadProposedActions | ActionType::ReadPendingActions => {
            effects.push(json!({
                "description": "List proposed actions for review",
                "target": action.workspace_id,
                "effect_type": "read",
            }));
        }
        ActionType::ReadDecisions => {
            effects.push(json!({
                "description": "Read decision records",
                "target": action.workspace_id,
                "effect_type": "read",
            }));
        }
        ActionType::ReadAudit => {
            effects.push(json!({
                "description": "Read audit event history",
                "target": action.workspace_id,
                "effect_type": "read",
            }));
        }
        ActionType::ReadStatus => {
            effects.push(json!({
                "description": "Read system status overview",
                "effect_type": "read",
            }));
        }
        ActionType::SystemCheck => {
            effects.push(json!({
                "description": "Run system diagnostics check",
                "effect_type": "inspection",
            }));
        }
        ActionType::WriteMemory
        | ActionType::CreateMemoryFact
        | ActionType::LinkMemoryFact
        | ActionType::InvalidateMemoryFact
        | ActionType::CreateFailureInsightMemory
        | ActionType::CreateHolographicTrace => {
            effects.push(json!({
                "description": "Write data to Graph Memory",
                "target": action.target,
                "effect_type": "memory_write",
                "payload_preview": action.payload,
            }));
            if action.risk_level == RiskLevel::Informational || action.risk_level == RiskLevel::Low
            {
                warnings
                    .push("Memory write would be simulated — no actual persistence.".to_owned());
            }
        }
        ActionType::ReadDocument => {
            effects.push(json!({
                "description": "Read document content",
                "target": action.target,
                "effect_type": "read",
            }));
        }
        ActionType::WriteDocument => {
            effects.push(json!({
                "description": "Write or update a document",
                "target": action.target,
                "effect_type": "write",
            }));
            warnings.push("Document write is simulated — no file would be modified.".to_owned());
        }
        ActionType::ProposeToolUse => {
            effects.push(json!({
                "description": "Propose tool execution",
                "tool": action.target,
                "effect_type": "proposal",
            }));
        }
        ActionType::DirectToolCall => {
            effects.push(json!({
                "description": "Evaluate LLM tool-call intent through Decision Gate",
                "tool": action.target,
                "effect_type": "llm_tool_call_governance",
            }));
        }
        ActionType::SimulateEmail => {
            effects.push(json!({
                "description": "Simulate sending email communication",
                "recipient": action.target,
                "effect_type": "communication",
            }));
            warnings.push("Email simulation — no message would be sent.".to_owned());
        }
        ActionType::ManageTask => {
            effects.push(json!({
                "description": "Manage task lifecycle",
                "target": action.target,
                "effect_type": "management",
            }));
        }
        ActionType::Custom(ref name) => {
            effects.push(json!({
                "description": format!("Custom action type '{name}'"),
                "target": action.target,
                "effect_type": "custom",
            }));
            warnings.push(format!(
                "Custom action '{name}' has no pre-defined simulation. Verify manually."
            ));
        }
    }

    let simulation_warning =
        "⚠ DRY-RUN SIMULATION — No real side effects were executed.".to_owned();
    warnings.push(simulation_warning.clone());

    let output = json!({
        "simulation_mode": true,
        "action_id": action.id,
        "action_type": action.action_type,
        "risk_level": action.risk_level,
        "rationale": action.rationale,
        "simulated_effects": effects,
        "warnings": warnings,
        "non_authorizing_warning": "This is a dry-run simulation only. No real execution occurred.",
    });

    let status = if effects.is_empty() {
        SandboxRunStatus::Failed
    } else {
        SandboxRunStatus::Completed
    };

    (output, warnings, status)
}

impl AppState {
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, InMemoryStore>, ApiError> {
        self.store
            .lock()
            .map_err(|_| ApiError::internal("in-memory store lock poisoned"))
    }
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: message.into(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: message.into(),
        }
    }
}

impl From<arpagona_llm::LlmError> for ApiError {
    fn from(error: arpagona_llm::LlmError) -> Self {
        match error {
            arpagona_llm::LlmError::MissingApiKey => ApiError {
                status: StatusCode::SERVICE_UNAVAILABLE,
                message: error.to_string(),
            },
            _ => ApiError::internal(error.to_string()),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorResponse {
                error: self.message,
            }),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_decision_status_to_proposed_action_status() {
        assert_eq!(
            status_from_decision(&DecisionStatus::Approved),
            ProposedActionStatus::Approved
        );
        assert_eq!(
            status_from_decision(&DecisionStatus::Blocked),
            ProposedActionStatus::Blocked
        );
        assert_eq!(
            status_from_decision(&DecisionStatus::NeedsHumanApproval),
            ProposedActionStatus::NeedsHumanApproval
        );
    }

    #[tokio::test]
    async fn agent_propose_openai_without_key_returns_error_without_side_effects() {
        std::env::remove_var("OPENAI_API_KEY");
        let state = AppState::default();
        let error = agent_propose(
            State(state.clone()),
            Json(AgentProposeRequest {
                workspace_id: "workspace-alpha".to_owned(),
                task_id: Some("task-1".to_owned()),
                prompt: "Prépare un brouillon de réponse client".to_owned(),
                provider: "openai".to_owned(),
            }),
        )
        .await
        .expect_err("missing OPENAI_API_KEY should fail clearly");

        assert_eq!(error.status, StatusCode::SERVICE_UNAVAILABLE);
        assert!(error.message.contains("OPENAI_API_KEY"));

        let store = state.lock().expect("store should lock");
        assert_eq!(store.proposed_actions.len(), 0);
        assert_eq!(store.decisions.len(), 0);
        assert_eq!(store.audit_events.len(), 0);
    }

    #[tokio::test]
    async fn agent_propose_openai_direct_reply_has_no_side_effects() {
        std::env::remove_var("OPENAI_API_KEY");
        let state = AppState::default();
        let response = agent_propose(
            State(state.clone()),
            Json(AgentProposeRequest {
                workspace_id: "workspace-alpha".to_owned(),
                task_id: Some("task-1".to_owned()),
                prompt: "salut".to_owned(),
                provider: "openai".to_owned(),
            }),
        )
        .await
        .expect("deterministic direct reply should not need OpenAI auth");

        assert_eq!(response.0.kind, "direct_reply");
        assert!(response
            .0
            .message
            .as_deref()
            .unwrap_or("")
            .contains("ARPAGONA"));
        assert!(response.0.proposed_action.is_none());

        let store = state.lock().expect("store should lock");
        assert_eq!(store.proposed_actions.len(), 0);
        assert_eq!(store.decisions.len(), 0);
        assert_eq!(store.audit_events.len(), 0);
    }

    #[tokio::test]
    async fn agent_propose_openai_system_check_stores_pending_action() {
        std::env::remove_var("OPENAI_API_KEY");
        let state = AppState::default();
        let response = agent_propose(
            State(state.clone()),
            Json(AgentProposeRequest {
                workspace_id: "workspace-alpha".to_owned(),
                task_id: Some("task-1".to_owned()),
                prompt: "vérifie l’état du système sans rien exécuter de dangereux".to_owned(),
                provider: "openai".to_owned(),
            }),
        )
        .await
        .expect("deterministic system check should not need OpenAI auth");

        assert_eq!(response.0.kind, "proposed_action");
        let action = response
            .0
            .proposed_action
            .as_ref()
            .expect("system check should include action");
        assert_eq!(action.action_type, ActionType::SystemCheck);
        assert_eq!(action.status, ProposedActionStatus::PendingDecision);

        let store = state.lock().expect("store should lock");
        assert_eq!(store.proposed_actions.len(), 1);
        assert_eq!(store.decisions.len(), 0);
        assert_eq!(store.audit_events.len(), 0);
    }

    #[tokio::test]
    async fn agent_propose_openai_clarifying_question_has_no_side_effects() {
        std::env::remove_var("OPENAI_API_KEY");
        let state = AppState::default();
        let response = agent_propose(
            State(state.clone()),
            Json(AgentProposeRequest {
                workspace_id: "workspace-alpha".to_owned(),
                task_id: Some("task-1".to_owned()),
                prompt: "aide".to_owned(),
                provider: "openai".to_owned(),
            }),
        )
        .await
        .expect("deterministic clarifying question should not need OpenAI auth");

        assert_eq!(response.0.kind, "clarifying_question");
        assert!(response.0.question.is_some());
        assert!(response.0.proposed_action.is_none());

        let store = state.lock().expect("store should lock");
        assert_eq!(store.proposed_actions.len(), 0);
        assert_eq!(store.decisions.len(), 0);
        assert_eq!(store.audit_events.len(), 0);
    }

    #[tokio::test]
    async fn agent_propose_with_mock_stores_pending_action_without_decision() {
        let state = AppState::default();
        let response = agent_propose(
            State(state.clone()),
            Json(AgentProposeRequest {
                workspace_id: "workspace-alpha".to_owned(),
                task_id: Some("task-1".to_owned()),
                prompt: "Prépare un brouillon de réponse client".to_owned(),
                provider: "mock".to_owned(),
            }),
        )
        .await
        .expect("mock proposal should succeed");

        let proposed_action = response
            .0
            .proposed_action
            .as_ref()
            .expect("mock response should include proposed action");
        assert_eq!(
            proposed_action.status,
            ProposedActionStatus::PendingDecision
        );
        assert_eq!(proposed_action.proposed_by.as_str(), "agent-proposer-v0");

        let store = state.lock().expect("store should lock");
        assert_eq!(store.proposed_actions.len(), 1);
        assert_eq!(store.decisions.len(), 0);
        assert_eq!(store.audit_events.len(), 0);
    }

    // -- Executor state management endpoints -------------------------------

    #[tokio::test]
    async fn list_executors_returns_noop_executor_with_disabled_state() {
        let state = AppState::default();
        let response = list_executors(State(state.clone()))
            .await
            .expect("list executors should succeed");
        let executors = response.0;
        let noop = executors
            .iter()
            .find(|e| e.executor_id == "noop-executor")
            .expect("noop-executor should be in list");
        assert_eq!(noop.executor_state, ExecutorState::Disabled);
        assert!(!noop.supported_action_types.is_empty());
    }

    #[tokio::test]
    async fn list_executors_returns_all_registered_executors() {
        let state = AppState::default();
        let response = list_executors(State(state.clone()))
            .await
            .expect("list executors should succeed");
        assert_eq!(response.0.len(), 1); // only noop-executor by default
        assert_eq!(response.0[0].executor_id, "noop-executor");
    }

    #[tokio::test]
    async fn set_executor_state_changes_state_from_disabled_to_ready() {
        let state = AppState::default();
        let response = set_executor_state_handler(
            State(state.clone()),
            Path("noop-executor".to_owned()),
            Json(SetExecutorStateRequest {
                state: ExecutorState::Ready,
            }),
        )
        .await
        .expect("set executor state should succeed");
        assert_eq!(response.0.executor_id, "noop-executor");
        assert_eq!(response.0.executor_state, ExecutorState::Ready);

        // Verify the state persisted in the store
        let store = state.lock().expect("store should lock");
        assert_eq!(
            store.executor_registry.get_state("noop-executor"),
            Some(ExecutorState::Ready)
        );
    }

    #[tokio::test]
    async fn set_executor_state_unknown_executor_returns_404() {
        let state = AppState::default();
        let result = set_executor_state_handler(
            State(state.clone()),
            Path("unknown-executor".to_owned()),
            Json(SetExecutorStateRequest {
                state: ExecutorState::Ready,
            }),
        )
        .await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.status, StatusCode::NOT_FOUND);
        assert!(err.message.contains("unknown-executor"));
    }

    #[tokio::test]
    async fn set_executor_state_transitions_through_all_states() {
        let state = AppState::default();

        // Disabled -> Ready
        let resp = set_executor_state_handler(
            State(state.clone()),
            Path("noop-executor".to_owned()),
            Json(SetExecutorStateRequest {
                state: ExecutorState::Ready,
            }),
        )
        .await
        .expect("transition to Ready should succeed");
        assert_eq!(resp.0.executor_state, ExecutorState::Ready);

        // Ready -> Blocked
        let resp = set_executor_state_handler(
            State(state.clone()),
            Path("noop-executor".to_owned()),
            Json(SetExecutorStateRequest {
                state: ExecutorState::Blocked,
            }),
        )
        .await
        .expect("transition to Blocked should succeed");
        assert_eq!(resp.0.executor_state, ExecutorState::Blocked);

        // Blocked -> Ready
        let resp = set_executor_state_handler(
            State(state.clone()),
            Path("noop-executor".to_owned()),
            Json(SetExecutorStateRequest {
                state: ExecutorState::Ready,
            }),
        )
        .await
        .expect("transition back to Ready should succeed");
        assert_eq!(resp.0.executor_state, ExecutorState::Ready);
    }

    // ── Route matching tests ───────────────────────────────────────────────
    //
    // These tests prove that all dynamic path parameter routes using `:param`
    // syntax actually match at runtime. They start a real HTTP server and
    // send requests through Axum's router.

    async fn start_test_server() -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind server");
        let port = listener.local_addr().expect("port").port();
        let url = format!("http://127.0.0.1:{}", port);

        let state = AppState::default();
        let app = app(state);

        tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve");
        });

        // Small yield to let the server start accepting
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        url
    }

    #[tokio::test]
    async fn route_health_matches() {
        let base = start_test_server().await;
        let resp = reqwest::get(&format!("{}/health", base))
            .await
            .expect("GET /health");
        assert_eq!(resp.status().as_u16(), 200);
    }

    #[tokio::test]
    async fn route_get_executors_matches() {
        let base = start_test_server().await;
        let resp = reqwest::get(&format!("{}/executors", base))
            .await
            .expect("GET /executors");
        assert_eq!(resp.status().as_u16(), 200);
        let body = resp.text().await.unwrap();
        assert!(body.contains("noop-executor"));
    }

    #[tokio::test]
    async fn route_get_executor_by_id_matches() {
        let base = start_test_server().await;
        // Known executor → 200
        let resp = reqwest::get(&format!("{}/executors/noop-executor", base))
            .await
            .expect("GET /executors/noop-executor");
        assert_eq!(resp.status().as_u16(), 200);
        let body = resp.text().await.unwrap();
        assert!(body.contains("noop-executor"));
    }

    #[tokio::test]
    async fn route_get_executor_by_id_not_found() {
        let base = start_test_server().await;
        // Unknown executor → handler 404 (not router 404)
        let resp = reqwest::get(&format!("{}/executors/unknown-executor", base))
            .await
            .expect("GET /executors/unknown-executor");
        assert_eq!(resp.status().as_u16(), 404);
        let body = resp.text().await.unwrap();
        assert!(
            body.contains("error"),
            "handler 404 must have error body: {}",
            body
        );
    }

    #[tokio::test]
    async fn route_post_executor_state_matches() {
        let base = start_test_server().await;
        let client = reqwest::Client::new();
        // Set noop-executor to Ready
        let resp = client
            .post(&format!("{}/executors/noop-executor/state", base))
            .header("Content-Type", "application/json")
            .body(r#"{"state": "ready"}"#)
            .send()
            .await
            .expect("POST /executors/noop-executor/state");
        assert_eq!(resp.status().as_u16(), 200);
        let body = resp.text().await.unwrap();
        assert!(body.contains("ready"), "state should be ready: {}", body);
    }

    #[tokio::test]
    async fn route_post_executor_state_unknown_returns_handler_404() {
        let base = start_test_server().await;
        let client = reqwest::Client::new();
        let resp = client
            .post(&format!("{}/executors/unknown-executor/state", base))
            .header("Content-Type", "application/json")
            .body(r#"{"state": "ready"}"#)
            .send()
            .await
            .expect("POST /executors/unknown-executor/state");
        // Handler 404 (not router 404) proves route matched
        assert_eq!(resp.status().as_u16(), 404);
        let body = resp.text().await.unwrap();
        assert!(
            body.contains("error"),
            "handler 404 must have error body: {}",
            body
        );
    }

    #[tokio::test]
    async fn route_get_execution_capabilities_matches() {
        let base = start_test_server().await;
        let resp = reqwest::get(&format!("{}/execution-capabilities", base))
            .await
            .expect("GET /execution-capabilities");
        assert_eq!(resp.status().as_u16(), 200);
    }

    #[tokio::test]
    async fn route_get_execution_capability_by_type_matches() {
        let base = start_test_server().await;
        let resp = reqwest::get(&format!("{}/execution-capabilities/read_memory", base))
            .await
            .expect("GET /execution-capabilities/read_memory");
        assert_eq!(resp.status().as_u16(), 200);
        let body = resp.text().await.unwrap();
        assert!(body.contains("read_memory"));
    }

    #[tokio::test]
    async fn route_get_execution_capability_unknown_type_matches() {
        let base = start_test_server().await;
        // Unknown type — handler returns valid capability, proving route matched
        // (not a router 404)
        let resp = reqwest::get(&format!("{}/execution-capabilities/unknown_type", base))
            .await
            .expect("GET /execution-capabilities/unknown_type");
        assert_eq!(resp.status().as_u16(), 200);
        let body = resp.text().await.unwrap();
        assert!(
            body.contains("action_type"),
            "should return capability: {}",
            body
        );
    }

    #[tokio::test]
    async fn route_post_proposed_actions_review_matches() {
        let base = start_test_server().await;
        let client = reqwest::Client::new();
        // Non-existent proposal → handler 404, proving route matched
        let resp = client
            .post(&format!("{}/proposed-actions/test-id/review", base))
            .header("Content-Type", "application/json")
            .body(r#"{"action": "approve", "reason": "test", "actor": "test"}"#)
            .send()
            .await
            .expect("POST /proposed-actions/test-id/review");
        assert_eq!(resp.status().as_u16(), 404);
        let body = resp.text().await.unwrap();
        assert!(
            body.contains("error"),
            "handler 404 must have error body: {}",
            body
        );
        assert!(
            body.contains("not found"),
            "should mention not found: {}",
            body
        );
    }

    #[tokio::test]
    async fn route_post_proposed_actions_sandbox_matches() {
        let base = start_test_server().await;
        let client = reqwest::Client::new();
        // No body → may get 422 from serde, proving route matched (not router 404)
        let resp = client
            .post(&format!("{}/proposed-actions/test-id/sandbox", base))
            .send()
            .await
            .expect("POST /proposed-actions/test-id/sandbox");
        let status = resp.status().as_u16();
        assert!(
            status != 404 || resp.text().await.unwrap().contains("error"),
            "route should match, got status={}",
            status
        );
    }

    #[tokio::test]
    async fn route_post_proposed_actions_dry_run_matches() {
        let base = start_test_server().await;
        let client = reqwest::Client::new();
        let resp = client
            .post(&format!("{}/proposed-actions/test-id/dry-run", base))
            .send()
            .await
            .expect("POST /proposed-actions/test-id/dry-run");
        let status = resp.status().as_u16();
        assert!(
            status != 404 || resp.text().await.unwrap().contains("error"),
            "route should match, got status={}",
            status
        );
    }

    #[tokio::test]
    async fn route_post_proposed_actions_policy_check_matches() {
        let base = start_test_server().await;
        let client = reqwest::Client::new();
        let resp = client
            .post(&format!("{}/proposed-actions/test-id/policy-check", base))
            .send()
            .await
            .expect("POST /proposed-actions/test-id/policy-check");
        let status = resp.status().as_u16();
        assert!(
            status != 404 || resp.text().await.unwrap().contains("error"),
            "route should match, got status={}",
            status
        );
    }

    #[tokio::test]
    async fn route_post_proposed_actions_execute_matches() {
        let base = start_test_server().await;
        let client = reqwest::Client::new();
        let resp = client
            .post(&format!("{}/proposed-actions/test-id/execute", base))
            .send()
            .await
            .expect("POST /proposed-actions/test-id/execute");
        let status = resp.status().as_u16();
        assert!(
            status != 404 || resp.text().await.unwrap().contains("error"),
            "route should match, got status={}",
            status
        );
    }

    #[tokio::test]
    async fn route_unknown_path_returns_router_404() {
        let base = start_test_server().await;
        let resp = reqwest::get(&format!("{}/this/path/does/not/exist", base))
            .await
            .expect("GET unknown path");
        assert_eq!(resp.status().as_u16(), 404);
        // Router 404 has empty or non-JSON body
        let body = resp.text().await.unwrap();
        assert!(
            !body.contains("error"),
            "router 404 should NOT have JSON error body: {}",
            body
        );
    }
}
