---
name: project-documentation
description: Contrat de documentation commun aux projets Fab (FabSpeed, FabVents, FabWeave, FabBoard et les suivants) — README, AGENTS.md repris par CLAUDE.md, STATUS.md, feuille de route (renvoi à fab-roadmap), et, quand le projet les tient, DEVLOG, ARCHITECTURE, ADR, FEATURES, VALIDATION. Utiliser pour écrire ou mettre à jour l'un de ces documents, décider où ranger une information, préparer un arrêt de session que Claude Code ou Codex pourra reprendre, ou mettre à niveau la documentation d'un projet existant (references/mise-en-conformite.md). Pour Claude Code et Codex.
---

# Documentation de projet

La documentation fait partie du livrable. Chaque document a **un rôle** et un seul : on ne recopie pas le même récit dans plusieurs fichiers, on renvoie de l'un à l'autre.

## Les documents

| Document | Rôle | Statut |
| --- | --- | --- |
| `README.md` | Ce qu'est le projet, comment l'installer, le lancer et le développer | obligatoire |
| `AGENTS.md` | Consignes d'agent. `CLAUDE.md` ne contient que `@AGENTS.md` | obligatoire |
| `STATUS.md` | État vérifié, aujourd'hui, en une page | obligatoire |
| `roadmap/` | La feuille de route et les actions de l'utilisateur, lues par FabBoard. `ROADMAP.md` en est la vue générée | obligatoire, relève de `fab-roadmap` |
| `DEVLOG.md` | Journal chronologique : ce qui a été fait, choisi, vérifié | facultatif |
| `docs/ARCHITECTURE.md` | Les frontières stables et le flux de données | facultatif |
| `docs/adr/` | Une décision d'architecture par fichier, avec le gabarit [ADR-TEMPLATE.md](assets/ADR-TEMPLATE.md) | facultatif |
| `docs/FEATURES.md` | L'inventaire des fonctions, à identifiants stables. La feuille de route fait foi pour l'avancement. | facultatif |
| `docs/VALIDATION.md` | Les cas de validation et leur état | facultatif |

Un document facultatif se tient s'il existe déjà ou si l'utilisateur le demande. Ne pas en créer de nouveau de ta propre initiative.

Le détail de chaque document (contenu, exclusions, structure, qui le met à jour) : [references/document-contract.md](references/document-contract.md).

## Règles

1. **Faits et hypothèses** : un fait vérifié s'écrit comme un fait, avec la façon dont il a été vérifié ; une hypothèse ou un inconnu se marque comme tel (« non vérifié », « à confirmer »).
2. **Preuves** : une affirmation technique renvoie à un fichier ou un module (`crates/fabvents-core/src/layout.rs`), pas à une impression.
3. **`STATUS.md` reste court.** L'historique va dans `DEVLOG.md`, les décisions dans les ADR.
4. **Même changement** : la documentation se met à jour dans le changement qui modifie le comportement qu'elle décrit.
5. **Les rapports datés et les ADR acceptés ne se réécrivent pas.** Une décision qui change donne un nouvel ADR qui remplace l'ancien ; l'ancien passe à « Remplacée par ADR-… ».
6. **Un document, un rôle.** Avant d'écrire, se demander où l'information a sa place, et renvoyer par un lien plutôt que recopier.
7. **Ne rien inventer.** Une information qu'on ne peut ni lire dans le code ni déduire des documents se demande à l'utilisateur.
8. **Style du projet conservé.** Retoucher le minimum : ne pas réécrire un document juste, ni changer sa langue, ses titres ou son ton.

## La feuille de route

Elle n'est pas décrite ici : tout ce qui la touche (éléments `R-…`, actions `A-…`, statuts, spécifications, `ROADMAP.md` généré) relève de la skill [`fab-roadmap`](../fab-roadmap/SKILL.md).

- `STATUS.md` dit l'état vérifié et renvoie aux éléments (`R-012`) et aux actions (`A-007`) ; il ne porte pas de « prochaine action » pour l'utilisateur.
- Ce que l'utilisateur doit faire (tester, valider, fournir, décider) devient une action dans `roadmap/actions/`, pas une phrase dans un document.
- `AGENTS.md` a une section « Feuille de route » de quelques lignes qui renvoie à `fab-roadmap`.

## À un arrêt

Un arrêt (fin de session, jalon, attente de l'utilisateur) laisse le projet reprenable **sans l'historique de la conversation**, par une nouvelle session de Claude Code ou de Codex. Avant de t'arrêter :

1. `STATUS.md` dit ce qui est vérifié, ce qui ne l'est pas, ce qui est cassé ou en cours, avec les références aux éléments `R-…`.
2. `DEVLOG.md` (s'il existe) a l'entrée de la session : ce qui a été fait, choisi, vérifié, laissé.
3. Tout ce qui attend l'utilisateur est une action `A-…` (`fab-roadmap`).
4. Les décisions de fond sont dans un ADR (s'il en existe) ; les commandes pour reprendre sont dans `AGENTS.md` ou le `README.md`.
5. Aucune information utile ne reste seulement dans la conversation, ni dans un fichier temporaire hors dépôt.

## Mettre un projet d'équerre

Pour amener la documentation d'un projet existant au contrat : [references/mise-en-conformite.md](references/mise-en-conformite.md). Cette procédure est celle de l'étape « Documentation » de la session de mise d'équerre lancée depuis FabBoard.
