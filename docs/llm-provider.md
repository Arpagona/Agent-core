# LLM Provider / Agent Proposer V0

## Rôle

`crates/llm` ajoute une brique expérimentale de routage de tour agentique. Elle transforme une demande utilisateur en `AgentTurnDraft` : `DirectReply`, `ClarifyingQuestion` ou `ProposedAction`. Seule la variante `ProposedAction` contient un `ProposedActionDraft`, que l'API matérialise en `ProposedAction` avec le statut `pending_decision`.

Le LLM ne doit jamais exécuter. Il propose uniquement.

Flux :

```text
Prompt utilisateur -> AgentTurnDraft -> DirectReply | ClarifyingQuestion | ProposedActionDraft -> ProposedAction pending_decision -> Decision Gate explicite
```

## Providers

- `openai` : provider expérimental utilisant l'API OpenAI Responses.
- `mock` : provider déterministe sans réseau, prévu pour tests et démonstrations locales.

## Configuration OpenAI

Variables d'environnement :

```bash
export OPENAI_API_KEY="..."
export OPENAI_MODEL="gpt-4.1-mini" # optionnel
export OPENAI_RESPONSES_ENDPOINT="https://api.openai.com/v1/responses" # optionnel
```

Règles :

- `OPENAI_API_KEY` est obligatoire pour `provider=openai`.
- La clé n'est jamais loggée.
- Si la clé manque, l'API retourne une erreur HTTP claire et ne crée aucune `ProposedAction`.
- `OPENAI_MODEL` est optionnel ; le modèle par défaut est configuré côté crate LLM.

## Sécurité

Contraintes non négociables :

- le LLM ne peut produire qu'un tour JSON alpha : réponse directe, question de clarification ou proposition ;
- aucune exécution d'outil réel ;
- aucun shell ;
- aucun envoi email réel ;
- pas d'OpenAI tools ;
- pas de web search ;
- pas de function calling d'exécution ;
- pas d'appel automatique au Decision Gate ;
- pas de création automatique de `Decision` ;
- une réponse directe ou une question de clarification ne crée aucune `ProposedAction` ;
- la `ProposedAction` produite reste toujours `pending_decision`.

Le Decision Gate reste obligatoire avant toute suite.

## API

Endpoint expérimental :

```text
POST /agent/propose
```

Payload :

```json
{
  "workspace_id": "workspace-alpha",
  "task_id": "task-1",
  "prompt": "Prépare un brouillon de réponse client pour expliquer que nous allons envoyer un devis.",
  "provider": "openai"
}
```

Pour un test sans réseau :

```json
{
  "workspace_id": "workspace-alpha",
  "task_id": "task-1",
  "prompt": "Prépare un brouillon de réponse client",
  "provider": "mock"
}
```

La réponse contient un champ discriminant `kind`. Les valeurs alpha sont :

- `direct_reply` avec `message` : aucune `ProposedAction`, aucune `Decision`, aucun `AuditEvent` ;
- `clarifying_question` avec `question` : aucune `ProposedAction`, aucune `Decision`, aucun `AuditEvent` ;
- `proposed_action` avec `proposed_action` : une `ProposedAction` stockée avec `status: "pending_decision"`.

Le routage déterministe utilisé pour certaines intentions évidentes n'est pas une couche d'autorisation. Il ne décide pas qu'une action est permise ; il évite seulement de transformer une conversation ou une clarification en action simulée. La seule autorisation reste le Decision Gate explicite.

## CLI

Terminal 1 :

```bash
export OPENAI_API_KEY="..."
cargo run -p arpagona-api-server
```

Terminal 2 :

```bash
cargo run -p arpagona-cli -- agent propose "Prépare un brouillon de réponse client"
cargo run -p arpagona-cli -- action evaluate action-1
cargo run -p arpagona-cli -- audit list
```

La commande `agent propose` accepte :

```bash
cargo run -p arpagona-cli -- agent propose \
  "Prépare un brouillon de réponse client" \
  --provider openai \
  --task-id task-1 \
  --workspace-id workspace-alpha
```

Sortie attendue :

```text
Proposed action: action-1
Type: simulate_email
Risk: low
Status: pending_decision
```

Pour tester sans clé OpenAI :

```bash
cargo run -p arpagona-cli -- agent propose \
  "Prépare un brouillon de réponse client" \
  --provider mock
```

## Évaluation ensuite

`agent propose` ne décide rien. Pour évaluer la proposition :

```bash
cargo run -p arpagona-cli -- action evaluate action-1 --permission simulate_email
```

Cette commande appelle explicitement le Decision Gate, crée une `Decision`, puis un `AuditEvent` de décision.

## Limites V0

- Pas de streaming.
- Pas de schéma JSON strict avancé au-delà du contrat alpha `AgentTurnDraft` / `ProposedActionDraft`.
- Pas de provider local encore branché dans `crates/llm`.
- Stockage API toujours in-memory.
- Pas d'authentification HTTP autour de `/agent/propose` dans cette alpha locale.
