# ARPAGONA Agent Core

ARPAGONA Agent Core est un **runtime agentique cognitif en Rust**, local-first, inspiré dans l'esprit des systèmes Hermes/OpenClaw, mais conçu pour aller plus loin : mémoire vivante, continuité cognitive, graphe de contexte, routage intelligent des ressources de calcul, réflexion post-cycle et auto-amélioration contrôlée.

Le projet ne vise pas seulement à gouverner des agents IA. Il vise d'abord à construire un système cognitif logiciel capable de raisonner, mémoriser, maintenir une continuité, proposer des actions, apprendre de ses erreurs et devenir progressivement plus utile.

La gouvernance, la traçabilité et le Decision Gate restent essentiels, mais comme **système immunitaire** du runtime : ils rendent cette ambition cognitive utilisable sans laisser les agents agir directement ou devenir opaques.

## Documents canoniques

Avant toute modification du dépôt, un contributeur humain ou agentique doit lire :

- `WHITEPAPER.md` : vision fondatrice recentrée sur le Cognitive Agent Runtime ;
- `PROJECT_OBJECTIVES.md` : objectifs canoniques du projet ;
- `PROJECT_STATUS.md` : état opérationnel courant, stabilité des briques, risques et stop-list ;
- `docs/operating-doctrine.md` : doctrine de travail courante ;
- `docs/development-acceleration.md` : direction d'accélération vers une alpha Hermes-like cognitive ;
- `docs/failure-to-insight.md` : doctrine pour transformer échecs, blocages et corrections humaines en apprentissages durables non autorisants.

## Intention centrale

```text
Cognitive ambition first.
Governance as the immune system.
```

Objectif : construire un Hermes-like local en Rust avec des capacités cognitives avancées :

- Working Memory ;
- Reservoir Echo court terme ;
- Graph Memory structurée ;
- Compute Reservoir ;
- Reflection Engine / Failure-to-Insight ;
- CLI supervision comme premier Mission Control ;
- Orchestrator neutre ;
- future autonomie contrôlée.

## Principe de sécurité non négociable

- Aucun agent n'agit directement.
- Un agent propose une action.
- Le Decision Gate évalue les actions sensibles ou risquées.
- Toute décision importante est tracée.
- Toute action sensible requiert une approbation humaine.
- Aucun secret n'est exposé au LLM.
- Aucun shell libre n'est disponible en V0.

En bref :

```text
Agent -> ProposedAction -> DecisionGate -> Execution éventuelle -> Audit
```

Ce flux est un garde-fou, pas l'identité complète du projet.

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
  WHITEPAPER.md
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
    causal-trace.md
    operating-doctrine.md
    development-acceleration.md
    failure-to-insight.md
  crates/
    core/
    decision-gate/
    compute-reservoir/
    tool-registry/
    graph-memory/
    llm/
    runtime/
    cli/
  apps/
    api-server/
    mission-control/
  workers/
    python-ingestion/
```

## État actuel

Les premières briques fournissent :

- la structure monorepo ;
- la documentation fondatrice ;
- le crate Rust `crates/core` ;
- des types fondamentaux sérialisables ;
- des primitives de Cognitive Runtime incluant le Reservoir Echo court terme ;
- un Decision Gate alpha extrait dans `crates/decision-gate` ;
- un crate alpha minimal `crates/compute-reservoir` ;
- un crate alpha minimal `crates/tool-registry` ;
- un crate expérimental `crates/graph-memory` pour la mémoire graphe SurrealDB ;
- un crate expérimental `crates/llm` limité à la proposition d'action pending ;
- un crate expérimental `crates/runtime` pour une boucle cognitive qui s'arrête à la proposition d'action ;
- une application alpha `apps/api-server` ;
- une CLI alpha `crates/cli` jouant le rôle de premier Mission Control local ;
- une doctrine Failure-to-Insight pour transformer les erreurs et corrections en apprentissages durables non autorisants.

Le crate `core` doit rester limité au vocabulaire domaine et aux types purs réutilisables. Le Decision Gate, le Compute Reservoir et le Tool Registry restent des crates séparés.

Le Runtime, l'API server et la CLI sont alpha. Ils doivent rester des surfaces d'expérimentation, de readback et de supervision, pas des chemins d'exécution alternatifs.

## Priorité immédiate

La priorité actuelle est d'accélérer vers une alpha fonctionnelle **Hermes-like dans son ergonomie, ARPAGONA dans son architecture cognitive** : Rust-first, local-first, graph-native, compute-aware, inspectable, réflexive et gouvernée.

Priorité produit actuelle : **CLI supervision first**.

La CLI doit progressivement permettre de comprendre :

- ce que le système essaie de faire ;
- ce qu'il garde en mémoire active ;
- ce qu'il relit dans Graph Memory ;
- quel modèle ou ressource est choisi ;
- quelle action est proposée ;
- pourquoi une action est approuvée, bloquée ou en attente ;
- ce que le système apprend d'une erreur ou correction humaine.

Aucune exécution réelle d'outil ne doit être ajoutée avant stabilisation du Decision Gate, du Tool Registry, de l'Audit et des boucles de réflexion.

## Compiler et tester

Depuis la racine du projet :

```bash
cargo fmt
cargo check
cargo test
```

## Volontairement non implémenté en V0

- Pas d'interface web réelle.
- Pas de serveur Axum stable.
- Pas d'appel LLM d'exécution : l'intégration LLM expérimentale propose uniquement des actions pending.
- Pas de shell libre.
- Pas d'envoi email réel.
- Pas de registre d'outils exécutable.
- Pas de scheduler actif.
- Pas de worker d'ingestion fonctionnel.
- Pas de Mission Control UI.
- Pas d'intégration MCP.
- Pas d'autonomie multi-agent.
- Pas d'accès aux secrets par le LLM.

Cette fondation existe pour construire un runtime cognitif de plus en plus capable, tout en gardant les garde-fous nécessaires à une autonomie progressive.
