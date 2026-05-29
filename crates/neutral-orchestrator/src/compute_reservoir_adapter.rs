//! ComputeReservoirAdapter — bridges the real `arpagona-compute-reservoir`
//! crate into the Neutral Orchestrator's compute route resolution step.
//!
//! The orchestrator previously used a hard-coded deterministic
//! `ComputeRouteResult` with a static label. This adapter replaces that with
//! a real call to `arpagona_compute_reservoir::allocate_compute()` using a
//! default inventory of compute nodes and a default `ComputePolicy`.
//!
//! # How it works
//!
//! 1. Builds a `ComputeRequest` from `ObjectiveInput` properties (text length,
//!    domain hint, workspace)
//! 2. Calls `allocate_compute()` with default compute nodes and policy
//! 3. Maps the `ComputeAllocation` → `ComputeRouteResult` with enriched
//!    justification including cost, latency, sensitivity, and capability details
//!
//! # Safety invariants
//!
//! - The resulting `ComputeRouteResult` is advisory and non-authorizing
//! - It does not approve, execute, or authorize any action
//! - The advisory_warning is always set
//! - No LLM call, I/O, persistence, or external effects

use arpagona_agent_core::cognitive_work::Objective;
use arpagona_agent_core::ids::{ComputeRouteId, ContextBundleId};
#[cfg(test)]
use arpagona_agent_core::orchestrator::ContextBundle;
use arpagona_agent_core::orchestrator::{ComputeRouteResult, ObjectiveInput};
use arpagona_compute_reservoir::{
    allocate_compute, ComputeAllocation, ComputeAllocationStatus, ComputeBudget, ComputeCapability,
    ComputeNode, ComputeNodeId, ComputeNodeStatus, ComputePolicy, ComputeRequest,
    ComputeResourceKind, DataSensitivity,
};
use chrono::{DateTime, Utc};

// ─── Default compute nodes ─────────────────────────────────────────────────

/// Create a default inventory of compute nodes for V0.
///
/// This provides a reasonable set of local and cloud resources for the
/// orchestrator to route between. The inventory is declarative and does not
/// require any external service availability checks.
fn default_compute_nodes() -> Vec<ComputeNode> {
    vec![
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
        },
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
        },
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
        },
    ]
}

// ─── ComputeReservoirResolver ───────────────────────────────────────────────

/// Bridges the `arpagona-compute-reservoir` crate into orchestrator compute
/// route resolution.
///
/// This resolver evaluates the objective input against a declared inventory of
/// compute resources and returns an advisory `ComputeRouteResult` with
/// explainable cost, latency, sensitivity, and capability trade-offs.
///
/// # Usage
///
/// ```ignore
/// use arpagona_neutral_orchestrator::ComputeReservoirResolver;
///
/// let resolver = ComputeReservoirResolver::new();
/// let route = resolver.resolve(&input, &objective, &bundle, now);
/// ```
///
/// Custom nodes or policy:
///
/// ```ignore
/// use arpagona_compute_reservoir::ComputePolicy;
///
/// let resolver = ComputeReservoirResolver::new()
///     .with_policy(ComputePolicy { allow_cloud_for_complex_reasoning: true, ..Default::default() });
/// ```
#[derive(Clone, Debug)]
pub struct ComputeReservoirResolver {
    nodes: Vec<ComputeNode>,
    policy: ComputePolicy,
}

impl ComputeReservoirResolver {
    /// Create a new resolver with default compute nodes and the default policy.
    pub fn new() -> Self {
        Self {
            nodes: default_compute_nodes(),
            policy: ComputePolicy::default(),
        }
    }

    /// Replace the compute node inventory.
    pub fn with_nodes(mut self, nodes: Vec<ComputeNode>) -> Self {
        self.nodes = nodes;
        self
    }

    /// Replace the compute policy.
    pub fn with_policy(mut self, policy: ComputePolicy) -> Self {
        self.policy = policy;
        self
    }

