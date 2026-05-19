# ARPAGONA Agent Core

ARPAGONA Agent Core est un runtime agentique généraliste, local-first, conçu pour faire fonctionner des agents IA dans un environnement professionnel contrôlé.

Ce projet n'est pas un simple chatbot, ni un fork de framework existant, ni un script d'automatisation fragile. Il vise à fournir un noyau agentique professionnel, gouverné par graphe, où les agents peuvent raisonner et proposer des actions sans jamais les exécuter directement.

## Documents canoniques

Avant toute modification du dépôt, un contributeur humain ou agentique doit lire :

- `PROJECT_OBJECTIVES.md` : vision canonique du projet, objectifs fondateurs et principes non négociables ;
- `PROJECT_STATUS.md` : état opérationnel courant, niveau de stabilité des briques, risques architecturaux, stop-list et prochaine séquence recommandée.

L'objectif immédiat du projet est la consolidation architecturale, pas l'ajout de fonctionnalités visibles. Plusieurs briques existent déjà en alpha ou en expérimentation ; elles ne doivent pas être interprétées comme stables ni comme une autorisation d'étendre l'autonomie du système.

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
  PROJECT_OBJECTIVES.md
  PROJECT_STATUS.md
  Cargo.toml
  docs/
    architecture.md
    ontology.md
    security-model.md
    roadmap.md
    compute-reservoir.md
    tool-registry.md
  crates/
    compute-reservoir/
      Cargo.toml
      src/
        lib.rs
    decision-gate/
      Cargo.toml
      src/
        lib.rs
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
    llm/
      Cargo.toml
      src/
        lib.rs
    runtime/
      Cargo.toml
      src/
        lib.rs
    tool-registry/
      Cargo.toml
      src/
        lib.rs
    cli/
      Cargo.toml
      src/
        main.rs
  apps/
    api-server/
      Cargo.toml
      src/
        main.rs
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
- un Decision Gate alpha extrait dans `crates/decision-gate` ;
- un crate alpha minimal `crates/compute-reservoir` pour déclarer des ressources compute et recommander une allocation via une fonction pure ;
- un crate alpha minimal `crates/tool-registry` pour déclarer un catalogue d'outils, de capacités, de schémas, de permissions, de risques et d'états sans exécution ;
- des primitives de Cognitive Runtime incluant le Reservoir Echo court terme ;
- un crate expérimental `crates/graph-memory` pour persister la mémoire graphe dans SurrealDB ;
- un crate expérimental `crates/llm` pour transformer une demande utilisateur en `ProposedAction` pending, sans exécution ;
- un crate expérimental `crates/runtime` pour une boucle cognitive qui s'arrête à la proposition d'action ;
- une application alpha `apps/api-server` ;
- une interface terminal alpha `crates/cli` ;
- une migration SurrealDB initiale pour faits, sources, épisodes, observations, audit, décisions et actions proposées ;
- quelques tests unitaires de base.

Le crate `core` doit rester limité au vocabulaire domaine et aux types purs réutilisables. Il ne doit pas devenir un fourre-tout. Le Decision Gate est désormais dans `crates/decision-gate` et le Compute Reservoir minimal dans `crates/compute-reservoir`.

Le crate `compute-reservoir` est alpha minimal : il fournit des types sérialisables et `allocate_compute`, une fonction pure d'allocation de ressources selon capacités, coût, latence, confidentialité et fallback. Il ne fait aucun appel modèle, aucune exécution, aucun I/O, et ne remplace jamais le Decision Gate.

Le crate `tool-registry` est alpha minimal : il fournit un catalogue déclaratif en mémoire pour les définitions d'outils, les capacités, les schémas, les permissions, les risques et les états. Il permet d'enregistrer, chercher, lister et désactiver des déclarations d'outils. Il ne contient aucun runner, hook d'exécution, shell, scheduler, MCP, navigateur ou runtime multi-agent.

Le crate `graph-memory` porte l'adapter SurrealDB séparé du domaine core. La source de vérité domaine est le trait synchrone `GraphMemoryStore` dans `crates/core`; l'adapter persistant expose `SurrealGraphMemoryStore` et un port async expérimental `AsyncGraphMemoryStore`, testés avec SurrealDB en mémoire.

Le crate `llm` porte les providers expérimentaux. Même lorsqu'un provider OpenAI est utilisé, il ne peut produire qu'un `ProposedActionDraft`, ensuite matérialisé en `ProposedAction` avec le statut `pending_decision`. Aucun outil réel n'est exécuté et le Decision Gate reste obligatoire avant toute suite.

Le Runtime, l'API server et la CLI sont alpha. Ils doivent rester des surfaces d'expérimentation et de contrôle, pas des couches de gouvernance métier ni des chemins d'exécution alternatifs.

## Priorité immédiate

La priorité actuelle est de revenir à l'ordre architectural cible :

1. stabiliser les Core Domain Types ;
2. maintenir le Decision Gate dans `crates/decision-gate` ;
3. consolider le Compute Reservoir minimal ;
4. consolider le Tool Registry déclaratif ;
5. stabiliser Graph Memory + SurrealDB ;
6. stabiliser Audit ;
7. reprendre ensuite seulement la croissance Runtime / API / CLI.

Aucune exécution réelle d'outil ne doit être ajoutée avant stabilisation du Decision Gate, du Tool Registry et de l'Audit.

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
- Pas de serveur Axum stable.
- Pas de connexion SurrealDB dans `crates/core`.
- Pas d'appel LLM d'exécution : l'intégration LLM expérimentale propose uniquement des actions pending.
- Pas de shell libre.
- Pas d'envoi email réel.
- Pas de registre d'outils exécutable.
- Pas de scheduler actif.
- Pas de worker d'ingestion fonctionnel.
- Pas de CLI complexe.
- Pas de Mission Control UI.
- Pas d'intégration MCP.
- Pas d'autonomie multi-agent.
- Pas d'accès aux secrets par le LLM.

Cette fondation existe pour stabiliser le modèle conceptuel avant d'ajouter les couches runtime, sécurité, API et UI.
