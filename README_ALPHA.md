# README Alpha — ARPAGONA Agent Core

Cette alpha expose une vertical slice locale : création de tâche, proposition d'action, évaluation par Decision Gate, événement d'audit consultable en CLI.

Flux démontré :

```text
Task -> ProposedAction -> DecisionGate -> Decision -> AuditEvent -> Consultation CLI
```

## Périmètre livré

- Serveur API Axum minimal : `arpagona-api-server`.
- CLI installable : binaire `arpagona`, package Cargo `arpagona-cli`.
- Interface terminal interactive alpha : `arpagona chat`.
- Bannière terminal ARPAGONA avec couleurs ANSI/rainbow glow léger.
- Commandes OpenAI : `arpagona auth status` et `arpagona auth openai`.
- Graph Memory V0 séparée du serveur API alpha.
- Decision Gate alpha.
- Stockage alpha en mémoire dans le serveur API.
- Provider LLM expérimental : `agent propose` et `chat` routent un prompt en `DirectReply`, `ClarifyingQuestion` ou `ProposedAction` pending selon l'intention.
- Aucun scheduler, aucun envoi email réel, aucune exécution d'outil.

## Vérification globale

Avant une release alpha, lancer à la racine du workspace :

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --workspace --all-targets -- -D warnings
```

La checklist complète est dans [`docs/alpha-release-checklist.md`](docs/alpha-release-checklist.md).

## Démo terminal interactive recommandée

Terminal 1 :

```bash
cargo run -p arpagona-api-server
```

Terminal 2 :

```bash
cargo run -p arpagona-cli -- chat --provider mock
```

Dans le chat :

```text
Prépare un brouillon de réponse client
/evaluate action-1
/audit
/quit
```

Le mode `chat` utilise `mock` par défaut afin de fonctionner sur Ubuntu sans clé API ni réseau LLM. Avec OpenAI :

```bash
cargo run -p arpagona-cli -- auth openai
export OPENAI_API_KEY="..."
cargo run -p arpagona-cli -- auth status
cargo run -p arpagona-cli -- chat --provider openai
```

Documentation dédiée : [`docs/terminal-interface.md`](docs/terminal-interface.md).

## Démo commandes séparées

Terminal 1 :

```bash
cargo run -p arpagona-api-server
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

Sorties attendues principales :

```text
ARPAGONA API: ok

Created task: task-1
Title: Préparer une réponse client

Created proposed action: action-1
Status: pending_decision

Proposed action: action-2
Type: simulate_email
Risk: low
Status: pending_decision

Decision: needs_human_approval
Audit: audit-decision-action-1
```

`audit list` doit afficher `audit-decision-action-1`.

## Script de démo

Un script de démo est fourni :

```bash
scripts/demo-alpha.sh
```

Il suppose que le serveur API tourne déjà sur `http://127.0.0.1:3000`, exécute les commandes CLI dans l’ordre, et échoue clairement si l’API ne répond pas.

URL alternative :

```bash
ARPAGONA_API_URL=http://127.0.0.1:3000 scripts/demo-alpha.sh
```

## Installation du binaire CLI

Pendant le développement, utiliser :

```bash
cargo run -p arpagona-cli -- health
```

Quand le binaire `arpagona` est installé :

```bash
cargo install --path crates/cli
arpagona health
arpagona chat --provider mock
```

## Configuration

Par défaut, la CLI appelle :

```text
http://127.0.0.1:3000
```

Override possible :

```bash
ARPAGONA_API_URL=http://127.0.0.1:3000 arpagona health
# ou
arpagona --api-url http://127.0.0.1:3000 health
```

## Commandes disponibles

```bash
arpagona serve
arpagona chat [--provider mock] [--workspace-id workspace-alpha] [--task-id task-1] [--permission simulate_email]
arpagona auth status
arpagona auth openai
arpagona health
arpagona task create "Titre" [--description "..."] [--workspace-id workspace-alpha]
arpagona agent propose "Prompt" [--provider openai] [--task-id task-1] [--workspace-id workspace-alpha]
arpagona action propose [--type simulate_email] [--risk medium] [--task-id task-1] [--target client@example.com] [--rationale "Préparer un brouillon sans l’envoyer"] [--permission simulate_email]
arpagona action evaluate action-1 [--permission simulate_email]
arpagona audit list
```

