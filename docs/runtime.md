# Cognitive Runtime V0

`crates/runtime` est la première boucle applicative expérimentale d'ARPAGONA Agent Core.

Elle relie les briques déjà présentes sans donner de pouvoir d'exécution au LLM.

## Flux V0

```text
CognitiveCycleInput
-> CognitivePulse
-> ReservoirState
-> LlmProvider
-> ProposedAction
```

La boucle s'arrête volontairement à `ProposedAction`.

Le Decision Gate n'est pas appelé automatiquement. L'application hôte doit évaluer explicitement la proposition avant toute suite :

```text
ProposedAction -> DecisionGate -> Decision -> AuditEvent
```

## Rôle

Le runtime V0 sert à commencer le mini système agentique Hermes-like, mais avec les garde-fous ARPAGONA :

- pas d'exécution directe ;
- pas de shell ;
- pas d'outil réel ;
- pas de scheduler ;
- provider LLM interchangeable ;
- réservoir court terme volatile ;
- Decision Gate obligatoire en aval ;
- audit explicite en aval.

## Composants

### RuntimeConfig

Configure :

- capacité du réservoir ;
- taux de decay ;
- agent proposant par défaut.

### CognitiveRuntimeState

Contient :

- `ReservoirState` ;
- `CognitiveCyclePlan` alpha-safe ;
- compteur d'actions proposées.

### CognitiveRuntime<P>

Runtime générique sur un provider `P: LlmProvider`.

Cela permet :

- `MockProvider` pour tests et démos sans réseau ;
- `OpenAiProvider` pour proposer via OpenAI ;
- futurs providers locaux Ollama.

### propose_once

Exécute un seul cycle de proposition :

1. transforme l'entrée en `CognitivePulse` ;
2. absorbe le pulse dans le réservoir ;
3. construit un prompt enrichi par les échos actifs ;
4. appelle le provider LLM ;
5. matérialise un `ProposedAction` ;
6. retourne un `RuntimeCycleOutput`.

## Invariants

- La `ProposedAction` retournée a toujours `PendingDecision`.
- Aucun `Decision` n'est créé.
- Aucun `AuditEvent` n'est créé dans cette couche.
- Aucun outil n'est exécuté.
- Le réservoir reste volatile et non persistant.
- Graph Memory reste responsable de la mémoire durable.

## Limites

- Pas encore branché à l'API server.
- Pas encore branché à la CLI.
- Pas de provider Ollama local.
- Pas de scheduler.
- Pas de persistance du réservoir.
- Pas de boucle multi-step autonome.

## Prochaine étape

Ajouter un endpoint expérimental contrôlé :

```text
POST /runtime/propose
```

ou intégrer `CognitiveRuntime<MockProvider/OpenAiProvider>` derrière `/agent/propose`, mais uniquement après validation locale de :

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --workspace --all-targets -- -D warnings
```
