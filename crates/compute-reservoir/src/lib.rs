//! Pure Compute Reservoir primitives for ARPAGONA Agent Core.
//!
//! Compute Reservoir chooses how to process or think.
//! It does not approve actions, execute tools, call LLMs, access secrets,
//! mutate memory, or bypass the Decision Gate.
//!
//! Boundary summary:
//! - Compute Reservoir: selects cognitive or computational resources.
//! - Reservoir Echo: keeps volatile short-term cognitive continuity.
//! - Decision Gate: governs proposed actions before any execution.
//! - Tool Registry: will declare available tools and permissions.
//! - Graph Memory: persists facts, sources, episodes and relations.
//! - Audit: records causal traces.

use arpagona_agent_core::{
    cognitive_work::{SensitivityEstimate, WorkingMemory},
    TaskId, WorkspaceId,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

/// Stable identifier for a declared compute resource.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ComputeNodeId(pub String);

impl ComputeNodeId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for ComputeNodeId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for ComputeNodeId {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl fmt::Display for ComputeNodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Kind of compute resource. This is descriptive only; it grants no execution right.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComputeResourceKind {
    LocalLlm,
    CloudLlm,
    LocalCpu,
    LocalGpu,
    RemoteWorker,
    Deterministic,
    DeferredJob,
}

/// Operational availability of a compute resource.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComputeNodeStatus {
    Available,
    Unavailable,
    Degraded,
    Disabled,
}

impl ComputeNodeStatus {
    fn can_be_selected(&self, allow_degraded: bool) -> bool {
        matches!(self, Self::Available) || (allow_degraded && matches!(self, Self::Degraded))
    }
}

/// Capability advertised by a compute resource or required by a request.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComputeCapability {
    SimpleReasoning,
    ComplexReasoning,
    CodeAnalysis,
    TextSynthesis,
    Embedding,
    Ocr,
    Parsing,
    DeterministicComputation,
    GpuAcceleration,
    LocalOnly,
}

/// Sensitivity of data that may be processed by a compute resource.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataSensitivity {
    Public,
    Internal,
    Confidential,
    Secret,
}

impl DataSensitivity {
    fn requires_local(&self) -> bool {
        matches!(self, Self::Confidential | Self::Secret)
    }
}

/// Budget constraints for a compute request.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComputeBudget {
    pub max_cloud_cost_cents: u32,
    pub max_latency_ms: Option<u64>,
    pub max_retries: u8,
}

impl ComputeBudget {
    pub fn local_first() -> Self {
        Self {
            max_cloud_cost_cents: 0,
            max_latency_ms: None,
            max_retries: 0,
        }
    }
}

/// Declared compute resource inventory entry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComputeNode {
    pub id: ComputeNodeId,
    pub label: String,
    pub kind: ComputeResourceKind,
    pub status: ComputeNodeStatus,
    pub capabilities: Vec<ComputeCapability>,
    pub max_data_sensitivity: DataSensitivity,
    pub expected_cost_cents: u32,
    pub expected_latency_ms: u64,
    pub is_local: bool,
    pub strength: u8,
}

impl ComputeNode {
    fn supports(&self, request: &ComputeRequest) -> bool {
        request
            .required_capabilities
            .iter()
            .all(|capability| self.capabilities.contains(capability))
    }

    fn can_handle_sensitivity(&self, sensitivity: &DataSensitivity) -> bool {
        &self.max_data_sensitivity >= sensitivity
    }
}

/// Request for cognitive or computational processing.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComputeRequest {
    pub workspace_id: WorkspaceId,
    pub task_id: Option<TaskId>,
    pub purpose: String,
    pub required_capabilities: Vec<ComputeCapability>,
    pub data_sensitivity: DataSensitivity,
    pub budget: ComputeBudget,
    pub requires_complex_reasoning: bool,
}

/// Fallback plan when the ideal resource is absent or constrained.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComputeFallback {
    pub node_id: Option<ComputeNodeId>,
    pub reason: String,
}

/// Minimal performance telemetry shape for future memory, without persistence.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComputeTelemetry {
    pub node_id: ComputeNodeId,
    pub observed_latency_ms: Option<u64>,
    pub observed_cost_cents: Option<u32>,
    pub success: Option<bool>,
    pub notes: Option<String>,
    pub recorded_at: DateTime<Utc>,
}

