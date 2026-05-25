//! Pure domain types for the General Cognitive Work Loop V0.
//!
//! This module defines the first general-purpose cognitive cycle skeleton for
//! ARPAGONA Agent Core. Every type is pure, serializable, I/O-free, LLM-free,
//! tool-free, persistence-free, and non-authorizing.
//!
//! The cycle transforms an `Objective` + optional context into a
//! `CognitiveCycleResult` containing a `WorkingMemory`, `CognitivePlan`,
//! `RequiredObservation`s, a `ProposedNextAction`, and
//! `ImprovementCandidate`s. The engine is heuristic and deterministic — it
//! does not pretend to be intelligent. It produces structure that a future
//! LLM/orchestrator can consume and refine.
//!
//! WorkingMemory is the central state container for a cognitive cycle. It
//! bundles the objective, context, observations, improvement candidates,
//! failure insight candidates, and the proposed next action into a single
//! self-describing structure.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::ObjectiveId;
use crate::observation::CognitiveObservation;
use crate::observation::FailureInsightCandidate;

// ─── Objective ────────────────────────────────────────────────────────────

/// High-level domain classification for an objective.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectiveDomain {
    General,
    Business,
    Research,
    Teaching,
    Engineering,
    Administration,
    PersonalProductivity,
    Coding,
    Unknown,
}

/// Lifecycle status of an objective.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectiveStatus {
    Proposed,
    Active,
    Completed,
    Superseded,
    Cancelled,
}

/// A measurable criterion that defines success for an objective.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SuccessCriterion {
    pub id: String,
    pub description: String,
    pub measurable: bool,
}

/// A professional objective that the agent is asked to work toward.
///
/// Pure domain: no I/O, no execution, no authorization.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Objective {
    pub id: ObjectiveId,
    pub title: String,
    pub description: String,
    pub domain: ObjectiveDomain,
    pub status: ObjectiveStatus,
    pub success_criteria: Vec<SuccessCriterion>,
    pub created_at: DateTime<Utc>,
}

// ─── Working Memory ───────────────────────────────────────────────────────

/// A single item of context carried in working memory.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContextItem {
    pub key: String,
    pub value: String,
    pub source: String,
}

/// An assumption made during the cognitive cycle.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Assumption {
    pub id: String,
    pub description: String,
    pub confidence: f32,
}

/// A known constraint that bounds the objective space.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Constraint {
    pub id: String,
    pub description: String,
    pub kind: String,
}

/// A piece of context that is missing for effective planning.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MissingContext {
    pub id: String,
    pub description: String,
    pub why_needed: String,
}

/// Lifecycle status of a cognitive cycle.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CycleStatus {
    /// The cycle has been initialised with an objective but no processing.
    Initialized,
    /// The cycle is gathering context and observations.
    ContextGathering,
    /// The cycle is analysing the objective and context.
    Analyzing,
    /// Observations are being collected.
    Observing,
    /// A next action is being proposed.
    Proposing,
    /// The cycle has completed and is ready for review.
    Completed,
    /// The cycle needs human or orchestrator review before proceeding.
    NeedsReview,
}

/// The agent's current working memory for a single cognitive cycle.
///
/// This is the central state container that bundles the objective, context,
/// observations, improvement candidates, failure insight candidates, and
/// the proposed next action into a single self-describing structure.
///
/// # Safety
///
/// - All fields are descriptive, not prescriptive.
/// - `evidence_only_warning` is a static invariant marker.
/// - No field authorises execution, persistence, or side effects.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkingMemory {
    /// The objective this cycle is working toward.
    pub objective: Option<Objective>,
    /// Context items provided or discovered during the cycle.
    pub context_items: Vec<ContextItem>,
    /// Assumptions made during the cycle.
    pub assumptions: Vec<Assumption>,
    /// Known constraints that bound the objective space.
    pub constraints: Vec<Constraint>,
    /// Context that is missing and needs to be gathered.
    pub missing_context: Vec<MissingContext>,
    /// Observations that are required before continuing.
    pub required_observations: Vec<RequiredObservation>,
    /// Cognitive observations produced during the cycle.
    pub cognitive_observations: Vec<CognitiveObservation>,
    /// Improvement candidates identified during the cycle.
    pub improvement_candidates: Vec<ImprovementCandidate>,
    /// Failure insight candidates (bridge from improvement candidates).
    pub failure_insight_candidates: Vec<FailureInsightCandidate>,
    /// The proposed next action for the cycle (None if not yet determined).
    pub proposed_next_action: Option<ProposedNextAction>,
    /// The lifecycle status of this cycle.
    pub cycle_status: CycleStatus,
    /// Static warning that this output is evidence-only and non-authorizing.
    pub evidence_only_warning: String,
}

// ─── Cognitive Plan ───────────────────────────────────────────────────────

/// A single step in a cognitive plan.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: String,
    pub description: String,
    pub order: usize,
}

/// A minimal plan produced by the cognitive cycle.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CognitivePlan {
    pub steps: Vec<PlanStep>,
    pub rationale: String,
}

// ─── Required Observation ─────────────────────────────────────────────────

/// An observation the agent needs to acquire before continuing.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RequiredObservation {
    pub id: String,
    pub description: String,
    pub why_needed: String,
}

// ─── Proposed Next Action ─────────────────────────────────────────────────

/// The kind of next action proposed by the cognitive cycle.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NextActionKind {
    RequestContext,
    UseTool,
    ProposeMemoryUpdate,
    ProposeFailureInsight,
    ProposePlan,
    StopWithReport,
    EscalateToHuman,
}

