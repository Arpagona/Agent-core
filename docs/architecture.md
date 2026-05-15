# Architecture cible

ARPAGONA Agent Core est organisé autour d'un runtime gouverné par graphe. Les agents ne sont pas des exécutants directs : ils produisent des intentions structurées qui sont évaluées, tracées et éventuellement exécutées par des composants contrôlés.

## Flux central

```text
Agent -> ProposedAction -> DecisionGate -> Execution éventuelle -> Audit
```

Ce flux est non négociable : toute action sensible passe par une décision explicite et traçable.

## Runtime Core

Le Runtime Core définit les concepts fondamentaux : agents, workspaces, tâches, objectifs, actions proposées, décisions, politiques, permissions, risques, sources, épisodes, faits et événements d'audit.

Dans cette première brique, `crates/core` reste un crate de types purs. Il ne dépend pas d'Axum, de SurrealDB, d'un provider LLM ou d'un système d'exécution.

## Mission Control

Mission Control sera l'interface web de supervision. Elle permettra de consulter les tâches, inspecter les actions proposées, approuver les actions sensibles, explorer la mémoire graphe et suivre les événements d'audit.

## Graph Memory

Graph Memory sera la couche de mémoire structurée. Elle stockera les faits, sources, épisodes, observations, relations et décisions importantes. SurrealDB est la base cible, mais le crate core ne contient aucune logique de persistance.

## Decision Gate

Le Decision Gate évaluera chaque `ProposedAction` selon :

- le type d'action ;
- le niveau de risque ;
- les permissions requises ;
- les politiques actives ;
- le contexte du workspace ;
- le besoin éventuel d'approbation humaine.

Il produira une `Decision` : autorisation, blocage ou demande de validation humaine.

## Tool Registry

Le Tool Registry décrira les capacités disponibles sans donner un accès libre aux agents. Un agent peut demander une action outillée, mais l'exécution effective reste contrôlée par le Decision Gate et les couches runtime.

## Audit Store

L'Audit Store conservera les événements importants : action proposée, décision prise, approbation humaine, exécution, échec, révocation, changement de politique, etc.

## Orchestrator

L'Orchestrator coordonnera les agents, les tâches, les objectifs et les propositions d'action. Il restera neutre et adaptable : assistant d'entreprise, agent documentaire, agent de recherche, agent métier, agent de code ou assistant personnel/pro.

## LLM Providers

Les providers LLM seront branchés plus tard. Le core ne contient aucune logique de prompt, de streaming, d'appel modèle ou de gestion de secrets.

## Scheduler

Le Scheduler déclenchera des tâches planifiées ou périodiques, mais ses actions devront suivre le même circuit de décision que les actions proposées par un agent.

## Workers d'ingestion

Les workers d'ingestion intégreront documents, données et sources externes dans la mémoire graphe. Ils devront produire des sources, observations et faits traçables.
