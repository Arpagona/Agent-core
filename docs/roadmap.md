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

État : première implémentation expérimentale disponible dans `crates/graph-memory`.

- Adapter SurrealDB.
- Modèle de persistance pour faits, sources, épisodes, observations et décisions.
- Requêtes de base et migrations initiales.

Inclus en V0 expérimentale :

- interface `GraphMemoryStore` ;
- adapter `SurrealGraphMemoryStore` ;
- migration `0001_graph_memory.surql` ;
- persistance de base pour `Fact`, `Source` et `AuditEvent` ;
- tests SurrealDB en mémoire.

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