/// A proposed next action. Non-authorizing — it must be reviewed before
/// execution by a human or an orchestrator.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProposedNextAction {
    pub kind: NextActionKind,
    pub description: String,
    pub rationale: String,
    pub requires_authorization: bool,
    /// Always true in V0. This field exists as an explicit invariant marker.
    pub non_authorizing: bool,
}

// ─── Improvement Candidate ────────────────────────────────────────────────

/// The kind of improvement the cycle can identify.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImprovementCandidateKind {
    MissingContext,
    WeakPlan,
    RepeatedFriction,
    MissingTool,
    MissingMemory,
    PolicyGap,
    ProcessImprovement,
    PromptImprovement,
    TestImprovement,
}

/// A candidate improvement identified during the cycle.
///
/// Non-authorizing — does not mutate any state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImprovementCandidate {
    pub id: String,
    pub kind: ImprovementCandidateKind,
    pub description: String,
    pub rationale: String,
}

// ─── Cycle Result ─────────────────────────────────────────────────────────

/// The complete result of one general cognitive work cycle.
///
/// This is the single output type that a future LLM/orchestrator can consume
/// to decide the next step. Everything is read-only in V0.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CognitiveCycleResult {
    pub objective: Objective,
    pub working_memory: WorkingMemory,
    pub plan: CognitivePlan,
    pub required_observations: Vec<RequiredObservation>,
    pub proposed_next_action: ProposedNextAction,
    pub improvement_candidates: Vec<ImprovementCandidate>,
    /// Static warning that this output is evidence-only and non-authorizing.
    pub warning: &'static str,
}

/// Warning token embedded in every CognitiveCycleResult.
pub const COGNITIVE_READBACK_WARNING: &str =
    "Readback only — evidence and analysis, not approval, not authorization, not execution. \
     Review all candidates before acting.";

// ─── WorkingMemory Builder ─────────────────────────────────────────────────

/// Build a complete `WorkingMemory` from optional cycle components.
///
/// This is a pure, non-authorizing builder that creates a self-describing
/// WorkingMemory from whatever components are available. Every parameter is
/// optional, making it suitable for incremental cycle construction.
///
/// # Parameters
///
/// - `objective` — the objective driving this cycle
/// - `context` — optional context items (key:value pairs)
/// - `observations` — optional cognitive observations produced so far
/// - `improvement_candidates` — optional improvement candidates identified
/// - `failure_insight_candidates` — optional failure insight candidates
///
/// # Returns
///
/// A fully populated `WorkingMemory` with `cycle_status: Completed`.
///
/// # Safety
///
/// - Pure function: no I/O, no LLM calls, no side effects.
/// - The returned WorkingMemory carries `evidence_only_warning`.
/// - No field authorises execution or persistence.
pub fn build_working_memory_cycle(
    objective: Option<Objective>,
    context: Option<Vec<ContextItem>>,
    observations: Option<Vec<CognitiveObservation>>,
    improvement_candidates: Option<Vec<ImprovementCandidate>>,
    failure_insight_candidates: Option<Vec<FailureInsightCandidate>>,
) -> WorkingMemory {
    WorkingMemory {
        objective,
        context_items: context.unwrap_or_default(),
        assumptions: vec![],
        constraints: vec![],
        missing_context: vec![],
        required_observations: vec![],
        cognitive_observations: observations.unwrap_or_default(),
        improvement_candidates: improvement_candidates.unwrap_or_default(),
        failure_insight_candidates: failure_insight_candidates.unwrap_or_default(),
        proposed_next_action: None,
        cycle_status: CycleStatus::Completed,
        evidence_only_warning: COGNITIVE_READBACK_WARNING.to_owned(),
    }
}

// ─── Heuristic Engine ─────────────────────────────────────────────────────

/// Run one cycle of the general cognitive work loop.
///
/// This is a purely heuristic, deterministic, I/O-free engine. It:
///  1. Creates an `Objective` from the input text.
///  2. Classifies the domain heuristically if not provided.
///  3. Builds `WorkingMemory` from optional context and objective analysis.
///  4. Detects missing context.
///  5. Generates a minimal `CognitivePlan`.
///  6. Produces `RequiredObservation`s.
///  7. Proposes a `ProposedNextAction`.
///  8. Collects `ImprovementCandidate`s.
///
/// The engine does NOT call LLMs, execute tools, persist data, or authorize
/// side effects.
pub fn run_cognitive_work_cycle(
    objective_input: &str,
    optional_domain: Option<ObjectiveDomain>,
    optional_context: Option<&str>,
) -> CognitiveCycleResult {
    let now = Utc::now();

    // 1. Determine domain
    let domain = optional_domain.unwrap_or_else(|| classify_domain(objective_input));

    // 2. Create Objective
    let objective = Objective {
        id: ObjectiveId::new(format!("obj-{}", now.timestamp())),
        title: objective_input.to_owned(),
        description: objective_input.to_owned(),
        domain: domain.clone(),
        status: ObjectiveStatus::Proposed,
        success_criteria: vec![],
        created_at: now,
    };

    // 3. Parse context into WorkingMemory
    let context_items = parse_context(optional_context);
    let assumptions = generate_assumptions(objective_input, &domain);
    let constraints = generate_constraints(&domain);
    let missing_context = detect_missing_context(objective_input, &domain, optional_context);

    // 4. Generate plan
    let plan = generate_plan(&domain, &objective_input, &missing_context);

    // 5. Generate required observations
    let required_observations = generate_required_observations(&domain, &missing_context);

    // 6. Determine next action
    let proposed_next_action = determine_next_action(&missing_context, &domain);

    // 7. Collect improvement candidates
    let improvement_candidates =
        generate_improvement_candidates(&objective_input, &missing_context, &domain);

    // 8. Build enriched WorkingMemory with all cycle state
    let working_memory = WorkingMemory {
        objective: Some(objective.clone()),
        context_items,
        assumptions,
        constraints,
        missing_context: missing_context.clone(),
        required_observations: required_observations.clone(),
        cognitive_observations: vec![],
        improvement_candidates: improvement_candidates.clone(),
        failure_insight_candidates: vec![],
        proposed_next_action: Some(proposed_next_action.clone()),
        cycle_status: CycleStatus::Completed,
        evidence_only_warning: COGNITIVE_READBACK_WARNING.to_owned(),
    };

    CognitiveCycleResult {
        objective,
        working_memory,
        plan,
        required_observations,
        proposed_next_action,
        improvement_candidates,
        warning: COGNITIVE_READBACK_WARNING,
    }
}

