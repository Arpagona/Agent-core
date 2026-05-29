//! Trace-to-Insight — heuristic analysis of CycleTrace into FailureInsight candidates.
//!
//! This module connects orchestrated context assembly metadata to Failure-to-Insight
//! by inspecting a `CycleTrace` for patterns that indicate failures, gaps, or
//! improvement opportunities. Each pattern produces one `FailureInsight` candidate.
//!
//! # How it works
//!
//! 1. Receives a `&CycleTrace` (already produced by the orchestrator)
//! 2. Applies heuristic checks for observable failure patterns
//! 3. Returns `Vec<FailureInsight>` candidates — each non-authorizing, advisory only
//!
//! # Heuristic checks (current)
//!
//! | Heuristic | FailureClass | Trigger |
//! |---|---|---|
//! | Unavailable sources | MissingContext | `unavailable_sources` not empty |
//! | Blocked decision | PolicyGap | `decision_status` contains "Blocked"/"Denied"/"Rejected" |
//! | No context found | MissingContext | `total_context_items == 0` across all sources |
//! | No compute route | WrongComputeChoice | `compute_route_label` is None |
//! | Cycle status incomplete | InsufficientObservability | `cycle_status` is not "Completed" |
//!
//! # Safety invariants
//!
//! - Every returned `FailureInsight` has `status: InsightStatus::Proposed`
//! - Every returned `FailureInsight` is advisory and non-authorizing
//! - No I/O, no LLM calls, no persistence, no external effects
//! - Pure deterministic computation: same trace → same candidates
//! - No trace fields are modified

use arpagona_agent_core::failure_insight::{
    CorrectionTarget, DetectionSignal, DetectionSignalType, FailureClass, FailureInsight,
    InsightSeverity,
};
use arpagona_agent_core::ids::FailureInsightId;
use arpagona_agent_core::orchestrator::CycleTrace;
use chrono::Utc;

// ─── Public API ─────────────────────────────────────────────────────────────

/// Analyze a `CycleTrace` and produce `FailureInsight` candidates.
///
/// Each candidate is based on observable patterns in the trace metadata:
/// - Unavailable memory sources
/// - Blocked/rejected decisions
/// - Zero context items from any source
/// - Missing compute route
/// - Incomplete cycle status
///
/// # Returns
///
/// A `Vec<FailureInsight>`. May be empty if no heuristic pattern matched.
///
/// # Safety
///
/// - All returned insights have `status: InsightStatus::Proposed`
/// - All insights are advisory and non-authorizing
/// - No trace fields are modified
pub fn extract_candidates(trace: &CycleTrace) -> Vec<FailureInsight> {
    let mut candidates: Vec<FailureInsight> = Vec::new();

    // 1. Unavailable sources → MissingContext
    if !trace.unavailable_sources.is_empty() {
        candidates.push(unavailable_sources_insight(trace));
    }

    // 2. Blocked/Denied/Rejected decision → PolicyGap
    if let Some(ref status) = trace.decision_status {
        let lower = status.to_lowercase();
        if lower.contains("blocked")
            || lower.contains("denied")
            || lower.contains("rejected")
            || lower.contains("overruled")
        {
            candidates.push(blocked_decision_insight(trace));
        }
    }

    // 3. No context from any source → MissingContext
    if trace.total_context_items == 0 && !trace.unavailable_sources.is_empty() {
        // Only add if we didn't already emit an unavailable-sources insight
        // that overlaps — the unavailable sources insight already covers this.
        // If all sources returned 0 items and none are unavailable, add standalone.
        if trace.unavailable_sources.is_empty() {
            candidates.push(no_context_insight(trace));
        }
    }

    // 4. No compute route → WrongComputeChoice
    if trace.compute_route_label.is_none() {
        candidates.push(no_compute_route_insight(trace));
    }

    // 5. Cycle status is not "Completed" → InsufficientObservability
    if !trace.cycle_status.to_lowercase().starts_with("completed") {
        // Only emit if we haven't already emitted a blocked-decision insight
        // for this cycle, since a blocked decision naturally produces a non-completed status.
        let already_has_blocked = candidates.iter().any(|c| {
            matches!(
                c.failure_class,
                FailureClass::PolicyGap | FailureClass::BlockedWithoutExplanation
            )
        });
        if !already_has_blocked {
            candidates.push(incomplete_cycle_insight(trace));
        }
    }

    candidates
}

