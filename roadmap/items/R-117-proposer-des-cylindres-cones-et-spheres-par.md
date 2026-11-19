---
id: R-117
titre: "Proposer des cylindres, cônes et sphères par normales sur bandes résiduelles"
resume: "Utiliser les rayons de normales et l'angle de normale constant sur les chaînes de bandes non planes pour découvrir des parois interrompues, en conservant toutes les preuves géométriques existantes."
statut: en-cours
jalon: v1.0
cree: 2026-11-19
maj: 2026-11-19
livree:
---


## But

Aujourd’hui, la reconnaissance de cylindres, cônes et sphères repose principalement sur la présence d’anneaux plans fermés ou de paires d’arcs plans parallèles. Dès qu’une paroi courbe est interrompue par une nervure, un bossage, un congé ou une découpe oblique (comme les zones résiduelles de *Flansch* ou de pièces moulées), elle reste facettisée. Proposer des candidats à partir des rayons de normales (*normal-ray*) et de l’angle de normale constant sur les chaînes de bandes résiduelles permet d’identifier ces supports partiels sans exiger d’anneaux complets, tout en soumettant chaque candidat aux preuves strictes de FabSTEP.

## Inclus

- Regroupement déterministe des triangles non attribués à des plans en chaînes de bandes (*strip chains*) à partir de l’adjacence source et des dièdres d’arêtes, borné par le budget de travail existant.
- Estimation de candidats supports en Rust pur (`fabstep-core`, `f64`) :
  - **Sphères** par concourance des rayons de normales (centre et rayon) ;
  - **Cylindres** par axe perpendiculaire aux normales (sphère de Gauss) et concourance des rayons de normales ;
  - **Cônes** par angle constant entre normales et axe, puis sommet et demi-angle ;
  - Raffinement géométrique borné et déterministe (sans dépendance externe).
- Passage obligatoire de chaque candidat par une preuve indépendante complète sur les triangles source : résidu radial, couverture du domaine, orientation, absence de repli, contrôle des colliers/bords partagés et borne continue sous le budget d’écart.
- Qualification initiale sur un corpus synthétique indépendant de parois cylindriques, coniques et sphériques interrompues (sans anneaux complets) et de contre-exemples (surfaces quasi cylindriques, tores, bruit hors seuil).

## Hors périmètre

- Validation d’un candidat sur la seule qualité de l’ajustement des normales (les normales servent uniquement à nommer un support candidat).
- Remplacement des reconnaisseurs historiques déjà prouvés (extrusions, révolutions à anneaux, `TorusAtlas`), qui gardent la priorité tant que la parité n’est pas démontrée.
- Relâchement des tolérances absolues en millimètres, introduction d’une tolérance relative à la boîte englobante ou modification des plafonds de triangles/faces.
- Intégration directe de `trimesh`, Python ou CGAL.

## Dépendances

- `R-113` (contrat de découpe et de parois de révolution interrompues) pour la construction BRep native des domaines découpés hors anneaux complets.
- Complémentaire de `R-085` (pourtour de *Flansch*) et `R-116` (normales lissées en lecture seule sur STL bruités).

## Critères d'acceptation

- Sur le corpus indépendant (cylindre interrompu par un bossage, cône partiel sans anneaux complets, calotte/zone sphérique découpée, congé cylindrique étroit entre deux plans), les candidats proposés retrouvent l’axe/centre et le rayon dans les tolérances, et les STEP réimportés valident les types de surfaces, le volume et la fidélité bidirectionnelle.
- Sur les contre-exemples (cylindre elliptique, surface de révolution cubique, facettes planes en escalier, bruit au-dessus du seuil), aucun support erroné n’est promu ; les zones non prouvées restent facettisées en `Auto` ou refusées avec un code stable en mode explicite.
- Les cas historiques et le corpus v1/post-v1 conservent leurs résultats sans régression de faces ni de volume.
- Le coût de découverte des bandes et d’ajustement est débité sur le budget `Work` existant et reste déterministe.
- `sh scripts/check.sh` passe sans erreur ni avertissement.

## Vérifications de l'utilisateur

- Inspecter dans FabSTEP et dans Fusion les STEP produits sur les témoins de parois cylindriques et coniques interrompues.
- Vérifier sur une pièce réelle comportant des parois partielles si la réduction du nombre de facettes est utile sans dégrader la géométrie.

## Tranche intégrée au bureau

Auto reconstruit les bandes cylindriques étroites à deux bords parallèles dans le domaine décrit en V-1269. Les deux petits fichiers de recette sont `fixtures/r117-narrow-cylinder.stl` et `fixtures/r117-narrow-fillet.stl`. Le premier passe de 16 à 5 faces, le second de 19 à 8. Le jalon reste en cours pour les bords interrompus et les domaines coniques et sphériques.

Pour les cônes et les sphères, la découverte par les normales est qualifiée sur des bandes synthétiques et des fenêtres extraites de deux STL fermés (`m4_frustum.stl` et `m5b_small_cap.stl`). La prochaine tranche consiste à vérifier leurs courbes communes, les colliers source et la couverture du domaine avant de publier une face BRep. Aucun nouveau domaine conique ou sphérique découpé n’est exporté sur la seule base de ces mesures.

## Secteur conique à deux arcs parallèles

Auto regroupe les rangées du demi-tronc de cône public `fixtures/r117-half-frustum.stl`. Les deux arcs et les deux génératrices délimitent une seule paroi. Sur cette pièce de quatre rangées, le STEP passe de 195 plans à trois plans et un cône. Les 192 triangles de la paroi ont une attribution exacte. Les témoins elliptique, bombé et hors budget conservent leurs facettes. L’extension sphérique est suivie dans ce même jalon.
