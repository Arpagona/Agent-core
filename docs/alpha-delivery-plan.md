# Alpha Delivery Plan

Objectif : livrer une version alpha installable d'ARPAGONA Agent Core qui prouve le flux essentiel sans élargir le périmètre.

## Définition de l'alpha

Un utilisateur doit pouvoir lancer ARPAGONA localement, créer une tâche, proposer une action, faire évaluer cette action par le Decision Gate minimal, produire une décision et un événement d'audit, puis consulter cet audit depuis la CLI.

Flux obligatoire :

```text
Workspace / Task -> ProposedAction -> DecisionGate -> Decision -> AuditEvent -> Consultation CLI
```

## État de livraison actuel

Livré dans cette alpha :

- `apps/api-server` : API HTTP locale Axum.
- `crates/core` : types domaine purs, Decision Gate et helpers d'audit.
- `crates/graph-memory` : adapter expérimental SurrealDB séparé.
- `crates/cli` : CLI `arpagona` appelant l'API locale.
- `crates/llm` : provider LLM expérimental, proposition uniquement.
- Documentation alpha : `README_ALPHA.md`, `docs/cli.md`, ce plan.

## Démo complète

Terminal 1 :

```bash
cargo run -p arpagona-cli -- serve
# équivalent alpha direct : cargo run -p arpagona-api-server
```

Terminal 2 :

```bash
cargo run -p arpagona-cli -- health
cargo run -p arpagona-cli -- task create "Préparer une réponse client"
cargo run -p arpagona-cli -- agent propose "Prépare un brouillon de réponse client" --provider mock
cargo run -p arpagona-cli -- action propose --type simulate_email --risk medium
cargo run -p arpagona-cli -- action evaluate action-1
cargo run -p arpagona-cli -- audit list
```

Si le binaire `arpagona` est bien configuré pendant le développement :

```bash
cargo run -p arpagona-cli -- health
```

Plus tard, après installation :

```bash
cargo install --path crates/cli
arpagona health
```

## Principes non négociables

- Aucun agent n'exécute directement.
- Une `ProposedAction` est une intention structurée, pas une exécution.
- Toute action proposée passe par le Decision Gate.
- Toute décision importante produit un `AuditEvent`.
- Aucune exécution d'outil réelle dans l'alpha.
- Aucun envoi email réel : `simulate_email` reste une simulation.
- Aucun shell libre.
- Aucun secret exposé au LLM.
- Aucun appel LLM obligatoire dans l'alpha : `provider=mock` permet le test sans réseau.
- Un appel LLM éventuel produit uniquement une `ProposedAction` pending, jamais une exécution.
- Le système doit rester local-first et installable.

## Périmètre inclus

### 1. API Server minimal

Endpoints alpha :

```text
GET  /health
POST /tasks
GET  /tasks
POST /proposed-actions
GET  /proposed-actions
POST /agent/propose
POST /decision-gate/evaluate
GET  /decisions
GET  /audit
```

Stockage alpha : état en mémoire dans le serveur API.

### 2. CLI installable

Commandes alpha :

```bash
arpagona health
arpagona task create "Préparer une réponse client"
arpagona agent propose "Prépare un brouillon de réponse client" --provider openai
arpagona action propose --type simulate_email --risk medium
arpagona action evaluate action-1
arpagona audit list
```

### 2 bis. Agent Proposer V0

`POST /agent/propose` et `arpagona agent propose` ajoutent une brique LLM provider expérimentale. Configuration :

```bash
export OPENAI_API_KEY="..."
export OPENAI_MODEL="gpt-4.1-mini" # optionnel
```

Règles : le LLM propose uniquement, la `ProposedAction` reste `pending_decision`, aucune `Decision` n'est créée automatiquement et le Decision Gate doit être appelé explicitement ensuite.

Documentation détaillée : [`docs/llm-provider.md`](llm-provider.md).

### 3. Decision Gate minimal

Règles alpha :

- `RiskLevel::Informational` ou `RiskLevel::Low` -> `Approved`, sauf policy contraire.
- `RiskLevel::Medium` -> `NeedsHumanApproval` par défaut.
- `RiskLevel::High` ou `RiskLevel::Critical` -> `NeedsHumanApproval` ou `Blocked` selon policy.
- Permission manquante -> `Blocked`.
- Action inconnue ou `Custom` non explicitement autorisée -> `NeedsHumanApproval`.
- La raison de la décision doit être lisible par un humain.

## Périmètre explicitement exclu

- Mission Control complet.
- Multi-agent réel.
- Scheduler autonome.
- Navigateur contrôlé.
- Shell.
- Exécution réelle d'outils.
- Envoi email réel.
- Appel LLM obligatoire : le provider mock doit suffire pour l'alpha locale.
- Authentification/API keys.
- Persistance serveur de la vertical slice CLI/API.

## Définition de terminé

- `cargo check` passe.
- `cargo test` passe.
- Vérification manuelle effectuée : `health`, `task create`, `agent propose --provider mock`, `action propose`, `action evaluate`, `audit list`.
- Documentation alpha à jour.
- Commit sur `main` avec message `Add alpha CLI`.
