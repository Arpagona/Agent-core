# Modèle de sécurité

Le modèle de sécurité d'ARPAGONA Agent Core part d'une règle simple : les agents ne doivent jamais disposer d'un pouvoir d'exécution direct.

## Principe central

```text
No agent executes directly.
Agents only propose actions.
Every action passes through the Decision Gate.
Every decision is recorded in the graph.
Every sensitive action requires human approval.
```

## Séparation intention / décision / exécution

Une action proposée n'est qu'une intention structurée. Le système distingue :

- la proposition : produite par un agent ;
- la décision : produite par le Decision Gate ;
- l'exécution éventuelle : produite par une couche contrôlée ;
- l'audit : produit à chaque étape importante.

## Permissions

Les permissions décrivent les capacités demandées par une action. Elles doivent être explicites et éviter les droits globaux. Exemples : lire un document, écrire un fichier contrôlé, appeler une API approuvée, demander un envoi email simulé.

## Niveaux de risque

Les niveaux de risque permettent de guider les politiques : informational, low, medium, high, critical. Les actions à risque élevé ou critique doivent être bloquées ou soumises à validation humaine selon les politiques actives.

## Politiques

Les politiques peuvent cibler un type d'action, un seuil de risque ou une exigence d'approbation humaine. Elles doivent être traçables et désactivables explicitement.

## Secrets

Aucun secret ne doit être exposé au LLM. Les futures couches d'intégration devront utiliser des références opaques, des coffres de secrets et des permissions contrôlées.

## V0 volontairement restreinte

- Aucun shell libre.
- Aucun envoi email réel.
- Aucun accès direct aux secrets.
- Aucun outil exécutable par un agent.
- Aucun contournement du Decision Gate.

## Auditabilité

Les décisions et événements sensibles devront être enregistrés dans le graphe ou l'Audit Store. L'objectif est de pouvoir répondre à : qui a proposé quoi, dans quel contexte, quelle politique a été appliquée, qui a approuvé et quel a été le résultat.
