---
id: "R-051"
titre: "PERF1 — preuves source réutilisées"
resume: "Cache privé et exact à chaque recherche ; mesures avant/après et équivalence des résultats consignées."
statut: "livree"
jalon: "audit"
cree: "2026-11-13"
maj: "2026-11-13"
livree: "2026-11-10"
---

## PERF1 — réduire le travail source répété pendant la recherche

**PERF1 terminé et validé (V-1168/V-1169).** La recherche réelle prend désormais 8,294 s au lieu de 13,057 s sur la pièce mixte (−36,5 %), 21,209 s au lieu de 41,508 s sur le 3MF (−48,9 %) et 13,538 s au lieu de 17,465 s sur le sock-clip (−22,5 %), médianes de trois répétitions en développement. Rapports complets, refus et traces de contrôle comparés avec/sans cache ; propositions avant/après identiques. Voir V-1168.

**Profilage initial.** Trois répétitions du scénario des sept candidats, en profil de développement non optimisé : médianes 12,838 s sur la pièce mixte, 40,648 s sur le 3MF à deux instances et 17,223 s sur le sock-clip. La validation source représente 43,18 %, 28,20 % et 27,76 % ; la séparation source ajoute 28,16 % sur ce 3MF. Les détails et limites sont en V-1167 dans [VALIDATION](../../docs/VALIDATION.md). Ces temps ne sont pas des gains ni des prévisions du build release.

- Contexte privé limité à une recherche ; identité exacte des coordonnées en millimètres après placement et des indices/ordres des triangles, bornée à 32 corps / 100 000 triangles / 300 000 sommets. Seules les preuves réussies sont réutilisées, sans cache global ni objet OCCT exposé.
- Garder la validation d’un corps à sa position logique actuelle, après reconnaissance : sa réutilisation doit être paresseuse, sans modifier la priorité des erreurs. Conserver les gardes d’annulation et les refus par corps.
- Évaluer séparément la séparation des corps à la source et la validation de chacun. Toujours reconstruire, réimporter, contrôler et auditer chaque résultat candidat ; aucune réutilisation d’un audit d’une autre géométrie.
- Comparer avant/après mêmes fichiers et réglages : résultats, refus, classement, provenance, dimensions/volumes et fidélité inchangés, compteurs démontrant le travail source évité et temps mesurés avec dispersion. Revenir au parcours sans cache si ces preuves ne sont pas obtenues.

La double segmentation `plane` / `max(plane, fit)` reste hors de cette première optimisation : contrats différents et coût isolé non mesuré. La fusion des lectures/tessellations STEP reste différée : AuditTessellation représente seulement 0,93–1,44 % ici, et le coût de lecture seul n’est pas isolé. L’audit comparatif reste important (47,34 % sur le sock-clip) mais ne peut pas être supprimé ni partagé entre résultats distincts.

Source : [ancienne feuille de route](../../docs/ROADMAP-avant-fabboard.md). Les validations historiques sont conservées ; aucun essai nouveau n’est déduit de la migration.
