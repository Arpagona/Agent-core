# Architecture cible

ARPAGONA Agent Core est un **Cognitive Agent Runtime** en Rust.

L'architecture cible n'est pas seulement un runtime gouverné par graphe. C'est d'abord un système agentique cognitif local-first, inspiré par l'ergonomie des systèmes Hermes/OpenClaw, mais conçu avec des couches cognitives explicites : Working Memory, Reservoir Echo, Graph Memory, Compute Reservoir, Reflection / Failure-to-Insight, puis gouvernance et audit comme garde-fous.

## Flux cognitif central

```text
Input utilisateur
-> Intent parsing
-> Working Memory
-> Reservoir Echo
-> Graph Memory recall
-> Compute Reservoir allocation
-> Agent Proposal
-> Decision Gate si nécessaire
-> Human Boundary si nécessaire
-> Observation
-> Audit
-> Reflection / Failure-to-Insight
-> Memory consolidation
```

Le flux de sécurité reste non négociable pour toute action sensible :

```text
Agent -> ProposedAction -> DecisionGate -> Execution éventuelle -> Audit
```

Mais ce flux est le système immunitaire du runtime. L'identité centrale du projet reste l'ambition cognitive.

## Architectural Re-Centering

Le dépôt contient déjà plusieurs briques alpha ou expérimentales. Elles doivent maintenant être recadrées autour de deux exigences simultanées :

1. construire un Hermes-like local, utile et ergonomique ;
2. préserver une architecture cognitive avancée avec garde-fous explicites.

`crates/core` doit rester le vocabulaire domaine du système : types, identifiants, événements, risques, permissions, actions proposées, décisions, sources, faits, épisodes, audit, graph primitives, cognitive primitives et contrats purs. Il ne doit pas devenir un fourre-tout pour la logique runtime, les adapters, les providers, l'API, la CLI ou la gouvernance métier avancée.

Le Decision Gate est une brique de sécurité, pas le cœur cognitif. Sa responsabilité est de prendre une `ProposedAction`, des politiques, des permissions et du contexte applicable, puis de produire une `Decision` traçable. Il ne doit pas appeler de LLM, exécuter d'outil, accéder au shell ou faire d'I/O direct.

Le Compute Reservoir est une brique cognitive centrale : il choisit quelle ressource doit penser, lire, résumer, parser ou raisonner. Il est distinct du Reservoir Echo :

- Reservoir Echo : continuité cognitive court terme, traces volatiles, activation/décroissance, influence limitée sur les prochains cycles ;
- Compute Reservoir : sélection de ressources, routage modèle/worker, arbitrage coût/latence/confidentialité/capability/fallback, mémoire de performance.

Le Tool Registry doit précéder toute exécution réelle. Il décrit déclarativement les outils, leurs schémas, permissions, risques et états activé/désactivé. Un agent ne doit jamais obtenir d'accès direct à un outil.

L'API, la CLI et le Runtime ne doivent pas prendre de responsabilité de gouvernance métier. Ils exposent ou orchestrent les couches, mais ne remplacent ni Decision Gate, ni Tool Registry, ni Audit, ni Graph Memory.

## Cognitive Runtime Core

Le Runtime Core définit les concepts fondamentaux : agents, workspaces, tâches, objectifs, working memory, actions proposées, décisions, politiques, permissions, risques, sources, épisodes, faits, événements d'audit, réservoirs cognitifs, compute allocations et failure insights.

`crates/core` doit rester un crate de types purs. Il ne dépend pas d'Axum, de SurrealDB, d'un provider LLM ou d'un système d'exécution.

## Working Memory

Working Memory est le contexte actif du cycle courant. Elle doit progressivement contenir : objectif actif, tâche courante, observations récentes, contexte graphe rappelé, contraintes actives, ressource compute sélectionnée et proposition en cours.

Elle n'est pas une mémoire durable. C'est l'espace mental actif du runtime.

## Reservoir Echo

Reservoir Echo appartient aux primitives cognitives court terme. Il sert à maintenir une continuité volatile entre cycles : traces, activation, décroissance et influence limitée sur les prochains tours.

Il ne doit pas être confondu avec une mémoire persistante, un routeur de modèles, une couche de budget ou le Compute Reservoir.

## Graph Memory

Graph Memory est la couche de mémoire structurée durable. Elle stocke les faits, sources, épisodes, observations, relations, décisions importantes, failure insights, traces et informations invalidables.

