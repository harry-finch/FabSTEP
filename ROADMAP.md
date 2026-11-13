<!-- Généré par FabBoard depuis roadmap/ : ne pas modifier à la main. -->

Les jalons décrivent des critères de sortie, pas des délais garantis. La v1 mécanique et son paquet macOS local ont été acceptés ; les extensions post-v1 appartiennent au build de développement et conservent leurs essais ouverts.

L’[ancienne feuille de route](docs/ROADMAP-avant-fabboard.md) est conservée intégralement pour l’histoire. Elle contient les tableaux, critères et décisions d’origine. Les textes de chaque lot sont aussi conservés dans les éléments. Le [registre INT-AUDIT-01](docs/INT_AUDIT_01.md) garde autorité sur les suites de l’audit ; la [recherche post-v1](docs/POST_V1_RESEARCH.md) détaille les contrats géométriques.

L’ordre des lots restants reprend le document d’origine ; les priorités et dépendances exprimées dans leurs textes restent applicables. Aucun nouveau développement n’est autorisé par cette migration.

**Avancement : 19 livrés sur 55** (35 %)

## Restant à faire

1. **[R-018 Distribution — signature Developer ID et notarisation](roadmap/items/R-018-distribution-signature-developer-id-et-notarisation.md)** · Idée · jalon distribution
   Signer et notariser le paquet macOS ; ces étapes restent ouvertes.
2. **[R-019 Distribution — Windows et Linux](roadmap/items/R-019-distribution-windows-et-linux.md)** · Idée · jalon distribution
   Construire et qualifier la distribution sur Windows et Linux.
3. **[R-042 UX4 — simplifier l’atelier](roadmap/items/R-042-ux4-simplifier-latelier.md)** · À tester · jalon atelier
   Donner la priorité à la pièce et aux actions courantes ; retirer les exemples de l’écran principal ; options et données techniques à la demande
4. **[R-021 Étudier la réutilisation entre instances 3MF](roadmap/items/R-021-etudier-la-reutilisation-entre-instances-3mf.md)** · Idée · jalon audit
   AUD-006 — Démontrer un gain sur plusieurs instances ; établir un contrat tenant compte des transformations, de l’arrondi, du soudage et de la provenance avant tout cache
5. **[R-022 Réduire le coût de démarrage du frontend](roadmap/items/R-022-reduire-le-cout-de-demarrage-du-frontend.md)** · Idée · jalon audit
   AUD-009 — Mesurer le démarrage réel de la WebView ; retenir un chargement différé ou un découpage uniquement si le gain est démontré
6. **[R-023 Durcir les permissions locales Tauri](roadmap/items/R-023-durcir-les-permissions-locales-tauri.md)** · Idée · jalon audit
   AUD-014 — Définir le manifeste applicatif et tester les commandes autorisées/refusées, fenêtres et origines ; ne pas ajouter un fichier de capacités supposé suffisant
7. **[R-024 Préserver les caractères composés dans les noms de produits tronqués](roadmap/items/R-024-preserver-les-caracteres-composes-dans-les-noms-de-produits-tronques.md)** · Idée · jalon audit
   AUD-011 — Reproduire les cas Unicode utiles ; conserver le nom source et éviter une dépendance disproportionnée pour ce seul besoin
8. **[R-025 Découper les gros modules et mutualiser les helpers au besoin](roadmap/items/R-025-decouper-les-gros-modules-et-mutualiser-les-helpers-au-besoin.md)** · Idée · jalon audit
   AUD-021 — Intervenir par module lors d’un changement fonctionnel justifié, avec tests ; aucune refonte globale fondée uniquement sur le nombre de lignes
9. **[R-026 Qualifier le garde de cocircularité des anneaux fermés](roadmap/items/R-026-qualifier-le-garde-de-cocircularite-des-anneaux-fermes.md)** · Idée · jalon audit
   AUD-017 — Construire une reproduction du défaut supposé ; conserver la protection des anneaux peu échantillonnés. Aucun remplacement direct par `fit_circle` sans cette preuve
10. **[R-027 A — cône et encoche ouverte](roadmap/items/R-027-a-cone-et-encoche-ouverte.md)** · À tester · jalon post-v1
   Fraisure complète et cylindre ouvert distincts dans un même corps, en Surfaces
