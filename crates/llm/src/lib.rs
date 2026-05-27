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

// ─── Ollama provider ───────────────────────────────────────────────────────

const DEFAULT_OLLAMA_ENDPOINT: &str = "http://localhost:11434/api/chat";
const DEFAULT_OLLAMA_MODEL: &str = "qwen3.5:9b";

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

// ─── OpenAI provider ───────────────────────────────────────────────────────

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

// ─── Ollama provider ───────────────────────────────────────────────────────

/// Provider that calls a local Ollama instance for LLM inference.
///
/// Connects to `OLLAMA_ENDPOINT` (default `http://localhost:11434/api/chat`)
/// and uses `OLLAMA_MODEL` (default `gemma4:26b`).
///
/// # Safety
/// - No tool execution, no persistence, no authorization.
/// - The output is evidence-only.
#[derive(Clone, Debug)]
pub struct OllamaProvider {
    client: Client,
    endpoint: String,
    model: String,
}

impl OllamaProvider {
    /// Construct from environment variables.
    pub fn from_env() -> Self {
        let endpoint =
            env::var("OLLAMA_ENDPOINT").unwrap_or_else(|_| DEFAULT_OLLAMA_ENDPOINT.to_owned());
        let model = env::var("OLLAMA_MODEL").unwrap_or_else(|_| DEFAULT_OLLAMA_MODEL.to_owned());
        Self {
            client: Client::new(),
            endpoint,
            model,
        }
    }

    /// Construct with explicit parameters.
    pub fn new(endpoint: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            endpoint: endpoint.into(),
            model: model.into(),
        }
    }

    /// Call Ollama for free-form text synthesis (non-authorizing).
    pub async fn synthesize(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String, LlmError> {
        let body = serde_json::json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt}
            ],
            "stream": false,
            "options": {
                "temperature": 0.7
            }
        });

        let response = self
            .client
            .post(&self.endpoint)
            .json(&body)
            .send()
            .await
            .map_err(|err| LlmError::Transport(format!("Ollama connection failed: {err}")))?;

        let status = response.status();
        let value: serde_json::Value = response
            .json()
            .await
            .map_err(|err| LlmError::InvalidResponse(format!("invalid JSON from Ollama: {err}")))?;

        if !status.is_success() {
            return Err(LlmError::Provider(format!(
                "Ollama returned HTTP {status}: {}",
                value
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown error")
            )));
        }

        let content = value
            .pointer("/message/content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                LlmError::InvalidResponse("Ollama response missing /message/content".to_owned())
            })?;

        Ok(content.to_owned())
    }

    /// Propose a full agent turn (direct_reply, clarifying_question, or proposed_action).
    ///
    /// Uses the [`provider_system_prompt`] to instruct the model to return a
    /// structured JSON turn.  Falls back to the deterministic router first.
    pub async fn propose_turn(
        &self,
        request: LlmActionRequest,
    ) -> Result<AgentTurnDraft, LlmError> {
        if let Some(turn) = deterministic_turn_for_prompt(&request.prompt) {
            return Ok(turn);
        }

        let text = self
            .synthesize(provider_system_prompt(), &request.prompt)
            .await?;
        parse_agent_turn(&text)
    }
}

impl LlmProvider for OllamaProvider {
    fn propose_action(
        &self,
        request: LlmActionRequest,
    ) -> impl Future<Output = Result<ProposedActionDraft, LlmError>> + Send {
        let self_clone = self.clone();
        async move {
            let text = self_clone
                .synthesize(provider_system_prompt(), &request.prompt)
                .await?;
            match parse_agent_turn(&text)? {
                AgentTurnDraft::ProposedAction { action } => Ok(action),
                AgentTurnDraft::DirectReply { .. } => Err(LlmError::InvalidResponse(
                    "Ollama returned direct_reply where proposed_action was required".to_owned(),
                )),
                AgentTurnDraft::ClarifyingQuestion { .. } => Err(LlmError::InvalidResponse(
                    "Ollama returned clarifying_question where proposed_action was required"
                        .to_owned(),
                )),
            }
        }
    }
}

