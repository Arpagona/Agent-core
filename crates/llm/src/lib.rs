//! Experimental LLM providers for ARPAGONA Agent Core.
//!
//! This crate is intentionally non-executing: providers route user prompts into
//! [`AgentTurnDraft`] values. A turn may be a direct conversational reply, a
//! clarifying question, or a [`ProposedActionDraft`]. Providers never execute
//! tools, never call the Decision Gate, and every materialized [`ProposedAction`]
//! stays `PendingDecision` until another layer explicitly evaluates it.

use std::env;
use std::error::Error;
use std::fmt;
use std::future::Future;

use arpagona_agent_core::{
    ActionType, AgentId, Permission, ProposedAction, ProposedActionId, ProposedActionStatus,
    RiskLevel, TaskId, WorkspaceId,
};
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const DEFAULT_OPENAI_ENDPOINT: &str = "https://api.openai.com/v1/responses";
const DEFAULT_OPENAI_MODEL: &str = "gpt-4.1-mini";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlmActionRequest {
    pub prompt: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AgentTurnDraft {
    DirectReply { message: String },
    ProposedAction { action: ProposedActionDraft },
    ClarifyingQuestion { question: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntentClass {
    DirectReply,
    ProposedAction,
    ClarifyingQuestion,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProposedActionDraft {
    pub action_type: ActionType,
    pub target: Option<String>,
    pub risk_level: RiskLevel,
    pub required_permissions: Vec<Permission>,
    pub rationale: String,
    #[serde(default = "empty_payload")]
    pub payload: Value,
}

impl ProposedActionDraft {
    pub fn into_proposed_action(
        self,
        workspace_id: WorkspaceId,
        task_id: Option<TaskId>,
        proposed_by: AgentId,
        generated_action_id: ProposedActionId,
    ) -> ProposedAction {
        ProposedAction {
            id: ProposedActionId::new(safe_id(generated_action_id.as_str())),
            workspace_id,
            task_id,
            proposed_by,
            action_type: self.action_type,
            target: self.target,
            payload: self.payload,
            risk_level: self.risk_level,
            required_permissions: self.required_permissions,
            rationale: self.rationale,
            context_refs: vec![],
            status: ProposedActionStatus::PendingDecision,
            created_at: Utc::now(),
        }
    }
}

pub trait LlmProvider {
    fn propose_action(
        &self,
        request: LlmActionRequest,
    ) -> impl Future<Output = Result<ProposedActionDraft, LlmError>> + Send;
}

#[derive(Clone, Debug)]
pub struct MockProvider {
    draft: ProposedActionDraft,
}

impl MockProvider {
    pub fn new(draft: ProposedActionDraft) -> Self {
        Self { draft }
    }

    pub fn safe_default() -> Self {
        Self::new(ProposedActionDraft {
            action_type: ActionType::SimulateEmail,
            target: Some("client-response-draft".to_owned()),
            risk_level: RiskLevel::Low,
            required_permissions: vec![Permission::SimulateEmail],
            rationale: "Mock provider returns a draft only; execution remains gated.".to_owned(),
            payload: json!({
                "draft": "Nous allons préparer et envoyer un devis prochainement.",
                "llm_executed": false
            }),
        })
    }
}

impl LlmProvider for MockProvider {
    fn propose_action(
        &self,
        _request: LlmActionRequest,
    ) -> impl Future<Output = Result<ProposedActionDraft, LlmError>> + Send {
        let draft = self.draft.clone();
        async move { Ok(draft) }
    }
}

#[derive(Clone, Debug)]
pub struct OpenAiProvider {
    client: Client,
    endpoint: String,
    model: String,
    api_key: String,
}

impl OpenAiProvider {
    pub fn from_env() -> Result<Self, LlmError> {
        let api_key = env::var("OPENAI_API_KEY").map_err(|_| LlmError::MissingApiKey)?;
        let model = env::var("OPENAI_MODEL").unwrap_or_else(|_| DEFAULT_OPENAI_MODEL.to_owned());
        let endpoint = env::var("OPENAI_RESPONSES_ENDPOINT")
            .unwrap_or_else(|_| DEFAULT_OPENAI_ENDPOINT.to_owned());

        Ok(Self {
            client: Client::new(),
            endpoint,
            model,
            api_key,
        })
    }

    pub fn new(
        api_key: impl Into<String>,
        model: impl Into<String>,
        endpoint: impl Into<String>,
    ) -> Self {
        Self {
            client: Client::new(),
            endpoint: endpoint.into(),
            model: model.into(),
            api_key: api_key.into(),
        }
    }
}

impl OpenAiProvider {
    pub fn propose_turn(
        &self,
        request: LlmActionRequest,
    ) -> impl Future<Output = Result<AgentTurnDraft, LlmError>> + Send {
        let client = self.client.clone();
        let endpoint = self.endpoint.clone();
        let model = self.model.clone();
        let api_key = self.api_key.clone();

        async move {
            if let Some(turn) = deterministic_turn_for_prompt(&request.prompt) {
                return Ok(turn);
            }

            let body = json!({
                "model": model,
                "tools": [],
                "input": [
                    {
                        "role": "system",
                        "content": [
                            {"type": "input_text", "text": provider_system_prompt()}
                        ]
                    },
                    {
                        "role": "user",
                        "content": [
                            {"type": "input_text", "text": request.prompt}
                        ]
                    }
                ],
                "text": {
                    "format": { "type": "json_object" }
                }
            });

            let response = client
                .post(&endpoint)
                .bearer_auth(&api_key)
                .json(&body)
                .send()
                .await
                .map_err(|err| LlmError::Transport(err.to_string()))?;

            let status = response.status();
            let value: Value = response.json().await.map_err(|err| {
                LlmError::InvalidResponse(format!("invalid JSON response: {err}"))
            })?;

            if !status.is_success() {
                let message = value
                    .pointer("/error/message")
                    .and_then(Value::as_str)
                    .unwrap_or("OpenAI Responses API returned an error");
                return Err(LlmError::Provider(format!(
                    "OpenAI error {status}: {message}"
                )));
            }

            let text = extract_output_text(&value).ok_or_else(|| {
                LlmError::InvalidResponse("missing output_text in OpenAI response".to_owned())
            })?;

            parse_agent_turn(&text)
        }
    }
}

impl LlmProvider for OpenAiProvider {
    fn propose_action(
        &self,
        request: LlmActionRequest,
    ) -> impl Future<Output = Result<ProposedActionDraft, LlmError>> + Send {
        let turn = self.propose_turn(request);
        async move {
            match turn.await? {
                AgentTurnDraft::ProposedAction { action } => Ok(action),
                AgentTurnDraft::DirectReply { .. } => Err(LlmError::InvalidResponse(
                    "provider returned direct_reply where proposed_action was required".to_owned(),
                )),
                AgentTurnDraft::ClarifyingQuestion { .. } => Err(LlmError::InvalidResponse(
                    "provider returned clarifying_question where proposed_action was required"
                        .to_owned(),
                )),
            }
        }
    }
}

pub fn provider_system_prompt() -> &'static str {
    r#"You are ARPAGONA Agent Core Chat Router V0.
Return only one JSON object using exactly one of these shapes:

DirectReply for normal conversation that needs no system action:
{
  "kind": "direct_reply",
  "message": string
}

ClarifyingQuestion when the user intent is ambiguous and a safe action cannot be selected:
{
  "kind": "clarifying_question",
  "question": string
}

ProposedAction only when the user asks for an operation, decision, task, system check, memory read/write, audit read, external action, or workflow:
{
  "kind": "proposed_action",
  "action": {
    "action_type": "simulate_email" | "read_document" | "read_memory" | "manage_task" | {"custom":"read_audit"} | {"custom":"system_check"} | {"custom":"send_email"} | {"custom":"..."},
    "target": string | null,
    "risk_level": "informational" | "low" | "medium" | "high" | "critical",
    "required_permissions": ["simulate_email" | "read_document" | "read_memory" | "manage_task" | "propose_tool_use"],
    "rationale": string,
    "payload": object
  }
}

Routing rules:
- Greetings such as "salut", identity questions such as "qui es-tu ?", and capability questions such as "explique-moi ce que tu peux faire" are DirectReply.
- Requests to verify system state are ProposedAction with custom action_type "system_check" and permission "propose_tool_use".
- Requests to read audit logs are ProposedAction with custom action_type "read_audit" and permission "read_memory".
- Requests to send or draft email are ProposedAction using simulate_email unless explicit send_email capability and permissions are available.
- If no action is needed, never use simulate_email as a fallback.
Never execute anything. Never claim that an email, document write, tool call, web search, memory write, or system check has been executed.
Do not use OpenAI tools, web search, function calling, or external side effects.
Every ProposedAction is only a pending proposal for the Decision Gate and human-in-the-loop approval."#
}

pub fn classify_prompt_intent(prompt: &str) -> IntentClass {
    let normalized = normalize_prompt(prompt);

    if is_direct_reply_prompt(&normalized) {
        return IntentClass::DirectReply;
    }

    if is_clarifying_prompt(&normalized) {
        return IntentClass::ClarifyingQuestion;
    }

    if is_action_prompt(&normalized) {
        return IntentClass::ProposedAction;
    }

    IntentClass::DirectReply
}

pub fn deterministic_turn_for_prompt(prompt: &str) -> Option<AgentTurnDraft> {
    let normalized = normalize_prompt(prompt);
    match classify_prompt_intent(prompt) {
        IntentClass::DirectReply if is_greeting_prompt(&normalized) => Some(AgentTurnDraft::DirectReply {
            message: "Salut. Je suis ARPAGONA Agent Core, en mode alpha gouverné.".to_owned(),
        }),
        IntentClass::DirectReply if is_identity_prompt(&normalized) => Some(AgentTurnDraft::DirectReply {
            message: "Je suis ARPAGONA Agent Core : un runtime agentique local-first, graph-native et gouverné. En alpha, je peux expliquer, préparer des propositions d’action et garder toute action en attente du Decision Gate.".to_owned(),
        }),
        IntentClass::DirectReply if is_capability_prompt(&normalized) => Some(AgentTurnDraft::DirectReply {
            message: "Je peux répondre directement aux questions simples, proposer des actions gouvernées quand une opération est demandée, et demander une clarification si l’intention est ambiguë. Les actions restent toujours pending_decision.".to_owned(),
        }),
        IntentClass::ClarifyingQuestion => Some(AgentTurnDraft::ClarifyingQuestion {
            question: "Que veux-tu que je prépare exactement : une réponse simple, une tâche, une lecture mémoire/audit, ou une proposition d’action gouvernée ?".to_owned(),
        }),
        IntentClass::ProposedAction if contains_any(&normalized, &["journal", "journaux", "audit", "audits", "log", "logs"]) => {
            Some(AgentTurnDraft::ProposedAction {
                action: ProposedActionDraft {
                    action_type: ActionType::Custom("read_audit".to_owned()),
                    target: Some("audit_logs".to_owned()),
                    risk_level: RiskLevel::Informational,
                    required_permissions: vec![Permission::ReadMemory],
                    rationale: "L’utilisateur demande une lecture des journaux d’audit; cela doit rester une action proposée et gouvernée.".to_owned(),
                    payload: json!({"operation": "read_audit", "llm_executed": false}),
                },
            })
        }
        IntentClass::ProposedAction if contains_any(&normalized, &["systeme", "system", "etat", "status", "verifie", "verifier", "check"]) => {
            Some(AgentTurnDraft::ProposedAction {
                action: ProposedActionDraft {
                    action_type: ActionType::Custom("system_check".to_owned()),
                    target: Some("system_state".to_owned()),
                    risk_level: RiskLevel::Informational,
                    required_permissions: vec![Permission::ProposeToolUse],
                    rationale: "L’utilisateur demande une vérification système; seule une proposition gouvernée est créée.".to_owned(),
                    payload: json!({"operation": "system_check", "dangerous_execution_allowed": false, "llm_executed": false}),
                },
            })
        }
        IntentClass::ProposedAction if contains_any(&normalized, &["mail", "email", "e-mail", "courriel"]) => {
            Some(AgentTurnDraft::ProposedAction {
                action: ProposedActionDraft {
                    action_type: ActionType::SimulateEmail,
                    target: None,
                    risk_level: RiskLevel::Medium,
                    required_permissions: vec![Permission::SimulateEmail],
                    rationale: "L’utilisateur demande une action email; en alpha, elle reste simulée et soumise au Decision Gate.".to_owned(),
                    payload: json!({"operation": "simulate_email", "llm_executed": false}),
                },
            })
        }
        _ => None,
    }
}

fn parse_agent_turn(text: &str) -> Result<AgentTurnDraft, LlmError> {
    if let Ok(turn) = serde_json::from_str::<AgentTurnDraft>(text) {
        return Ok(turn);
    }

    serde_json::from_str::<ProposedActionDraft>(text)
        .map(|action| AgentTurnDraft::ProposedAction { action })
        .map_err(|err| {
            LlmError::InvalidResponse(format!(
                "model did not return a valid AgentTurnDraft JSON: {err}"
            ))
        })
}

fn normalize_prompt(prompt: &str) -> String {
    prompt
        .trim()
        .to_ascii_lowercase()
        .replace('é', "e")
        .replace('è', "e")
        .replace('ê', "e")
        .replace('à', "a")
        .replace('ù', "u")
        .replace('ç', "c")
}

fn is_direct_reply_prompt(prompt: &str) -> bool {
    is_greeting_prompt(prompt) || is_identity_prompt(prompt) || is_capability_prompt(prompt)
}

fn is_greeting_prompt(prompt: &str) -> bool {
    matches!(
        prompt.trim_matches(|ch: char| ch.is_ascii_punctuation() || ch.is_whitespace()),
        "salut" | "bonjour" | "hello" | "hey" | "coucou" | "bonsoir"
    )
}

fn is_identity_prompt(prompt: &str) -> bool {
    contains_any(
        prompt,
        &["qui es-tu", "qui es tu", "tu es qui", "parle", "toi"],
    )
}

fn is_capability_prompt(prompt: &str) -> bool {
    contains_any(
        prompt,
        &[
            "ce que tu peux faire",
            "que peux-tu faire",
            "que peux tu faire",
            "tes capacites",
            "explique-moi",
        ],
    )
}

fn is_clarifying_prompt(prompt: &str) -> bool {
    matches!(
        prompt.trim(),
        "aide" | "help" | "tu peux m'aider" | "tu peux maider" | "fais quelque chose"
    )
}

fn is_action_prompt(prompt: &str) -> bool {
    contains_any(
        prompt,
        &[
            "verifie",
            "verifier",
            "check",
            "etat",
            "systeme",
            "system",
            "lis",
            "lire",
            "journaux",
            "audit",
            "logs",
            "memoire",
            "memory",
            "ecris",
            "ecrire",
            "cree",
            "creer",
            "tache",
            "workflow",
            "envoie",
            "envoyer",
            "mail",
            "email",
            "decision",
            "operation",
            "execute",
            "lance",
        ],
    )
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

fn extract_output_text(value: &Value) -> Option<String> {
    if let Some(text) = value.get("output_text").and_then(Value::as_str) {
        return Some(text.to_owned());
    }

    value
        .get("output")?
        .as_array()?
        .iter()
        .flat_map(|item| {
            item.get("content")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        })
        .filter_map(|content| content.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("")
        .into_non_empty()
}

fn safe_id(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            ':' => '-',
            ch if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') => ch,
            _ => '-',
        })
        .collect()
}

fn empty_payload() -> Value {
    json!({})
}

trait NonEmptyString {
    fn into_non_empty(self) -> Option<String>;
}

impl NonEmptyString for String {
    fn into_non_empty(self) -> Option<String> {
        if self.is_empty() {
            None
        } else {
            Some(self)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlmError {
    MissingApiKey,
    Transport(String),
    Provider(String),
    InvalidResponse(String),
}

impl fmt::Display for LlmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LlmError::MissingApiKey => write!(
                f,
                "OPENAI_API_KEY is required for provider=openai and was not found"
            ),
            LlmError::Transport(message) => write!(f, "LLM transport error: {message}"),
            LlmError::Provider(message) => write!(f, "LLM provider error: {message}"),
            LlmError::InvalidResponse(message) => write!(f, "Invalid LLM response: {message}"),
        }
    }
}

impl Error for LlmError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_provider_returns_configured_draft() {
        let provider = MockProvider::safe_default();
        let draft = provider
            .propose_action(LlmActionRequest {
                prompt: "Prépare un brouillon de réponse client".to_owned(),
            })
            .await
            .expect("mock provider should not fail");

        assert_eq!(draft.action_type, ActionType::SimulateEmail);
        assert_eq!(draft.risk_level, RiskLevel::Low);
    }

    #[test]
    fn proposed_action_draft_serializes_and_deserializes() {
        let draft = ProposedActionDraft {
            action_type: ActionType::SimulateEmail,
            target: Some("client-response-draft".to_owned()),
            risk_level: RiskLevel::Low,
            required_permissions: vec![Permission::SimulateEmail],
            rationale: "Prepare a draft only.".to_owned(),
            payload: json!({"body": "Bonjour"}),
        };

        let encoded = serde_json::to_string(&draft).expect("draft should serialize");
        let decoded: ProposedActionDraft =
            serde_json::from_str(&encoded).expect("draft should deserialize");

        assert_eq!(decoded, draft);
    }

    #[test]
    fn draft_materializes_pending_proposed_action_with_safe_id() {
        let draft = ProposedActionDraft {
            action_type: ActionType::ManageTask,
            target: Some("task-1".to_owned()),
            risk_level: RiskLevel::Low,
            required_permissions: vec![Permission::ManageTask],
            rationale: "Create a task proposal only.".to_owned(),
            payload: json!({"title": "Follow up"}),
        };

        let action = draft.into_proposed_action(
            WorkspaceId::new("workspace-alpha"),
            Some(TaskId::new("task-1")),
            AgentId::new("agent-proposer-v0"),
            ProposedActionId::new("action:1"),
        );

        assert_eq!(action.id.as_str(), "action-1");
        assert_eq!(action.status, ProposedActionStatus::PendingDecision);
        assert_eq!(action.context_refs.len(), 0);
    }

    #[test]
    fn llm_originated_action_remains_pending_decision() {
        let action = MockProvider::safe_default().draft.into_proposed_action(
            WorkspaceId::new("workspace-alpha"),
            None,
            AgentId::new("agent-proposer-v0"),
            ProposedActionId::new("action-llm-1"),
        );

        assert_eq!(action.status, ProposedActionStatus::PendingDecision);
    }

    #[test]
    fn extracts_response_output_text() {
        let value = json!({"output_text": "{\"rationale\":\"ok\"}"});
        assert_eq!(
            extract_output_text(&value).as_deref(),
            Some("{\"rationale\":\"ok\"}")
        );
    }

    #[test]
    fn routes_conversational_prompts_to_direct_reply() {
        assert_eq!(classify_prompt_intent("salut"), IntentClass::DirectReply);
        assert_eq!(
            classify_prompt_intent("qui es-tu ?"),
            IntentClass::DirectReply
        );
        assert_eq!(
            classify_prompt_intent("explique-moi ce que tu peux faire"),
            IntentClass::DirectReply
        );
        assert!(matches!(
            deterministic_turn_for_prompt("tu peux me parler de toi ?"),
            Some(AgentTurnDraft::DirectReply { .. })
        ));
    }

    #[test]
    fn routes_system_audit_and_email_requests_to_proposed_actions() {
        let system = deterministic_turn_for_prompt(
            "vérifie l’état du système sans rien exécuter de dangereux",
        )
        .expect("system check should route deterministically");
        match system {
            AgentTurnDraft::ProposedAction { action } => {
                assert_eq!(
                    action.action_type,
                    ActionType::Custom("system_check".to_owned())
                );
                assert_eq!(
                    action.required_permissions,
                    vec![Permission::ProposeToolUse]
                );
            }
            other => panic!("expected proposed action, got {other:?}"),
        }

        let audit = deterministic_turn_for_prompt("lis les journaux d’audit")
            .expect("audit read should route deterministically");
        match audit {
            AgentTurnDraft::ProposedAction { action } => {
                assert_eq!(
                    action.action_type,
                    ActionType::Custom("read_audit".to_owned())
                );
                assert_eq!(action.required_permissions, vec![Permission::ReadMemory]);
            }
            other => panic!("expected proposed action, got {other:?}"),
        }

        let email = deterministic_turn_for_prompt("envoie un mail")
            .expect("email should route deterministically");
        match email {
            AgentTurnDraft::ProposedAction { action } => {
                assert_eq!(action.action_type, ActionType::SimulateEmail);
                assert_eq!(action.required_permissions, vec![Permission::SimulateEmail]);
            }
            other => panic!("expected proposed action, got {other:?}"),
        }
    }

    #[test]
    fn parses_agent_turn_without_simulate_email_fallback_for_direct_reply() {
        let turn = parse_agent_turn(r#"{"kind":"direct_reply","message":"Salut."}"#)
            .expect("direct reply should parse");
        assert_eq!(
            turn,
            AgentTurnDraft::DirectReply {
                message: "Salut.".to_owned()
            }
        );
    }

    #[test]
    fn non_action_turns_are_not_proposed_action_drafts() {
        let direct = deterministic_turn_for_prompt("salut")
            .expect("greeting should route deterministically");
        assert!(matches!(direct, AgentTurnDraft::DirectReply { .. }));

        let clarifying = deterministic_turn_for_prompt("aide")
            .expect("ambiguous prompt should route deterministically");
        assert!(matches!(
            clarifying,
            AgentTurnDraft::ClarifyingQuestion { .. }
        ));
    }

    #[test]
    fn deterministic_routing_is_not_action_authorization() {
        let turn = deterministic_turn_for_prompt("vérifie l’état du système")
            .expect("system check should route deterministically");

        match turn {
            AgentTurnDraft::ProposedAction { action } => {
                let proposed_action = action.into_proposed_action(
                    WorkspaceId::new("workspace-alpha"),
                    None,
                    AgentId::new("agent-proposer-v0"),
                    ProposedActionId::new("action-system-check"),
                );

                assert_eq!(
                    proposed_action.status,
                    ProposedActionStatus::PendingDecision
                );
                assert_ne!(proposed_action.status, ProposedActionStatus::Approved);
            }
            other => panic!("expected proposed action, got {other:?}"),
        }
    }
}
