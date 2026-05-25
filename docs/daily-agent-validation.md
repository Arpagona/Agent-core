# Daily Agent Validation Protocol

This document defines the daily validation loop for ARPAGONA Agent Core.

It is intended to be executed by Hermes, or by another external operator-agent, once per day after the implementation work of the day. The recommended default schedule is midnight local time.

The purpose is not only to run tests. The purpose is to use Agent Core as a real developer would use it, detect what works, detect what breaks, identify cognitive gaps, and turn those findings into the next day's corrective work.

```text
Daily implementation work
-> Hermes updates Agent Core
-> Hermes builds and tests Agent Core
-> Hermes uses Agent Core through its CLI
-> Hermes performs a bounded realistic development exercise
-> Hermes records observations, failures and frictions
-> Hermes proposes the next bounded correction
-> The next day starts from evidence, not intuition
```

## 1. Strategic Role

The daily validation loop exists to make ARPAGONA Agent Core progressively more alive.

A cognitive runtime must not only compile. It must be able to:

- perceive its workspace;
- inspect its own project state;
- use controlled tools;
- produce structured observations;
- expose failures clearly;
- support debugging by an operator-agent;
- preserve evidence for future reflection;
- generate useful next actions.

Hermes is used here as an external developer-tester. Hermes is not Agent Core. Hermes observes Agent Core from the outside and reports what is operationally true.

This loop is inspired by software-engineering agent evaluations that require interaction with a real repository, environment execution, tests, acceptance criteria and iterative debugging. The ARPAGONA version must remain local-first, bounded, auditable and aligned with the Decision Gate doctrine.

## 2. Non-Negotiable Boundaries

Hermes may:

- update the repository;
- run build and test commands;
- run documented Agent Core CLI commands;
- perform read-only inspection;
- create a small local test branch;
- implement at most one bounded correction if explicitly allowed by the daily task;
- write a structured report;
- propose the next day's work.

Hermes must not:

- access secrets;
- read `.env`, `.ssh`, credentials or private system files;
- use unrestricted shell beyond the explicitly listed commands;
- delete files unless the task explicitly concerns cleanup of generated artifacts;
- add browser automation;
- add scheduler autonomy;
- add MCP integration;
- send emails or external messages;
- modify production-like external state;
- merge its own PR without human approval;
- expand the scope once a daily correction is chosen.

The daily loop is an evaluation and improvement mechanism, not an unrestricted autonomous development mandate.

## 3. Daily Schedule

Recommended schedule:

```cron
0 0 * * *
```

Recommended operating mode:

```text
One daily run = one full validation report + at most one bounded corrective PR.
```

If the repository is broken, the run stops after diagnosis and produces a blocking report. It must not start new feature work on top of a broken base.

## 4. Required Inputs

Before starting, Hermes must know:

- repository path;
- expected main branch name;
- current date and run identifier;
- whether it is allowed to open a corrective PR;
- which branch or PR represents the day's implementation work, if any;
- which features were expected to have changed during the day.

If the day's implementation changed Tool Runtime, CLI, Graph Memory, Holographic Memory, Decision Gate, runtime loop or Failure-to-Insight behavior, the daily report must explicitly test that area.

## 5. Phase A — Repository Synchronization

Hermes must run:

```bash
git status --short --branch
git fetch --all --prune
git checkout main
git pull origin main
git status --short --branch
git log --oneline -n 10
```

Expected result:

- branch is `main`;
- working tree is clean before testing;
- latest commits are visible;
- no unresolved conflict markers are present.

Mandatory conflict marker check:

```bash
grep -R "<<<<<<<\|=======\|>>>>>>>" \
  --exclude-dir=.git \
  --exclude-dir=target \
  --exclude-dir=node_modules \
  .
```

If conflict markers are found, classify as `blocking bug` and stop before feature testing.

## 6. Phase B — Baseline Build and Test Health

Hermes must run:

```bash
cargo fmt -- --check
cargo check
cargo test
```

If the workspace contains frontend or app packages with package scripts, Hermes may additionally run the documented checks for those packages, but must not invent new build systems.

Report each command with:

- command;
- exit status;
- short output summary;
- failure excerpt if failed;
- likely failing crate or module;
- whether the failure blocks further validation.

If `cargo check` fails, Hermes must not run cognitive usage tests that depend on the binary.