Le contrat domaine vit dans `crates/core` sous forme d'un port Rust pur (`GraphMemoryStore`) et d'une implémentation en mémoire (`InMemoryGraphMemoryStore`). Le crate `crates/graph-memory` est l'adapter persistant SurrealDB.

Graph Memory ne donne aucun pouvoir d'exécution aux agents. Elle fournit du contexte, des traces et des matériaux de consolidation.

## Compute Reservoir

Le Compute Reservoir choisit la ressource cognitive ou computationnelle adaptée à une tâche : modèle local, modèle cloud, worker local, GPU, CPU, tâche différée ou fallback.

Il arbitre selon : capability, confidentialité, coût, latence, budget, disponibilité, performance observée et stratégie de fallback.

Il ne remplace pas le Decision Gate, ne remplace pas Graph Memory et ne donne pas de droit d'exécution.

Document de cadrage : `docs/compute-reservoir.md`.

## Reflection / Failure-to-Insight

Reflection analyse les cycles terminés : succès, erreurs, blocages, mauvais routages, mauvais contextes, policy gaps et corrections humaines.

Failure-to-Insight transforme ces signaux en apprentissages durables non autorisants : propositions d'amélioration de mémoire, policies, tests, prompts, outils, documentation ou routage compute.

Cette couche est essentielle à l'ambition auto-améliorante du projet. Elle ne doit pas devenir un mécanisme d'auto-modification non contrôlée.

## Decision Gate

Le Decision Gate évalue les `ProposedAction` selon :

- le type d'action ;
- le niveau de risque ;
- les permissions requises ;
- les politiques actives ;
- le contexte du workspace ;
- le besoin éventuel d'approbation humaine.

Il produit une `Decision` : autorisation, blocage, demande de validation humaine ou demande de contexte supplémentaire.

État actuel : alpha dans le crate dédié `crates/decision-gate`, sans I/O direct et sans exécution. `crates/core` conserve uniquement les types domaine partagés.

## Tool Registry

Le Tool Registry décrit les capacités disponibles sans donner un accès libre aux agents. Son état actuel est alpha minimal dans `crates/tool-registry` : catalogue déclaratif en mémoire, schémas, permissions, risques, statuts, lookup et désactivation de déclarations uniquement.

Le Tool Registry doit exister et être stabilisé avant toute exécution réelle d'outil.

## Audit Store

L'Audit Store conserve les événements importants : action proposée, décision prise, approbation humaine, exécution, échec, révocation, changement de politique, failure insight, correction humaine, etc.

Audit doit servir la compréhension du runtime cognitif. Il ne doit pas réduire le projet à une logique de conformité.

## Orchestrator

L'Orchestrator coordonnera les agents, les tâches, les objectifs, le contexte, le Compute Reservoir, les propositions d'action et les boucles de réflexion. Il restera neutre et adaptable : assistant personnel/pro, agent documentaire, agent de recherche, agent métier, agent de code ou système local multi-agents.

Il ne doit jamais contourner le flux : `ProposedAction -> DecisionGate -> Audit` pour les actions sensibles.

## LLM Providers

Les providers LLM vivent hors de `crates/core`, dans `crates/llm`. La V0 expérimentale transforme une demande utilisateur en `ProposedActionDraft`, puis en `ProposedAction` avec le statut `pending_decision`.

Contraintes permanentes : le LLM ne doit jamais exécuter, ne doit pas utiliser d'outils OpenAI, ne doit pas faire de web search et ne doit jamais contourner le Decision Gate. `OPENAI_API_KEY` est lu uniquement par le provider OpenAI et ne doit jamais être loggé.

## CLI and Mission Control

La CLI est le premier Mission Control local. Elle doit rendre inspectable la boucle cognitive : tâches, actions, décisions, mémoire, audit, failure insights, choix compute, traces et prochaines étapes.

Mission Control Web viendra plus tard pour rendre cette supervision visuelle.

## API Server

L'API Server expose des objets et flux alpha. Il ne doit pas contenir la gouvernance métier profonde. Il doit appeler les crates responsables et ne jamais devenir un bypass d'exécution.

## Scheduler

Le Scheduler déclenchera des tâches planifiées ou périodiques, mais ses actions devront suivre le même circuit de décision que les actions proposées par un agent.

Le scheduler est deferred tant que le chemin de gouvernance et la réflexion post-cycle ne sont pas stables.

## Workers d'ingestion

Les workers d'ingestion intégreront documents, données et sources externes dans la mémoire graphe. Ils devront produire des sources, observations et faits traçables.

Ils ne doivent pas contourner Graph Memory, Audit ou les politiques de confidentialité.