    /// Resolve a compute route for the given objective and context.
    ///
    /// This method:
    /// 1. Builds a `ComputeRequest` from the objective text, domain hint, and
    ///    workspace
    /// 2. Calls `allocate_compute()` with the configured nodes and policy
    /// 3. Maps the `ComputeAllocation` to an advisory `ComputeRouteResult`
    ///    with enriched justification
    ///
    /// # Safety
    ///
    /// - The returned `ComputeRouteResult` is advisory and non-authorizing
    /// - It does not approve, execute, or authorize any action
    pub fn resolve(
        &self,
        input: &ObjectiveInput,
        objective: &Objective,
        bundle_id: &ContextBundleId,
        now: DateTime<Utc>,
    ) -> ComputeRouteResult {
        let request = self.build_compute_request(input, objective);
        let allocation = allocate_compute(&request, &self.nodes, &self.policy);
        self.allocation_to_route(request, allocation, input, objective, bundle_id, now)
    }

    /// Build a `ComputeRequest` from the objective input properties.
    fn build_compute_request(
        &self,
        input: &ObjectiveInput,
        objective: &Objective,
    ) -> ComputeRequest {
        // Determine required capabilities from domain and text length
        let mut required_capabilities = vec![ComputeCapability::SimpleReasoning];

        let is_complex = objective.title.len() > 100
            || objective.title.to_lowercase().contains("analyze")
            || objective.title.to_lowercase().contains("complex");
        if is_complex {
            required_capabilities.push(ComputeCapability::ComplexReasoning);
        }

        // For debug/technical objectives, add CodeAnalysis
        if objective.title.to_lowercase().contains("code")
            || objective.title.to_lowercase().contains("debug")
        {
            required_capabilities.push(ComputeCapability::CodeAnalysis);
        }

        // Sensitivity defaults to Public — no data sensitivity info available
        // from the ObjectiveInput. Future work may pass sensitivity hints.
        let data_sensitivity = DataSensitivity::Public;

        // Budget: local-first by default (matching the orchestrator's default
        // local_preferred=true). Cloud only if explicitly needed.
        let budget = ComputeBudget::local_first();

        ComputeRequest {
            workspace_id: input.workspace_id.clone(),
            task_id: None,
            purpose: format!(
                "Orchestrator compute route: domain={:?}, text_len={}, complex={}",
                objective.domain,
                objective.title.len(),
                is_complex,
            ),
            required_capabilities,
            data_sensitivity,
            budget,
            requires_complex_reasoning: is_complex,
        }
    }

    /// Map a `ComputeAllocation` to an advisory `ComputeRouteResult`.
    fn allocation_to_route(
        &self,
        request: ComputeRequest,
        allocation: ComputeAllocation,
        input: &ObjectiveInput,
        objective: &Objective,
        bundle_id: &ContextBundleId,
        now: DateTime<Utc>,
    ) -> ComputeRouteResult {
        let route_id =
            ComputeRouteId::new(format!("cr-{}", now.timestamp_nanos_opt().unwrap_or(0)));

        // Build a rich label from allocation details
        let label = match (&allocation.resource_kind, &allocation.selected_node_id) {
            (Some(kind), Some(node_id)) => format!(
                "{} ({}, ${}, {}ms)",
                node_id.as_str(),
                format!("{:?}", kind).to_lowercase(),
                allocation.expected_cost_cents.unwrap_or(0),
                allocation.expected_latency_ms.unwrap_or(0),
            ),
            _ => "no_suitable_resource".to_owned(),
        };

        // Determine local preference from the selected node
        let local_preferred = self
            .nodes
            .iter()
            .any(|n| Some(&n.id) == allocation.selected_node_id.as_ref() && n.is_local);

        // Build a rich justification from allocation details
        let mut justification_parts: Vec<String> = Vec::new();

        match allocation.status {
            ComputeAllocationStatus::Selected => {
                justification_parts.push(
                    "Compute Reservoir allocation: resource selected for processing only (non-authorizing).".to_owned(),
                );
            }
            ComputeAllocationStatus::FallbackSelected => {
                justification_parts.push(
                    "Compute Reservoir allocation: ideal resource unavailable; fallback selected for processing only (non-authorizing).".to_owned(),
                );
            }
            ComputeAllocationStatus::NoSuitableResource => {
                justification_parts.push(
                    "Compute Reservoir: no suitable resource found. No processing is approved or executed.".to_owned(),
                );
            }
        }

        if let Some(node_id) = &allocation.selected_node_id {
            justification_parts.push(format!("Selected node: {}", node_id.as_str()));
        }
        if let Some(kind) = &allocation.resource_kind {
            justification_parts.push(format!("Resource kind: {:?}", kind));
        }
        if let Some(cost) = allocation.expected_cost_cents {
            justification_parts.push(format!(
                "Expected cost: ${} (cents: {})",
                cost as f64 / 100.0,
                cost
            ));
        }
        if let Some(latency) = allocation.expected_latency_ms {
            justification_parts.push(format!("Expected latency: {}ms", latency));
        }
        justification_parts.push(format!(
            "Data sensitivity: {:?}",
            allocation.data_sensitivity
        ));
        justification_parts.push(format!(
            "Required capabilities: {:?}",
            request.required_capabilities
        ));

        if let Some(fallback) = &allocation.fallback {
            justification_parts.push(format!("Fallback reason: {}", fallback.reason));
        }

        // Add the base allocation justification
        justification_parts.push(allocation.justification);

        let justification = justification_parts.join("\n");

        // Determine resource kind string (lowercase debug repr of ComputeResourceKind)
        let resource_kind_str = allocation
            .resource_kind
            .as_ref()
            .map(|k| format!("{:?}", k).to_lowercase());

        let mut route = ComputeRouteResult::new(
            route_id,
            input.cycle_id.clone(),
            objective.id.clone(),
            bundle_id.clone(),
            label,
            local_preferred,
            justification,
        );

        // Attach structured cost/quality metadata from the Compute Reservoir allocation
        if let Some(cost) = allocation.expected_cost_cents {
            route = route.with_expected_cost_cents(cost);
        }
        if let Some(latency) = allocation.expected_latency_ms {
            route = route.with_expected_latency_ms(latency);
        }
        if let Some(kind_str) = resource_kind_str {
            route = route.with_resource_kind(kind_str);
        }

        route
    }
}

