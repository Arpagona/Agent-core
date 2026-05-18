# Checklist release alpha — ARPAGONA Agent Core

Objectif : vérifier que `main` est prêt pour un tag `v0.1.0-alpha` sans élargir le périmètre fonctionnel.

## Prérequis

- Être à la racine du workspace Cargo.
- Être sur `main` et synchronisé avec `origin/main`.
- Disposer d’une toolchain Rust compatible avec le workspace.
- Port local `127.0.0.1:3000` disponible pour la démo API.
- Aucun secret requis pour cette alpha.

## Commandes Cargo obligatoires

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --workspace --all-targets -- -D warnings
```

Tous les checks doivent passer sans warning clippy masqué par facilité.

## Séquence de démo

Terminal 1 :

```bash
cargo run -p arpagona-api-server
```

Terminal 2 :

```bash
cargo run -p arpagona-cli -- health
cargo run -p arpagona-cli -- task create "Préparer une réponse client"
cargo run -p arpagona-cli -- action propose --type simulate_email --risk medium
cargo run -p arpagona-cli -- action evaluate action-1
cargo run -p arpagona-cli -- audit list
```

Alternative scriptée, serveur déjà lancé :

```bash
scripts/demo-alpha.sh
```

## Résultats attendus

- `health` répond `ARPAGONA API: ok`.
- `task-1` est créée.
- `action-1` est créée avec le statut `pending_decision`.
- La décision est `needs_human_approval`.
- `audit-decision-action-1` est visible via `audit list`.
- Aucune action réelle n’est exécutée.

## Limites connues alpha

- Stockage API en mémoire : les données disparaissent au redémarrage serveur.
- IDs déterministes locaux (`task-1`, `action-1`, etc.).
- Pas d’authentification/API key dans cette vertical slice.
- Pas de base persistante obligatoire.
- Pas d’appel LLM.
- Pas de scheduler.
- Pas de Mission Control complet.
- Pas d’exécution réelle d’outil ni d’envoi email.
- Pas de store HTTP pour les policies.
- `arpagona serve` dépend de `cargo` et du workspace pendant l’alpha.

## Critères pour tagger `v0.1.0-alpha`

- Les quatre commandes Cargo obligatoires passent.
- La séquence CLI/API manuelle passe depuis un serveur propre.
- `scripts/demo-alpha.sh` passe lorsque le serveur est déjà lancé.
- La documentation alpha décrit fidèlement le périmètre et les limites.
- Le dépôt est clean après commit.
- Le commit de stabilisation est poussé sur `main`.

## Checklist avant release

- [ ] `git status --short --branch` confirme `main` propre.
- [ ] `cargo fmt --check` OK.
- [ ] `cargo check` OK.
- [ ] `cargo test` OK.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` OK.
- [ ] Démo manuelle CLI/API OK.
- [ ] `scripts/demo-alpha.sh` OK serveur déjà lancé.
- [ ] README alpha à jour.
- [ ] Limites alpha acceptées explicitement.
- [ ] Commit poussé sur `main`.
- [ ] Tag `v0.1.0-alpha` créé uniquement après validation finale.
