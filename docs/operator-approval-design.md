# Operator Approval Design Study (D5)

**Status:** Design study — not implemented.
**Phase:** Track D, step D5 (AGENT_FOCUS_LOOP.md §8 D5).
**Last updated:** 2026-05-28

## 1. Purpose

This document defines the operator approval semantics for ARPAGONA Agent Core. It is a **design study only** — it specifies the required capabilities, audit requirements, risk thresholds and interaction patterns without authorizing implementation.

After this design is reviewed by GONA/Thibaud, the D5 crate/vocabulary may be implemented in a bounded, governed PR.

## 2. Motivation

The governed cognitive loop currently stops at `ProposedAction` and produces structured audit events, but no interactive operator approval mechanism exists outside the existing password-gated override engine.

The target experience is:

```text
Agent proposes an action (e.g., send an email, write a file, approve memory storage)
  → Operator inspects the proposal, context, risk level, rationale
  → Operator explicitly approves, rejects, overrides or requests retry
  → Decision Gate finalises the decision
  → Audit records every operator interaction
```

## 3. Non-negotiable boundaries

- Operator approval is **not** a bypass of the Decision Gate. The Decision Gate still evaluates every proposal.
- Operator approval does **not** grant ongoing authorisation. Every action requires its own inspection cycle.
- The operator **must not** be able to approve actions without seeing the full context (objective, proposal, risk, rationale).
- No hidden auto-approval. No silent escalation.
- All operator interactions produce audit events that cannot be silently deleted.

## 4. Operator actions

### 4.1 Inspect

**Description:** View a pending `ProposedAction` with its full context.

