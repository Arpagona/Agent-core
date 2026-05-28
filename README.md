# ARPAGONA Agent Core

ARPAGONA Agent Core est un **runtime agentique cognitif en Rust**, local-first, inspiré dans l'esprit des systèmes Hermes/OpenClaw, mais conçu pour aller plus loin : mémoire vivante, continuité cognitive, graphe de contexte, routage intelligent des ressources de calcul, réflexion post-cycle et auto-amélioration contrôlée.

Le projet ne vise pas seulement à gouverner des agents IA. Il vise d'abord à construire un système cognitif logiciel capable de raisonner, mémoriser, maintenir une continuité, proposer des actions, apprendre de ses erreurs et devenir progressivement plus utile.

La gouvernance, la traçabilité et le Decision Gate restent essentiels, mais comme **système immunitaire** du runtime : ils rendent cette ambition cognitive utilisable sans laisser les agents agir directement ou devenir opaques.

## Démo en 10 minutes

### Prérequis

- **Rust** (≥ 1.75) et **Cargo** installés (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- **Git**
- **Python 3** pour la validation des sorties JSON (inclus avec la plupart des distributions Linux / macOS)
- **Ollama** (optionnel) avec `qwen3.5:9b` pour la synthèse LLM locale :
  ```bash
  ollama pull qwen3.5:9b   # optionnel, 5.3 Go
  ```
- Pas besoin de clé API, d'infrastructure cloud ou de compte externe.

### 1. Compiler et vérifier

```bash
git clone git@github.com:Arpagona/Agent-core.git
cd Agent-core
cargo build --bin arpagona
```

> ⏱️ Première compilation : ~2–5 minutes selon la puissance machine.

### 2. Voir l'état du système

```bash
cargo run -q --bin arpagona -- status
```

**Sortie attendue** — un tableau de bord local montrant les sous-systèmes ARPAGONA :

```
===== ARPAGONA Agent Core — Read-Only Status =====
...
── Subsystems ──
  handoff: ...
  open_backlog_items: ...
  mcp_server_available: ...
  cli_version: ...
── Local subsystems ──
  holographic_memory_db: ...
  openai_key_configured: ...
  ollama_reachable: ...
```

✅ Le système fonctionne sans serveur API, sans LLM, sans base de données externe.

### 3. Lancer le cycle cognitif gouverné

```bash
cargo run -q --bin arpagona -- cognitive run \
  --objective "Analyser les tendances du marché IA en France pour 2026" \
  --domain business \
  --assess --observe --govern --json
```

**Sortie attendue** — un JSON structuré avec :

| Champ | Signification |
|-------|---------------|
| `objective` | L'objectif original |
| `working_memory` | Contexte de travail déduit |
| `plan` | Étapes proposées |
| `failure_insight_candidates` | Apprentissages potentiels |
| `governance_results` | Décisions du Decision Gate + traces d'audit |
| `decision_count` | ≥ 1 (décisions prises) |
| `audit_event_count` | ≥ 1 (événements tracés) |

✅ Ce qu'il se passe :
- Le runtime analyse l'objectif et classe le domaine (business/coding/research)
- Il détecte le contexte manquant et génère un plan
- Il évalue les risques et propose des actions
- Le **Decision Gate** les évalue
- Tout est tracé dans l'audit

❌ Ce qu'il ne se passe PAS :
- Aucun appel LLM
- Aucun effet externe
- Aucune exécution d'outil réelle
- Aucune écriture mémoire

### 4. Inspecter les outils cognitifs read-only

```bash
# Lister les outils disponibles
cargo run -q --bin arpagona -- tool list

# Lire un fichier du dépôt
cargo run -q --bin arpagona -- tool demo read-file PROJECT_STATUS.md --json

# Chercher un motif
cargo run -q --bin arpagona -- tool demo search-text "Decision Gate" .
```

**Tests de sécurité** — ces tentatives sont bloquées proprement :
```bash
# Chemin parent interdit
cargo run -q --bin arpagona -- tool demo read-file ../Cargo.toml --json
# → blocked / is_security: true

# Fichier système interdit
cargo run -q --bin arpagona -- tool demo read-file /etc/passwd --json
# → blocked / is_security: true
```

### 5. Synthèse LLM (avec mock provider, sans clé API externe)

```bash
cargo run -q --bin arpagona -- cognitive run \
  --objective "Analyser les tendances du marché IA" \
  --domain business \
  --llm --provider mock --json
```

**Sortie attendue** — synthèse structurée avec `[STATE]`, `[KEY GAP / RISK]`, `[RECOMMENDED NEXT STEP]`.

### 6. Journal LLM et routage Compute Reservoir

```bash
# Journal des interactions LLM
cargo run -q --bin arpagona -- llm journal --json

# Aperçu du routage de modèle
cargo run -q --bin arpagona -- compute routing \
  --purpose "Analyse de proposition commerciale" \
  --sensitivity confidential \
  --complexity 0.8 \
  --local-first --json
```

**Sortie attendue** — allocation compute avec nœud sélectionné, justification, fournisseur résolu (cloud-strong → openai, local-smol → ollama) et analyse des compromis (coût, latence, vie privée).

### 7. Démo complète (3 domaines)

```bash
bash scripts/demo-full-loop.sh
```

Exécute 3 cycles cognitifs complets (business, coding, research) et valide la sortie JSON — décisions, audit, scores.

### Récapitulatif des commandes

```bash
# État du système
cargo run -q --bin arpagona -- status --json

# Cycle cognitif gouverné
cargo run -q --bin arpagona -- cognitive run --objective "..." --assess --govern

# Synthèse LLM (mock)
cargo run -q --bin arpagona -- cognitive run --objective "..." --llm --provider mock

# Outils read-only
cargo run -q --bin arpagona -- tool demo read-file README.md

# Journal LLM
cargo run -q --bin arpagona -- llm journal

# Routage Compute
cargo run -q --bin arpagona -- compute routing --purpose "..." --sensitivity confidential

# MCP server (pour intégration externe)
cargo run -q --bin arpagona -- mcp-server --help
```

### Démarche rapide

| Étape | Commande | Durée |
|-------|----------|-------|
| Build | `cargo build --bin arpagona` | 2–5 min |
| Status | `cargo run -q -- bin arpagona -- status` | 2 s |
| Cycle gouverné | `cognitive run --assess --govern` | 2 s |
| Outils | `tool demo read-file ...` | 1 s |
| Synthèse LLM | `cognitive run --llm --provider mock` | 1 s |
| Journal | `llm journal` | 1 s |
| Compute Routing | `compute routing ...` | 1 s |
| **Total** | | **~5–10 min** |

### Dépannage

| Problème | Cause probable | Solution |
|----------|---------------|----------|
| `cargo build` lent | Première compilation des dépendances Rust | Normal pour une première fois. Les builds suivants sont plus rapides. |
| `command not found: cargo` | Rust non installé | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| `tool demo` ne fonctionne pas | Mauvais chemin de fichier | Utiliser un chemin relatif depuis la racine du dépôt |
| `--llm --provider ollama` échoue | Ollama non installé ou modèle manquant | Utiliser `--provider mock` à la place, ou installer Ollama |
| `compute routing --sensitivity secret` refuse | Données secrètes non routables vers cloud | Utiliser `--sensitivity confidential` ou `internal` |
| Sortie JSON manque des champs | Version de la CLI trop ancienne | `git pull && cargo build` |
| Port 3000 déjà utilisé | API server en cours d'exécution | Changer de port avec `ARPAGONA_API_URL=http://127.0.0.1:3001` |

## Périmètre de sécurité (ce que ARPAGONA NE fait PAS)

ARPAGONA est conçu pour **ne pas** exposer de capacités dangereuses en alpha :

| Bloqué | Pourquoi |
|--------|---------|
| ❌ Aucune exécution d'outil réelle | Les agents **proposent** uniquement. Le Decision Gate évalue. |
| ❌ Aucun shell libre | Pas de `bash`, `sh`, `exec` dans le runtime outil. |
| ❌ Aucun accès email | Pas de SMTP, pas d'envoi de messages. |
| ❌ Aucun browser automation | Pas de Selenium/Puppeteer/Playwright. |
| ❌ Aucun accès aux secrets | `.env`, `.ssh`, clés API ne sont jamais lus. |
| ❌ Aucun accès aux chemins absolus | Les outils read-only sont verrouillés dans le workspace. |
| ❌ Aucune modification destructive | Pas d'écriture, pas de suppression de fichiers. |
| ❌ Aucune autonomie non supervisée | Pas de scheduler, pas de boucle autonome. |
| ❌ Aucune décision non tracée | Toute action proposée est enregistrée dans l'audit. |

> Toute sortie readback est une **preuve d'observation**, pas une autorisation.

---

## Architecture et développement

*Cette section est destinée aux contributeurs qui souhaitent comprendre ou modifier le code.*

### Intention centrale

```text
Cognitive ambition first.
Governance as the immune system.
```

Objectif : construire un Hermes-like local en Rust avec des capacités cognitives avancées :

- Working Memory ;
- Reservoir Echo court terme ;
- Graph Memory structurée (SurrealDB) ;
- Holographic Memory (résonance associative symbolique) ;
- Compressed Convolutional Memory Retrieval (expérimental) ;
- Compute Reservoir (routage de modèle) ;
- Tool Runtime read-only ;
- Reflection Engine / Failure-to-Insight ;
- CLI supervision comme premier Mission Control ;
- Orchestrator neutre ;
- future autonomie contrôlée.

### Principe de sécurité non négociable

```text
Agent -> ProposedAction -> DecisionGate -> Execution éventuelle -> Audit
```

- Aucun agent n'agit directement.
- Un agent propose une action.
- Le Decision Gate évalue les actions sensibles ou risquées.
- Toute décision importante est tracée.
- Toute action sensible requiert une approbation humaine.
- Aucun secret n'est exposé au LLM.
- Aucun shell libre n'est disponible en V0.

### Stack cible

- Backend : Rust
- Framework backend : Axum
- Frontend : Next.js + TypeScript (différé)
- Dashboard : Mission Control (différé)
- Base principale : SurrealDB
- API : REST en V0, WebSocket (prévu plus tard)
- Architecture : monorepo

### Structure du monorepo

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
    core/                 — Vocabulaire domaine partagé
    decision-gate/        — Évaluation des actions avant exécution
    compute-reservoir/    — Routage compute intelligent
    tool-registry/        — Catalogue déclaratif d'outils
    tool-runtime/         — Runtime d'outils read-only
    graph-memory/         — Mémoire graphe SurrealDB
    holographic-memory/   — Mémoire à résonance associative
    llm/                  — Abstraction fournisseur LLM (proposition only)
    runtime/              — Boucle cognitive runtime
    mcp-server/           — Serveur MCP natif
    cli/                  — CLI de supervision locale
  apps/
    api-server/
    mission-control/      — Placeholder (différé)
  workers/
    python-ingestion/     — Placeholder (différé)
```

### Documents canoniques

Avant toute modification du dépôt, un contributeur humain ou agentique doit lire :

- `WHITEPAPER.md` : vision fondatrice ;
- `PROJECT_OBJECTIVES.md` : objectifs canoniques du projet ;
- `PROJECT_STATUS.md` : état opérationnel courant, stabilité des briques, risques et stop-list ;
- `docs/operating-doctrine.md` : doctrine de travail courante ;
- `docs/development-acceleration.md` : direction d'accélération ;
- `docs/failure-to-insight.md` : doctrine pour transformer échecs et corrections en apprentissages durables non autorisants.

### Compiler et tester

```bash
cargo fmt -- --check
cargo check
cargo test
```

### État actuel

Le projet a livré les jalons alpha suivants :

| Jalon | Statut |
|-------|--------|
| P1-P8 : Fondation cognitive gouvernée | ✅ Livré |
| Track A : Serveur MCP (A1-A5) | ✅ Livré |
| Track B : Holographic Memory (B1-B7) | ✅ Livré |
| Track C : LLM + outillage gouverné (C1-C5) | ✅ Livré |
| D1-D3 : Surfaces de supervision opérateur | ✅ Livré |
| E1 : Démo assistant SME documentaire | ✅ Livré |
| C4 : Routage Compute Reservoir (PR #150) | 🔄 PR ouverte, verte, prête à merger |
| E2 : Démo prospection commerciale (PR #149) | 🔄 PR ouverte, verte, prête à merger |
| E4 : README démo 10 min | ✅ Livré (ce document) |
| Mission Control Web | 🔜 Différé après CLI |

### Limites alpha connues

- Les sorties JSON sont le format canonique du cycle cognitif.
- Le LLM `mock` provider est déterministe sans dépendance externe.
- Le provider Ollama nécessite `ollama serve` local.
- Le provider OpenAI nécessite `OPENAI_API_KEY` dans l'environnement.
- Les outils sont read-only et limités au workspace du dépôt.
- `docs/gona-deep-governance.md` et `docs/steroid-hermes-action-plan.md` ne sont pas encore présents comme documents canoniques sur la branche courante — ils sont livrés par le canal de gouvernance DEEP/GONA (fichiers de protocole spec, pas de code Rust).