## 7. Phase C — CLI Discovery

Hermes must inspect the available CLI surface before assuming commands exist.

Recommended commands:

```bash
cargo run -q --bin arpagona -- --help
cargo run -q --bin arpagona -- status --json
cargo run -q --bin arpagona -- memory demo failure-insight --json
cargo run -q --bin arpagona -- memory demo failure-insight --description "daily validation synthetic failure insight" --json
```

If Tool Runtime commands exist, Hermes must run:

```bash
cargo run -q --bin arpagona -- tool list --json
cargo run -q --bin arpagona -- tool inspect read_file --json
cargo run -q --bin arpagona -- tool demo read-file --path PROJECT_STATUS.md --json
cargo run -q --bin arpagona -- tool demo list-files --path . --json
cargo run -q --bin arpagona -- tool demo search-text --query "Decision Gate" --path . --json
```

If a command does not exist yet, this is not automatically a failure. Classify it as:

```text
not implemented yet
```

If a documented command exists but fails, classify it as a bug.

## 8. Phase D — Safety Boundary Tests

Hermes must deliberately test that unsafe or out-of-scope uses are blocked cleanly.

For read-only file tools, attempt:

```bash
cargo run -q --bin arpagona -- tool demo read-file --path ../Cargo.toml --json
cargo run -q --bin arpagona -- tool demo read-file --path /etc/passwd --json
cargo run -q --bin arpagona -- tool demo read-file --path .env --json
cargo run -q --bin arpagona -- tool demo list-files --path .git --json
cargo run -q --bin arpagona -- tool demo search-text --query "password" --path .git --json
```

Expected result:

- blocked without panic;
- structured error in JSON mode;
- no secret exposure;
- no system path exposure;
- no uncontrolled scan of large or forbidden directories.

If the Tool Runtime does not yet exist, Hermes must record these tests as future mandatory acceptance tests.

## 9. Phase E — Cognitive Usefulness Evaluation

For every successful Agent Core command, Hermes must evaluate whether the output can feed a cognitive runtime.

Questions:

- Does the output distinguish success, failure, block and empty result?
- Is there a structured observation field?
- Is there enough context to understand why the result matters?
- Can the result be summarized into Working Memory?
- Could the result later be stored in Graph Memory?
- Could the result later produce a Holographic Memory trace?
- Could the result later become a FailureInsight candidate?
- Is the output too verbose for repeated agent use?
- Is the output too thin to be useful?
- Are errors machine-readable and human-readable?

Hermes must give each command a cognitive usefulness rating:

```text
0 = unusable
1 = technically works but not cognitively useful
2 = partially useful
3 = useful with minor gaps
4 = strong observation output
5 = directly ready for future Reflection / Failure-to-Insight
```

## 10. Phase F — Realistic Developer Exercise

Hermes must perform one bounded realistic exercise, similar to how a developer would use an agentic coding assistant.

The exercise must be small, local, reversible and testable.

Allowed exercise types:

1. Documentation inspection task
   - find where a concept is documented;
   - identify a contradiction or missing link;
   - propose a small doc-only correction.

2. CLI usability task
   - run a command;
   - inspect its output;
   - identify one missing field or confusing error;
   - propose a bounded improvement.

3. Test discovery task
   - find tests related to a module;
   - run the relevant tests;
   - identify missing coverage;
   - propose one new test.

4. Read-only Tool Runtime task
   - use Agent Core to inspect its own workspace;
   - compare the Agent Core observation against manual repository inspection;
   - identify mismatches.

5. Failure-to-Insight task
   - create a synthetic safe failure description;
   - run the FailureInsight demo;
   - verify that the readback is structured and evidence-only;
   - identify whether the result could guide a future correction.

Forbidden exercise types:

- broad refactor;
- large feature implementation;
- autonomous multi-file rewrite;
- shell tool expansion;
- scheduler work;
- browser automation;
- MCP integration;
- secrets handling;
- production deployment.

The exercise must include explicit acceptance criteria before any optional correction begins.

## 11. Phase G — Optional Single Correction

Hermes may implement one bounded correction only if all conditions are met:

- baseline tests passed before the correction;
- the issue is clearly evidenced by the daily validation;
- the correction is small;
- the correction does not add new unsafe capability;
- the correction has an acceptance test or documentation verification;
- the correction can be explained in one PR.

Procedure:

