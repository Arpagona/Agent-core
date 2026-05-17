# Contexte agent — ARPAGONA Agent Core

Ce fichier est destiné aux futurs agents de codage. Il doit être relu au début de chaque session sur ce dépôt.

## Vision du projet

ARPAGONA Agent Core est un runtime agentique professionnel, local-first et sécurisé. Il vise à faire fonctionner des agents IA dans des environnements métier contrôlés, avec mémoire graphe, décisions traçables et supervision humaine.

Le projet n'est pas un chatbot, pas un orchestrateur libre et pas un framework d'automatisation dangereuse. La priorité est la gouvernance des actions.

Flux central non négociable :

```text
Agent -> ProposedAction -> DecisionGate -> Execution éventuelle -> Audit
```

Aucun agent ne doit exécuter directement une action.

## Architecture actuelle

Monorepo Rust avec :

- `crates/core` : types domaine purs et sérialisables ;
- `crates/graph-memory` : première couche de persistance SurrealDB expérimentale ;
- `docs/` : documentation d'architecture, roadmap, sécurité, ontologie et contexte.

`crates/core` définit notamment :

- `Fact`, `Source`, `Episode`, `Observation`, `GraphRef` ;
- `ProposedAction`, `Decision`, `AuditEvent` ;
- IDs typés (`FactId`, `SourceId`, `WorkspaceId`, etc.) ;
- risques, permissions, politiques, tâches, objectifs, agents et workspaces.

## Contraintes non négociables

`crates/core` doit rester pur :

- pas de SurrealDB ;
- pas d'Axum ;
- pas d'appel LLM ;
- pas de shell ;
- pas d'exécution d'outils ;
- pas de secrets ;
- pas de logique runtime agentique.

Les couches runtime futures doivent respecter le flux `ProposedAction -> DecisionGate -> Audit`.

Ne pas créer en avance :

- Mission Control ;
- API Axum ;
- orchestrateur ;
- intégration LLM ;
- registre d'outils exécutable ;
- shell libre.

## État des briques

- Brique 1 — Fondation core : commencée. Le crate `crates/core` compile et contient les types fondamentaux.
- Brique 2 — Graph Memory : première implémentation expérimentale ajoutée dans `crates/graph-memory`.
- Brique 3 — Decision Gate : pas encore implémentée. Ne pas la démarrer sans demande explicite.
- Briques suivantes — Tool Registry, API Server, Mission Control, Orchestrator, Workers d'ingestion : prévues mais non implémentées.

## Décisions techniques prises

- Graph Memory est séparée de `crates/core` pour garder le domaine indépendant de SurrealDB.
- Le crate `crates/graph-memory` dépend de `arpagona-agent-core`, `surrealdb`, `serde`, `serde_json`, `chrono`, `thiserror`, `async-trait` et `tokio` pour les tests.
- `GraphMemoryStore` définit une interface async minimale.
- `SurrealGraphMemoryStore` est l'adapter SurrealDB.
- La migration initiale est `crates/graph-memory/migrations/0001_graph_memory.surql`.
- Les documents SurrealDB stockent les structs core dans un champ `data`, avec quelques champs dupliqués pour les requêtes et index (`entity_type`, `entity_id`, `workspace_id`, `created_at`).
- Les tests utilisent SurrealDB en mémoire.

## Prochaines priorités recommandées

1. Stabiliser la Graph Memory V0 après revue.
2. Ajouter des méthodes pour `Episode`, `Observation`, `ProposedAction` et `Decision` seulement quand un besoin concret apparaît.
3. Clarifier les relations graphe et les requêtes nécessaires aux décisions.
4. Implémenter ensuite la Brique 3 — Decision Gate, en gardant toute exécution réelle hors de portée des agents.