## Agent Proposer V0 / provider LLM

`arpagona agent propose "..."` appelle `POST /agent/propose`. Par défaut, le provider est `openai`; pour une démo sans réseau ni clé, utiliser `--provider mock`.

Le provider OpenAI/chat applique désormais un routage d'intention explicite :

- `DirectReply` pour les salutations, questions d'identité et demandes d'explication ne nécessitant aucune action ;
- `ProposedAction` uniquement si l'utilisateur demande une opération, une décision, une tâche, une vérification système, une lecture mémoire/audit, une écriture mémoire, une action externe ou un workflow ;
- `ClarifyingQuestion` si l'intention est ambiguë.

Exemples attendus : `salut`, `qui es-tu ?` et `explique-moi ce que tu peux faire` répondent directement sans créer d'action ; `aide` demande une clarification sans créer d'action ; `vérifie l’état du système` propose une action gouvernée `system_check`; `lis les journaux d’audit` propose `read_audit`; `envoie un mail` propose une action email simulée ou gouvernée. `simulate_email` ne doit jamais servir de fallback quand aucune action n'est nécessaire.

Contrat alpha de `POST /agent/propose` :

- `kind: "direct_reply"` retourne un `message` et ne crée ni `ProposedAction`, ni `Decision`, ni `AuditEvent` ;
- `kind: "clarifying_question"` retourne une `question` et ne crée ni `ProposedAction`, ni `Decision`, ni `AuditEvent` ;
- `kind: "proposed_action"` retourne une `proposed_action`, matérialisée avec `status: pending_decision` ;
- le routage déterministe n'est pas une autorisation. Il classe l'intention ; seul le Decision Gate peut évaluer une action proposée.

`arpagona chat` appelle aussi `POST /agent/propose`, mais son provider par défaut est `mock` pour faciliter l'installation Ubuntu et les tests sans clé.

Configuration OpenAI :

```bash
arpagona auth openai
export OPENAI_API_KEY="..."
export OPENAI_MODEL="gpt-4.1-mini" # optionnel
arpagona auth status
```

La clé est masquée par `auth status` et ne doit jamais être committée. L'alpha utilise l'auth par API key ; OAuth complet est post-alpha / provider-dependent.

Le LLM propose uniquement. La sortie reste une `ProposedAction` avec `status: pending_decision`; la CLI n'appelle pas le Decision Gate automatiquement. L'étape suivante reste explicite :

```bash
cargo run -p arpagona-cli -- action evaluate action-1 --permission simulate_email
```

Voir [`docs/llm-provider.md`](docs/llm-provider.md).

## Note sur `serve` pendant l’alpha

`arpagona serve` est un raccourci alpha qui délègue à :

```bash
cargo run -p arpagona-api-server
```

Il dépend donc de `cargo` et d’une exécution depuis le workspace source. Pour la démo reproductible, préférer lancer explicitement :

```bash
cargo run -p arpagona-api-server
```

## Limites alpha

- Données perdues au redémarrage du serveur : stockage API in-memory.
- IDs déterministes locaux (`task-1`, `action-1`, etc.).
- Interface terminal interactive colorée mais pas encore TUI plein écran.
- Pas d'exécution réelle d'email : `simulate_email` ne fait que proposer une action.
- Pas d'exécution réelle d'outil.
- Pas de persistance serveur obligatoire.
- Auth OpenAI par API key seulement ; OAuth complet est post-alpha / provider-dependent.
- Provider LLM V0 limité à la proposition d'action ; pas de scheduler, Mission Control complet, UI ou validation humaine interactive.
- Pas de store HTTP pour les policies : ces sujets sont post-alpha.