11. **[R-028 B — zones sphériques](roadmap/items/R-028-b-zones-spheriques.md)** · À tester · jalon post-v1
   Zone entre deux anneaux plans sans pôle, puis variante concave
12. **[R-029 Consolidation M2c — arcs décalés et adjacents](roadmap/items/R-029-consolidation-m2c-arcs-decales-et-adjacents.md)** · À tester · jalon post-v1
   Recherche bornée d'arcs fragmentés, angles de triangulation différents selon la hauteur, génératrices source partagées par deux parois
13. **[R-030 Petits raccords cylindriques contextuels](roadmap/items/R-030-petits-raccords-cylindriques-contextuels.md)** · À tester · jalon post-v1
   Intervalle entier de 4–7 points entre deux parois prouvées ; rayon résolu et tangentes compatibles
14. **[R-031 C1a — chaînes coniques alternées](roadmap/items/R-031-c1a-chaines-coniques-alternees.md)** · À tester · jalon post-v1
   Trois à huit cônes alternés et portées résolues ; réimport indépendant et parcours graphique à qualifier.
15. **[R-032 C1b — chaînes sans portée ou de même signe](roadmap/items/R-032-c1b-chaines-sans-portee-ou-de-meme-signe.md)** · Idée · jalon post-v1
   Étudier les ruptures de pente sans portée et les cônes de même signe.
16. **[R-033 C2 — profils de révolution courbes](roadmap/items/R-033-c2-profils-de-revolution-courbes.md)** · Idée · jalon post-v1
   Lignes et arcs circulaires, puis méridienne lisse de complexité bornée
17. **[R-034 D — congés partiels constants](roadmap/items/R-034-d-conges-partiels-constants.md)** · Idée · jalon post-v1
   Une ou deux arêtes droites arrondies, puis coin à trois branches
18. **[R-035 E — boîtiers creux ouverts et percés](roadmap/items/R-035-e-boitiers-creux-ouverts-et-perces.md)** · Idée · jalon post-v1
   Bac ouvert à fond/parois/rebord présents ; trous éloignés des raccords
19. **[R-036 F — perçages sécants](roadmap/items/R-036-f-percages-secants.md)** · Idée · jalon post-v1
   Deux trous cylindriques orthogonaux de rayons distincts, intersection transverse
20. **[R-037 G — congés variables](roadmap/items/R-037-g-conges-variables.md)** · Idée · jalon post-v1
   Arête droite entre deux plans, rayon linéaire positif, arrêts source
21. **[R-038 H — surfaces libres](roadmap/items/R-038-h-surfaces-libres.md)** · Idée · jalon post-v1
   Patch bicubique de type hauteur, quatre limites source sur un solide fermé
22. **[R-039 UX1 — choix compréhensibles](roadmap/items/R-039-ux1-choix-comprehensibles.md)** · À tester · jalon atelier
   Mettre en avant conserver les facettes ou reconnaître les surfaces courbes ; regrouper les reconnaissances ciblées ; aide brève selon le choix, limites détaillées à la demande
23. **[R-040 UX2 — recherche assistée des réglages](roadmap/items/R-040-ux2-recherche-assistee-des-reglages.md)** · À tester · jalon atelier
   Comparer au plus sept tolérances radiales et appliquer explicitement la proposition
24. **[R-041 UX3 — rendre les faces lisibles](roadmap/items/R-041-ux3-rendre-les-faces-lisibles.md)** · À tester · jalon atelier
   Couleurs par face STEP, contours contrastés et sélection identifiable
25. **[R-043 I1 — sélectionner et comprendre](roadmap/items/R-043-i1-selectionner-et-comprendre.md)** · À tester · jalon atelier
   Cliquer sur la surface visible, surligner une zone contiguë du corps choisi, ajuster/effacer la sélection et afficher ce qui est reconnu ou refusé
26. **[R-044 I1b — inspecter le STEP](roadmap/items/R-044-i1b-inspecter-le-step.md)** · À tester · jalon atelier
   Afficher le résultat réimporté, sélectionner une ou plusieurs faces natives et étendre aux voisines ; conserver la sélection source en complément
27. **[R-045 I1c — relier le STEP à la source](roadmap/items/R-045-i1c-relier-le-step-a-la-source.md)** · À tester · jalon atelier
   Retrouver les triangles source des faces STEP sélectionnées sans correspondance approximative