**Required fields:**
- `proposed_action_id`
- `action_type`, `target`, `payload`
- `risk_level`, `required_permissions`
- `rationale` (agent's reasoning)
- `workspace_id`, `task_id`, `agent_id`
- `created_at`
- `objective` or `working_memory` context (if available)
- `reservoir_tags` (tags that triggered the proposal, if available)
- `holographic_matches` (matching holographic traces, if available)
- `audit_event_ids` (causal links to upstream events)

**Security:** Inspect is read-only. It must not change state, create audit events, or act as implicit approval.

### 4.2 Approve

**Description:** Explicitly approve a `ProposedAction` for execution.

**Required preconditions:**
- The action has been evaluated by the Decision Gate and is in `NeedsHumanApproval` state.
- The operator has permission to approve actions of this risk level.

**Required fields:**
- `proposed_action_id`
- `operator_id` or operator identity
- `rationale` (operator's reason for approval)
- `timestamp`

**Result:**
- Decision becomes `Approved`.
- Audit event `DecisionCreated(ActionType::Approve)` is recorded.
- The runtime may proceed with execution (through Tool Runtime / MCP if applicable).

**Audit requirements:**
- Operator identity, rationale, risk level, action type and timestamp are all recorded.
- No secrets or passwords are logged.

### 4.3 Reject

**Description:** Explicitly reject a `ProposedAction`.

**Required preconditions:**
- The action is in `NeedsHumanApproval` state (or any revisable state).

**Required fields:**
- `proposed_action_id`
- `operator_id`
- `rationale` (why the action was rejected)
- `timestamp`

**Result:**
- Decision becomes `Rejected`.
- Audit event `DecisionCreated(ActionType::Reject)` is recorded.
- The proposal may optionally be used by Failure-to-Insight to generate a learning candidate.

**Policy considerations:**
- Rejected actions of type `WriteMemoryFact` or `CreateHolographicTrace` are safe candidates for Failure-to-Insight processing — they represent blocked memory growth.
- Rejected tool-call actions may indicate missing permissions or dangerous intent.

### 4.4 Override

**Description:** Override a Decision Gate block explicitly.

**Pre-existing mechanism:**
- The password-gated override engine already exists in `crates/decision-gate/src/override_engine.rs` with Argon2-or-DefaultHasher verification, TTL, lockout, and full audit trail.

**Additional requirements for D5:**
- Override must require explicit operator identity (not just a shared password).
- Override must log which operator performed the override, alongside the existing password verification.
- Override must be rate-limited (existing lockout covers this).
- Override durable audit events must survive process restart (covered by Graph Memory persistence when available).

### 4.5 Retry

**Description:** Ask the agent to re-evaluate or re-propose with revised constraints.

**Required fields:**
- `proposed_action_id`
- `operator_id`
- `modified_constraints` (optional: what should change in the next proposal)
- `rationale`
- `timestamp`

**Result:**
- The proposal is not executed or rejected outright.
- The runtime is asked to produce a new `ProposedAction` with the operator's constraints.
- Audit event `ProposedActionRetryRequested` is recorded.

**Risk:** Retry must not be used to bypass risk thresholds. If the original action was correctly blocked, retry with constraints should not produce a higher-risk action.

## 5. Risk thresholds for operator involvement

| Risk Level | Decision Gate default | Operator required? | Overrideable? |
|---|---|---|---|
| `None` | Auto-approved | No | N/A |
| `Low` | Approved | No (unless permission missing) | N/A |
| `Medium` | NeedsHumanApproval | Yes | Yes (safe with operator approval) |
| `High` | NeedsHumanApproval | Yes | Yes (audit required) |
| `Critical` | NeedsHumanApproval or Blocked | Yes | Yes (must be explicit with rationale) |

The existing Decision Gate already implements these thresholds. D5 adds the interactive operator surface.

## 6. Audit requirements for operator actions

Every operator interaction produces at least one audit event.

| Operator action | Audit event type | Minimum fields |
|---|---|---|
| Inspect | *No audit event (read-only)* | N/A |
| Approve | `DecisionCreated` | `action: Approve`, `operator_id`, `rationale`, `risk_level`, `proposed_action_id`, `timestamp` |
| Reject | `DecisionCreated` | `action: Reject`, `operator_id`, `rationale`, `risk_level`, `proposed_action_id`, `timestamp` |
| Override | `OverrideAttemptWithResult` | `action_type`, `operator_id`, `risk_level`, `result`, `timestamp` |
| Retry | `ProposedActionRetryRequested` | `operator_id`, `original_proposal_id`, `rationale`, `modified_constraints` (optional), `timestamp` |

### 6.1 Audit invariants

- Audit events are append-only. No event is deleted or modified after creation.
- Operator identity is logged as a stable handle (operator name, key ID, or HMAC identity), not a plain-text password.
- No audit event reveals the override password or its hash.
- Audit events are searchable by `proposed_action_id`, `operator_id`, `risk_level` and time range.

## 7. Interaction surfaces

### 7.1 CLI (primary, MVP)

Interactive CLI commands:

```text
arpagona operator inspect <PROPOSAL_ID> [--json]
arpagona operator approve <PROPOSAL_ID> --rationale "..." [--json]
arpagona operator reject <PROPOSAL_ID> --rationale "..." [--json]
arpagona operator override <PROPOSAL_ID> --password <pass> --rationale "..." [--json]
arpagona operator retry <PROPOSAL_ID> [--constraint "..."] [--rationale "..."] [--json]
```

Non-interactive status:

```text
arpagona operator list-pending [--risk-level <LEVEL>] [--json]
```

### 7.2 MCP resources (secondary)

MCP resources exposing pending proposals for external agent operators:

- `arpagona://operator/pending-proposals` — list pending actions
- `arpagona://operator/proposal/{id}` — full proposal detail

MCP prompts for summarised operator view:

- `summarize-pending-proposals` — human-readable pending items with risk levels
- `proposal-context` — full context for a specific proposal

### 7.3 Web Mission Control (deferred)

Web UI should display the same operator surface but is deferred until D1-D3 provide clear read-only contracts (per AGENT_FOCUS_LOOP.md §8 D4).

## 8. Relationship to existing components

| Component | Relationship to D5 |
|---|---|
| `crates/core/src/action.rs` | `ActionType::Approve`, `ActionType::Reject`, `ActionType::Override` — new action types needed |
| `crates/core/src/decision.rs` | `DecisionStatus` may need `NeedsHumanApproval` (already exists) and approval-specific metadata |
| `crates/decision-gate` | Override engine already exists; needs operator identity integration for D5 |
| `crates/runtime` | Needs an operator interaction bridge that pauses on `NeedsHumanApproval` |
| `crates/cli` | New `operator` command group with inspect/approve/reject/override/retry/status |
| `crates/audit` (core types) | New `AuditEventVariant` entries for operator actions |
| `crates/graph-memory` | Persistent storage of operator decision events |
| `crates/holographic-memory` | Operator decisions and retry patterns may be stored as holographic traces for future resonance |

## 9. Non-goals (explicitly excluded from D5)

- No interactive GUI or web-based approval panel (deferred to D4/Mission Control).
- No asynchronous approval (email-based, push-based, scheduled). Every approval is synchronous CLI interaction.
- No multi-operator approval workflows (requires orchestration beyond D5).
- No agent self-approval. No automated approval of high/critical risk actions.
- No delegation or proxy approval (one operator cannot act for another).
- No heartbeat or timeout-based implicit approval.

## 10. Open questions for GONA/Thibaud

1. **Operator identity model:** Should operator identity be a simple name string, a key ID, or a cryptographic identity (HMAC, SSH key)? The current override engine uses a shared password. Shared passwords are not operator identity.
2. **Persistent operator state:** Should there be a local operator profile file (`~/.arpagona/operator`) or is ephemeral CLI state sufficient for V0?
3. **Retry semantics:** When an operator requests retry, does the runtime automatically produce a new proposal, or does the operator manually trigger `cognitive run` again with modified context?
4. **Audit storage:** Should operator audit events be durable (survive restart) from day one, or is in-memory journaling sufficient for alpha?
5. **Override audit requirement:** Should each override produce a mandatory *reason* that goes into the audit trail, or can it be optional?

## 11. Implementation order (recommended)

After this design is reviewed:

1. **Vocabulary only** — Add `ActionType::Approve`, `Reject`, `Override`, `Retry` to `crates/core` action types + new `AuditEventVariant` entries.
2. **Decision Gate extension** — Wire the new action types into the Decision Gate's evaluation flow so it can recognise and route them.
3. **CLI operator command** — Add `arpagona operator` command group with inspect and list-pending (read-only).
4. **CLI approve/reject/override** — Add approve/reject/override with password (reuses override engine).
5. **CLI retry** — Add retry with constraint propagation.
6. **Tests** — Full coverage for all 5 operator actions, including edge cases (unknown proposal ID, wrong risk level, missing rationale, lockout while overriding).
