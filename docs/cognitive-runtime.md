# Cognitive Runtime — ARPAGONA Agent Core

Ce document reconnecte l'alpha actuelle avec la vision initiale : un système agentique de type Hermes/OpenClaw, mais plus contrôlé, plus traçable et conçu autour de couches cognitives explicites.

## Vision

ARPAGONA Agent Core ne doit pas être seulement un wrapper LLM. Le runtime cible doit combiner :

- un provider LLM interchangeable ;
- une mémoire graphe persistante ;
- un réservoir d'échos court terme ;
- un Decision Gate obligatoire ;
- une séparation stricte entre proposition, décision et exécution ;
- une boucle de réflexion/audit pour améliorer le système.

Flux cible :

```text
Input utilisateur
-> Intent parsing
-> Working memory
-> Reservoir echo
-> Graph Memory recall
-> Agent Proposal
-> Decision Gate
-> Human Boundary si nécessaire
-> Audit
-> Reflection
```

## Couches Rippletide V0

Le module `crates/core/src/cognitive.rs` formalise les couches suivantes :

- `Input` : entrée brute utilisateur ou système ;
- `IntentParsing` : compréhension structurée de l'intention ;
- `WorkingMemory` : contexte actif court terme ;
- `ReservoirEcho` : continuité transitoire par échos activés/décroissants ;
- `GraphMemory` : mémoire persistante structurée ;
- `AgentProposal` : production d'une `ProposedAction` ;
- `DecisionGate` : validation ou blocage ;
- `HumanBoundary` : validation humaine ;
- `ExecutionBoundary` : frontière avant toute future exécution ;
- `Audit` : traçabilité ;
- `Reflection` : analyse post-cycle.

Ces couches sont pour l'instant des primitives domaine : elles ne font aucun I/O et n'appellent aucun modèle.

## Reservoir Echo

Le réservoir V0 n'est pas un vrai modèle neuronal. C'est une abstraction logicielle simple pour représenter la continuité cognitive :

- un `CognitivePulse` arrive ;
- il est absorbé par `ReservoirState` ;
- il crée ou renforce une `ReservoirTrace` ;
- chaque tick diminue l'activation ;
- les traces les plus fortes peuvent influencer les prochains prompts/intentions.

Important : une trace de réservoir n'est pas un fait. Elle est temporaire, volatile et doit être consolidée explicitement avant de devenir mémoire persistante.

## Boucle agentique alpha-safe

Le plan `CognitiveCyclePlan::alpha_safe_default()` encode l'ordre de sécurité :

```text
ReceiveInput
RecallContext
EchoReservoir
DraftIntent
ProposeAction
DecisionGate
AwaitHumanIfNeeded
Audit
Reflect
```

Invariant central : `ProposeAction` arrive avant `DecisionGate`, et aucune couche d'exécution n'est disponible dans l'alpha.

## Ce que cette brique n'est pas encore

Cette brique ne contient pas encore :

- runtime autonome ;
- scheduler ;
- exécution d'outils ;
- intégration directe avec `crates/llm` ;
- persistance du réservoir ;
- consolidation automatique vers Graph Memory ;
- apprentissage adaptatif.

Elle fournit un vocabulaire de domaine testable pour implémenter ces étapes proprement.

## Prochaine sous-brique recommandée

Créer un crate applicatif expérimental :

```text
crates/runtime
```

Rôle : orchestrer une boucle V0 sans exécution :

```text
CognitiveCycleInput
-> CognitivePulse
-> ReservoirState
-> LlmProvider / MockProvider
-> ProposedAction
-> DecisionGate explicite ou différé
```

Contraintes de `crates/runtime` :

- pas d'exécution d'outils ;
- provider mock par défaut ;
- OpenAI seulement si explicitement configuré ;
- toute sortie reste `ProposedAction`; 
- audit et décision restent explicites.

## Heuristique de développement

Chaque nouvelle capacité doit répondre à cette question :

```text
Est-ce que cela renforce la boucle agentique sans donner au LLM un pouvoir d'exécution directe ?
```

Si non, reporter.
