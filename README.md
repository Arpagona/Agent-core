# ARPAGONA Agent Core

ARPAGONA Agent Core est un runtime agentique généraliste, local-first, conçu pour faire fonctionner des agents IA dans un environnement professionnel contrôlé.

Ce projet n'est pas un simple chatbot, ni un fork de framework existant, ni un script d'automatisation fragile. Il vise à fournir un noyau agentique professionnel, gouverné par graphe, où les agents peuvent raisonner et proposer des actions sans jamais les exécuter directement.

## Principes fondateurs

- Aucun agent n'agit directement.
- Un agent propose une action.
- Le Decision Gate évalue chaque action proposée.
- L'action est autorisée, bloquée ou soumise à validation humaine.
- Toute décision est enregistrée dans le graphe.
- Toute action sensible requiert une approbation humaine.
- Aucun secret n'est exposé au LLM.
- Aucun shell libre n'est disponible en V0.

En bref :

```text
Agent -> ProposedAction -> DecisionGate -> Execution éventuelle -> Audit
```

## Stack cible

- Backend : Rust
- Framework backend : Axum, plus tard
- Frontend : Next.js + TypeScript, plus tard
- Dashboard : Mission Control, plus tard
- Base principale : SurrealDB
- API : REST en V0, plus tard
- WebSocket : prévu plus tard
- Architecture : monorepo

## Structure du monorepo

```text
arpagona-agent-core/
  README.md
  Cargo.toml
  docs/
    architecture.md
    ontology.md
    security-model.md
    roadmap.md
  crates/
    core/
      Cargo.toml
      src/
        lib.rs
        ids.rs
        agent.rs
        workspace.rs
        task.rs
        goal.rs
        action.rs
        decision.rs
        policy.rs
        tool.rs
        memory.rs
        graph.rs
        audit.rs
        risk.rs
        permission.rs
        episode.rs
        source.rs
        errors.rs
    graph-memory/
      Cargo.toml
      src/
        lib.rs
      migrations/
        0001_graph_memory.surql
  apps/
    mission-control/
      README.md
  workers/
    python-ingestion/
      README.md
```

## État actuel

Les premières briques fournissent :

- la structure monorepo ;
- la documentation fondatrice ;
- le crate Rust `crates/core` ;
- des types fondamentaux sérialisables ;
- un crate expérimental `crates/graph-memory` pour persister la mémoire graphe dans SurrealDB ;
- une migration SurrealDB initiale pour faits, sources, épisodes, observations, audit, décisions et actions proposées ;
- quelques tests unitaires de base.

Le crate `core` est volontairement limité à des types purs et réutilisables. Il ne contient ni logique LLM, ni logique SurrealDB, ni API, ni exécution d'outils.

Le crate `graph-memory` porte l'adapter SurrealDB séparé du domaine core. Il expose une interface `GraphMemoryStore` et une implémentation `SurrealGraphMemoryStore` testée avec SurrealDB en mémoire.

## Compiler et tester

Depuis la racine du projet :

```bash
cargo fmt
cargo check
cargo test
```

La documentation de la mémoire graphe se trouve dans `docs/graph-memory.md`.

## Volontairement non implémenté en V0

- Pas d'interface web réelle.
- Pas de serveur Axum.
- Pas de connexion SurrealDB dans `crates/core`.
- Pas d'appel LLM.
- Pas de shell libre.
- Pas d'envoi email réel.
- Pas de registre d'outils exécutable.
- Pas de scheduler actif.
- Pas de worker d'ingestion fonctionnel.
- Pas de CLI complexe.

Cette fondation existe pour stabiliser le modèle conceptuel avant d'ajouter les couches runtime, sécurité, API et UI.