pub fn provider_system_prompt() -> &'static str {
    r#"You are ARPAGONA Agent Core Chat Router V0.
Return only one JSON object using exactly one of these shapes:

DirectReply for normal conversation, advice, analysis, risk discussion, or planning guidance that needs no runtime readback or system operation:
{
  "kind": "direct_reply",
  "message": string
}

ClarifyingQuestion when the user intent is ambiguous and a safe action cannot be selected:
{
  "kind": "clarifying_question",
  "question": string
}

ProposedAction only when the user explicitly asks to inspect, read, list, display, verify, create, update, send, or otherwise operate on runtime state, memory, documents, tools, email, audit, status, tasks, decisions, proposed actions, or a workflow:
{
  "kind": "proposed_action",
  "action": {
    "action_type": "simulate_email" | "read_document" | "read_memory" | "read_tasks" | "read_proposed_actions" | "read_pending_actions" | "read_decisions" | "read_audit" | "read_status" | "system_check" | "manage_task" | {"custom":"send_email"} | {"custom":"..."},
    "target": string | null,
    "risk_level": "informational" | "low" | "medium" | "high" | "critical",
    "required_permissions": ["simulate_email" | "read_document" | "read_memory" | "read_tasks" | "read_proposed_actions" | "read_decisions" | "read_audit" | "read_status" | "manage_task" | "propose_tool_use"],
    "rationale": string,
    "payload": object
  }
}

