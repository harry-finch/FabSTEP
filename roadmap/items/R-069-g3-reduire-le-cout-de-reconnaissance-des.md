---
id: R-069
titre: "G3 — réduire le coût de reconnaissance des maillages denses"
resume: "Préparation mono-corps réutilisée et recherche d’arêtes limitée aux patchs, sans relâcher les preuves. Sur le STL de 540 170 triangles, analyse 99→51 s avec égalité bit à bit ; essai 111→66 s, encore refusé dans la recherche d’arcs ouverts (R-070), sans STEP. Build et tests réussis."
statut: livree
jalon: post-v1
cree: 2026-11-17
maj: 2026-11-19
livree: 2026-11-19
---

## But

Réduire le travail inutile qui épuise le budget de reconnaissance des maillages denses. G3 est autorisé par l’utilisateur après G2 ; le fichier FlanschTurboLuftfilter_REV02.STL sert de cas mesuré, sans garantie de conversion si un autre refus survient.

## Inclus

- Mesurer les coûts de reconnaissance et de préparation topologique.
- Optimiser localement les recherches ou calculs répétés démontrés inutiles, avec résultats déterministes et travail borné.
- Conserver les prédicats, tolérances, preuves géométriques, familles reconnues et contrôles natifs.
- Comparer avant/après sur le fichier privé et des corpus reproductibles ; documenter tout blocage restant.

## Hors périmètre

Augmentation des budgets, nouvelle primitive, décimation, refonte de l’interface, changement du contrat core/CAO, relâchement des plafonds source/résultat/mémoire/durée.

## Dépendances

G2 intégré (R-068, ffcb5d4). Accord « c’est parti pour G3 » reçu ; aucun arbitrage structurel nécessaire pour ces optimisations locales.

## Critères d'acceptation

- Profilage identifiant le travail supprimé et mesures comparatives consignées.
- Résultats géométriques et refus de preuve conservés sur corpus différentiel et régressions pertinentes.
- Cas adverses toujours bornés, allocations limitées et annulation conservée.
- Tests, formatage, analyse statique et builds passent ; aucun résultat non validé n’est exportable.

## Vérifications de l'utilisateur

- Réessayer le fichier Flansch dans le build mis à jour, observer le résultat ou refus final et vérifier que l’annulation laisse l’application utilisable.
