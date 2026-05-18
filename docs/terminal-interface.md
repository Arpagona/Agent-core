# Alpha Terminal Interface

`arpagona chat` fournit une première expérience interactive dans le terminal, inspirée de Hermes/OpenClaw, sans TUI plein écran.

Objectif : tester ARPAGONA Agent Core sur Ubuntu avec une boucle conversationnelle simple :

```text
Utilisateur
-> /agent/propose
-> ProposedAction pending_decision
-> /evaluate explicite
-> DecisionGate
-> Audit
```

## Lancer le serveur

Terminal 1 :

```bash
cargo run -p arpagona-api-server
```

Le serveur écoute par défaut sur :

```text
http://127.0.0.1:3000
```

## Lancer le mode chat

Terminal 2 :

```bash
cargo run -p arpagona-cli -- chat --provider mock
```

Le provider `mock` est le défaut du mode chat alpha. Il permet de tester sans réseau et sans clé OpenAI.

Avec OpenAI :

```bash
export OPENAI_API_KEY="..."
cargo run -p arpagona-cli -- chat --provider openai
```

La clé n'est jamais affichée par la CLI.

## Commandes internes

Dans le chat :

```text
/help                 Afficher l'aide
/quit ou /exit        Quitter
/tasks                Lister les tâches
/actions              Lister les actions proposées
/evaluate action-1    Évaluer une action via Decision Gate
/audit                Lister les événements d'audit
/provider mock        Basculer sur le provider mock
/provider openai      Basculer sur le provider OpenAI
```

Tout autre texte est traité comme une demande utilisateur et envoyé à :

```text
POST /agent/propose
```

## Exemple

```text
ARPAGONA Agent Core — Alpha Terminal
Connected to http://127.0.0.1:3000
Provider: mock
Type /help for commands. Nothing is executed directly.

You > Prépare un brouillon de réponse client
Proposed action: action-1
Type: simulate_email
Risk: low
Status: pending_decision
Rationale: Mock provider returns a draft only; execution remains gated.
Next: /evaluate action-1

You > /evaluate action-1
Decision: approved
Audit: audit-decision-action-1

You > /audit
- id: audit-decision-action-1
  event_type: decision_created
  proposed_action_id: action-1
  decision_id: decision-action-1
```

## Invariants

- Le mode chat ne lance aucun outil.
- Le mode chat ne donne pas de shell au LLM.
- Le provider propose uniquement.
- La proposition reste `pending_decision` jusqu'à `/evaluate`.
- Le Decision Gate reste explicite.
- L'audit est créé uniquement après évaluation.
- Le provider OpenAI est optionnel.

## Limites alpha

- Pas de TUI plein écran.
- Pas de persistance API : stockage in-memory.
- Pas de validation humaine interactive avancée.
- Pas de scheduler.
- Pas de Mission Control.
- Pas d'exécution d'outils.
- `arpagona serve` dépend encore de Cargo et du workspace source.

## Installation Ubuntu depuis les sources

```bash
sudo apt update
sudo apt install -y git curl build-essential pkg-config libssl-dev
curl https://sh.rustup.rs -sSf | sh
source "$HOME/.cargo/env"

git clone https://github.com/Arpagona/Agent-core.git
cd Agent-core
cargo test
cargo install --path crates/cli
```

Puis :

```bash
cargo run -p arpagona-api-server
```

Dans un autre terminal :

```bash
arpagona chat --provider mock
```
