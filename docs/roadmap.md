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

État : démarrage par une abstraction pure Rust dans `crates/core`, avant l'adapter SurrealDB.

- Port synchrone `GraphMemoryStore` sans dépendance DB.
- Implémentation `InMemoryGraphMemoryStore` pour tests et développement du domaine.
- Stockage minimal de `Source`, `Fact`, `Episode`, `Observation`, `AuditEvent` et relations `GraphRelation` / `RelationType`.
- Requêtes de base, dont récupération des faits actifs par entité.

Sous-brique suivante :

- adapter `SurrealGraphMemoryStore` séparé du domaine core ;
- migration `0001_graph_memory.surql` ;
- persistance SurrealDB pour les mêmes objets et relations ;
- tests d'adapter avec SurrealDB en mémoire.

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
