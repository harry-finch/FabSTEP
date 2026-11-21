---
id: R-049
titre: "I3 — éditer les indications"
resume: "Modifier ou retirer une indication, annuler/rétablir et comparer le résultat calculé au maillage source"
statut: livree
jalon: atelier
cree: 2026-11-13
maj: 2026-11-20
livree: 2026-11-20
---
## But

Modifier une indication validée et revenir sur les essais de la session.

## Inclus

Éditer la famille, retirer l’indication, annuler et rétablir, y compris pour un groupe de parois similaires. Recalculer, vérifier et comparer en vues Source et STEP.

## Hors périmètre

Historique de la session locale sur un maillage ordinaire. La sauvegarde de projets et le mode gros STL ne stockent pas cet historique.

## Dépendances

Provenance du maillage et validations du résultat natif.

## Critères d'acceptation

Un refus conserve les indications, bloque l’export périmé et permet une nouvelle action. Un nouveau choix efface la branche de rétablissement. La mémoire de l’historique est bornée.

## Vérifications de l'utilisateur

Sur un bossage, essayer un cylindre, modifier la famille, annuler puis rétablir. Comparer les vues et vérifier la disponibilité de l’export.

| I3 — éditer les indications | Modifier ou retirer une indication, annuler/rétablir et comparer le résultat calculé au maillage source | Indications attachées à l’identité/révision de source ; résultats anciens non exportables ; limites de zones/triangles/travail ; hypothèses contradictoires signalées | Plusieurs indications et retrait individuel disponibles avec I2b. Édition, annuler/rétablir et persistance intersessions restent à développer |

Source : [ancienne feuille de route](../../docs/ROADMAP-avant-fabboard.md). Les validations historiques sont conservées ; aucun essai nouveau n’est déduit de la migration.

## Préparation du 17 novembre — proposition à confirmer

Le parcours actuel sait ajouter et retirer des indications, mais Annuler/Rétablir concerne uniquement les options et réinitialise la session guidée. Proposition pour la suite : modifier la famille d’une indication existante ; annuler/rétablir les changements d’indications dans la session du fichier courant ; conserver l’alternance Source/STEP pour comparer. Chaque opération reconstruit et valide le document entier. Une tentative refusée ne devient pas un état exportable ; une nouvelle source ou de nouveaux réglages invalident l’historique incompatible.

La persistance des corrections après fermeture et la modification de leur zone source nécessitent un périmètre distinct ; elles ne sont pas promises dans cette première proposition. L’historique doit rester borné et conserver des descriptions d’indications plutôt que des collections illimitées de maillages ou de STEP. Aucun changement du code réalisé à ce stade. L’utilisateur choisit de poursuivre la géométrie, puis la simplification de l’interface. Cette proposition I3 reste une idée, sans développement autorisé dans cette séquence.