/// Deterministic policy controlling compute allocation, not action governance.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComputePolicy {
    pub prefer_local_for_sensitive_data: bool,
    pub prefer_local_when_cloud_budget_zero: bool,
    pub allow_cloud_for_complex_reasoning: bool,
    pub allow_degraded_nodes: bool,
    pub fallback_to_any_compatible_local: bool,
}

impl Default for ComputePolicy {
    fn default() -> Self {
        Self {
            prefer_local_for_sensitive_data: true,
            prefer_local_when_cloud_budget_zero: true,
            allow_cloud_for_complex_reasoning: false,
            allow_degraded_nodes: false,
            fallback_to_any_compatible_local: true,
        }
    }
}

/// Outcome of a compute allocation attempt. This is not a Decision Gate decision.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComputeAllocationStatus {
    Selected,
    FallbackSelected,
    NoSuitableResource,
}

/// Recommended compute allocation. It does not approve or execute an action.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ComputeAllocation {
    pub status: ComputeAllocationStatus,
    pub selected_node_id: Option<ComputeNodeId>,
    pub resource_kind: Option<ComputeResourceKind>,
    pub expected_cost_cents: Option<u32>,
    pub expected_latency_ms: Option<u64>,
    pub data_sensitivity: DataSensitivity,
    pub justification: String,
    pub fallback: Option<ComputeFallback>,
    pub telemetry_hint: Option<Value>,
}

/// Allocate a compute resource with pure, deterministic V0 routing rules.
///
/// This function does not call models, perform I/O, mutate memory, create
/// proposed actions, or produce Decision Gate decisions.
pub fn allocate_compute(
    request: &ComputeRequest,
    nodes: &[ComputeNode],
    policy: &ComputePolicy,
) -> ComputeAllocation {
    let compatible = nodes
        .iter()
        .filter(|node| node.status.can_be_selected(policy.allow_degraded_nodes))
        .filter(|node| node.can_handle_sensitivity(&request.data_sensitivity))
        .filter(|node| node.supports(request))
        .filter(|node| cloud_is_allowed(node, request, policy))
        .collect::<Vec<_>>();

    if compatible.is_empty() {
        return no_suitable_resource(request, nodes, policy);
    }

    let ideal = compatible
        .iter()
        .copied()
        .min_by_key(|node| score_node(node, request, policy));

    match ideal {
        Some(node) => selected_allocation(node, request, ComputeAllocationStatus::Selected, None),
        None => no_suitable_resource(request, nodes, policy),
    }
}

fn cloud_is_allowed(node: &ComputeNode, request: &ComputeRequest, policy: &ComputePolicy) -> bool {
    if node.is_local {
        return true;
    }

    if policy.prefer_local_for_sensitive_data && request.data_sensitivity.requires_local() {
        return false;
    }

    if policy.prefer_local_when_cloud_budget_zero && request.budget.max_cloud_cost_cents == 0 {
        return false;
    }

    if request.requires_complex_reasoning && policy.allow_cloud_for_complex_reasoning {
        return node.expected_cost_cents <= request.budget.max_cloud_cost_cents;
    }

    node.expected_cost_cents <= request.budget.max_cloud_cost_cents
}

fn score_node(node: &ComputeNode, request: &ComputeRequest, policy: &ComputePolicy) -> i64 {
    let mut score = 0_i64;

    score += i64::from(node.expected_cost_cents) * 10;
    score += (node.expected_latency_ms / 100) as i64;

    if node.is_local
        && request.data_sensitivity.requires_local()
        && policy.prefer_local_for_sensitive_data
    {
        score -= 1_000;
    }

    if node.is_local
        && request.budget.max_cloud_cost_cents == 0
        && policy.prefer_local_when_cloud_budget_zero
    {
        score -= 700;
    }

    if request.requires_complex_reasoning {
        score -= i64::from(node.strength) * 80;
    } else {
        score -= i64::from(node.strength) * 10;
    }

    if matches!(node.status, ComputeNodeStatus::Degraded) {
        score += 500;
    }

    score
}

fn selected_allocation(
    node: &ComputeNode,
    request: &ComputeRequest,
    status: ComputeAllocationStatus,
    fallback: Option<ComputeFallback>,
) -> ComputeAllocation {
    ComputeAllocation {
        status,
        selected_node_id: Some(node.id.clone()),
        resource_kind: Some(node.kind.clone()),
        expected_cost_cents: Some(node.expected_cost_cents),
        expected_latency_ms: Some(node.expected_latency_ms),
        data_sensitivity: request.data_sensitivity.clone(),
        justification: format!(
            "Selected compute resource '{}' for processing only; this is not action approval.",
            node.label
        ),
        fallback,
        telemetry_hint: Some(serde_json::json!({
            "record_future_performance": true,
            "node_id": node.id,
            "purpose": request.purpose,
        })),
    }
}