// ─── Heuristic helpers ────────────────────────────────────────────────────

/// Classify domain based on keywords in the objective text.
fn classify_domain(input: &str) -> ObjectiveDomain {
    let lower = input.to_lowercase();

    let business_keywords = [
        "stratégie",
        "stratégique",
        "business",
        "marché",
        "client",
        "prospection",
        "vente",
        "chiffre",
        "commerce",
        "marketing",
        "rentabilité",
        "clientèle",
        "business development",
        "sales",
        "market",
        "revenue",
        "profit",
    ];
    let research_keywords = [
        "recherche",
        "étude",
        "analyse",
        "rechercher",
        "investigate",
        "research",
        "study",
        "survey",
        "analyse",
        "enquête",
        "scientifique",
        "étudier",
    ];
    let teaching_keywords = [
        "cours",
        "formation",
        "enseigner",
        "apprendre",
        "didacticiel",
        "tutoriel",
        "teaching",
        "training",
        "course",
        "lesson",
        "pédagogie",
        "apprenant",
    ];
    let engineering_keywords = [
        "architecture",
        "système",
        "infrastructure",
        "déploiement",
        "engineering",
        "system design",
        "architecture",
        "pipeline",
        "infrastructure",
    ];
    let admin_keywords = [
        "administration",
        "organisation",
        "planifier",
        "gérer",
        "administration",
        "organize",
        "manage",
        "schedule",
        "coordonner",
        "processus",
    ];
    let productivity_keywords = [
        "productivité",
        "personnel",
        "habitude",
        "routine",
        "productivity",
        "personal",
        "habit",
        "routine",
        "efficiency",
        "efficacité",
    ];
    let coding_keywords = [
        "code",
        "développement",
        "programmation",
        "implémenter",
        "refactor",
        "coding",
        "development",
        "programming",
        "implement",
        "refactor",
        "api",
        "library",
        "crate",
        "module",
        "rust",
        "python",
        "typescript",
    ];

    if contains_any(&lower, &coding_keywords) {
        ObjectiveDomain::Coding
    } else if contains_any(&lower, &business_keywords) {
        ObjectiveDomain::Business
    } else if contains_any(&lower, &research_keywords) {
        ObjectiveDomain::Research
    } else if contains_any(&lower, &teaching_keywords) {
        ObjectiveDomain::Teaching
    } else if contains_any(&lower, &engineering_keywords) {
        ObjectiveDomain::Engineering
    } else if contains_any(&lower, &admin_keywords) {
        ObjectiveDomain::Administration
    } else if contains_any(&lower, &productivity_keywords) {
        ObjectiveDomain::PersonalProductivity
    } else {
        ObjectiveDomain::General
    }
}

fn contains_any(text: &str, keywords: &[&str]) -> bool {
    keywords.iter().any(|kw| text.contains(kw))
}

/// Parse optional context text into context items.
fn parse_context(optional_context: Option<&str>) -> Vec<ContextItem> {
    let Some(context) = optional_context else {
        return vec![];
    };
    let trimmed = context.trim();
    if trimmed.is_empty() {
        return vec![];
    }
    // Try to split by newlines and treat each line as key:value or a sentence
    trimmed
        .lines()
        .enumerate()
        .filter_map(|(i, line)| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            // Try "key: value" format first
            if let Some((key, value)) = line.split_once(':') {
                let k = key.trim().to_owned();
                let v = value.trim().to_owned();
                if !k.is_empty() && !v.is_empty() {
                    return Some(ContextItem {
                        key: k,
                        value: v,
                        source: "user_provided".to_owned(),
                    });
                }
            }
            // Fall back to treating the line as a free-form item
            Some(ContextItem {
                key: format!("context_{}", i + 1),
                value: line.to_owned(),
                source: "user_provided".to_owned(),
            })
        })
        .collect()
}