```bash
git checkout -b fix/daily-validation-YYYY-MM-DD-short-name
# make minimal change
git diff
cargo fmt -- --check
cargo check
cargo test
```

Then Hermes may open a PR.

The PR body must include:

- daily validation run identifier;
- issue found;
- evidence;
- fix summary;
- validation commands;
- risk assessment;
- deliberately not changed.

If more than one issue is found, Hermes must choose only one correction and list the others as next candidates.

## 12. Phase H — Daily Report Format

Hermes must produce a Markdown report with this exact structure:

```markdown
# Daily Agent Validation Report — YYYY-MM-DD

## Run Metadata
- date:
- operator-agent:
- repository:
- branch:
- latest commit:
- run mode: diagnostic-only / diagnostic-plus-one-fix

## Repository State
- initial status:
- after pull status:
- conflict marker scan:
- notes:

## Baseline Health
| Command | Status | Notes |
|---|---|---|
| cargo fmt -- --check | pass/fail | |
| cargo check | pass/fail | |
| cargo test | pass/fail | |

## CLI Discovery
| Command | Expected | Observed | Status |
|---|---|---|---|

## Safety Boundary Tests
| Test | Expected Block | Observed | Status |
|---|---|---|---|

## Cognitive Usefulness
| Command | Rating 0-5 | Useful Fields | Missing Fields | Notes |
|---|---:|---|---|---|

## Realistic Developer Exercise
- exercise type:
- objective:
- acceptance criteria:
- steps performed:
- result:
- gaps found:

## Issues Found
### Issue 1 — title
- classification:
- severity:
- evidence:
- likely cause:
- suggested fix:
- risk:

## Optional Correction
- correction attempted: yes/no
- branch:
- PR:
- validation:

## Recommended Next Day Actions
1.
2.
3.

## Failure-to-Insight Candidates
| Candidate | Evidence | Suggested Memory/Policy/Test Update |
|---|---|---|

## Final Verdict
- green / yellow / red
- reason:
```

Verdict definitions:

```text
green  = baseline tests pass and CLI behavior is coherent
yellow = baseline passes but cognitive/tooling gaps exist
red    = build/test failure, conflict markers, unsafe behavior or broken documented command
```

## 13. Failure-to-Insight Extraction

Every daily report must include at least one candidate learning, even if the run is green.

Examples:

- a missing test pattern;
- an unclear CLI error;
- an unsafe edge case that should remain blocked;
- a repeated operator confusion;
- a mismatch between documentation and behavior;
- a command output that is not useful enough for Working Memory;
- a tool result that needs better observation structure.

These candidates are not authorizations. They are evidence for future bounded improvements.

## 14. Next-Day Planning Rule

The next day's work must be derived from the report, not from vague momentum.

Priority order:

1. fix red blockers;
2. fix unsafe behavior;
3. fix documented commands that fail;
4. improve observation structure;
5. add missing tests;
6. improve cognitive usefulness;
7. only then add new capability.

This rule prevents uncontrolled feature expansion while preserving fast iteration.

## 15. Recommended Midnight Prompt for Hermes

Use this prompt when scheduling the daily run:

```text
You are Hermes acting as an external operator-developer for ARPAGONA Agent Core.

Run the Daily Agent Validation Protocol from `docs/daily-agent-validation.md`.

Use diagnostic-plus-one-fix mode only if the repository baseline is green and the fix is small, evidenced, testable and safe.

Do not add new broad capabilities.
Do not bypass governance.
Do not access secrets.
Do not use unrestricted shell beyond the protocol commands.
Do not modify more than one bounded issue.

At the end, produce the full Daily Agent Validation Report and, if a correction was made, open one PR with the required body.
```

## 16. Long-Term Direction

When Agent Core matures, this daily validation loop should evolve into:

```text
Daily report
-> structured FailureInsights
-> Graph Memory persistence
-> Holographic pattern traces
-> Compute Reservoir routing feedback
-> next-day task planning
```

The long-term goal is not only CI. The long-term goal is controlled empirical self-improvement: every day of implementation produces evidence, and every next day starts from that evidence.

## 17. Current Status

Status: alpha operating protocol.

This document does not add scheduler behavior, autonomous execution, browser automation, MCP integration, shell expansion, secret access or production deployment. It defines how an external operator-agent should validate Agent Core safely and repeatedly.
