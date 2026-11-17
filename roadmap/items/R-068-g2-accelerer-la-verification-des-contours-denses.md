---
id: R-068
titre: "G2 — accélérer la vérification des contours denses"
resume: "Filtrage spatial borné des contours intégré et testé, avec prédicats inchangés. Le fichier de 540 170 triangles franchit ce contrôle puis atteint le budget de reconnaissance (R-069) ; aucun STEP produit. Build de développement mis à jour, qualification utilisateur ouverte."
statut: a-tester
jalon: post-v1
cree: 2026-11-17
maj: 2026-11-17
livree:
---

## But

Réduire le travail inutile de vérification des contours denses, notamment pour FlanschTurboLuftfilter_REV02.STL (540 170 triangles). L’autorisation « go pour G2 » porte sur cette optimisation à contrôles géométriques conservés, sans garantie de conversion si un autre refus apparaît.

## Inclus

- Écarter uniquement les couples de segments prouvés spatialement séparés, avec traitement déterministe et borné.
- Conserver les prédicats de croisement, contact, inclusion, orientation et superposition, les coordonnées source et les codes de refus.
- Garder un budget fini pour la recherche spatiale et les comparaisons, sans relever les plafonds de G1.
- Comparer les résultats à la vérification exhaustive sur un corpus synthétique et mesurer avant/après sur le STL utilisateur.
- Qualifier le pipeline réel avec la supervision de G1 ; identifier honnêtement tout refus suivant.

## Hors périmètre

Nouvelle primitive, modification des tolérances, décimation, suppression des audits, augmentation des plafonds de taille/mémoire/durée, garantie de convertir toute pièce dense, refonte de l’interface.

## Dépendances

G1 (R-067) intégré. Accord utilisateur reçu dans la conversation ; aucun nouvel arbitrage structurel nécessaire pour l’optimisation locale du core pur.

## Critères d'acceptation

- Corpus différentiel : mêmes résultats géométriques que l’exhaustif sur contacts, croisements, quasi-colinéarités, trous et translations ; aucune modification des contours conservés.
- Cas denses spatialement séparés vérifiés avec moins de travail ; cas pathologiques toujours bornés et refusés explicitement.
- Mesures avant/après consignées, passage du budget initial ou nouveau blocage précisément identifié.
- Tests core et pipeline pertinents, analyse statique et builds réussis ; résultat final proposé seulement après les validations existantes.

## Vérifications de l'utilisateur

- Réessayer FlanschTurboLuftfilter_REV02.STL dans le build mis à jour et confirmer la progression, l’éventuel nouveau refus et le maintien de l’annulation.