/// Generate heuristic assumptions from the objective text and domain.
fn generate_assumptions(input: &str, domain: &ObjectiveDomain) -> Vec<Assumption> {
    let mut assumptions = Vec::new();

    // Every objective has at least a framing assumption
    assumptions.push(Assumption {
        id: "assumption-framing".to_owned(),
        description: "The objective is framed as stated with no hidden agenda.".to_owned(),
        confidence: 0.9,
    });

    let lower = input.to_lowercase();

    // Heuristic: is there a time element?
    if contains_any(
        &lower,
        &["urgent", "rapidement", "vite", "quick", "asap", "soon"],
    ) {
        assumptions.push(Assumption {
            id: "assumption-time-pressure".to_owned(),
            description: "The objective implies time pressure.".to_owned(),
            confidence: 0.6,
        });
    }

    // Heuristic: is there a budget element?
    if contains_any(&lower, &["budget", "coût", "cost", "ressource", "resource"]) {
        assumptions.push(Assumption {
            id: "assumption-resource-constraint".to_owned(),
            description: "Resource or budget constraints are relevant.".to_owned(),
            confidence: 0.7,
        });
    }

    // Domain-specific assumption
    match domain {
        ObjectiveDomain::Business => {
            assumptions.push(Assumption {
                id: "assumption-business-goal".to_owned(),
                description: "The objective targets a business outcome.".to_owned(),
                confidence: 0.8,
            });
        }
        ObjectiveDomain::Teaching => {
            assumptions.push(Assumption {
                id: "assumption-audience".to_owned(),
                description: "There is a defined learner audience with specific needs.".to_owned(),
                confidence: 0.7,
            });
        }
        ObjectiveDomain::Coding => {
            assumptions.push(Assumption {
                id: "assumption-tech-stack".to_owned(),
                description: "A specific tech stack or language is implied.".to_owned(),
                confidence: 0.6,
            });
        }
        _ => {}
    }

    assumptions
}

/// Generate domain-appropriate constraints.
fn generate_constraints(domain: &ObjectiveDomain) -> Vec<Constraint> {
    let mut constraints = Vec::new();
    constraints.push(Constraint {
        id: "constraint-no-external-effect".to_owned(),
        description: "The agent must not execute external side effects.".to_owned(),
        kind: "policy".to_owned(),
    });
    constraints.push(Constraint {
        id: "constraint-v0-readonly".to_owned(),
        description: "General Cognitive Work Loop V0 is read-only.".to_owned(),
        kind: "policy".to_owned(),
    });

    match domain {
        ObjectiveDomain::Business => {
            constraints.push(Constraint {
                id: "constraint-business-ethics".to_owned(),
                description: "Business proposals must respect ethical and legal boundaries."
                    .to_owned(),
                kind: "policy".to_owned(),
            });
        }
        ObjectiveDomain::Teaching => {
            constraints.push(Constraint {
                id: "constraint-pedagogical".to_owned(),
                description: "Content must be accurate, age-appropriate, and pedagogically sound."
                    .to_owned(),
                kind: "quality".to_owned(),
            });
        }
        ObjectiveDomain::Coding => {
            constraints.push(Constraint {
                id: "constraint-code-quality".to_owned(),
                description:
                    "Generated code should follow idiomatic practices for the target language."
                        .to_owned(),
                kind: "quality".to_owned(),
            });
        }
        ObjectiveDomain::Research => {
            constraints.push(Constraint {
                id: "constraint-citation".to_owned(),
                description: "Research findings should be verifiable and cite sources.".to_owned(),
                kind: "quality".to_owned(),
            });
        }
        _ => {}
    }

    constraints
}

/// Detect missing context based on the objective and domain.
fn detect_missing_context(
    input: &str,
    domain: &ObjectiveDomain,
    optional_context: Option<&str>,
) -> Vec<MissingContext> {
    let mut missing = Vec::new();
    let lower = input.to_lowercase();
    let has_context = optional_context.is_some_and(|c| !c.trim().is_empty());

    if !has_context {
        missing.push(MissingContext {
            id: "missing-general-context".to_owned(),
            description: "No context provided.".to_owned(),
            why_needed: "Without context, the plan may miss critical constraints and stakeholder expectations."
                .to_owned(),
        });
    }

    // Domain-specific missing context detection
    match domain {
        ObjectiveDomain::Business => {
            if !contains_any(
                &lower,
                &["client", "marché", "market", "customer", "target"],
            ) {
                missing.push(MissingContext {
                    id: "missing-target-audience".to_owned(),
                    description: "Target audience or market is not specified.".to_owned(),
                    why_needed: "Business strategy requires knowing who the target is.".to_owned(),
                });
            }
            if !contains_any(&lower, &["budget", "resource", "ressource"]) {
                missing.push(MissingContext {
                    id: "missing-budget".to_owned(),
                    description: "Budget or resource constraints not specified.".to_owned(),
                    why_needed: "Resource allocation affects feasibility and scope.".to_owned(),
                });
            }
        }
        ObjectiveDomain::Teaching => {
            if !contains_any(
                &lower,
                &[
                    "débutant",
                    "intermédiaire",
                    "avancé",
                    "niveau",
                    "level",
                    "beginner",
                    "advanced",
                ],
            ) {
                missing.push(MissingContext {
                    id: "missing-learner-level".to_owned(),
                    description: "Learner level or prerequisites not specified.".to_owned(),
                    why_needed: "Course content depends heavily on the audience's prior knowledge."
                        .to_owned(),
                });
            }
            if !contains_any(
                &lower,
                &[
                    "durée", "duration", "temps", "time", "session", "heure", "hour",
                ],
            ) {
                missing.push(MissingContext {
                    id: "missing-duration".to_owned(),
                    description: "Course duration not specified.".to_owned(),
                    why_needed: "Duration determines scope and depth of content.".to_owned(),
                });
            }
        }
        ObjectiveDomain::Coding => {
            if !contains_any(
                &lower,
                &[
                    "langage",
                    "language",
                    "rust",
                    "python",
                    "typescript",
                    "javascript",
                    "go",
                ],
            ) {
                missing.push(MissingContext {
                    id: "missing-language".to_owned(),
                    description: "Target programming language not specified.".to_owned(),
                    why_needed: "Implementation details depend on language choice.".to_owned(),
                });
            }
        }
        ObjectiveDomain::Research => {
            if !contains_any(
                &lower,
                &[
                    "méthode", "method", "approach", "approche", "source", "source",
                ],
            ) {
                missing.push(MissingContext {
                    id: "missing-methodology".to_owned(),
                    description: "Research methodology not specified.".to_owned(),
                    why_needed: "Methodology determines how to gather and evaluate evidence."
                        .to_owned(),
                });
            }
        }
        _ => {}
    }

    missing
}