28. **[R-046 I2a — première reconstruction guidée](roadmap/items/R-046-i2a-premiere-reconstruction-guidee.md)** · À tester · jalon atelier
   À partir de la sélection, indiquer une hypothèse de surface parmi les familles prises en charge et relancer une conversion contrôlée
29. **[R-047 I2b — enchaîner les cylindres guidés](roadmap/items/R-047-i2b-enchainer-les-cylindres-guides.md)** · À tester · jalon atelier
   Ajouter depuis le STEP courant, retirer une indication et récupérer la session après refus
30. **[R-048 I2c — choisir la famille guidée](roadmap/items/R-048-i2c-choisir-la-famille-guidee.md)** · À tester · jalon atelier
   Choisir Cylindre ou Cône sur les facettes sélectionnées ; enchaîner les indications
31. **[R-049 I3 — éditer les indications](roadmap/items/R-049-i3-editer-les-indications.md)** · Idée · jalon atelier
   Modifier ou retirer une indication, annuler/rétablir et comparer le résultat calculé au maillage source
32. **[R-050 I2 — élargir les familles guidées](roadmap/items/R-050-i2-elargir-les-familles-guidees.md)** · Idée · jalon atelier
   Après Cylindre/Cône, préciser et prouver les autres familles guidées, dont les sphères.
33. **[R-052 DXF1 — exporter une face plane](roadmap/items/R-052-dxf1-exporter-une-face-plane.md)** · À tester · jalon dxf
   Sélectionner une face du STEP validé, prévisualiser son contour dans son plan, puis **Exporter le profil DXF**
34. **[R-053 DXF2a — coupe du STEP et ellipses exactes](roadmap/items/R-053-dxf2a-coupe-du-step-et-ellipses-exactes.md)** · À tester · jalon dxf
   Placer un plan XY/XZ/YZ ou reprendre le plan d’une face plane, régler son décalage et son orientation, voir la coupe et exporter la section DXF
35. **[R-054 DXF2b — coupe du maillage source](roadmap/items/R-054-dxf2b-coupe-du-maillage-source.md)** · À tester · jalon dxf
   Couper le maillage préparé sans exiger un STEP réussi ; profils polygonaux et export DXF distinct.
36. **[R-055 3MF Production — références multi-modèles](roadmap/items/R-055-3mf-production-references-multi-modeles.md)** · Idée · jalon post-v1
   Étendre l’import au-delà de 3MF Core ; les références Production restent actuellement refusées.

## Actions à faire

