use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use arpagona_agent_core::{
    ActionType, AgentId, AuditEvent, Decision, DecisionStatus, Permission, ProposedAction,
    ProposedActionId, ProposedActionStatus, RiskLevel, Task, TaskId, TaskPriority, TaskStatus,
    WorkspaceId,
};
use arpagona_decision_gate::{audit_event_for_decision, evaluate_proposed_action};
use arpagona_llm::{
    deterministic_turn_for_prompt, AgentTurnDraft, LlmActionRequest, LlmProvider, MockProvider,
    OpenAiProvider,
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

#[derive(Deserialize)]
struct AgentProposeRequest {
    workspace_id: String,
    task_id: Option<String>,
    prompt: String,
    provider: String,
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
        .route("/agent/propose", post(agent_propose))
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
        "mock" => AgentTurnDraft::ProposedAction {
            action: MockProvider::safe_default()
                .propose_action(llm_request)
                .await?,
        },
        other => {
            return Err(ApiError::bad_request(format!(
                "unsupported agent proposer provider '{other}' (expected 'openai' or 'mock')"
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
        assert_eq!(
            action.action_type,
            ActionType::Custom("system_check".to_owned())
        );
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
}