/// Generate a minimal cognitive plan.
fn generate_plan(
    domain: &ObjectiveDomain,
    input: &str,
    missing_context: &[MissingContext],
) -> CognitivePlan {
    let mut steps = Vec::new();
    let has_context_gap = !missing_context.is_empty();

    if has_context_gap {
        steps.push(PlanStep {
            id: "step-gather-context".to_owned(),
            description: "Gather missing context before detailed planning.".to_owned(),
            order: 1,
        });
    }

    steps.push(PlanStep {
        id: "step-analyze-objective".to_owned(),
        description: "Analyse the objective and decompose into sub-tasks.".to_owned(),
        order: if has_context_gap { 2 } else { 1 },
    });

    steps.push(PlanStep {
        id: "step-identify-observations".to_owned(),
        description: "Identify what observations are required to inform decisions.".to_owned(),
        order: if has_context_gap { 3 } else { 2 },
    });

    let domain_step_id = match domain {
        ObjectiveDomain::Business => "step-business-strategy".to_owned(),
        ObjectiveDomain::Teaching => "step-design-curriculum".to_owned(),
        ObjectiveDomain::Coding => "step-plan-implementation".to_owned(),
        ObjectiveDomain::Research => "step-design-research".to_owned(),
        ObjectiveDomain::Engineering => "step-architect-solution".to_owned(),
        ObjectiveDomain::Administration => "step-plan-process".to_owned(),
        ObjectiveDomain::PersonalProductivity => "step-plan-routine".to_owned(),
        _ => "step-execute-plan".to_owned(),
    };

    steps.push(PlanStep {
        id: domain_step_id,
        description: format!("Execute domain-specific work for {:?}.", domain),
        order: if has_context_gap { 4 } else { 3 },
    });

    CognitivePlan {
        steps,
        rationale: format!(
            "Minimal plan for objective '{}' (domain: {:?}). Context gaps adjust the order.",
            input, domain
        ),
    }
}

/// Generate required observations based on domain and missing context.
fn generate_required_observations(
    domain: &ObjectiveDomain,
    missing_context: &[MissingContext],
) -> Vec<RequiredObservation> {
    let mut observations = Vec::new();

    // Add observations for each missing context item
    for mc in missing_context {
        observations.push(RequiredObservation {
            id: format!("obs-{}", mc.id),
            description: format!("Clarify: {}", mc.description),
            why_needed: mc.why_needed.clone(),
        });
    }

    // Domain-specific observations
    match domain {
        ObjectiveDomain::Business => {
            observations.push(RequiredObservation {
                id: "obs-market-landscape".to_owned(),
                description: "Identify current market landscape and competitors.".to_owned(),
                why_needed: "Informs differentiation and positioning.".to_owned(),
            });
        }
        ObjectiveDomain::Teaching => {
            observations.push(RequiredObservation {
                id: "obs-existing-materials".to_owned(),
                description: "Check for existing teaching materials on the topic.".to_owned(),
                why_needed: "Avoids duplication and identifies gaps.".to_owned(),
            });
        }
        ObjectiveDomain::Coding => {
            observations.push(RequiredObservation {
                id: "obs-existing-codebase".to_owned(),
                description: "Review existing codebase or project structure.".to_owned(),
                why_needed: "Ensures consistency and reuse.".to_owned(),
            });
        }
        ObjectiveDomain::Research => {
            observations.push(RequiredObservation {
                id: "obs-literature-review".to_owned(),
                description: "Perform a literature review on the topic.".to_owned(),
                why_needed: "Builds on existing knowledge and avoids repetition.".to_owned(),
            });
        }
        _ => {
            observations.push(RequiredObservation {
                id: "obs-general-info".to_owned(),
                description: "Gather general information relevant to the objective.".to_owned(),
                why_needed: "Informed decision-making requires baseline data.".to_owned(),
            });
        }
    }

    observations
}

/// Determine what the next action should be based on the current state.
fn determine_next_action(
    missing_context: &[MissingContext],
    _domain: &ObjectiveDomain,
) -> ProposedNextAction {
    if !missing_context.is_empty() {
        ProposedNextAction {
            kind: NextActionKind::RequestContext,
            description: format!(
                "Request additional context: {} gap(s) identified (e.g. {}).",
                missing_context.len(),
                missing_context
                    .iter()
                    .take(2)
                    .map(|m| m.description.as_str())
                    .collect::<Vec<_>>()
                    .join("; ")
            ),
            rationale: "Completed initial analysis but key context is missing for robust planning."
                .to_owned(),
            requires_authorization: false,
            non_authorizing: true,
        }
    } else {
        ProposedNextAction {
            kind: NextActionKind::StopWithReport,
            description: "Objective analyzed, working memory populated, plan ready.".to_owned(),
            rationale:
                "All available context has been processed. Ready for execution or human review."
                    .to_owned(),
            requires_authorization: false,
            non_authorizing: true,
        }
    }
}