Routing rules:
- Greetings such as "salut", identity questions such as "qui es-tu ?", capability questions such as "explique-moi ce que tu peux faire", and advice/risk questions such as "que devrais-je vérifier avant..." or "quels sont les risques avant..." are DirectReply.
- Runtime task readback requests are ProposedAction with action_type "read_tasks" and permission "read_tasks".
- Proposed/pending action readback requests are ProposedAction with action_type "read_proposed_actions" or "read_pending_actions" and permission "read_proposed_actions".
- Decision readback requests are ProposedAction with action_type "read_decisions" and permission "read_decisions".
- Audit readback requests are ProposedAction with action_type "read_audit" and permission "read_audit".
- Global runtime status readback requests are ProposedAction with action_type "read_status" and permission "read_status".
- General system verification requests are ProposedAction with action_type "system_check" and permission "propose_tool_use" only when they are not specifically about tasks, proposed actions, pending actions, decisions, audit, memory, or runtime status.
- Use "read_memory" only for long-term/cognitive memory readback. Never use it as a fallback for tasks, proposed actions, decisions, audit, status, or generic runtime objects.
- Requests to send or draft email are ProposedAction using simulate_email unless explicit send_email capability and permissions are available.
- If no action is needed, never use simulate_email, read_memory, or system_check as a fallback.
Never execute anything. Never claim that an email, document write, tool call, web search, memory write, runtime readback, or system check has been executed.
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
            message: "Je suis ARPAGONA Agent Core : un runtime agentique local-first, graph-native et gouverné. En alpha, je peux expliquer, préparer des propositions d'action et garder toute action en attente du Decision Gate.".to_owned(),
        }),
        IntentClass::DirectReply if is_capability_prompt(&normalized) => Some(AgentTurnDraft::DirectReply {
            message: "Je peux répondre directement aux questions simples, proposer des actions gouvernées quand une opération est demandée, et demander une clarification si l'intention est ambiguë. Les actions restent toujours pending_decision.".to_owned(),
        }),
        IntentClass::DirectReply if is_advisory_or_analysis_prompt(&normalized) => Some(AgentTurnDraft::DirectReply {
            message: "Avant d'ajouter une capacité d'exécution réelle, vérifie surtout les garde-fous: permissions explicites, Decision Gate, audit causal, absence de shell libre, périmètre read-only clair, rollback, et validation humaine pour toute action sensible.".to_owned(),
        }),
        IntentClass::ClarifyingQuestion => Some(AgentTurnDraft::ClarifyingQuestion {
            question: "Que veux-tu que je prépare exactement : une réponse simple, une tâche, une lecture mémoire/audit, ou une proposition d'action gouvernée ?".to_owned(),
        }),
        IntentClass::ProposedAction if is_proposed_actions_read_prompt(&normalized) => Some(
            read_only_turn(
                ActionType::ReadProposedActions,
                Some("proposed_actions"),
                Permission::ReadProposedActions,
                "read_proposed_actions",
                "L'utilisateur demande une lecture des actions proposées; cela doit rester une action proposée et gouvernée.",
            ),
        ),
        IntentClass::ProposedAction if is_pending_actions_read_prompt(&normalized) => Some(
            read_only_turn(
                ActionType::ReadPendingActions,
                Some("pending_actions"),
                Permission::ReadProposedActions,
                "read_pending_actions",
                "L'utilisateur demande une lecture des actions en attente; cela doit rester une action proposée et gouvernée.",
            ),
        ),
        IntentClass::ProposedAction if is_tasks_read_prompt(&normalized) => Some(read_only_turn(
            ActionType::ReadTasks,
            Some("tasks"),
            Permission::ReadTasks,
            "read_tasks",
            "L'utilisateur demande une lecture de l'état des tâches; cela doit rester une action proposée et gouvernée.",
        )),
        IntentClass::ProposedAction if is_decisions_read_prompt(&normalized) => Some(
            read_only_turn(
                ActionType::ReadDecisions,
                Some("decisions"),
                Permission::ReadDecisions,
                "read_decisions",
                "L'utilisateur demande une lecture des décisions; cela doit rester une action proposée et gouvernée.",
            ),
        ),
        IntentClass::ProposedAction if is_audit_read_prompt(&normalized) => Some(read_only_turn(
            ActionType::ReadAudit,
            Some("audit_logs"),
            Permission::ReadAudit,
            "read_audit",
            "L'utilisateur demande une lecture des journaux d'audit; cela doit rester une action proposée et gouvernée.",
        )),
        IntentClass::ProposedAction if is_status_read_prompt(&normalized) => Some(read_only_turn(
            ActionType::ReadStatus,
            Some("runtime_status"),
            Permission::ReadStatus,
            "read_status",
            "L'utilisateur demande une lecture du statut global du runtime; cela doit rester une action proposée et gouvernée.",
        )),
        IntentClass::ProposedAction if is_memory_read_prompt(&normalized) => Some(read_only_turn(
            ActionType::ReadMemory,
            Some("cognitive_memory"),
            Permission::ReadMemory,
            "read_memory",
            "L'utilisateur demande une lecture de la mémoire longue durée/cognitive; cela doit rester une action proposée et gouvernée.",
        )),
        IntentClass::ProposedAction if is_system_check_prompt(&normalized) => Some(read_only_turn(
            ActionType::SystemCheck,
            Some("system_state"),
            Permission::ProposeToolUse,
            "system_check",
            "L'utilisateur demande une vérification système générale; seule une proposition gouvernée est créée.",
        )),
        IntentClass::ProposedAction if contains_any(&normalized, &["mail", "email", "e-mail", "courriel"]) => {
            Some(AgentTurnDraft::ProposedAction {
                action: ProposedActionDraft {
                    action_type: ActionType::SimulateEmail,
                    target: None,
                    risk_level: RiskLevel::Medium,
                    required_permissions: vec![Permission::SimulateEmail],
                    rationale: "L'utilisateur demande une action email; en alpha, elle reste simulée et soumise au Decision Gate.".to_owned(),
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
        .replace('â', "a")
        .replace('î', "i")
        .replace('ï', "i")
        .replace('ô', "o")
        .replace('ù', "u")
        .replace('ç', "c")
}

fn is_direct_reply_prompt(prompt: &str) -> bool {
    is_greeting_prompt(prompt)
        || is_identity_prompt(prompt)
        || is_capability_prompt(prompt)
        || is_advisory_or_analysis_prompt(prompt)
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

fn is_advisory_or_analysis_prompt(prompt: &str) -> bool {
    contains_any(
        prompt,
        &[
            "que devrais-je",
            "que devrais je",
            "que dois-je",
            "que dois je",
            "quoi verifier avant",
            "quels sont les risques",
            "quels risques",
            "risques avant",
            "avant d'ajouter",
            "avant dajouter",
        ],
    )
}

fn read_only_turn(
    action_type: ActionType,
    target: Option<&str>,
    permission: Permission,
    operation: &str,
    rationale: &str,
) -> AgentTurnDraft {
    AgentTurnDraft::ProposedAction {
        action: ProposedActionDraft {
            action_type,
            target: target.map(str::to_owned),
            risk_level: RiskLevel::Informational,
            required_permissions: vec![permission],
            rationale: rationale.to_owned(),
            payload: json!({
                "operation": operation,
                "read_only": true,
                "llm_executed": false
            }),
        },
    }
}

fn is_proposed_actions_read_prompt(prompt: &str) -> bool {
    contains_any(
        prompt,
        &["actions proposees", "action proposee", "proposed actions"],
    )
}

fn is_pending_actions_read_prompt(prompt: &str) -> bool {
    contains_any(
        prompt,
        &[
            "actions en attente",
            "action en attente",
            "pending actions",
            "pending action",
        ],
    )
}

fn is_tasks_read_prompt(prompt: &str) -> bool {
    contains_any(prompt, &["taches", "tache", "tasks", "task"])
}

fn is_decisions_read_prompt(prompt: &str) -> bool {
    contains_any(prompt, &["decisions", "decision"])
}

fn is_audit_read_prompt(prompt: &str) -> bool {
    contains_any(
        prompt,
        &["journal", "journaux", "audit", "audits", "log", "logs"],
    )
}

fn is_status_read_prompt(prompt: &str) -> bool {
    contains_any(
        prompt,
        &[
            "statut global",
            "status global",
            "statut du runtime",
            "status du runtime",
            "runtime status",
        ],
    ) || (contains_any(prompt, &["etat global"])
        && contains_any(prompt, &["runtime", "systeme", "system"]))
}

fn is_memory_read_prompt(prompt: &str) -> bool {
    contains_any(prompt, &["memoire", "memory"])
}

fn is_system_check_prompt(prompt: &str) -> bool {
    contains_any(
        prompt,
        &["systeme", "system", "verifie", "verifier", "check"],
    ) || (contains_any(prompt, &["etat general"]) && contains_any(prompt, &["systeme", "system"]))
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
            "liste",
            "lister",
            "affiche",
            "afficher",
            "consulte",
            "consulter",
            "etat",
            "statut",
            "status",
            "runtime",
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

// ─── P5/P6 — Cognitive synthesis bridge ────────────────────────────────────
//
// These functions connect the cognitive work loop to an LLM for synthesis.
// The LLM receives the objective and working memory summary and produces
// a free-form synthesis text. This is non-authorizing — the output is
// evidence only, not an approved action.

/// System prompt for cognitive synthesis.
pub const COGNITIVE_SYNTHESIS_SYSTEM_PROMPT: &str = r#"You are ARPAGONA Agent Core — a cognitive loop synthesizer.

The system has run a heuristic cognitive cycle on an objective. You are given:
1. The objective (what the user wants to achieve)
2. The domain classification
3. The working memory summary (Domain, Sensitivity, Complexity, Context items count,
   Missing context count, Assumptions count, Proposed next action kind)
4. The proposed next action

Your task: synthesize this information into a structured self-scorecard with
grounded bullets. Reference concrete field values from the working-memory summary.
Use exactly this format:

[STATE] Domain: <value>, Sensitivity: <value>, Complexity: <value>
  - <1-2 grounded sentences about the current analysis state, referencing specific field values>

[KEY GAP / RISK] <the most critical missing context, risk, or ambiguity, referencing the specific field>
  - <why this gap matters for progress>

[RECOMMENDED NEXT STEP] <what a human should do next, based on the proposed next action kind>
  - <1 brief rationale sentence>

Do NOT propose tool calls, memory writes, or system operations.
Do NOT claim any action has been executed or authorized.
Keep your total response under 150 words. Respond in the same language as the objective."#;

impl OpenAiProvider {
    /// Call OpenAI for cognitive synthesis (free-form text, non-authorizing).
    pub async fn synthesize(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String, LlmError> {
        let body = json!({
            "model": self.model,
            "tools": [],
            "input": [
                {
                    "role": "system",
                    "content": [
                        {"type": "input_text", "text": system_prompt}
                    ]
                },
                {
                    "role": "user",
                    "content": [
                        {"type": "input_text", "text": user_prompt}
                    ]
                }
            ],
            "text": {
                "format": { "type": "text" }
            }
        });

        let response = self
            .client
            .post(&self.endpoint)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|err| LlmError::Transport(err.to_string()))?;

        let status = response.status();
        let value: Value = response
            .json()
            .await
            .map_err(|err| LlmError::InvalidResponse(format!("invalid JSON response: {err}")))?;

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

        Ok(text)
    }
}

impl MockProvider {
    /// Mock synthesis — returns a deterministic structured output that mimics
    /// the format requested by `COGNITIVE_SYNTHESIS_SYSTEM_PROMPT`.
    ///
    /// Parses the working-memory summary prefix from the user prompt to extract
    /// concrete field values (Domain, Sensitivity, Complexity, etc.) and produces
    /// a grounded self-scorecard. No real LLM is called.
    pub async fn synthesize(
        &self,
        _system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String, LlmError> {
        let (domain, sensitivity, complexity, missing, next_action) =
            parse_wm_summary_fields(user_prompt);

        let gap_line = if missing > 0 {
            format!(
                "Missing context: {} item(s) identified — context gaps may block progress",
                missing
            )
        } else {
            "No missing context detected — analysis is self-contained".to_owned()
        };

        let next_step_line = match next_action.as_str() {
            "requestcontext" | "RequestContext" => {
                "Request context: gather the missing information before proceeding further"
            }
            "stopwithreport" | "StopWithReport" => {
                "Stop with report: sufficient analysis completed, ready for human review"
            }
            _ => "Review output: the proposed next action guides the next step",
        };

        Ok(format!(
            "[STATE] Domain: {}, Sensitivity: {}, Complexity: {:.2}\n\
             ﹂ Grounded assessment based on {} context items and {} assumptions.\n\
             \n\
             [KEY GAP / RISK] {}\n\
             ﹂ The working memory summary flags this as the primary constraint.\n\
             \n\
             [RECOMMENDED NEXT STEP] {}\n\
             ﹂ {}",
            domain,
            sensitivity,
            complexity,
            "?",
            "?",
            gap_line,
            next_step_line,
            "This synthesis is evidence-only and non-authorizing. Review before acting.",
        ))
    }
}

/// Parse working-memory summary fields from the user prompt prefix.
///
/// The user prompt is structured as:
/// ```text
/// Objective: <text>
///
/// Working Memory Summary:
/// Domain: <value>
/// Sensitivity: <value>
/// Complexity: <value>
/// Context items: <count>
/// Missing context: <count>
/// Assumptions: <count>
/// Proposed next action: <value>
/// ```
///
/// Returns (domain, sensitivity, complexity, missing_count, next_action_kind).
/// On parse failure, returns safe defaults.
fn parse_wm_summary_fields(prompt: &str) -> (String, String, f32, usize, String) {
    let mut domain = String::from("Unknown");
    let mut sensitivity = String::from("Public");
    let mut complexity: f32 = 0.0;
    let mut missing: usize = 0;
    let mut next_action = String::from("StopWithReport");

    for line in prompt.lines() {
        let line = line.trim();
        if let Some(val) = line.strip_prefix("Domain:") {
            domain = val.trim().to_owned();
        } else if let Some(val) = line.strip_prefix("Sensitivity:") {
            sensitivity = val.trim().to_owned();
        } else if let Some(val) = line.strip_prefix("Complexity:") {
            complexity = val.trim().parse::<f32>().unwrap_or(0.0);
        } else if let Some(val) = line.strip_prefix("Missing context:") {
            missing = val.trim().parse::<usize>().unwrap_or(0);
        } else if let Some(val) = line.strip_prefix("Proposed next action:") {
            next_action = val.trim().to_owned();
        }
    }

    (domain, sensitivity, complexity, missing, next_action)
}

#[cfg(test)]
mod mock_synthesis_tests {
    use super::*;

    #[test]
    fn parse_wm_summary_extracts_all_known_fields() {
        let prompt = "Objective: Test objective\n\nWorking Memory Summary:\nDomain: General\nSensitivity: Public\nComplexity: 0.4\nContext items: 2\nMissing context: 1\nAssumptions: 3\nProposed next action: RequestContext\n";
        let (domain, sensitivity, complexity, missing, next_action) =
            parse_wm_summary_fields(prompt);
        assert_eq!(domain, "General");
        assert_eq!(sensitivity, "Public");
        assert!((complexity - 0.4).abs() < 0.01);
        assert_eq!(missing, 1);
        assert_eq!(next_action, "RequestContext");
    }

    #[test]
    fn parse_wm_summary_returns_defaults_for_empty_prompt() {
        let (domain, sensitivity, complexity, missing, next_action) = parse_wm_summary_fields("");
        assert_eq!(domain, "Unknown");
        assert_eq!(sensitivity, "Public");
        assert!((complexity - 0.0).abs() < 0.01);
        assert_eq!(missing, 0);
        assert_eq!(next_action, "StopWithReport");
    }

    #[test]
    fn parse_wm_summary_handles_missing_lines() {
        let prompt =
            "Objective: Test\n\nWorking Memory Summary:\nDomain: Coding\nComplexity: 0.7\n";
        let (domain, sensitivity, complexity, missing, next_action) =
            parse_wm_summary_fields(prompt);
        assert_eq!(domain, "Coding");
        // Sensitivity defaults (not present in input)
        assert_eq!(sensitivity, "Public");
        // Complexity correctly parsed from input
        assert!((complexity - 0.7).abs() < 0.01);
        assert_eq!(missing, 0);
        // Next action defaults (not present in input)
        assert_eq!(next_action, "StopWithReport");
    }

    #[test]
    fn mock_synthesis_output_contains_structured_sections() {
        let provider = MockProvider::safe_default();
        let prompt = "Objective: Analyse financial trends\n\nWorking Memory Summary:\nDomain: Business\nSensitivity: Internal\nComplexity: 0.6\nContext items: 3\nMissing context: 2\nAssumptions: 2\nProposed next action: RequestContext\n";
        let result = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(provider.synthesize(COGNITIVE_SYNTHESIS_SYSTEM_PROMPT, prompt))
            .expect("mock synthesis should not fail");
        assert!(
            result.contains("[STATE]"),
            "output should contain STATE section"
        );
        assert!(result.contains("Business"), "output should contain domain");
        assert!(
            result.contains("Internal"),
            "output should contain sensitivity"
        );
        assert!(
            result.contains("[KEY GAP / RISK]"),
            "output should contain KEY GAP section"
        );
        assert!(
            result.contains("[RECOMMENDED NEXT STEP]"),
            "output should contain RECOMMENDED section"
        );
        assert!(
            result.contains("non-authorizing"),
            "output should contain safety warning"
        );
    }

    #[test]
    fn mock_synthesis_output_references_concrete_fields() {
        let provider = MockProvider::safe_default();
        let prompt = "Objective: Research quantum computing\n\nWorking Memory Summary:\nDomain: Research\nSensitivity: Public\nComplexity: 0.9\nContext items: 1\nMissing context: 0\nAssumptions: 1\nProposed next action: StopWithReport\n";
        let result = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(provider.synthesize(COGNITIVE_SYNTHESIS_SYSTEM_PROMPT, prompt))
            .expect("mock synthesis should not fail");
        assert!(
            result.contains("Research"),
            "should reference Research domain"
        );
        assert!(
            result.contains("Stop with report"),
            "should reference StopWithReport next step"
        );
    }

    #[test]
    fn synthetic_synthesis_uses_request_context_for_missing_observations() {
        let provider = MockProvider::safe_default();
        let prompt = "Objective: Plan team offsite\n\nWorking Memory Summary:\nDomain: Business\nSensitivity: Internal\nComplexity: 0.3\nContext items: 0\nMissing context: 3\nAssumptions: 1\nProposed next action: RequestContext\n";
        let result = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(provider.synthesize(COGNITIVE_SYNTHESIS_SYSTEM_PROMPT, prompt))
            .expect("mock synthesis should not fail");
        assert!(
            result.contains("Request context"),
            "should reference RequestContext next step for missing context"
        );
        assert!(result.contains("3"), "should reference missing count of 3");
    }
}

/// Run a cognitive synthesis with the specified provider.
///
/// # Parameters
/// * `objective` — the original objective text
/// * `wm_summary` — a human-readable summary of the working memory state
/// * `provider_name` — `"mock"`, `"openai"`, or `"ollama"`
///
/// # Returns
/// The synthesis text from the LLM.
///
/// # Safety
/// * No tool execution, no persistence, no authorization.
/// * The output is evidence-only.
pub async fn run_cognitive_synthesis(
    objective: &str,
    wm_summary: &str,
    provider_name: &str,
) -> Result<String, LlmError> {
    let user_prompt = format!(
        r#"Objective: {}

Working Memory Summary:
{}"#,
        objective, wm_summary
    );

    match provider_name {
        "mock" => {
            let provider = MockProvider::safe_default();
            provider
                .synthesize(COGNITIVE_SYNTHESIS_SYSTEM_PROMPT, &user_prompt)
                .await
        }
        "openai" => {
            let provider = OpenAiProvider::from_env()?;
            provider
                .synthesize(COGNITIVE_SYNTHESIS_SYSTEM_PROMPT, &user_prompt)
                .await
        }
        "ollama" => {
            let provider = OllamaProvider::from_env();
            provider
                .synthesize(COGNITIVE_SYNTHESIS_SYSTEM_PROMPT, &user_prompt)
                .await
        }
        other => Err(LlmError::Provider(format!("Unknown provider: {other}"))),
    }
}

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
    fn routes_read_only_runtime_requests_to_precise_action_types() {
        assert_read_only_route(
            "Liste les actions en attente sans les modifier.",
            ActionType::ReadPendingActions,
            Permission::ReadProposedActions,
            "read_pending_actions",
        );
        assert_read_only_route(
            "Liste les actions proposées.",
            ActionType::ReadProposedActions,
            Permission::ReadProposedActions,
            "read_proposed_actions",
        );
        assert_read_only_route(
            "Lis l'état des tâches sans rien modifier.",
            ActionType::ReadTasks,
            Permission::ReadTasks,
            "read_tasks",
        );
        assert_read_only_route(
            "Consulte l'audit récent sans rien modifier.",
            ActionType::ReadAudit,
            Permission::ReadAudit,
            "read_audit",
        );
        assert_read_only_route(
            "Affiche le statut global du runtime.",
            ActionType::ReadStatus,
            Permission::ReadStatus,
            "read_status",
        );
        assert_read_only_route(
            "Vérifie l'état général du système sans rien exécuter.",
            ActionType::SystemCheck,
            Permission::ProposeToolUse,
            "system_check",
        );
    }

    #[test]
    fn routes_email_requests_to_proposed_actions() {
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
    fn keeps_advice_and_risk_questions_as_direct_replies() {
        for prompt in [
            "Que devrais-je vérifier avant d'ajouter une capacité d'exécution réelle ?",
            "Quels sont les risques avant d'ajouter une capacité d'exécution réelle ?",
        ] {
            assert_eq!(classify_prompt_intent(prompt), IntentClass::DirectReply);
            assert!(matches!(
                deterministic_turn_for_prompt(prompt),
                Some(AgentTurnDraft::DirectReply { .. })
            ));
        }
    }

    fn assert_read_only_route(
        prompt: &str,
        expected_action_type: ActionType,
        expected_permission: Permission,
        expected_operation: &str,
    ) {
        let turn = deterministic_turn_for_prompt(prompt)
            .unwrap_or_else(|| panic!("{prompt} should route deterministically"));
        match turn {
            AgentTurnDraft::ProposedAction { action } => {
                assert_eq!(action.action_type, expected_action_type);
                assert_eq!(action.risk_level, RiskLevel::Informational);
                assert_eq!(action.required_permissions, vec![expected_permission]);
                assert_eq!(action.payload["operation"], expected_operation);
                assert_eq!(action.payload["read_only"], true);
            }
            other => panic!("expected proposed action for {prompt}, got {other:?}"),
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
        let turn = deterministic_turn_for_prompt("vérifie l'état du système")
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

    #[test]
    fn cognitive_synthesis_prompt_contains_structured_sections() {
        let prompt = COGNITIVE_SYNTHESIS_SYSTEM_PROMPT;
        // Verify the updated prompt asks for grounded bullets referencing concrete fields
        assert!(
            prompt.contains("[STATE]"),
            "prompt should request STATE section"
        );
        assert!(
            prompt.contains("[KEY GAP / RISK]"),
            "prompt should request KEY GAP section"
        );
        assert!(
            prompt.contains("[RECOMMENDED NEXT STEP]"),
            "prompt should request RECOMMENDED section"
        );
        assert!(
            prompt.contains("Reference concrete field values"),
            "prompt should ask to reference concrete field values"
        );
    }

    #[test]
    fn cognitive_synthesis_prompt_retains_safety_warnings() {
        let prompt = COGNITIVE_SYNTHESIS_SYSTEM_PROMPT;
        assert!(
            prompt.contains("Do NOT propose tool calls"),
            "prompt must retain the no-tool-calls warning"
        );
        assert!(
            prompt.contains("Do NOT claim any action has been executed"),
            "prompt must retain the no-authorization warning"
        );
    }

    #[test]
    fn cognitive_synthesis_user_prompt_contains_objective_and_wm_summary() {
        let objective = "Analyse les tendances du marché français";
        let wm_summary = "Domain: Business\nSensitivity: Internal\nComplexity: 0.5\nContext items: 3\nMissing context: 1\nAssumptions: 2\nProposed next action: RequestContext";
        let user_prompt = format!(
            "Objective: {}\n\nWorking Memory Summary:\n{}",
            objective, wm_summary
        );
        assert!(user_prompt.contains(objective));
        assert!(user_prompt.contains("Domain: Business"));
        assert!(user_prompt.contains("Missing context: 1"));
        assert!(user_prompt.contains("Working Memory Summary:"));
    }
}