- [ ] [A-001](roadmap/actions/A-001-essayer-la-combinaison-cone-et-encoche-dans-le.md) Essayer la combinaison cône et encoche dans le bureau (tester, R-027)
- [ ] [A-002](roadmap/actions/A-002-reimporter-le-resultat-cone-et-encoche-dans-une.md) Réimporter le résultat cône et encoche dans une CAO indépendante (tester, R-027)
- [ ] [A-003](roadmap/actions/A-003-essayer-les-zones-spheriques-dans-le-bureau.md) Essayer les zones sphériques dans le bureau (tester, R-028)
- [ ] [A-004](roadmap/actions/A-004-reimporter-les-zones-spheriques-dans-une-cao.md) Réimporter les zones sphériques dans une CAO indépendante (tester, R-028)
- [ ] [A-005](roadmap/actions/A-005-essayer-les-cylindres-ouverts-decales-dans-le.md) Essayer les cylindres ouverts décalés dans le bureau (tester, R-029)
- [ ] [A-006](roadmap/actions/A-006-reimporter-les-cylindres-decales-dans-une-cao.md) Réimporter les cylindres décalés dans une CAO indépendante (tester, R-029)
- [ ] [A-007](roadmap/actions/A-007-essayer-les-petits-raccords-contextuels-dans-le.md) Essayer les petits raccords contextuels dans le bureau (tester, R-030)
- [ ] [A-008](roadmap/actions/A-008-reimporter-les-petits-raccords-dans-une-cao.md) Réimporter les petits raccords dans une CAO indépendante (tester, R-030)
- [ ] [A-009](roadmap/actions/A-009-essayer-les-chaines-coniques-dans-le-bureau.md) Essayer les chaînes coniques dans le bureau (tester, R-031)
- [ ] [A-010](roadmap/actions/A-010-reimporter-les-chaines-coniques-dans-une-cao.md) Réimporter les chaînes coniques dans une CAO indépendante (tester, R-031)
- [ ] [A-011](roadmap/actions/A-011-verifier-les-choix-de-reconstruction-en-francais.md) Vérifier les choix de reconstruction en français et anglais (tester, R-039)
- [ ] [A-012](roadmap/actions/A-012-essayer-la-recherche-assistee-et-son-application.md) Essayer la recherche assistée et son application explicite (tester, R-040)
- [ ] [A-013](roadmap/actions/A-013-verifier-la-lisibilite-des-faces-step-dans-les.md) Vérifier la lisibilité des faces STEP dans les deux thèmes (tester, R-041)
- [ ] [A-014](roadmap/actions/A-014-qualifier-le-parcours-complet-de-l-atelier-ux4.md) Qualifier le parcours complet de l’atelier UX4 (tester, R-042)
- [ ] [A-015](roadmap/actions/A-015-qualifier-le-clavier-et-les-petits-ecrans-de-l.md) Qualifier le clavier et les petits écrans de l’atelier (tester, R-042)
- [ ] [A-016](roadmap/actions/A-016-qualifier-la-selection-des-zones-source.md) Qualifier la sélection des zones source (tester, R-043)
- [ ] [A-017](roadmap/actions/A-017-qualifier-la-selection-des-faces-step.md) Qualifier la sélection des faces STEP (tester, R-044)
- [ ] [A-018](roadmap/actions/A-018-essayer-le-retour-exact-du-step-vers-la-source.md) Essayer le retour exact du STEP vers la source (tester, R-045)
- [ ] [A-019](roadmap/actions/A-019-essayer-une-reconstruction-cylindrique-guidee.md) Essayer une reconstruction cylindrique guidée (tester, R-046)
- [ ] [A-020](roadmap/actions/A-020-essayer-les-indications-guidees-successives-et.md) Essayer les indications guidées successives et leur retrait (tester, R-047)
- [ ] [A-021](roadmap/actions/A-021-essayer-le-choix-cylindre-ou-cone-et-leur.md) Essayer le choix Cylindre ou Cône et leur enchaînement (tester, R-048)
- [ ] [A-022](roadmap/actions/A-022-reimporter-le-resultat-guide-cylindre-et-cone.md) Réimporter le résultat guidé Cylindre et Cône dans une CAO indépendante (tester, R-048)
- [ ] [A-023](roadmap/actions/A-023-essayer-l-apercu-et-la-sauvegarde-du-profil-de.md) Essayer l’aperçu et la sauvegarde du profil de face (tester, R-052)
- [ ] [A-024](roadmap/actions/A-024-verifier-le-profil-de-face-dxf-dans-fusion.md) Vérifier le profil de face DXF dans Fusion (tester, R-052)
- [ ] [A-025](roadmap/actions/A-025-essayer-les-plans-de-coupe-step-et-les-poignees.md) Essayer les plans de coupe STEP et les poignées (tester, R-053)
- [ ] [A-026](roadmap/actions/A-026-verifier-les-coupes-step-et-ellipses-dxf-dans.md) Vérifier les coupes STEP et ellipses DXF dans Fusion (tester, R-053)
- [ ] [A-027](roadmap/actions/A-027-essayer-les-coupes-du-maillage-source-dans-le.md) Essayer les coupes du maillage source dans le bureau (tester, R-054)
- [ ] [A-028](roadmap/actions/A-028-verifier-les-coupes-polygonales-source-dans.md) Vérifier les coupes polygonales source dans Fusion (tester, R-054)
- [ ] [A-029](roadmap/actions/A-029-valider-la-migration-de-la-feuille-de-route-de.md) Valider la migration de la feuille de route de FabSTEP (valider)
- [ ] [A-030](roadmap/actions/A-030-valider-la-documentation-de-fabstep.md) Valider la documentation de FabSTEP (valider)
- [ ] [A-031](roadmap/actions/A-031-examiner-la-figure-de-la-bride-percee-de-fabstep.md) Examiner la figure de la bride percée de FabSTEP (tester)

## Livré

### v1.0 : V1 mécanique et paquet macOS local

- [R-017 Distribution — paquet macOS local](roadmap/items/R-017-distribution-paquet-macos-local.md) · livrée le 2026-11-09
  macOS autonome puis Windows/Linux. Signature ad hoc ; installation et STEP dans Fusion confirmés le 9 novembre.