// ─── Heuristic insight builders ─────────────────────────────────────────────

fn unavailable_sources_insight(trace: &CycleTrace) -> FailureInsight {
    let src_list = trace.unavailable_sources.join(", ");
    let now = Utc::now();

    FailureInsight::new(
        FailureInsightId::new(format!(
            "fi-trace-{}-unavailable-sources",
            trace.cycle_id.as_str()
        )),
        FailureClass::MissingContext,
        InsightSeverity::Medium,
        CorrectionTarget::Memory,
        format!(
            "{} memory source(s) unavailable during orchestrator cycle",
            trace.unavailable_sources.len()
        ),
        format!(
            "Sources returned as unavailable for objective \"{}\": {}",
            trace.objective_text, src_list
        ),
        "The orchestrator could not consult these memory sources for advisory context, reducing the quality of assembled context for proposal generation.".to_owned(),
        format!(
            "Ensure the following memory sources are initialized before orchestrator cycles: {}",
            src_list
        ),
        "Memory / Adapter Layer".to_owned(),
        DetectionSignal::new(
            DetectionSignalType::RuntimeObservation,
            format!(
                "CycleTrace {} recorded {} unavailable source(s): {}",
                trace.cycle_id.as_str(),
                trace.unavailable_sources.len(),
                src_list
            ),
        ),
        0.85,
        now,
    )
    .with_trace_links(None, None, None, None, None)
}

fn blocked_decision_insight(trace: &CycleTrace) -> FailureInsight {
    let now = Utc::now();
    let status = trace.decision_status.as_deref().unwrap_or("unknown");

    FailureInsight::new(
        FailureInsightId::new(format!(
            "fi-trace-{}-blocked-decision",
            trace.cycle_id.as_str()
        )),
        FailureClass::PolicyGap,
        InsightSeverity::High,
        CorrectionTarget::Policy,
        format!("Decision Gate {} the proposed action", status),
        format!(
            "For objective \"{}\": Decision Gate returned status {} — the proposal was rejected by the current policy configuration.",
            trace.objective_text, status
        ),
        "The orchestrator cycle could not complete with an approved action. Further cycles or human intervention may be needed.".to_owned(),
        format!(
            "Review Decision Gate policy for the proposed action type and risk level. Consider whether the policy needs adjustment or the proposal needs refinement. Decision status: {}",
            status
        ),
        "Decision Gate / Policy".to_owned(),
        DetectionSignal::new(
            DetectionSignalType::RuntimeObservation,
            format!(
                "CycleTrace {} recorded a blocked Decision Gate: status={}",
                trace.cycle_id.as_str(),
                status
            ),
        ),
        0.9,
        now,
    )
    .with_trace_links(None, None, None, None, None)
}

fn no_context_insight(trace: &CycleTrace) -> FailureInsight {
    let now = Utc::now();

    FailureInsight::new(
        FailureInsightId::new(format!(
            "fi-trace-{}-no-context",
            trace.cycle_id.as_str()
        )),
        FailureClass::MissingContext,
        InsightSeverity::High,
        CorrectionTarget::Memory,
        "No advisory context items from any memory source".to_owned(),
        format!(
            "For objective \"{}\": all queried memory sources returned zero context items (total_context_items=0). The orchestrator assembled no advisory context for the proposal.",
            trace.objective_text
        ),
        "The proposal was generated without any memory context, potentially leading to generic or incorrect proposals.".to_owned(),
        "Investigate whether Graph Memory, Holographic Memory, Reservoir Echo, or Working Memory contain relevant data for similar objectives. Ensure at least one adapter is populated.".to_owned(),
        "Memory / Adapter Layer".to_owned(),
        DetectionSignal::new(
            DetectionSignalType::RuntimeObservation,
            format!(
                "CycleTrace {} recorded zero context items across all {} source(s)",
                trace.cycle_id.as_str(),
                trace.context_source_summaries.len()
            ),
        ),
        0.8,
        now,
    )
    .with_trace_links(None, None, None, None, None)
}

