# Procedure de reprise de session - ARPAGONA Agent Core

Ce fichier sert de repere au debut de chaque nouvelle session de travail sur ARPAGONA Agent Core.

## Objectif

Garder la continuite du projet sans dependre uniquement de l'historique d'une conversation.

Le depot GitHub est la source de verite. La conversation sert au travail du jour.

## Depot de reference

Arpagona/Agent-core

## Fichiers a relire au debut d'une session

- README.md
- docs/session-procedure.md
- docs/architecture.md
- docs/ontology.md
- docs/security-model.md
- docs/roadmap.md
- Cargo.toml
- crates/core/Cargo.toml
- crates/core/src/lib.rs

Si le fichier docs/agent-core-brief.md existe plus tard, il devra etre lu juste apres README.md.

## Prompt de reprise recommande

Tu travailles avec moi sur ARPAGONA Agent Core.

Avant de repondre :
1. Consulte le depot GitHub Arpagona/Agent-core si tu y as acces.
2. Lis README.md et docs/session-procedure.md.
3. Lis ensuite les fichiers necessaires a la mission.
4. Resume l'etat actuel du projet.
5. Identifie les decisions actees, les zones floues et la prochaine action logique.
6. Ne modifie rien dans le depot sans demande explicite.

Mission de cette session :
[decrire ici la mission precise]

## Quand garder la meme conversation

- Strategie
- Architecture generale
- Priorisation
- Relecture de sorties d'agents
- Definition de concepts
- Critique produit ou technique

## Quand ouvrir une session fraiche

- Implementation d'un module precis
- Correction d'une erreur de compilation
- Patch cible
- Ecriture ou reecriture d'un fichier
- Review de pull request
- Prompt d'execution pour un agent local

## Regle de modification du depot

Par defaut, l'assistant peut lire et reviewer le depot.

Il ne doit ecrire dans le depot que si la demande utilisateur est explicite.

Pour un changement structurant, preferer une branche et une pull request.

Pour un petit changement de documentation, un commit direct sur main peut etre acceptable si l'utilisateur le demande clairement.

## Format conseille pour une review

- Etat actuel
- Decisions actees
- Points solides
- Risques ou incoherences
- Prochaines actions recommandees
- Premiere action concrete

L'assistant doit distinguer ce qui existe dans le code, ce qui est seulement documente, ce qui est implicite, ce qui manque, et ce qui presente un risque architectural.

## Principes a preserver

- Local-first
- Auditabilite
- Separation entre proposition d'action et execution
- Decision Gate obligatoire
- Types purs dans crates/core
- Pas de logique LLM dans le core
- Pas de logique base de donnees dans le core
- Pas de logique UI dans le core
- Validation humaine pour les actions sensibles

## Fin de session

A la fin d'une session significative, produire si possible :

- Ce qui a ete decide
- Ce qui a ete modifie
- Ce qui reste ouvert
- Prochaine action recommandee
- Fichiers a relire la prochaine fois

Si une decision structurante est prise, elle devra etre ajoutee plus tard a docs/decisions.md ou dans un format ADR.
