use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use arpagona_agent_core::{
    audit_event_for_decision, evaluate_proposed_action, ActionType, AgentId, AuditEvent,
    Decision, DecisionStatus, Permission, ProposedAction, ProposedActionId, ProposedActionStatus,
    RiskLevel, Task, TaskId, TaskPriority, TaskStatus, WorkspaceId,
};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Clone, Default)]
struct AppState {
    store: Arc<Mutex<InMemoryStore>>,
}

#[derive(Default)]
struct InMemoryStore {
    tasks: Vec<Task>,
    proposed_actions: Vec<ProposedAction>,
    decisions: Vec<Decision>,
    audit_events: Vec<AuditEvent>,
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

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
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

fn app(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/tasks", post(create_task).get(list_tasks))
        .route(
            "/proposed-actions",
            post(create_proposed_action).get(list_proposed_actions),
        )
        .route("/decision-gate/evaluate", post(evaluate_decision_gate))
        .route("/decisions", get(list_decisions))
        .route("/audit", get(list_audit))
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
    let decision = evaluate_proposed_action(&action, &[], &request.granted_permissions);
    let audit_event = audit_event_for_decision(&action, &decision);

    store.proposed_actions[action_index].status = status_from_decision(&decision.status);
    store.decisions.push(decision.clone());
    store.audit_events.push(audit_event.clone());

    Ok(Json(EvaluateDecisionGateResponse {
        decision,
        audit_event,
    }))
}

async fn list_decisions(State(state): State<AppState>) -> Result<Json<Vec<Decision>>, ApiError> {
    Ok(Json(state.lock()?.decisions.clone()))
}

async fn list_audit(State(state): State<AppState>) -> Result<Json<Vec<AuditEvent>>, ApiError> {
    Ok(Json(state.lock()?.audit_events.clone()))
}

fn status_from_decision(status: &DecisionStatus) -> ProposedActionStatus {
    match status {
        DecisionStatus::Approved => ProposedActionStatus::Approved,
        DecisionStatus::Blocked => ProposedActionStatus::Blocked,
        DecisionStatus::NeedsHumanApproval => ProposedActionStatus::NeedsHumanApproval,
    }
}

fn empty_payload() -> Value {
    json!({})
}

impl AppState {
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, InMemoryStore>, ApiError> {
        self.store
            .lock()
            .map_err(|_| ApiError::internal("in-memory store lock poisoned"))
    }
}

struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
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
}
