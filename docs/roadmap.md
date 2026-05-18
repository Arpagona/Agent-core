# Roadmap

## Brique 1 — Fondation core

Objectif : poser une base saine et compilable.

Inclus :

- structure monorepo ;
- documentation fondatrice ;
- crate Rust `core` ;
- types fondamentaux sérialisables ;
- tests unitaires simples.

Exclus : API, UI, base de données, LLM, outils exécutables, scheduler actif.

## Brique 2 — Graph Memory

État : abstraction domaine pure Rust dans `crates/core` et adapter SurrealDB expérimental dans `crates/graph-memory`.

- Port synchrone canonique `GraphMemoryStore` sans dépendance DB.
- Implémentation `InMemoryGraphMemoryStore` pour tests et développement du domaine.
- Stockage minimal de `Source`, `Fact`, `Episode`, `Observation`, `AuditEvent` et relations `GraphRelation` / `RelationType`.
- Requêtes de base, dont récupération des faits actifs par entité.
- Adapter `SurrealGraphMemoryStore` séparé du domaine core.
- Port async d'adapter nommé `AsyncGraphMemoryStore`, distinct du contrat domaine.
- Migration `0001_graph_memory.surql` et tests d'adapter avec SurrealDB en mémoire.

Sous-brique suivante :

- stabilisation des conventions SurrealDB et relations graphe ;
- préparation de la Brique 3 — Decision Gate sans exécution directe par les agents.

## Brique 3 — Decision Gate

État : implémentation alpha minimale dans `crates/core`.

- Module pur Rust `decision_gate` sans API, LLM, I/O, shell ni exécution d'outils.
- Fonction `evaluate_proposed_action(action, policies, granted_permissions) -> Decision`.
- Règles alpha : permissions manquantes bloquées, risques `Informational` / `Low` approuvés sauf politique d'escalade, `Medium` en validation humaine, `High` / `Critical` en validation humaine ou blocage selon policy, `Custom` non explicitement autorisé en validation humaine.
- Helper `audit_event_for_decision(action, decision) -> AuditEvent` pour matérialiser le flux `ProposedAction -> DecisionGate -> Decision -> AuditEvent`.
- Documentation dédiée : `docs/decision-gate.md`.

Sous-brique suivante :

- API server minimal : endpoints de création/consultation des `ProposedAction`, évaluation par Decision Gate, consultation des `Decision` et `AuditEvent`, sans exécution d'outils.

## Brique 4 — Tool Registry

- Description déclarative des outils.
- Permissions requises.
- Simulation en V0 avant exécution réelle.

## Brique 5 — API Server

État : alpha minimale dans `apps/api-server`.

- Serveur Axum lançable avec `cargo run -p arpagona-api-server`.
- Stockage in-memory des `Task`, `ProposedAction`, `Decision` et `AuditEvent`.
- Endpoints REST initiaux : `health`, `tasks`, `proposed-actions`, `agent/propose`, `decision-gate/evaluate`, `decisions`, `audit`.
- Consultation du flux `Task -> ProposedAction -> DecisionGate -> Decision -> AuditEvent` sans shell, scheduler, outil exécutable ni SurrealDB obligatoire.
- Provider LLM expérimental limité à la proposition de `ProposedAction`.
- Documentation dédiée : `docs/api-server.md`.

## Brique 6 — LLM Provider / Agent Proposer

État : V0 expérimentale dans `crates/llm` et endpoint `POST /agent/propose`.

- `LlmProvider` abstrait.
- `MockProvider` pour tests et démos sans réseau.
- `OpenAiProvider` utilisant l'API Responses via `OPENAI_API_KEY`.
- `ProposedActionDraft` transformé en `ProposedAction` avec `PendingDecision`.
- Aucune exécution, aucun tool OpenAI, aucun appel automatique au Decision Gate.
- Documentation dédiée : `docs/llm-provider.md`.

## Brique 7 — Cognitive Runtime / Rippletide Layer

État : primitives domaine ajoutées dans `crates/core/src/cognitive.rs`.

Objectif : reconnecter l'alpha avec la vision initiale d'un mini système agentique Hermes-like amélioré par des couches cognitives explicites.

Inclus :

- `CognitiveLayer` : Input, WorkingMemory, ReservoirEcho, GraphMemory, AgentProposal, DecisionGate, HumanBoundary, Audit, Reflection, etc. ;
- `AgentLoopPhase` : ordre alpha-safe d'une boucle agentique ;
- `CognitivePulse` : signal court terme ;
- `ReservoirTrace` : trace d'écho avec activation et décroissance ;
- `ReservoirState` : réservoir court terme borné et déterministe ;
- `CognitiveCycleInput` ;
- `CognitiveCyclePlan::alpha_safe_default()`.

Contraintes :

- pure domain, pas d'I/O ;
- pas d'appel LLM ;
- pas de scheduler ;
- pas d'exécution ;
- le réservoir n'est pas une mémoire persistante ;
- Graph Memory reste responsable de la mémoire durable.

Documentation dédiée : `docs/cognitive-runtime.md`.

Sous-brique suivante recommandée :

- créer `crates/runtime` pour orchestrer une boucle V0 : `CognitiveCycleInput -> ReservoirState -> LlmProvider -> ProposedAction`, sans exécution directe.

## Brique 8 — Mission Control

- Next.js + TypeScript.
- Dashboard de supervision.
- Validation humaine des actions sensibles.

## Brique 9 — Orchestrator

- Coordination des agents.
- Cycle tâche / objectif / proposition d'action.
- Abstraction des providers LLM.
- Raccordement au Cognitive Runtime.

## Brique 10 — Workers d'ingestion

- Ingestion documentaire.
- Extraction de sources, observations et faits.
- Raccordement contrôlé à Graph Memory.