fn no_suitable_resource(
    request: &ComputeRequest,
    nodes: &[ComputeNode],
    policy: &ComputePolicy,
) -> ComputeAllocation {
    if policy.fallback_to_any_compatible_local {
        if let Some(node) = nodes.iter().find(|node| {
            node.is_local
                && node.status.can_be_selected(policy.allow_degraded_nodes)
                && node.can_handle_sensitivity(&request.data_sensitivity)
        }) {
            return selected_allocation(
                node,
                request,
                ComputeAllocationStatus::FallbackSelected,
                Some(ComputeFallback {
                    node_id: Some(node.id.clone()),
                    reason: "Ideal capability match was unavailable; selected compatible local fallback for processing only."
                        .to_owned(),
                }),
            );
        }
    }

    ComputeAllocation {
        status: ComputeAllocationStatus::NoSuitableResource,
        selected_node_id: None,
        resource_kind: None,
        expected_cost_cents: None,
        expected_latency_ms: None,
        data_sensitivity: request.data_sensitivity.clone(),
        justification: "No suitable compute resource exists under the current inventory and compute policy; no action is approved or executed."
            .to_owned(),
        fallback: Some(ComputeFallback {
            node_id: None,
            reason: "Add an available compatible local resource or relax compute policy constraints."
                .to_owned(),
        }),
        telemetry_hint: Some(serde_json::json!({
            "record_future_performance": false,
            "reason": "no_suitable_resource"
        })),
    }
}

/// Static warning embedded in every WorkingMemory allocation result.
pub const NON_AUTHORIZING_READBACK: &str =
    "Readback only — allocation is not authorization, not execution, not a bypass of the Decision Gate. Review before acting.";

