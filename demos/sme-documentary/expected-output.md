# Expected Output — SME Documentary Assistant Demo

This file documents the expected output structure and key indicators for the E1 demo.
Actual output may vary due to runtime state, but the structural invariants below must hold.

## Phase 1: Tool Runtime — Document Discovery

### 1.1 — List available sample documents

```
  Found 3 sample documents:
    • client-brief.md
    • project-requirements.md
    • commercial-proposition.md
```

### 1.2 — Read client brief

```
  File: Brief client — Projet de refonte site e-commerce
  Lines: ~50
  Size: ~1600 chars
  Status: completed
```

### 1.3 — Read project requirements

```
  Lines: ~50
  Size: ~1500 chars
  Headers found:
      # Spécifications fonctionnelles — Projet Artisans du Sud
      ## 1. Fonctionnalités e-commerce
      ### 1.1 Catalogue produits
      ### 1.2 Panier et commande
      ### 1.3 Gestion des stocks multi-atelier
      ## 2. Fonctionnalités de personnalisation
      ## 3. Programme de fidélité
      ## 4. Contraintes techniques
```

### 1.4 — Search for budget-related keywords

```
  Found 6 budget-related matches across documents
```

## Phase 2: Cognitive Analysis

### 2.1 — Cognitive analysis (business domain)

```
  Objective: Évaluer la faisabilité du projet e-commerce Artisans du Sud ...
  Domain:    Business
  Sensitivity: Confidential
  Complexity: Moderate

  Plan steps (4):
    1. Analyser le brief client pour identifier le périmètre fonctionnel
    2. Évaluer la faisabilité technique du projet
    3. Analyser les risques et contraintes
    4. Proposer un plan d'action recommandé

  Proposed next action:
    Kind: RequestContext
    Description: Demander des informations complémentaires sur les compétences techniques internes...
    Non-authorizing: true

  Improvement candidates:
    • Missing context: compétences techniques internes
    • Missing context: hébergement actuel
```

## Phase 3: Governed Analysis Pipeline

### 3.1 — Full governed pipeline (business domain)

```
  decision_count:      1
  audit_event_count:   1
  assessed:            true
  governed:            true
  cognitive_observations: 2

  ✔ Governance chain produced decisions and audit events

  ProposedAction:   AssessAndImprove (risk: low)
  Decision:         approved (id: decision-...)
  AuditEvent:       decision (after: audit-...)
```

## Phase 4: Operator Readback Surfaces

### 4.1 — System status readback

```
  API health: unavailable (error sending request for url...
  Decision Gate: enabled
  CLI version: 0.1.0
  LLM provider: ollama (default)
  Tool count: 3
  Available tools:
    • read_file — Perception / Inspection
    • list_files — Perception
    • search_text — Inspection
```

### 4.2 — LLM interaction journal (mock mode)

```
  ℹ No LLM journal entries yet — run with --llm to populate
```

### 4.3 — (Optional) LLM-assisted run with mock provider

```
  Synthesis: [STATE] SME Documentary Assistant Demo
  [KEY GAP / RISK] ... [RECOMMENDED NEXT STEP] ...
```

### 4.4 — LLM journal readback (now populated)

```
  LLM journal entries: 1
    [1] Provider: mock, Model: mock-provider
        Prompt: 1200 chars, Response: 400 chars
```
