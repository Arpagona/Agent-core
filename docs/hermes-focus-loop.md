# Hermes Focus Loop

This document defines the controlled Hermes/GONA focus loop used to operate the ARPAGONA Agent Core project.

The focus loop is an external project-management and development workflow. It is not part of the ARPAGONA Agent Core runtime, and it must not be interpreted as permission to add scheduler autonomy, tool execution or multi-agent autonomy inside ARPAGONA Agent Core.

## Mental Model

```text
cron / scheduler Hermes
↓
GONA se réveille
↓
relit l’état canonique du projet
↓
choisit UNE priorité
↓
travaille ou délègue
↓
teste
↓
met à jour GitHub / PROJECT_STATUS.md / Kanban
↓
résume
↓
s’arrête
```

The focus loop is not an infinite loop. It is a short, controlled and completed sequence.

One scheduled activation means one bounded, verifiable unit of progress.

## Source of Truth

GitHub remains the official source of truth.

The Hermes Kanban is an internal work tool only.

Source priority order:

1. `PROJECT_OBJECTIVES.md`
2. `PROJECT_STATUS.md`
3. `README.md`
4. `docs/roadmap.md`
5. `docs/architecture.md`
6. GitHub Issues
7. Hermes Kanban

If the Hermes Kanban conflicts with GitHub, `PROJECT_STATUS.md` or `PROJECT_OBJECTIVES.md`, the Kanban loses.

## Loop Responsibilities

On each activation, GONA must:

1. Read the canonical project state.
2. Check the GitHub repository state.
3. Check the local Git state.
4. Check relevant GitHub Issues.
5. Check active Hermes Kanban items.
6. Select one priority only.
7. Work on one bounded unit.
8. Delegate secondary tasks to Ollama agents only when useful.
9. Run the appropriate checks.
10. Update GitHub, `PROJECT_STATUS.md` or the Kanban when appropriate.
11. Produce a short report.
12. Stop.

## Required Loop Shape

Each focus loop must follow this shape:

```text
Observe → Decide → Plan → Act → Verify → Record → Push/Prepare PR → Report → Stop
```

The loop must not continue indefinitely, chain unrelated tasks, or produce activity only to satisfy the hourly schedule.

## Priority Rule

One focus loop may select only one main priority.

The current priority is to create and stabilize `crates/tool-registry` as a minimal declarative Rust crate without real tool execution.

If that work is complete or blocked, GONA must choose the next task from `PROJECT_STATUS.md` or GitHub Issues.

## Authorized Work

GONA may:

- create or select a GitHub Issue;
- create a dedicated branch;
- use Hermes Kanban for internal task decomposition;
- delegate secondary tasks to Ollama sub-agents;
- make bounded code or documentation changes;
- run `cargo fmt`, `cargo check` and `cargo test`;
- inspect `git status` and `git diff`;
- push a dedicated branch;
- prepare or open a Pull Request.

## Forbidden Work Without Human Validation

GONA must stop and ask for human validation before:

- pushing directly to `main`;
- merging a Pull Request;
- adding real tool execution;
- exposing shell access;
- adding scheduler autonomy inside ARPAGONA Agent Core;
- adding browser automation;
- adding MCP integration;
- handling secrets;
- modifying `PROJECT_OBJECTIVES.md`;
- changing the major roadmap;
- adding self-modification;
- adding real multi-agent autonomy inside ARPAGONA Agent Core;
- performing a large architectural refactor.

## Anti-Stacking Rule

If a dedicated branch has already been pushed for the current priority but no Pull Request has been created, reviewed or merged yet, GONA must not keep stacking commits on that branch unless one of the following is true:

1. a test failure must be fixed;
2. a clearly necessary correction has been identified;
3. a human explicitly asks GONA to continue on that branch.

Otherwise, GONA must produce a short report stating that the next required action is human review, Pull Request creation, or Pull Request merge, then stop.

GONA must not create a competing branch for the same priority.

GONA must not modify documentation merely to produce hourly activity.

A focus loop may end with no code changes when the best decision is to wait for human validation.

## Ollama Delegation Boundary

Ollama sub-agents may help with:

- file summaries;
- name suggestions;
- simple tests;
- documentation review;
- inconsistency detection;
- non-critical code drafts;
- small technical comparisons;
- non-critical cleanup suggestions.

Ollama sub-agents must not:

- decide architecture;
- change the roadmap;
- validate Pull Requests;
- touch secrets;
- modify `PROJECT_OBJECTIVES.md`;
- add execution capabilities;
- create major abstractions without GONA arbitration;
- push to GitHub without supervision.

Principle:

```text
Codex pense et arbitre.
Ollama produit, relit, résume et assiste.
GitHub tranche.
```

## Required Report Format

At the end of every focus loop, GONA must produce:

```text
Focus Loop Report

- Trigger:
- Issue:
- Branch:
- Work completed:
- Files changed:
- Tests run:
- Test result:
- PROJECT_STATUS.md updated: yes/no
- GitHub push: yes/no
- Pull Request: created/prepared/not created
- Blockers:
- Risks:
- Recommended next loop:
```

The report must be short, factual and actionable.

## Stop Conditions

GONA must stop when:

- a Pull Request is ready or opened;
- tests fail and require arbitration;
- a security or governance rule is encountered;
- an architectural uncertainty appears;
- the task exceeds the planned scope;
- a coherent unit of work has been completed;
- human validation is needed;
- continuing would only create noisy activity.

## Final Rule

The focus loop exists to create steady, auditable progress.

It must never be used as a justification for uncontrolled autonomy.

One loop. One priority. One branch. One verification. One report. Then stop.