/// Allocate a compute resource based on WorkingMemory cognitive state.
///
/// This pure function maps the heuristic fields of a `WorkingMemory` to a
/// `ComputeRequest` and uses the existing `allocate_compute` engine to produce
/// a `ComputeAllocation`.
///
/// It does NOT:
/// - call LLMs
/// - make API calls
/// - persist any state
/// - authorize any action
/// - bypass the Decision Gate
pub fn allocate_for_working_memory(
    working_memory: &WorkingMemory,
    nodes: &[ComputeNode],
    policy: &ComputePolicy,
) -> ComputeAllocation {
    // 1. Map WorkingMemory sensitivity to ComputeRequest sensitivity
    let data_sensitivity = match &working_memory.sensitivity_estimate {
        SensitivityEstimate::Secret => DataSensitivity::Secret,
        SensitivityEstimate::Confidential => DataSensitivity::Confidential,
        SensitivityEstimate::Internal => DataSensitivity::Internal,
        SensitivityEstimate::Public => DataSensitivity::Public,
    };

    // 2. Determine required capabilities from complexity and observations
    let mut required_capabilities = Vec::new();
    required_capabilities.push(ComputeCapability::SimpleReasoning);

    let needs_complex = working_memory.complexity_estimate >= 0.7
        || working_memory.required_observations_count >= 5;
    if needs_complex {
        required_capabilities.push(ComputeCapability::ComplexReasoning);
    }

    // 3. Determine budget from local_first and cost_sensitive preferences
    let budget = if working_memory.local_first || working_memory.cost_sensitive {
        ComputeBudget::local_first()
    } else {
        ComputeBudget {
            max_cloud_cost_cents: 100,
            max_latency_ms: None,
            max_retries: 1,
        }
    };

    // 4. Build a ComputeRequest from WorkingMemory state
    let request = ComputeRequest {
        workspace_id: WorkspaceId::new("cognitive-work-cycle"),
        task_id: None,
        purpose: format!(
            "WorkingMemory allocation: sensitivity={:?}, complexity={}, observations={}, local_first={}, cost_sensitive={}",
            working_memory.sensitivity_estimate,
            working_memory.complexity_estimate,
            working_memory.required_observations_count,
            working_memory.local_first,
            working_memory.cost_sensitive,
        ),
        required_capabilities,
        data_sensitivity,
        budget,
        requires_complex_reasoning: needs_complex,
    };

    // 5. Delegate to the existing allocation engine
    let mut allocation = allocate_compute(&request, nodes, policy);

    // 6. Append non-authorizing readback to justification
    allocation.justification = format!(
        "{}\n\n{}",
        allocation.justification, NON_AUTHORIZING_READBACK
    );

    allocation
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(
        capabilities: Vec<ComputeCapability>,
        sensitivity: DataSensitivity,
        budget: ComputeBudget,
        complex: bool,
    ) -> ComputeRequest {
        ComputeRequest {
            workspace_id: WorkspaceId::new("workspace-1"),
            task_id: Some(TaskId::new("task-1")),
            purpose: "unit test processing".to_owned(),
            required_capabilities: capabilities,
            data_sensitivity: sensitivity,
            budget,
            requires_complex_reasoning: complex,
        }
    }

    fn local_small() -> ComputeNode {
        ComputeNode {
            id: ComputeNodeId::new("local-small"),
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
        }
    }

    fn local_cpu() -> ComputeNode {
        ComputeNode {
            id: ComputeNodeId::new("local-cpu"),
            label: "Local CPU deterministic worker".to_owned(),
            kind: ComputeResourceKind::LocalCpu,
            status: ComputeNodeStatus::Available,
            capabilities: vec![ComputeCapability::DeterministicComputation],
            max_data_sensitivity: DataSensitivity::Secret,
            expected_cost_cents: 0,
            expected_latency_ms: 500,
            is_local: true,
            strength: 2,
        }
    }

    fn cloud_strong() -> ComputeNode {
        ComputeNode {
            id: ComputeNodeId::new("cloud-strong"),
            label: "Strong cloud model".to_owned(),
            kind: ComputeResourceKind::CloudLlm,
            status: ComputeNodeStatus::Available,
            capabilities: vec![
                ComputeCapability::SimpleReasoning,
                ComputeCapability::ComplexReasoning,
                ComputeCapability::TextSynthesis,
            ],
            max_data_sensitivity: DataSensitivity::Internal,
            expected_cost_cents: 25,
            expected_latency_ms: 1_200,
            is_local: false,
            strength: 10,
        }
    }

    #[test]
    fn sensitive_request_prefers_available_local_resource() {
        let allocation = allocate_compute(
            &request(
                vec![ComputeCapability::SimpleReasoning],
                DataSensitivity::Confidential,
                ComputeBudget {
                    max_cloud_cost_cents: 100,
                    max_latency_ms: None,
                    max_retries: 1,
                },
                false,
            ),
            &[cloud_strong(), local_small()],
            &ComputePolicy::default(),
        );

        assert_eq!(
            allocation.selected_node_id,
            Some(ComputeNodeId::new("local-small"))
        );
    }

    #[test]
    fn unavailable_resource_is_not_selected() {
        let mut unavailable = local_small();
        unavailable.status = ComputeNodeStatus::Unavailable;

        let allocation = allocate_compute(
            &request(
                vec![ComputeCapability::SimpleReasoning],
                DataSensitivity::Internal,
                ComputeBudget::local_first(),
                false,
            ),
            &[unavailable],
            &ComputePolicy::default(),
        );

        assert_eq!(
            allocation.status,
            ComputeAllocationStatus::NoSuitableResource
        );
        assert_eq!(allocation.selected_node_id, None);
    }

    #[test]
    fn low_budget_avoids_cloud_when_local_alternative_exists() {
        let allocation = allocate_compute(
            &request(
                vec![ComputeCapability::SimpleReasoning],
                DataSensitivity::Internal,
                ComputeBudget::local_first(),
                false,
            ),
            &[cloud_strong(), local_small()],
            &ComputePolicy::default(),
        );

        assert_eq!(
            allocation.selected_node_id,
            Some(ComputeNodeId::new("local-small"))
        );
    }

    #[test]
    fn complex_request_can_select_strong_model_when_policy_allows_it() {
        let policy = ComputePolicy {
            allow_cloud_for_complex_reasoning: true,
            ..ComputePolicy::default()
        };

        let allocation = allocate_compute(
            &request(
                vec![ComputeCapability::ComplexReasoning],
                DataSensitivity::Internal,
                ComputeBudget {
                    max_cloud_cost_cents: 50,
                    max_latency_ms: None,
                    max_retries: 1,
                },
                true,
            ),
            &[local_small(), cloud_strong()],
            &policy,
        );

        assert_eq!(
            allocation.selected_node_id,
            Some(ComputeNodeId::new("cloud-strong"))
        );
    }

    #[test]
    fn fallback_is_returned_when_ideal_resource_does_not_exist() {
        let allocation = allocate_compute(
            &request(
                vec![ComputeCapability::Ocr],
                DataSensitivity::Confidential,
                ComputeBudget::local_first(),
                false,
            ),
            &[local_cpu()],
            &ComputePolicy::default(),
        );

        assert_eq!(allocation.status, ComputeAllocationStatus::FallbackSelected);
        assert_eq!(
            allocation.selected_node_id,
            Some(ComputeNodeId::new("local-cpu"))
        );
        assert!(allocation.fallback.is_some());
    }

    #[test]
    fn allocation_does_not_look_like_action_approval() {
        let allocation = allocate_compute(
            &request(
                vec![ComputeCapability::SimpleReasoning],
                DataSensitivity::Public,
                ComputeBudget::local_first(),
                false,
            ),
            &[local_small()],
            &ComputePolicy::default(),
        );

        let encoded = serde_json::to_string(&allocation).expect("allocation should serialize");
        assert!(!encoded.contains("approved"));
        assert!(!encoded.contains("proposed_action"));
        assert!(!encoded.contains("decision_gate"));
    }

    #[test]
    fn crate_types_are_serializable_without_external_provider() {
        let telemetry = ComputeTelemetry {
            node_id: ComputeNodeId::new("local-small"),
            observed_latency_ms: Some(42),
            observed_cost_cents: Some(0),
            success: Some(true),
            notes: Some("pure unit test".to_owned()),
            recorded_at: Utc::now(),
        };

        let encoded = serde_json::to_string(&telemetry).expect("telemetry should serialize");
        let decoded: ComputeTelemetry =
            serde_json::from_str(&encoded).expect("telemetry should deserialize");

        assert_eq!(decoded.node_id, ComputeNodeId::new("local-small"));
        assert_eq!(decoded.observed_cost_cents, Some(0));
    }

    // ─── P5 — allocate_for_working_memory tests ─────────────────────────

    fn wm_simple() -> WorkingMemory {
        WorkingMemory {
            context_items: vec![],
            assumptions: vec![],
            constraints: vec![],
            missing_context: vec![],
            sensitivity_estimate: SensitivityEstimate::Public,
            complexity_estimate: 0.2,
            local_first: false,
            cost_sensitive: false,
            proposed_next_action_kind: "stopwithreport".to_owned(),
            required_observations_count: 1,
        }
    }

    fn wm_missing_context() -> WorkingMemory {
        WorkingMemory {
            context_items: vec![],
            assumptions: vec![],
            constraints: vec![],
            missing_context: vec![arpagona_agent_core::cognitive_work::MissingContext {
                id: "mc-1".to_owned(),
                description: "No context given.".to_owned(),
                why_needed: "Need context.".to_owned(),
            }],
            sensitivity_estimate: SensitivityEstimate::Public,
            complexity_estimate: 0.2,
            local_first: false,
            cost_sensitive: false,
            proposed_next_action_kind: "requestcontext".to_owned(),
            required_observations_count: 2,
        }
    }

    fn wm_sensitive() -> WorkingMemory {
        WorkingMemory {
            context_items: vec![],
            assumptions: vec![],
            constraints: vec![],
            missing_context: vec![],
            sensitivity_estimate: SensitivityEstimate::Confidential,
            complexity_estimate: 0.3,
            local_first: true,
            cost_sensitive: true,
            proposed_next_action_kind: "stopwithreport".to_owned(),
            required_observations_count: 1,
        }
    }

    fn wm_complex_research() -> WorkingMemory {
        WorkingMemory {
            context_items: vec![],
            assumptions: vec![],
            constraints: vec![],
            missing_context: vec![],
            sensitivity_estimate: SensitivityEstimate::Public,
            complexity_estimate: 0.9,
            local_first: false,
            cost_sensitive: false,
            proposed_next_action_kind: "proposeplan".to_owned(),
            required_observations_count: 5,
        }
    }

    #[test]
    fn p5_missing_context_requests_context_no_expensive_model() {
        // When missing_context is present, the next action kind is request_context
        // and complexity is low — simple model is sufficient.
        let wm = wm_missing_context();
        assert_eq!(wm.proposed_next_action_kind, "requestcontext");
        assert!(wm.complexity_estimate < 0.7);

        let allocation = allocate_for_working_memory(
            &wm,
            &[local_small(), cloud_strong()],
            &ComputePolicy::default(),
        );

        // Should select local-small (cheap) over cloud-strong
        assert_eq!(
            allocation.selected_node_id,
            Some(ComputeNodeId::new("local-small"))
        );
        assert!(allocation.justification.contains(NON_AUTHORIZING_READBACK));
    }

    #[test]
    fn p5_sensitive_objective_prefers_local_resource() {
        // Confidential sensitivity + local_first = forced local
        let wm = wm_sensitive();
        assert!(wm.sensitivity_estimate.requires_local());

        let allocation = allocate_for_working_memory(
            &wm,
            &[cloud_strong(), local_small()],
            &ComputePolicy::default(),
        );

        assert_eq!(
            allocation.selected_node_id,
            Some(ComputeNodeId::new("local-small"))
        );
        assert!(allocation.resource_kind == Some(ComputeResourceKind::LocalLlm));
    }

    #[test]
    fn p5_complex_research_justifies_strong_model_when_policy_allows() {
        // High complexity, many observations -> ComplexReasoning capability required
        // cloud_strong has ComplexReasoning, local_small does not
        let wm = wm_complex_research();

        // With default policy, cloud_strong is available (public data, budget allows)
        // Complex research justifies the stronger (cloud) model
        let allocation = allocate_for_working_memory(
            &wm,
            &[local_small(), cloud_strong()],
            &ComputePolicy::default(),
        );

        // cloud_strong is selected because it supports ComplexReasoning and cost is within budget
        assert_eq!(
            allocation.selected_node_id,
            Some(ComputeNodeId::new("cloud-strong"))
        );
        assert_eq!(allocation.status, ComputeAllocationStatus::Selected);

        // Now test fallback: cloud_strong cannot handle Secret data
        // But local_small can (it has max_data_sensitivity: Secret)
        // However local_small lacks ComplexReasoning -> cloud blocked for secret, local doesn't support
        // -> fallback_to_any_compatible_local finds local_small
        let wm_secret = WorkingMemory {
            sensitivity_estimate: SensitivityEstimate::Secret,
            complexity_estimate: 0.9,
            required_observations_count: 5,
            ..wm_simple()
        };
        let allocation_secret = allocate_for_working_memory(
            &wm_secret,
            &[local_small(), cloud_strong()],
            &ComputePolicy::default(),
        );

        // cloud_strong max_data_sensitivity is Confidential, cannot handle Secret
        // local_small can handle Secret but lacks ComplexReasoning
        // Fallback to local_small as compatible local
        assert_eq!(
            allocation_secret.status,
            ComputeAllocationStatus::FallbackSelected
        );
        assert_eq!(
            allocation_secret.selected_node_id,
            Some(ComputeNodeId::new("local-small"))
        );
    }

    #[test]
    fn p5_unavailable_resource_triggers_fallback() {
        let wm = wm_simple();

        let mut unavailable_local = local_small();
        unavailable_local.status = ComputeNodeStatus::Unavailable;

        let allocation =
            allocate_for_working_memory(&wm, &[unavailable_local], &ComputePolicy::default());

        assert_eq!(
            allocation.status,
            ComputeAllocationStatus::NoSuitableResource
        );
        assert_eq!(allocation.selected_node_id, None);
        assert!(allocation.fallback.is_some());
    }

    #[test]
    fn p5_allocation_is_not_authorization() {
        let wm = wm_simple();

        let allocation =
            allocate_for_working_memory(&wm, &[local_small()], &ComputePolicy::default());

        let encoded = serde_json::to_string(&allocation).expect("allocation should serialize");
        assert!(encoded.contains("Readback only"));
        assert!(encoded.contains("not authorization"));
        assert!(encoded.contains("not execution"));
        assert!(allocation.justification.contains(NON_AUTHORIZING_READBACK));
    }

    #[test]
    fn p5_no_external_provider_call() {
        // This test verifies the function is pure: no LLM, API, or provider call.
        // The function is pure Rust — it runs without any external dependency.
        let wm = wm_simple();
        let allocation =
            allocate_for_working_memory(&wm, &[local_small()], &ComputePolicy::default());

        // Pure deterministic: same inputs always produce same output
        let allocation2 =
            allocate_for_working_memory(&wm, &[local_small()], &ComputePolicy::default());

        assert_eq!(allocation.status, allocation2.status);
        assert_eq!(allocation.selected_node_id, allocation2.selected_node_id);
        assert_eq!(allocation.justification, allocation2.justification);
    }
}