/// Generate improvement candidates based on what was observed.
fn generate_improvement_candidates(
    input: &str,
    missing_context: &[MissingContext],
    domain: &ObjectiveDomain,
) -> Vec<ImprovementCandidate> {
    let mut candidates = Vec::new();

    if !missing_context.is_empty() {
        candidates.push(ImprovementCandidate {
            id: "improve-missing-context".to_owned(),
            kind: ImprovementCandidateKind::MissingContext,
            description: format!(
                "Objective '{}' has {} missing context item(s).",
                input,
                missing_context.len()
            ),
            rationale: "Adding context would improve plan quality and reduce assumptions."
                .to_owned(),
        });
    }

    // Detect if the objective is very short/ambiguous
    if input.len() < 30 {
        candidates.push(ImprovementCandidate {
            id: "improve-weak-objective".to_owned(),
            kind: ImprovementCandidateKind::WeakPlan,
            description: "Objective is very short — may be ambiguous.".to_owned(),
            rationale: "A more detailed objective description would lead to better plans."
                .to_owned(),
        });
    }

    // Domain-specific improvement candidates
    match domain {
        ObjectiveDomain::Unknown => {
            candidates.push(ImprovementCandidate {
                id: "improve-domain-classification".to_owned(),
                kind: ImprovementCandidateKind::ProcessImprovement,
                description: "Domain could not be classified heuristically.".to_owned(),
                rationale:
                    "An LLM-based classifier or explicit user input would improve domain detection."
                        .to_owned(),
            });
        }
        _ => {}
    }

    candidates
}

// ─── Utility ──────────────────────────────────────────────────────────────

impl ProposedNextAction {
    /// Check that this action is non-authorizing.
    pub fn is_safe(&self) -> bool {
        self.non_authorizing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── Serialisation ───────────────────────────────────────────────────

    #[test]
    fn objective_serializes_roundtrip() {
        let obj = Objective {
            id: ObjectiveId::new("obj-1"),
            title: "Test".to_owned(),
            description: "A test objective.".to_owned(),
            domain: ObjectiveDomain::Business,
            status: ObjectiveStatus::Proposed,
            success_criteria: vec![],
            created_at: Utc::now(),
        };
        let json = serde_json::to_string(&obj).unwrap();
        let decoded: Objective = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.id, obj.id);
        assert_eq!(decoded.domain, ObjectiveDomain::Business);
    }

    // ─── Domain classification ───────────────────────────────────────────

    #[test]
    fn business_objective_produces_business_domain() {
        let result = run_cognitive_work_cycle(
            "Préparer une stratégie de prospection locale pour ARPAGONA",
            None,
            None,
        );
        assert_eq!(result.objective.domain, ObjectiveDomain::Business);
    }

    #[test]
    fn teaching_objective_produces_teaching_domain() {
        let result = run_cognitive_work_cycle("Créer un cours sur le diagramme FAST", None, None);
        assert_eq!(result.objective.domain, ObjectiveDomain::Teaching);
    }

    #[test]
    fn coding_objective_produces_coding_domain() {
        let result =
            run_cognitive_work_cycle("Refactoriser le module de parsing en Rust", None, None);
        assert_eq!(result.objective.domain, ObjectiveDomain::Coding);
    }

    #[test]
    fn research_objective_produces_research_domain() {
        let result = run_cognitive_work_cycle(
            "Étudier l'impact de l'IA sur l'éducation en France",
            None,
            None,
        );
        assert_eq!(result.objective.domain, ObjectiveDomain::Research);
    }

    // ─── Missing context ─────────────────────────────────────────────────

    #[test]
    fn objective_without_context_produces_missing_context() {
        let result = run_cognitive_work_cycle(
            "Préparer une stratégie de prospection locale pour ARPAGONA",
            None,
            None,
        );
        assert!(
            !result.working_memory.missing_context.is_empty(),
            "Expected missing context when no context provided"
        );
    }

    #[test]
    fn objective_with_context_has_less_missing_context() {
        let without = run_cognitive_work_cycle(
            "Préparer une stratégie de prospection locale pour ARPAGONA",
            None,
            None,
        );
        let with_context = run_cognitive_work_cycle(
            "Préparer une stratégie de prospection locale pour ARPAGONA",
            None,
            Some("budget: 5000€\nmarket: Île-de-France\ntarget: PME locales"),
        );
        assert!(
            with_context.working_memory.missing_context.len()
                < without.working_memory.missing_context.len(),
            "Context should reduce missing context items"
        );
    }

    // ─── Non-authorizing ─────────────────────────────────────────────────

    #[test]
    fn proposed_next_action_is_non_authorizing() {
        let result = run_cognitive_work_cycle("Test", None, None);
        assert!(
            result.proposed_next_action.non_authorizing,
            "Every ProposedNextAction must be non-authorizing"
        );
        assert!(
            result.proposed_next_action.is_safe(),
            "is_safe() must return true"
        );
    }

    #[test]
    fn improvement_candidates_do_not_mutate_anything() {
        let result = run_cognitive_work_cycle("Test", None, None);
        // The candidates exist but nothing was mutated — this is a structural assertion.
        assert!(!result.improvement_candidates.is_empty());
        assert_eq!(result.objective.status, ObjectiveStatus::Proposed);
    }