impl Default for ComputeReservoirResolver {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use arpagona_agent_core::cognitive_work::ObjectiveDomain;
    use arpagona_agent_core::ids::{AgentId, WorkspaceId};

    fn make_input(text: &str) -> ObjectiveInput {
        ObjectiveInput::new(
            text.to_owned(),
            WorkspaceId::new("ws-test"),
            AgentId::new("agent-test"),
            Utc::now(),
        )
    }

    fn make_objective(text: &str) -> Objective {
        Objective {
            id: arpagona_agent_core::ids::ObjectiveId::new("obj-test"),
            title: text.to_owned(),
            description: text.to_owned(),
            domain: ObjectiveDomain::General,
            status: arpagona_agent_core::cognitive_work::ObjectiveStatus::Proposed,
            success_criteria: vec![],
            created_at: Utc::now(),
        }
    }

    fn make_bundle(obj: &Objective) -> ContextBundle {
        ContextBundle::new(
            arpagona_agent_core::ids::ContextBundleId::new("cb-test"),
            arpagona_agent_core::ids::OrchestratorCycleId::new("oc-test"),
            obj.id.clone(),
        )
    }

    // ─── Default resolution ─────────────────────────────────────────────

    #[test]
    fn test_resolver_creates_valid_compute_route() {
        let resolver = ComputeReservoirResolver::new();
        let input = make_input("Summarize the project status");
        let objective = make_objective("Summarize the project status");
        let bundle = make_bundle(&objective);
        let now = Utc::now();

        let route = resolver.resolve(&input, &objective, &bundle.id, now);
        assert!(route.id.as_str().starts_with("cr-"));
        assert_eq!(route.cycle_id, input.cycle_id);
        assert_eq!(route.objective_id.as_str(), objective.id.as_str());
        assert_eq!(route.context_bundle_id.as_str(), "cb-test");

        // Should have selected a local resource (budget is local-first)
        assert!(route.local_preferred);
        assert!(route.selected_route_label.contains("local"));

        // Justification should contain allocation details
        assert!(route.justification.contains("Compute Reservoir"));
        assert!(
            route.justification.contains("cost")
                || route.justification.contains("latency")
                || route.justification.contains("capability")
        );

        // Advisory warning is always present
        assert!(route.advisory_warning.contains("non-authorizing"));
    }

