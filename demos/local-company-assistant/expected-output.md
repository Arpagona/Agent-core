# Expected Output — Local Company Assistant Demo

This document describes the output a successful E3 demo run produces. Use it as a reference for commercial conversations, acceptance testing, and CI validation.

## Full Run Summary

A successful `bash demos/local-company-assistant/demo.sh` prints:

```
  ARPAGONA E3 — Local Company Assistant
  Demo: Boulangerie du Marché (Lyon)


  --- Phase 1 — Outils et découverte ---
    OK Tool Runtime accessible
    OK Liste fichiers: ~13 lignes
    Documents:
    • feedback-customers.md (69 lignes)
    • operations-snapshot.md (38 lignes)
    • staff-suggestions.md (48 lignes)

  --- Phase 2 — Analyse cognitive ---
    Domaine: "domain": "business"
    Action proposée: "kind": "policy"
    OK Analyse cognitive terminée

  --- Phase 3 — Lecture documents ---
    OK Lecture feedback-customers.md
    OK Lecture operations-snapshot.md
    OK Lecture staff-suggestions.md
    OK Recherche budget: ~2 mentions

  --- Phase 4 — Pipeline gouverné ---
    OK Décisions: 1
    OK Événements d'audit: 1
    OK Avertissement gouvernance présent

  --- Phase 5 — Lecture opérateur ---
    OK Statut système: 3 outils

    Rapport d'audit:
    • 1 décisions, 1 événements d'audit
    • 3 documents, 3 outils, 0 effets externes
    • Toutes les actions sont non-autorisantes
```

## Acceptance Criteria

| # | Criterion | Expected Value |
|---|-----------|---------------|
| 1 | All phases pass | 0 failures across all tests |
| 2 | Phase 1 — Tool Runtime accessible | `read_file` found in tool list |
| 3 | Phase 2 — Domain classification | `"business"` |
| 4 | Phase 2 — Cognitive plan generated | `working_memory` present in JSON |
| 5 | Phase 3 — All 3 documents read | Each contains expected French text |
| 6 | Phase 3 — Budget search | ≥1 mention of `euros` |
| 7 | Phase 4 — Governance decisions | `decision_count > 0` |
| 8 | Phase 4 — Audit events | `audit_event_count > 0` |
| 9 | Phase 4 — Governance warning | `governance_warning` present |
| 10 | Phase 5 — Tool count | `tool_runtime_tool_count = 3` |

## Per-Phase Detailed Output

### Phase 1 — Tool Discovery
```
tool list --json → read_file, list_files, search_text available
tool demo list-files demos/local-company-assistant/samples → 3 .md files
```

### Phase 2 — Cognitive Analysis (JSON structure)
```json
{
  "objective": "Analyser les retours clients...",
  "domain": "business",
  "working_memory": { ... },
  "plan": [ { "step": "...", "order": 1 }, ... ],
  "proposed_next_action": { "kind": "policy", "non_authorizing": true },
  "improvement_candidates": [ ... ],
  "warning": "This output is non-authorizing..."
}
```

### Phase 3 — Tool Observations
Each `read-file` returns a `ToolExecutionResult` with:
- `observation` field containing file text
- `tool_use_rationale` / `cognitive_purpose`
- No file content from blocked paths (absolute, `.git`, `.env`)

### Phase 4 — Governance Pipeline (JSON structure)
```json
{
  "...governance pipeline output...": {
    "decision_count": 1,
    "audit_event_count": 1,
    "governance_warning": true
  }
}
```

### Phase 5 — Operator Readback
```
status --json → tool_runtime_tool_count: 3, no API server dependency
```

## Failure Modes

| Symptom | Likely Cause | Fix |
|---------|-------------|-----|
| `read_file` returns empty | Absolute path used instead of relative | Use path relative to project root |
| `tool list` fails | Binary not built | Run `cargo build` once |
| Phase 4 yields 0 decisions | `--assess` missing before `--govern` | Check the CLI flag order |
| `governance_warning` missing | Governance bridge produced 0 decisions | Check Decision Gate configuration |
