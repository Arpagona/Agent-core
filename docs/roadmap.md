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

- Évaluation des `ProposedAction`.
- Application des politiques.
- Production de décisions.
- Journalisation audit.

## Brique 4 — Tool Registry

- Description déclarative des outils.
- Permissions requises.
- Simulation en V0 avant exécution réelle.

## Brique 5 — API Server

- Serveur Axum.
- Endpoints REST initiaux.
- Consultation des tâches, actions proposées, décisions et audit.

## Brique 6 — Mission Control

- Next.js + TypeScript.
- Dashboard de supervision.
- Validation humaine des actions sensibles.

## Brique 7 — Orchestrator

- Coordination des agents.
- Cycle tâche / objectif / proposition d'action.
- Abstraction des providers LLM.

## Brique 8 — Workers d'ingestion

- Ingestion documentaire.
- Extraction de sources, observations et faits.
- Raccordement contrôlé à Graph Memory.