- [R-016 V1 mécanique](roadmap/items/R-016-v1-mecanique.md) · livrée le 2026-11-09
  Plusieurs axes, chaînes de cônes, mélanges de surfaces et congés d’arêtes/coins
- [R-015 Stabilisation 0.10](roadmap/items/R-015-stabilisation-0-10.md) · livrée le 2026-11-09
  Source native contrôlée aussi en Plans, réglages et erreurs structurées, paquet autonome
- [R-014 M6a — boîtiers arrondis](roadmap/items/R-014-m6a-boitiers-arrondis.md) · livrée le 2026-11-09
  Boîte convexe entière, douze arêtes et huit coins à rayon commun
- [R-013 M5d — tores entiers](roadmap/items/R-013-m5d-tores-entiers.md) · livrée le 2026-11-09
  Corps annulaires complets, rotations et miroirs
- [R-012 M5b2 — sphères entières](roadmap/items/R-012-m5b2-spheres-entieres.md) · livrée le 2026-11-09
  Corps sphérique complet, sans frontière source imposée, en dernier recours du mode Surfaces
- [R-011 M5b1 — calottes sphériques](roadmap/items/R-011-m5b1-calottes-spheriques.md) · livrée le 2026-11-09
  Petites et grandes calottes à anneau plan et pôle source, convexes ou concaves, courbure intérieure prouvée
- [R-010 M5a — portions arrondies](roadmap/items/R-010-m5a-portions-arrondies.md) · livrée le 2026-11-09
  Hémisphères avec équateur et pôle source, congés toriques quart de cercle cylindre–plan et raccords prouvés
- [R-009 M4b — double chanfrein](roadmap/items/R-009-m4b-double-chanfrein.md) · livrée le 2026-11-09
  Deux cônes opposés séparés par une portée cylindrique résolue
- [R-008 M4a — cônes et chanfreins circulaires](roadmap/items/R-008-m4a-cones-et-chanfreins-circulaires.md) · livrée le 2026-11-09
  Une portion conique par chaîne de bandes connectées, pointes source, transitions et fraisures prouvées ; arcs cylindriques ouverts avec cône dans un même corps différés
- [R-007 Consolidation M3 — précision](roadmap/items/R-007-consolidation-m3-precision.md) · livrée le 2026-11-09
  Fichiers réels, tolérances explicites et traçabilité des rapports
- [R-006 M3 — 3MF Core et corps séparés](roadmap/items/R-006-m3-3mf-core-et-corps-separes.md) · livrée le 2026-11-09
  Objets, unités, transformations et instances ; STEP multi-racines sans fusion
- [R-005 M2c — arcs ouverts](roadmap/items/R-005-m2c-arcs-ouverts.md) · livrée le 2026-11-09
  Parois cylindriques partielles, encoches ouvertes et pièces mixtes
- [R-004 M2b — pièces mixtes et trous borgnes](roadmap/items/R-004-m2b-pieces-mixtes-et-trous-borgnes.md) · livrée le 2026-11-09
  Cylindres locaux complets, bossages, fonds plans prouvés, perçages sur des axes distincts
- [R-003 M2a — extrusions cylindriques](roadmap/items/R-003-m2a-extrusions-cylindriques.md) · livrée le 2026-11-09
  Cylindres droits, tubes, trous traversants dans un profil extrudé constant
- [R-002 M1 — plans](roadmap/items/R-002-m1-plans.md) · livrée le 2026-11-09
  STL ASCII/binaire, régions planes, contours polygonaux, solide STEP, CLI et interface locale
- [R-001 M0 — cadrage](roadmap/items/R-001-m0-cadrage.md) · livrée le 2026-11-06
  FabApps, licence, frontières, définition d'éditable

### audit : Audit et performances

- [R-051 PERF1 — preuves source réutilisées](roadmap/items/R-051-perf1-preuves-source-reutilisees.md) · livrée le 2026-11-10
  Cache privé et exact à chaque recherche ; mesures avant/après et équivalence des résultats consignées.
- [R-020 INT-AUDIT-01 — intégration initiale](roadmap/items/R-020-int-audit-01-integration-initiale.md) · livrée le 2026-11-10
  Cinq jalons validés ; huit recommandations traitées, preuves et suites dans le registre.
