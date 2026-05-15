# Ontologie fondatrice

Cette ontologie décrit les concepts métier du runtime. Elle sert de base commune au core Rust, à la mémoire graphe, au Decision Gate et à Mission Control.

## Workspace

Espace de travail isolé. Il représente un contexte professionnel, personnel ou projet.

Champs principaux : id, name, description optionnelle, status, created_at, updated_at.

## Agent

Entité logicielle capable de raisonner, de produire des messages et de proposer des actions. Un agent ne peut jamais exécuter directement.

Champs principaux : id, name, description optionnelle, kind, status, created_at, updated_at.

## Task

Unité de travail suivie dans un workspace.

Champs principaux : id, workspace_id, title, description optionnelle, status, priority, created_at, updated_at.

## Goal

Objectif lié à une tâche.

Champs principaux : id, task_id, statement, status, created_at, updated_at.

## ProposedAction

Action proposée par un agent ou composant orchestrateur. Elle n'est pas une exécution.

Champs principaux : id, workspace_id, task_id optionnel, proposed_by, action_type, target, payload JSON, risk_level, required_permissions, rationale, context_refs, status, created_at.

## Decision

Résultat de l'évaluation d'une action proposée.

Champs principaux : id, proposed_action_id, status, reason, risk_level, policies_applied, decided_by optionnel, created_at.

## Policy

Règle de gouvernance applicable à certaines actions ou certains niveaux de risque.

Champs principaux : id, name, description, applies_to_action_type optionnel, risk_threshold optionnel, requires_human_approval, enabled.

## Fact

Information structurée stockable dans la mémoire graphe.

Champs principaux : id, entity_type, entity_id, attribute, value JSON, source_id optionnel, confidence, valid_from optionnel, valid_to optionnel, status, created_at, updated_at.

## Source

Origine d'une information : document, utilisateur, import, système ou API future.

Champs principaux : id, source_type, title optionnel, uri optionnel, content_hash optionnel, created_at.

## Episode

Résumé d'un moment de travail ou d'un événement agentique.

Champs principaux : id, workspace_id, task_id optionnel, agent_id optionnel, summary, created_at.

## Observation

Observation rattachée à un épisode.

Champs principaux : id, episode_id, content, source_id optionnel, created_at.

## AuditEvent

Événement append-only décrivant une action importante du système.

Champs principaux : id, event_type, actor, workspace_id optionnel, task_id optionnel, proposed_action_id optionnel, decision_id optionnel, payload JSON, created_at.

## GraphRef

Référence typée vers un nœud du graphe.

Champs principaux : node_type, node_id, relation_type optionnel.
