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

## Rendu terminal

Le mode chat affiche désormais :

- bannière ASCII ARPAGONA ;
- titre en gradient ANSI façon rainbow glow ;
- ligne de statut `provider / api / workspace / task` ;
- couleurs distinctes pour erreurs, succès, commandes, risques, statuts, décisions et audit.

Aucune dépendance TUI lourde n'est utilisée. Le rendu reste une interface ligne par ligne compatible avec un terminal Ubuntu standard.

## Commandes OpenAI

Vérifier la configuration :

```bash
arpagona auth status
```

Afficher les instructions de configuration :

```bash
arpagona auth openai
```

L'alpha utilise une clé API OpenAI via variable d'environnement :

```bash
export OPENAI_API_KEY="sk-..."
export OPENAI_MODEL="gpt-4.1-mini" # optionnel
```

OAuth complet est considéré post-alpha / provider-dependent. Aucune clé n'est stockée automatiquement dans cette étape.

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
╔════════════════════════════════════════════════════════════╗
║                 ARPAGONA AGENT CORE                      ║
║            Cognitive Runtime Alpha Terminal              ║
╚════════════════════════════════════════════════════════════╝
provider: mock | api: http://127.0.0.1:3000 | workspace: workspace-alpha | task: task-1
Type /help for commands. Nothing is executed directly.

You > Prépare un brouillon de réponse client
ProposedAction
id: action-1
type: simulate_email
risk: low
status: pending_decision
rationale: Mock provider returns a draft only; execution remains gated.
next: /evaluate action-1

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
- La clé OpenAI n'est jamais affichée en clair.

## Limites alpha

- Pas de TUI plein écran.
- Pas de `ratatui` / `crossterm`.
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

Pour OpenAI :

```bash
arpagona auth openai
source ~/.config/arpagona/env
arpagona auth status
arpagona chat --provider openai
```