---
id: R-064
titre: "UX6 — lancer le STEP depuis l’inspecteur"
resume: "Ouvrir le fichier en aperçu source puis lancer sa reconstruction avec un bouton explicite dans le panneau de droite."
statut: a-tester
jalon: atelier
cree: 2026-11-17
maj: 2026-11-17
livree:
---

## But

À la demande explicite de l’utilisateur, séparer l’ouverture du fichier et le premier calcul STEP.

## Inclus

Aperçu source au chargement STL/3MF ; bouton « Reconstruire le STEP » dans l’inspecteur à droite, désactivé pendant le travail. Réglages avant premier clic sans conversion. Nouveau fichier : nouvelle attente du clic. Passage à la vue STEP après succès conservé. Textes français/anglais.

## Hors périmètre

Modification du noyau, des familles géométriques ou du recalcul des réglages après lancement volontaire. Les commandes de correction gardent leur fonctionnement.

## Dépendances

Contrôleur workflow existant ; invalidation native, chargement et aperçu séparés déjà disponibles.

## Critères d'acceptation

Aucune commande convert_document à l’ouverture ; un clic déclenche une conversion seulement. Rejet des réponses périmées et impossibilité d’exporter un ancien résultat conservés. Tests de concurrence, interface et build bureau passent.

## Vérifications de l'utilisateur

- Ouvrir un STL ou 3MF, vérifier l’aperçu seul, choisir l’unité puis cliquer « Reconstruire le STEP » à droite ; vérifier le passage à la vue STEP, puis l’attente d’un nouveau clic à l’ouverture d’un autre fichier.
