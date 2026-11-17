---
id: R-070
titre: "G4 — borner la recherche des arcs ouverts denses"
resume: "Après G3, le fichier Flansch de 540 170 triangles s’arrête dans partial::candidates sur le budget partagé de reconnaissance. Mesurer la consommation avant cette recherche et ses fenêtres, puis étudier une réduction prouvée du travail ; les ajustements contraints par les extrémités ne sont pas monotones. Ne pas sauter des candidats ni relever le budget sans preuve."
statut: a-tester
jalon: post-v1
cree: 2026-11-17
maj: 2026-11-17
livree:
---

## But

Réduire le travail de reconnaissance des arcs ouverts denses qui épuise le budget partagé sur le fichier Flansch. G4 est autorisé par l’utilisateur ; les améliorations doivent conserver les preuves existantes et les candidats ouverts légitimes, sans promettre une conversion si un autre refus reste présent.

## Inclus

- Mesurer le budget à l’entrée de la recherche, les contours et fenêtres en cause.
- Supprimer le travail redondant démontré, ou accélérer des calculs avec une équivalence vérifiée et un repli conservateur.
- Conserver la recherche non monotone, la maximalité, les tolérances, la provenance, les refus géométriques et les plafonds existants.
- Comparer à l’algorithme de référence sur des cas denses, bruités, ambigus et proches des seuils ; mesurer le fichier réel.

## Hors périmètre

Décimation, augmentation des budgets, nouvelle primitive, nouvelle tolérance implicite, export partiel, modification des contrats publics ou du parcours utilisateur. Un changement substantiel de preuve doit être présenté avant intégration.

## Dépendances

G3 intégré (R-069, c928c1c). Accord « c’est parti pour G4 » reçu.

## Critères d'acceptation

- Profilage précis du budget partagé et justification du travail effectivement supprimé.
- Équivalence des décisions hors du secours explicite pour les cercles complets quantifiés ; preuve circulaire existante sur tous les points pour chaque nouvelle exclusion, cas limites et refus explicites.
- Travail et mémoire bornés, annulation et validations natives conservées.
- Tests, compilation, analyse statique et documentation à jour ; limites restantes annoncées sans masquer les refus.

## Vérifications de l'utilisateur

- Réessayer Flansch dans le build mis à jour et vérifier le résultat ou refus final ainsi que l’annulation.

## Qualification et ajustement local

Le profilage mesure 5 701 356 unités à l’entrée de la recherche, puis un dépassement à 10 000 162 sur une fenêtre de 216 points d’un contour de 362 points. Ce contour est circulaire complet : l’ajustement contraint par deux extrémités adjacentes échoue sous quantification, tandis que la preuve circulaire existante réussit à la même tolérance de 1e-5 mm (écart maximal 5,638e-6 mm).

La correction prévue réemploie cette preuve sur tous les sommets, les pas angulaires et le tour complet, uniquement lorsque l’ajustement contraint échoue. Les décisions angulaires historiques (profils D et cercles clairsemés) restent intactes. C’est une correction de classification numérique, pas une équivalence stricte des anciens candidats erronés. Aucun nouveau modèle ni prédicat numérique n’est introduit. Les fenêtres ouvertes gardent leur recherche complète.

En complément, les preuves Auto immuables déjà réussies sont conservées avec leur patch/section pour éviter leur deuxième calcul à l’assemblage. Couverture, attribution et validations finales restent exécutées.

## Résultat intégré

Les deux corrections passent la revue indépendante et la validation complète V-1194 : 491 tests workspace, 49 bureau, 8 helper, 63 ciblés avec `large-stl`, 1 transfert réel, 229 UI, 12 Python et 4 SDK. Le paquet de développement est reconstruit et ses exécutables vérifiés. Qualification manuelle A-060.

Flansch ne produit pas encore de STEP : le refus se déplace à la face 7 278, contour de 300 points réellement non circulaire. L’essai dure 72,797 s (contre 66,177 s après G3, analyse également plus lente) ; aucun gain global de durée revendiqué. La recherche générale résiduelle est suivie par l’idée R-071. Le secours ajoute un coût linéaire aux contours dont l’ajustement contraint échoue ; les seuils de refus de budget ne sont donc pas strictement équivalents.