fn no_compute_route_insight(trace: &CycleTrace) -> FailureInsight {
    let now = Utc::now();

    FailureInsight::new(
        FailureInsightId::new(format!(
            "fi-trace-{}-no-compute-route",
            trace.cycle_id.as_str()
        )),
        FailureClass::WrongComputeChoice,
        InsightSeverity::Low,
        CorrectionTarget::Code,
        "No compute route was selected for the orchestrator cycle".to_owned(),
        format!(
            "For objective \"{}\": compute_route_label is None. The Compute Reservoir did not produce a route label for the cycle.",
            trace.objective_text
        ),
        "The orchestrator ran without an assigned compute resource strategy. This may indicate a Compute Reservoir configuration gap or a non-critical fallback path.".to_owned(),
        "Ensure ComputeReservoirResolver is configured with at least one compute node. Verify the ComputePolicy allows route selection for the objective's domain and complexity.".to_owned(),
        "Compute Reservoir".to_owned(),
        DetectionSignal::new(
            DetectionSignalType::RuntimeObservation,
            format!(
                "CycleTrace {} has no compute route label",
                trace.cycle_id.as_str()
            ),
        ),
        0.7,
        now,
    )
    .with_trace_links(None, None, None, None, None)
}

fn incomplete_cycle_insight(trace: &CycleTrace) -> FailureInsight {
    let now = Utc::now();

    FailureInsight::new(
        FailureInsightId::new(format!(
            "fi-trace-{}-incomplete-cycle",
            trace.cycle_id.as_str()
        )),
        FailureClass::InsufficientObservability,
        InsightSeverity::Low,
        CorrectionTarget::None,
        format!("Orchestrator cycle completed with status: {}", trace.cycle_status),
        format!(
            "For objective \"{}\": cycle_status is \"{}\", not \"Completed\". The cycle finished without a normal completion status.",
            trace.objective_text, trace.cycle_status
        ),
        "The orchestrator cycle may need operator inspection. The cycle metadata may contain clues about what prevented a normal completion.".to_owned(),
        "Inspect the CycleTrace fields (decision_status, unavailable_sources, compute_route) to diagnose the incomplete status. Consider whether additional policies or fallback paths are needed.".to_owned(),
        "Orchestrator / Cognitive Runtime".to_owned(),
        DetectionSignal::new(
            DetectionSignalType::RuntimeObservation,
            format!(
                "CycleTrace {} has status \"{}\" instead of \"Completed\"",
                trace.cycle_id.as_str(),
                trace.cycle_status
            ),
        ),
        0.6,
        now,
    )
    .with_trace_links(None, None, None, None, None)
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use arpagona_agent_core::ids::OrchestratorCycleId;

    fn empty_trace() -> CycleTrace {
        let mut trace = CycleTrace::new(
            OrchestratorCycleId::new("oc-test"),
            "Test objective",
            "Completed",
            "Cycle completed successfully.",
        );
        trace.compute_route_label = Some("local-small (llm, $0, 800ms)".to_owned());
        trace
    }

    // ─── Happy path: no heuristics trigger ────────────────────────────────

    #[test]
    fn clean_trace_produces_no_insights() {
        let trace = empty_trace();
        let candidates = extract_candidates(&trace);
        assert!(
            candidates.is_empty(),
            "Clean trace should produce no candidates, got: {:?}",
            candidates
        );
    }

    // ─── Unavailable sources ──────────────────────────────────────────────

    #[test]
    fn unavailable_sources_produces_missing_context_insight() {
        let mut trace = empty_trace();
        trace.unavailable_sources =
            vec!["graph_memory".to_owned(), "holographic_memory".to_owned()];

        let candidates = extract_candidates(&trace);

        // Unavailable sources should produce an insight
        let has_unavailable = candidates
            .iter()
            .any(|c| c.failure_class == FailureClass::MissingContext);
        assert!(
            has_unavailable,
            "Should produce MissingContext insight for unavailable sources"
        );

        // Summary should mention how many sources
        let unavail = candidates
            .iter()
            .find(|c| c.failure_class == FailureClass::MissingContext)
            .expect("MissingContext insight");
        assert!(unavail.summary.contains("2"));
        assert!(unavail.root_cause.contains("graph_memory"));
        assert!(unavail.root_cause.contains("holographic_memory"));
        assert_eq!(unavail.correction_target, CorrectionTarget::Memory);
        assert!(unavail.detection_signal.description.contains("oc-test"));
    }

    // ─── Blocked decision ─────────────────────────────────────────────────

    #[test]
    fn blocked_decision_produces_policy_gap_insight() {
        let mut trace = empty_trace();
        trace.decision_status = Some("Blocked".to_owned());

        let candidates = extract_candidates(&trace);

        let has_policy_gap = candidates
            .iter()
            .any(|c| c.failure_class == FailureClass::PolicyGap);
        assert!(
            has_policy_gap,
            "Should produce PolicyGap insight for blocked decision"
        );

        let blocked = candidates
            .iter()
            .find(|c| c.failure_class == FailureClass::PolicyGap)
            .expect("PolicyGap insight");
        assert!(blocked.summary.contains("Blocked"));
        assert_eq!(blocked.correction_target, CorrectionTarget::Policy);
    }

    #[test]
    fn denied_decision_produces_policy_gap_insight() {
        let mut trace = empty_trace();
        trace.decision_status = Some("Denied".to_owned());

        let candidates = extract_candidates(&trace);
        assert!(candidates
            .iter()
            .any(|c| c.failure_class == FailureClass::PolicyGap));
    }

    #[test]
    fn rejected_decision_produces_policy_gap_insight() {
        let mut trace = empty_trace();
        trace.decision_status = Some("Rejected".to_owned());

        let candidates = extract_candidates(&trace);
        assert!(candidates
            .iter()
            .any(|c| c.failure_class == FailureClass::PolicyGap));
    }

    #[test]
    fn overruled_decision_produces_policy_gap_insight() {
        let mut trace = empty_trace();
        trace.decision_status = Some("Overruled".to_owned());

        let candidates = extract_candidates(&trace);
        assert!(candidates
            .iter()
            .any(|c| c.failure_class == FailureClass::PolicyGap));
    }

    #[test]
    fn approved_decision_does_not_produce_policy_gap_insight() {
        let mut trace = empty_trace();
        trace.decision_status = Some("Approved".to_owned());

        let candidates = extract_candidates(&trace);
        assert!(
            !candidates
                .iter()
                .any(|c| c.failure_class == FailureClass::PolicyGap),
            "Approved decisions should not produce PolicyGap"
        );
    }

    // ─── Missing compute route ────────────────────────────────────────────

    #[test]
    fn missing_compute_route_produces_wrong_compute_choice_insight() {
        let mut trace = empty_trace();
        trace.compute_route_label = None;

        let candidates = extract_candidates(&trace);

        let has_wrong_compute = candidates
            .iter()
            .any(|c| c.failure_class == FailureClass::WrongComputeChoice);
        assert!(
            has_wrong_compute,
            "Should produce WrongComputeChoice insight for missing compute route"
        );

        let compute = candidates
            .iter()
            .find(|c| c.failure_class == FailureClass::WrongComputeChoice)
            .expect("WrongComputeChoice insight");
        assert!(
            compute.summary.contains("compute route"),
            "Summary should mention compute route"
        );
        assert_eq!(compute.correction_target, CorrectionTarget::Code);
    }

    #[test]
    fn present_compute_route_does_not_produce_wrong_compute_insight() {
        let mut trace = empty_trace();
        trace.compute_route_label = Some("local-small (llm, $0, 800ms)".to_owned());

        let candidates = extract_candidates(&trace);
        assert!(!candidates
            .iter()
            .any(|c| c.failure_class == FailureClass::WrongComputeChoice));
    }

    // ─── Incomplete cycle status ──────────────────────────────────────────

    #[test]
    fn non_completed_cycle_produces_insufficient_observability_insight() {
        let mut trace = empty_trace();
        trace.cycle_status = "Failed".to_owned();

        let candidates = extract_candidates(&trace);

        let has_incomplete = candidates
            .iter()
            .any(|c| c.failure_class == FailureClass::InsufficientObservability);
        assert!(
            has_incomplete,
            "Should produce InsufficientObservability insight for non-completed cycle"
        );

        let incomplete = candidates
            .iter()
            .find(|c| c.failure_class == FailureClass::InsufficientObservability)
            .expect("InsufficientObservability insight");
        assert!(incomplete.summary.contains("Failed"));
        assert_eq!(incomplete.correction_target, CorrectionTarget::None);
    }

    #[test]
    fn completed_cycle_does_not_produce_incomplete_insight() {
        let mut trace = empty_trace();
        trace.cycle_status = "Completed".to_owned();

        let candidates = extract_candidates(&trace);
        assert!(!candidates
            .iter()
            .any(|c| c.failure_class == FailureClass::InsufficientObservability));
    }

    // ─── Blocked decision suppresses duplicate incomplete-cycle insight ──

    #[test]
    fn blocked_decision_suppresses_redundant_incomplete_insight() {
        let mut trace = empty_trace();
        trace.cycle_status = "Blocked".to_owned();
        trace.decision_status = Some("Blocked".to_owned());

        let candidates = extract_candidates(&trace);

        // Should have PolicyGap (blocked decision) but NOT InsufficientObservability
        assert!(candidates
            .iter()
            .any(|c| c.failure_class == FailureClass::PolicyGap));
        assert!(
            !candidates
                .iter()
                .any(|c| c.failure_class == FailureClass::InsufficientObservability),
            "Blocked decision should suppress redundant incomplete-cycle insight"
        );
    }

    // ─── Multiple heuristics produce multiple insights ────────────────────

    #[test]
    fn multiple_heuristics_produce_separate_insights() {
        let mut trace = empty_trace();
        trace.unavailable_sources = vec!["graph_memory".to_owned()];
        trace.decision_status = Some("Blocked".to_owned());

        let candidates = extract_candidates(&trace);

        assert!(
            candidates.len() >= 2,
            "Multiple heuristics should produce at least 2 insights, got {}",
            candidates.len()
        );
        assert!(candidates
            .iter()
            .any(|c| c.failure_class == FailureClass::MissingContext));
        assert!(candidates
            .iter()
            .any(|c| c.failure_class == FailureClass::PolicyGap));
    }

    // ─── Id uniqueness ────────────────────────────────────────────────────

    #[test]
    fn each_insight_has_unique_id() {
        let mut trace = empty_trace();
        trace.unavailable_sources = vec!["graph_memory".to_owned()];
        trace.decision_status = Some("Blocked".to_owned());

        let candidates = extract_candidates(&trace);

        let ids: std::collections::HashSet<&str> =
            candidates.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(
            ids.len(),
            candidates.len(),
            "Each insight must have a unique id"
        );
    }

    // ─── Non-authorizing invariant ────────────────────────────────────────

    #[test]
    fn all_insights_are_proposed_non_authorizing() {
        let mut trace = empty_trace();
        trace.unavailable_sources = vec!["graph_memory".to_owned()];
        trace.decision_status = Some("Denied".to_owned());
        trace.compute_route_label = None;

        let candidates = extract_candidates(&trace);

        for c in &candidates {
            assert!(
                matches!(
                    c.status,
                    arpagona_agent_core::failure_insight::InsightStatus::Proposed
                ),
                "All insights must be Proposed (non-authorizing): {:?}",
                c.id
            );
        }
    }

    // ─── Zero context when sources are unavailable is suppressed ──────────

    #[test]
    fn zero_context_without_unavailable_sources_produces_standalone_insight() {
        let mut trace = empty_trace();
        trace.total_context_items = 0;
        trace.unavailable_sources = vec![];

        let candidates = extract_candidates(&trace);

        // Should NOT produce MissingContext for unavailable (none)
        assert!(
            !candidates
                .iter()
                .any(|c| c.failure_class == FailureClass::MissingContext),
            "Zero context without unavailable sources should not produce MissingContext from that heuristic"
        );
    }
}
