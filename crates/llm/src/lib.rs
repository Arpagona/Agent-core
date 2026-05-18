//! Experimental LLM providers for ARPAGONA Agent Core.
//!
//! This crate is intentionally proposal-only: providers transform user prompts into
//! [`ProposedActionDraft`] values. They never execute tools, never call the
//! Decision Gate, and every materialized [`ProposedAction`] stays
//! `PendingDecision` until another layer explicitly evaluates it.

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

impl LlmProvider for OpenAiProvider {
    fn propose_action(
        &self,
        request: LlmActionRequest,
    ) -> impl Future<Output = Result<ProposedActionDraft, LlmError>> + Send {
        let client = self.client.clone();
        let endpoint = self.endpoint.clone();
        let model = self.model.clone();
        let api_key = self.api_key.clone();

        async move {
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

            serde_json::from_str::<ProposedActionDraft>(&text).map_err(|err| {
                LlmError::InvalidResponse(format!(
                    "model did not return a valid ProposedActionDraft JSON: {err}"
                ))
            })
        }
    }
}

pub fn provider_system_prompt() -> &'static str {
    r#"You are ARPAGONA Agent Proposer V0.
Return only one JSON object matching this schema:
{
  "action_type": "simulate_email" | "read_document" | "read_memory" | "manage_task" | {"custom":"..."},
  "target": string | null,
  "risk_level": "informational" | "low" | "medium" | "high" | "critical",
  "required_permissions": ["simulate_email" | "read_document" | "read_memory" | "manage_task"],
  "rationale": string,
  "payload": object
}
Never execute anything. Never claim that an email, document write, tool call, web search, or memory write has been executed.
Do not use OpenAI tools, web search, function calling, or external side effects.
Only propose a pending action for the Decision Gate.
Prefer action_type simulate_email, read_document, read_memory, manage_task, or custom when no safe predefined type applies.
Always include a rationale and choose a prudent risk_level.
If the user asks for direct execution, propose only a safe draft action that remains subject to human/Decision Gate approval."#
}

fn extract_output_text(value: &Value) -> Option<String> {
    if let Some(text) = value.get("output_text").and_then(Value::as_str) {
        return Some(text.to_owned());
    }

    value
        .get("output")?
        .as_array()?
        .iter()
        .flat_map(|item| item.get("content").and_then(Value::as_array).into_iter().flatten())
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
    fn extracts_response_output_text() {
        let value = json!({"output_text": "{\"rationale\":\"ok\"}"});
        assert_eq!(extract_output_text(&value).as_deref(), Some("{\"rationale\":\"ok\"}"));
    }
}