    #[test]
    fn warning_is_evidence_only() {
        let result = run_cognitive_work_cycle("Test", None, None);
        assert!(result.warning.contains("Readback only"));
        assert!(result.warning.contains("not approval"));
    }

    // ─── Cycle completeness ──────────────────────────────────────────────

    #[test]
    fn cycle_contains_all_fields() {
        let result = run_cognitive_work_cycle("Organiser une veille IA locale", None, None);
        assert!(result.working_memory.context_items.is_empty());
        assert!(!result.working_memory.assumptions.is_empty());
        assert!(!result.working_memory.constraints.is_empty());
        assert!(!result.plan.steps.is_empty());
        assert!(!result.required_observations.is_empty());
        assert!(!result.improvement_candidates.is_empty());
    }

    #[test]
    fn cycle_uses_explicit_domain_when_provided() {
        let result =
            run_cognitive_work_cycle("Générique", Some(ObjectiveDomain::Engineering), None);
        assert_eq!(result.objective.domain, ObjectiveDomain::Engineering);
    }

    #[test]
    fn context_parses_key_value_pairs() {
        let result = run_cognitive_work_cycle("Test", None, Some("key: value\nfoo: bar"));
        assert_eq!(result.working_memory.context_items.len(), 2);
        assert_eq!(result.working_memory.context_items[0].key, "key");
        assert_eq!(result.working_memory.context_items[0].value, "value");
    }

    #[test]
    fn empty_context_produces_no_context_items() {
        let result = run_cognitive_work_cycle("Test", None, Some(""));
        assert!(result.working_memory.context_items.is_empty());
    }

    #[test]
    fn proposed_next_action_kind_is_request_context_when_context_missing() {
        let result = run_cognitive_work_cycle(
            "Expansion stratégique",
            Some(ObjectiveDomain::Business),
            None,
        );
        assert_eq!(
            result.proposed_next_action.kind,
            NextActionKind::RequestContext
        );
    }

    #[test]
    fn improvement_candidates_include_weak_plan_for_short_objective() {
        let result = run_cognitive_work_cycle("Hi", None, None);
        assert!(result
            .improvement_candidates
            .iter()
            .any(|c| c.kind == ImprovementCandidateKind::WeakPlan));
    }

    #[test]
    fn domain_toggle_via_explicit_value_not_keyword() {
        let result = run_cognitive_work_cycle(
            "Generic phrase without any clear domain keywords",
            Some(ObjectiveDomain::PersonalProductivity),
            None,
        );
        assert_eq!(
            result.objective.domain,
            ObjectiveDomain::PersonalProductivity
        );
    }

    // ─── P4 — Working Memory Integration ─────────────────────────────────

    #[test]
    fn working_memory_contains_objective() {
        let result = run_cognitive_work_cycle(
            "Prepare a local business outreach plan for ARPAGONA",
            None,
            None,
        );
        let wm = &result.working_memory;
        assert!(
            wm.objective.is_some(),
            "WorkingMemory must contain objective"
        );
        assert!(wm.objective.as_ref().unwrap().title.contains("ARPAGONA"));
    }

    #[test]
    fn working_memory_contains_proposed_next_action() {
        let result = run_cognitive_work_cycle(
            "Prepare a local business outreach plan for ARPAGONA",
            None,
            None,
        );
        let wm = &result.working_memory;
        assert!(
            wm.proposed_next_action.is_some(),
            "WorkingMemory must contain proposed_next_action"
        );
        assert!(
            wm.proposed_next_action.as_ref().unwrap().non_authorizing,
            "Proposed next action must be non-authorizing"
        );
    }

    #[test]
    fn working_memory_contains_improvement_candidates() {
        let result = run_cognitive_work_cycle(
            "Prepare a local business outreach plan for ARPAGONA",
            None,
            None,
        );
        let wm = &result.working_memory;
        assert!(
            !wm.improvement_candidates.is_empty(),
            "WorkingMemory must contain improvement_candidates"
        );
        assert_eq!(
            wm.improvement_candidates.len(),
            result.improvement_candidates.len(),
            "WorkingMemory improvement_candidates must match top-level"
        );
    }

    #[test]
    fn working_memory_contains_required_observations() {
        let result = run_cognitive_work_cycle(
            "Prepare a local business outreach plan for ARPAGONA",
            None,
            None,
        );
        let wm = &result.working_memory;
        assert!(
            !wm.required_observations.is_empty(),
            "WorkingMemory must contain required_observations"
        );
        assert_eq!(
            wm.required_observations.len(),
            result.required_observations.len(),
            "WorkingMemory required_observations must match top-level"
        );
    }

    #[test]
    fn working_memory_has_cycle_status() {
        let result = run_cognitive_work_cycle("Test objective", None, None);
        assert_eq!(
            result.working_memory.cycle_status,
            CycleStatus::Completed,
            "After full cycle, status must be Completed"
        );
    }

    #[test]
    fn working_memory_has_evidence_only_warning() {
        let result = run_cognitive_work_cycle("Test", None, None);
        assert!(
            result
                .working_memory
                .evidence_only_warning
                .contains("Readback only"),
            "WorkingMemory must carry evidence_only_warning"
        );
    }