    #[test]
    fn test_resolver_handles_complex_objective() {
        let resolver = ComputeReservoirResolver::new();
        let input = make_input("Analyze the complex financial data and produce recommendations");
        let objective = Objective {
            id: arpagona_agent_core::ids::ObjectiveId::new("obj-complex"),
            title: "Analyze the complex financial data and produce recommendations".to_owned(),
            description: "Analyze the complex financial data and produce recommendations"
                .to_owned(),
            domain: ObjectiveDomain::Business,
            status: arpagona_agent_core::cognitive_work::ObjectiveStatus::Proposed,
            success_criteria: vec![],
            created_at: Utc::now(),
        };
        let bundle = make_bundle(&objective);
        let now = Utc::now();

        let route = resolver.resolve(&input, &objective, &bundle.id, now);

        // Complex objectives should still route locally (local-first budget)
        assert!(route.local_preferred);
        assert!(route.selected_route_label.contains("local"));

        // Justification should reflect the complex reasoning requirement
        assert!(
            route.justification.contains("ComplexReasoning")
                || route.justification.contains("capabilities")
        );
    }

    #[test]
    fn test_resolver_never_approves_actions() {
        let resolver = ComputeReservoirResolver::new();
        let input = make_input("Read only test");
        let objective = make_objective("Read only test");
        let bundle = make_bundle(&objective);
        let now = Utc::now();

        let route = resolver.resolve(&input, &objective, &bundle.id, now);

        let json = serde_json::to_value(&route).expect("should serialize");
        assert!(json.get("approved").is_none());
        assert!(json.get("authorized").is_none());
        assert!(json.get("execution_token").is_none());
    }

    // ─── Custom nodes ───────────────────────────────────────────────────

    #[test]
    fn test_resolver_with_custom_nodes_overrides_default() {
        let custom_node = ComputeNode {
            id: ComputeNodeId::new("custom-worker"),
            label: "Custom worker".to_owned(),
            kind: ComputeResourceKind::RemoteWorker,
            status: ComputeNodeStatus::Available,
            capabilities: vec![ComputeCapability::SimpleReasoning],
            max_data_sensitivity: DataSensitivity::Public,
            expected_cost_cents: 10,
            expected_latency_ms: 300,
            is_local: false,
            strength: 5,
        };

        // With local_first budget, cloud nodes are filtered out.
        // Only custom-worker should be filtered by cloud_is_allowed
        // since it's not local and budget is local_first.
        // Let's add a local node too so we get a selection.
        let local_node = ComputeNode {
            id: ComputeNodeId::new("local-custom"),
            label: "Local custom".to_owned(),
            kind: ComputeResourceKind::LocalLlm,
            status: ComputeNodeStatus::Available,
            capabilities: vec![ComputeCapability::SimpleReasoning],
            max_data_sensitivity: DataSensitivity::Public,
            expected_cost_cents: 0,
            expected_latency_ms: 100,
            is_local: true,
            strength: 4,
        };

        let resolver = ComputeReservoirResolver::new().with_nodes(vec![custom_node, local_node]);
        let input = make_input("Simple task");
        let objective = make_objective("Simple task");
        let bundle = make_bundle(&objective);
        let now = Utc::now();

        let route = resolver.resolve(&input, &objective, &bundle.id, now);
        assert!(route.local_preferred);
        assert!(route.selected_route_label.contains("local-custom"));
    }

    // ─── Build compute request tests ────────────────────────────────────

    #[test]
    fn test_build_compute_request_default() {
        let resolver = ComputeReservoirResolver::new();
        let input = make_input("Simple task");
        let objective = make_objective("Simple task");

        let request = resolver.build_compute_request(&input, &objective);

        assert!(request
            .required_capabilities
            .contains(&ComputeCapability::SimpleReasoning));
        assert!(!request
            .required_capabilities
            .contains(&ComputeCapability::ComplexReasoning));
        assert_eq!(request.data_sensitivity, DataSensitivity::Public);
        assert_eq!(request.budget.max_cloud_cost_cents, 0);
        assert!(!request.requires_complex_reasoning);
    }

    #[test]
    fn test_build_compute_request_complex_keyword() {
        let resolver = ComputeReservoirResolver::new();
        let input = make_input("Analyze the system's behavior");
        let objective = make_objective("Analyze the system's behavior");

        let request = resolver.build_compute_request(&input, &objective);

        assert!(request.requires_complex_reasoning);
        assert!(request
            .required_capabilities
            .contains(&ComputeCapability::ComplexReasoning));
    }

    #[test]
    fn test_build_compute_request_code_analysis() {
        let resolver = ComputeReservoirResolver::new();
        let input = make_input("Review code for security issues");
        let objective = make_objective("Review code for security issues");

        let request = resolver.build_compute_request(&input, &objective);

        assert!(request
            .required_capabilities
            .contains(&ComputeCapability::CodeAnalysis));
    }
}