    #[test]
    fn working_memory_failure_insight_candidates_empty_without_assess() {
        let result = run_cognitive_work_cycle("Test", None, None);
        assert!(
            result.working_memory.failure_insight_candidates.is_empty(),
            "failure_insight_candidates must be empty when --assess is not used"
        );
    }

    #[test]
    fn working_memory_can_be_built_via_builder() {
        let obj = Objective {
            id: ObjectiveId::new("obj-builder-test"),
            title: "Builder test".to_owned(),
            description: "Testing build_working_memory_cycle".to_owned(),
            domain: ObjectiveDomain::General,
            status: ObjectiveStatus::Proposed,
            success_criteria: vec![],
            created_at: Utc::now(),
        };
        let context = vec![ContextItem {
            key: "region".to_owned(),
            value: "Paris".to_owned(),
            source: "user".to_owned(),
        }];

        let wm = build_working_memory_cycle(
            Some(obj.clone()),
            Some(context.clone()),
            None, // no observations
            None, // no improvement candidates
            None, // no failure insight candidates
        );

        assert!(
            wm.objective.is_some(),
            "WorkingMemory from builder must contain objective"
        );
        assert_eq!(wm.objective.unwrap().id, obj.id);
        assert_eq!(wm.context_items.len(), 1);
        assert_eq!(wm.context_items[0].key, "region");
        assert!(
            wm.cognitive_observations.is_empty(),
            "No observations provided, must be empty"
        );
        assert!(
            wm.improvement_candidates.is_empty(),
            "No improvement candidates provided, must be empty"
        );
        assert!(
            wm.failure_insight_candidates.is_empty(),
            "No failure insight candidates provided, must be empty"
        );
        assert!(
            wm.proposed_next_action.is_none(),
            "No proposed next action provided, must be None"
        );
        assert_eq!(wm.cycle_status, CycleStatus::Completed);
        assert!(
            wm.evidence_only_warning.contains("Readback only"),
            "Builder must include evidence_only_warning"
        );
    }

    #[test]
    fn working_memory_builder_accepts_candidates() {
        let obj = Objective {
            id: ObjectiveId::new("obj-builder-candidates"),
            title: "Builder with candidates".to_owned(),
            description: "Testing full builder".to_owned(),
            domain: ObjectiveDomain::Business,
            status: ObjectiveStatus::Proposed,
            success_criteria: vec![],
            created_at: Utc::now(),
        };
        let improvement = vec![ImprovementCandidate {
            id: "improve-1".to_owned(),
            kind: ImprovementCandidateKind::MissingContext,
            description: "Missing context detected".to_owned(),
            rationale: "Test rationale".to_owned(),
        }];
        // Create a FailureInsightCandidate directly via the bridge
        let fic =
            crate::observation::FailureInsightCandidate::from_improvement_candidates(&improvement);

        let wm =
            build_working_memory_cycle(Some(obj), None, None, Some(improvement.clone()), Some(fic));

        assert_eq!(wm.improvement_candidates.len(), 1);
        assert_eq!(
            wm.improvement_candidates[0].kind,
            ImprovementCandidateKind::MissingContext
        );
        assert_eq!(wm.failure_insight_candidates.len(), 1);
        assert_eq!(
            wm.failure_insight_candidates[0].kind,
            crate::observation::FailureInsightCandidateKind::MissingContext
        );
    }

    #[test]
    fn working_memory_does_not_authorize() {
        // Structural test: WorkingMemory carries no write/execute fields
        let wm = WorkingMemory {
            objective: None,
            context_items: vec![],
            assumptions: vec![],
            constraints: vec![],
            missing_context: vec![],
            required_observations: vec![],
            cognitive_observations: vec![],
            improvement_candidates: vec![],
            failure_insight_candidates: vec![],
            proposed_next_action: None,
            cycle_status: CycleStatus::Initialized,
            evidence_only_warning: COGNITIVE_READBACK_WARNING.to_owned(),
        };
        // Assert: no execution, no persistence, no decision gate fields exist
        assert_eq!(wm.cycle_status, CycleStatus::Initialized);
        assert!(wm.proposed_next_action.is_none());
        assert!(wm.evidence_only_warning.contains("not approval"));
    }

    #[test]
    fn working_memory_serializes_roundtrip_with_new_fields() {
        let wm = WorkingMemory {
            objective: Some(Objective {
                id: ObjectiveId::new("obj-serialize"),
                title: "Serialize test".to_owned(),
                description: "Roundtrip test".to_owned(),
                domain: ObjectiveDomain::General,
                status: ObjectiveStatus::Proposed,
                success_criteria: vec![],
                created_at: Utc::now(),
            }),
            context_items: vec![],
            assumptions: vec![],
            constraints: vec![],
            missing_context: vec![],
            required_observations: vec![],
            cognitive_observations: vec![],
            improvement_candidates: vec![],
            failure_insight_candidates: vec![],
            proposed_next_action: None,
            cycle_status: CycleStatus::Initialized,
            evidence_only_warning: COGNITIVE_READBACK_WARNING.to_owned(),
        };
        let json = serde_json::to_string(&wm).unwrap();
        let decoded: WorkingMemory = serde_json::from_str(&json).unwrap();
        assert!(
            decoded.objective.is_some(),
            "Objective must survive roundtrip"
        );
        assert_eq!(
            decoded.objective.as_ref().unwrap().id.as_str(),
            "obj-serialize"
        );
        assert_eq!(decoded.cycle_status, CycleStatus::Initialized);
        assert!(decoded.evidence_only_warning.contains("Readback only"));
    }
}
