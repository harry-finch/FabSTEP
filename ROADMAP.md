<!-- Généré par FabBoard depuis roadmap/ : ne pas modifier à la main. -->

Les jalons décrivent des critères de sortie, pas des délais garantis. La v1 mécanique et son paquet macOS local ont été acceptés ; les extensions post-v1 appartiennent au build de développement et conservent leurs essais ouverts.

L’[ancienne feuille de route](docs/ROADMAP-avant-fabboard.md) est conservée intégralement pour l’histoire. Elle contient les tableaux, critères et décisions d’origine. Les textes de chaque lot sont aussi conservés dans les éléments. Le [registre INT-AUDIT-01](docs/INT_AUDIT_01.md) garde autorité sur les suites de l’audit ; la [recherche post-v1](docs/POST_V1_RESEARCH.md) détaille les contrats géométriques.

L’ordre des lots restants reprend le document d’origine ; les priorités et dépendances exprimées dans leurs textes restent applicables. Aucun nouveau développement n’est autorisé par cette migration.

**Avancement : 23 livrés sur 84** (27 %)

## Restant à faire

1. **[R-060 C1b2a — deux cônes directement raccordés](roadmap/items/R-060-c1b2a-deux-cones-directement-raccordes.md)** · À tester · jalon post-v1
   Reconstruire deux cônes adjacents lorsque leur rupture unique et leurs parois sont prouvées par les anneaux source.
2. **[R-061 C2a — deux raccords toriques sur une portée](roadmap/items/R-061-c2a-deux-raccords-toriques-sur-une-portee.md)** · À tester · jalon post-v1
   Reconstruire un profil exact arc–ligne–arc, avec deux congés toriques et une portée cylindrique commune.
3. **[R-062 D1–E1 — arêtes arrondies et bac ouvert](roadmap/items/R-062-d1-e1-aretes-arrondies-et-bac-ouvert.md)** · À tester · jalon post-v1
   Qualifier les arrondis cylindriques locaux et les bacs ouverts à coins verticaux arrondis, avec fond et vide source conservés.
4. **[R-063 F0 — qualifier les courbes des perçages sécants](roadmap/items/R-063-f0-qualifier-les-courbes-des-percages-secants.md)** · À tester · jalon post-v1
   Vérifier dans un laboratoire natif borné les courbes et paramètres de deux cylindres orthogonaux après réimport STEP, avant intégration du contrat R-036.
5. **[R-032 C1b1 — chaînes coniques de même signe avec portées](roadmap/items/R-032-c1b-chaines-sans-portee-ou-de-meme-signe.md)** · À tester · jalon post-v1
   Reconnaître deux à huit cônes de signes libres séparés par des portées cylindriques prouvées ; conserver les garde-fous contre les fausses ruptures.
6. **[R-042 UX4 — simplifier l’atelier](roadmap/items/R-042-ux4-simplifier-latelier.md)** · À tester · jalon atelier
   Donner la priorité à la pièce et aux actions courantes ; retirer les exemples de l’écran principal ; options et données techniques à la demande
7. **[R-027 A — cône et encoche ouverte](roadmap/items/R-027-a-cone-et-encoche-ouverte.md)** · À tester · jalon post-v1
   Fraisure complète et cylindre ouvert distincts dans un même corps, en Surfaces
8. **[R-028 B — zones sphériques](roadmap/items/R-028-b-zones-spheriques.md)** · À tester · jalon post-v1
   Zone entre deux anneaux plans sans pôle, puis variante concave
9. **[R-029 Consolidation M2c — arcs décalés et adjacents](roadmap/items/R-029-consolidation-m2c-arcs-decales-et-adjacents.md)** · À tester · jalon post-v1
   Recherche bornée d'arcs fragmentés, angles de triangulation différents selon la hauteur, génératrices source partagées par deux parois
10. **[R-030 Petits raccords cylindriques contextuels](roadmap/items/R-030-petits-raccords-cylindriques-contextuels.md)** · À tester · jalon post-v1
   Intervalle entier de 4–7 points entre deux parois prouvées ; rayon résolu et tangentes compatibles
11. **[R-031 C1a — chaînes coniques alternées](roadmap/items/R-031-c1a-chaines-coniques-alternees.md)** · À tester · jalon post-v1
   Trois à huit cônes alternés et portées résolues ; réimport indépendant et parcours graphique à qualifier.
12. **[R-039 UX1 — choix compréhensibles](roadmap/items/R-039-ux1-choix-comprehensibles.md)** · À tester · jalon atelier
   Mettre en avant conserver les facettes ou reconnaître les surfaces courbes ; regrouper les reconnaissances ciblées ; aide brève selon le choix, limites détaillées à la demande
13. **[R-040 UX2 — recherche assistée des réglages](roadmap/items/R-040-ux2-recherche-assistee-des-reglages.md)** · À tester · jalon atelier
   Comparer au plus sept tolérances radiales et appliquer explicitement la proposition
14. **[R-041 UX3 — rendre les faces lisibles](roadmap/items/R-041-ux3-rendre-les-faces-lisibles.md)** · À tester · jalon atelier
   Couleurs par face STEP, contours contrastés et sélection identifiable
15. **[R-043 I1 — sélectionner et comprendre](roadmap/items/R-043-i1-selectionner-et-comprendre.md)** · À tester · jalon atelier
   Cliquer sur la surface visible, surligner une zone contiguë du corps choisi, ajuster/effacer la sélection et afficher ce qui est reconnu ou refusé
16. **[R-044 I1b — inspecter le STEP](roadmap/items/R-044-i1b-inspecter-le-step.md)** · À tester · jalon atelier
   Afficher le résultat réimporté, sélectionner une ou plusieurs faces natives et étendre aux voisines ; conserver la sélection source en complément
17. **[R-045 I1c — relier le STEP à la source](roadmap/items/R-045-i1c-relier-le-step-a-la-source.md)** · À tester · jalon atelier
   Retrouver les triangles source des faces STEP sélectionnées sans correspondance approximative
18. **[R-046 I2a — première reconstruction guidée](roadmap/items/R-046-i2a-premiere-reconstruction-guidee.md)** · À tester · jalon atelier
   À partir de la sélection, indiquer une hypothèse de surface parmi les familles prises en charge et relancer une conversion contrôlée
19. **[R-047 I2b — enchaîner les cylindres guidés](roadmap/items/R-047-i2b-enchainer-les-cylindres-guides.md)** · À tester · jalon atelier
   Ajouter depuis le STEP courant, retirer une indication et récupérer la session après refus
20. **[R-048 I2c — choisir la famille guidée](roadmap/items/R-048-i2c-choisir-la-famille-guidee.md)** · À tester · jalon atelier
   Choisir Cylindre ou Cône sur les facettes sélectionnées ; enchaîner les indications
21. **[R-052 DXF1 — exporter une face plane](roadmap/items/R-052-dxf1-exporter-une-face-plane.md)** · À tester · jalon dxf
   Sélectionner une face du STEP validé, prévisualiser son contour dans son plan, puis **Exporter le profil DXF**
22. **[R-053 DXF2a — coupe du STEP et ellipses exactes](roadmap/items/R-053-dxf2a-coupe-du-step-et-ellipses-exactes.md)** · À tester · jalon dxf
   Placer un plan XY/XZ/YZ ou reprendre le plan d’une face plane, régler son décalage et son orientation, voir la coupe et exporter la section DXF
23. **[R-054 DXF2b — coupe du maillage source](roadmap/items/R-054-dxf2b-coupe-du-maillage-source.md)** · À tester · jalon dxf
   Couper le maillage préparé sans exiger un STEP réussi ; profils polygonaux et export DXF distinct.
24. **[R-056 I2d — fusion plane guidée](roadmap/items/R-056-i2d-fusion-plane-guidee.md)** · À tester · jalon atelier
   Ajouter Plan aux familles guidées : fusionner une sélection contiguë presque plane, sans déplacer ses sommets, après contrôle du solide.
25. **[R-057 UX5 — reconstruction automatique et sélection des parois](roadmap/items/R-057-ux5-reconstruction-automatique-et-selection-des.md)** · À tester · jalon atelier
   Ouvrir et obtenir le meilleur STEP prouvé, sélectionner une paroi en un geste et corriger localement sans perdre les surfaces reconnues.
26. **[R-064 UX6 — lancer le STEP depuis l’inspecteur](roadmap/items/R-064-ux6-lancer-le-step-depuis-l-inspecteur.md)** · À tester · jalon atelier
   Ouvrir le fichier en aperçu source puis lancer sa reconstruction avec un bouton explicite dans le panneau de droite.
27. **[R-065 UX7 — suivre les étapes réelles de reconstruction](roadmap/items/R-065-ux7-suivre-les-etapes-reelles-de-reconstruction.md)** · À tester · jalon atelier
   Afficher les étapes observées du moteur et le corps en cours, sans pourcentage ni temps restant inventés.
28. **[R-066 UX8 — préciser le nombre de triangles refusés](roadmap/items/R-066-ux8-preciser-le-nombre-de-triangles-refuses.md)** · À tester · jalon atelier
   Afficher le nombre exact de triangles d’un STL trop dense et le maximum autorisé.
29. **[R-067 G1 — gros maillages sur accord et calcul annulable](roadmap/items/R-067-g1-gros-maillages-sur-accord-et-calcul-annulable.md)** · À tester · jalon post-v1
   Autoriser un essai explicite au-delà de 100 000 triangles, avec avertissement, limites distinctes et arrêt du calcul ; qualifier d’abord le STL de 540 170 triangles.
30. **[R-068 G2 — accélérer la vérification des contours denses](roadmap/items/R-068-g2-accelerer-la-verification-des-contours-denses.md)** · À tester · jalon post-v1
   Filtrage spatial borné des contours intégré et testé, avec prédicats inchangés. Le fichier de 540 170 triangles franchit ce contrôle puis atteint le budget de reconnaissance (R-069) ; aucun STEP produit. Build de développement mis à jour, qualification utilisateur ouverte.
31. **[R-069 G3 — réduire le coût de reconnaissance des maillages denses](roadmap/items/R-069-g3-reduire-le-cout-de-reconnaissance-des.md)** · À tester · jalon post-v1
   Préparation mono-corps réutilisée et recherche d’arêtes limitée aux patchs, sans relâcher les preuves. Sur le STL de 540 170 triangles, analyse 99→51 s avec égalité bit à bit ; essai 111→66 s, encore refusé dans la recherche d’arcs ouverts (R-070), sans STEP. Build et tests réussis.
32. **[R-070 G4 — borner la recherche des arcs ouverts denses](roadmap/items/R-070-g4-borner-la-recherche-des-arcs-ouverts-denses.md)** · À tester · jalon post-v1
   Après G3, le fichier Flansch de 540 170 triangles s’arrête dans partial::candidates sur le budget partagé de reconnaissance. Mesurer la consommation avant cette recherche et ses fenêtres, puis étudier une réduction prouvée du travail ; les ajustements contraints par les extrémités ne sont pas monotones. Ne pas sauter des candidats ni relever le budget sans preuve.
33. **[R-071 Accélérer les fenêtres d’arcs des contours complexes](roadmap/items/R-071-accelerer-les-fenetres-d-arcs-des-contours.md)** · À tester · jalon post-v1
   Après G4, Flansch franchit le cercle dense mais épuise encore le budget sur la face 7 278, contour de 300 points réellement non circulaire. Étudier une réduction prouvée du coût des fenêtres, sans supposer la monotonie des ajustements ni augmenter le budget ; une nouvelle preuve numérique exige une qualification distincte.
34. **[R-073 Qualifier la surveillance mémoire des gros STL sur macOS](roadmap/items/R-073-qualifier-la-surveillance-memoire-des-gros-stl.md)** · À tester · jalon post-v1
   La sonde R-072 rapporte une empreinte physique de 6.9G malgré un RSS maximal échantillonné de 2,35 Gio. Comparer les métriques macOS et qualifier une surveillance adaptée avant toute augmentation de capacité ; préserver arrêt et nettoyage du helper.
35. **[R-075 Reconstruire un raccord tore–cône–cylindre prouvé](roadmap/items/R-075-reconstruire-un-raccord-tore-cone-cylindre.md)** · À tester · jalon post-v1
   Première extension issue de R-074 : reconstruire la zone de 64 080 triangles par trois surfaces, avec raccord tangent prouvé et bandes à deux anneaux. Contrat de données inchangé ; règle de preuve approuvée pour implémentation, sans promesse de convertir Flansch entier.
36. **[R-076 C2b — chaînes toriques composées et arcs au-delà de 90°](roadmap/items/R-076-c2b-chaines-toriques-composees-et-arcs-au-dela.md)** · À tester · jalon post-v1
   Reconstruire les trois autres zones courbes mesurées sur Flansch : raccord à deux cônes et arcs de 100°, avec frontières source et preuves continues.
37. **[R-077 E2 — bac ouvert avec perçage du fond](roadmap/items/R-077-e2-bac-ouvert-avec-percage-du-fond.md)** · À tester · jalon post-v1
   Qualifier un bac à coins arrondis avec un trou traversant le fond, sans remplir la cavité ni perdre les parois.
38. **[R-078 C1b2b — chaînes coniques adjacentes et mixtes](roadmap/items/R-078-c1b2b-chaines-coniques-adjacentes-et-mixtes.md)** · À tester · jalon post-v1
   Reconstruire deux à huit cônes adjacents ou mêlés à des portées, avec une décomposition unique prouvée par les anneaux source.
39. **[R-081 D2 — reconstruire un coin local à trois arrondis](roadmap/items/R-081-d2-reconstruire-un-coin-local-a-trois-arrondis.md)** · À tester · jalon post-v1
   Intégrer l’octant sphérique et ses trois raccords cylindriques à partir des frontières STL, avec preuve continue et STEP validé.
40. **[R-082 G1 — reconstruire un raccord à rayon linéaire](roadmap/items/R-082-g1-reconstruire-un-raccord-a-rayon-lineaire.md)** · À tester · jalon post-v1
   Reconnaître l’enveloppe conique inclinée à extrémités elliptiques d’un arrondi linéaire, sans la confondre avec des sections circulaires empilées.
41. **[R-083 F1 — reconnaître les perçages sécants depuis le STL](roadmap/items/R-083-f1-reconnaitre-les-percages-secants-depuis-le.md)** · À tester · jalon post-v1
   Prouver deux perçages centrés orthogonaux, leurs limites communes et leur reconstruction STEP, dans le domaine déjà autorisé par A-045.
42. **[R-033 C2 — profils de révolution courbes](roadmap/items/R-033-c2-profils-de-revolution-courbes.md)** · En cours · jalon post-v1
   Révolutions toriques composées et première méridienne cubique bornée ; la généralisation reste ouverte.
43. **[R-034 D — congés partiels constants](roadmap/items/R-034-d-conges-partiels-constants.md)** · En cours · jalon post-v1
   Coin à trois arrondis intégré ; reste la généralisation des congés partiels et de leurs combinaisons.
44. **[R-035 E — boîtiers creux ouverts et percés](roadmap/items/R-035-e-boitiers-creux-ouverts-et-perces.md)** · En cours · jalon post-v1
   Bac ouvert et fond percé intégrés ; restent les congés du fond/rebord et autres combinaisons.
45. **[R-036 F — perçages sécants](roadmap/items/R-036-f-percages-secants.md)** · En cours · jalon post-v1
   F1 centré orthogonal développé sous R-083 et qualifié automatiquement ; essais utilisateur et généralisation restent ouverts.
46. **[R-037 G — congés variables](roadmap/items/R-037-g-conges-variables.md)** · En cours · jalon post-v1
   Premier raccord à rayon linéaire intégré ; restent les autres configurations et raccords variables multiples.
47. **[R-059 C1b2 — chaînes coniques sans portée intermédiaire](roadmap/items/R-059-c1b2-chaines-coniques-sans-portee-intermediaire.md)** · Idée · jalon post-v1
   Prouver les ruptures de pente entre cônes directement adjacents, sans découper artificiellement une méridienne courbe.
48. **[R-025 Découper les gros modules et mutualiser les helpers au besoin](roadmap/items/R-025-decouper-les-gros-modules-et-mutualiser-les-helpers-au-besoin.md)** · Idée · jalon audit
   AUD-021 — Intervenir par module lors d’un changement fonctionnel justifié, avec tests ; aucune refonte globale fondée uniquement sur le nombre de lignes
49. **[R-026 Qualifier le garde de cocircularité des anneaux fermés](roadmap/items/R-026-qualifier-le-garde-de-cocircularite-des-anneaux-fermes.md)** · Idée · jalon audit
   AUD-017 — Construire une reproduction du défaut supposé ; conserver la protection des anneaux peu échantillonnés. Aucun remplacement direct par `fit_circle` sans cette preuve
50. **[R-038 H — surfaces libres](roadmap/items/R-038-h-surfaces-libres.md)** · Idée · jalon post-v1
   Patch bicubique de type hauteur, quatre limites source sur un solide fermé
51. **[R-049 I3 — éditer les indications](roadmap/items/R-049-i3-editer-les-indications.md)** · Idée · jalon atelier
   Modifier ou retirer une indication, annuler/rétablir et comparer le résultat calculé au maillage source
52. **[R-050 I2 — élargir les familles guidées](roadmap/items/R-050-i2-elargir-les-familles-guidees.md)** · Idée · jalon atelier
   Après Plan (R-056), ajouter le raccord torique guidé puis étudier les sphères.
53. **[R-024 Préserver les caractères composés dans les noms de produits tronqués](roadmap/items/R-024-preserver-les-caracteres-composes-dans-les-noms-de-produits-tronques.md)** · Idée · jalon audit
   AUD-011 — Reproduire les cas Unicode utiles ; conserver le nom source et éviter une dépendance disproportionnée pour ce seul besoin
54. **[R-023 Durcir les permissions locales Tauri](roadmap/items/R-023-durcir-les-permissions-locales-tauri.md)** · Idée · jalon audit
   AUD-014 — Définir le manifeste applicatif et tester les commandes autorisées/refusées, fenêtres et origines ; ne pas ajouter un fichier de capacités supposé suffisant
55. **[R-022 Réduire le coût de démarrage du frontend](roadmap/items/R-022-reduire-le-cout-de-demarrage-du-frontend.md)** · Idée · jalon audit
   AUD-009 — Mesurer le démarrage réel de la WebView ; retenir un chargement différé ou un découpage uniquement si le gain est démontré
56. **[R-080 Qualifier les 231 584 plans restants de Flansch](roadmap/items/R-080-qualifier-les-231-584-plans-restants-de-flansch.md)** · Idée · jalon post-v1
   Après les quatre grandes zones reconstruites, Flansch conserve 231 584 faces planes et 24 surfaces analytiques. Identifier les zones encore facettisées et leurs familles possibles avant toute nouvelle extension ou proposition de plafond ; la validation native dense reste un obstacle séparé.
57. **[R-018 Distribution — signature Developer ID et notarisation](roadmap/items/R-018-distribution-signature-developer-id-et-notarisation.md)** · Idée · jalon distribution
   Signer et notariser le paquet macOS ; ces étapes restent ouvertes.
58. **[R-019 Distribution — Windows et Linux](roadmap/items/R-019-distribution-windows-et-linux.md)** · Idée · jalon distribution
   Construire et qualifier la distribution sur Windows et Linux.
59. **[R-021 Étudier la réutilisation entre instances 3MF](roadmap/items/R-021-etudier-la-reutilisation-entre-instances-3mf.md)** · Idée · jalon audit
   AUD-006 — Démontrer un gain sur plusieurs instances ; établir un contrat tenant compte des transformations, de l’arrondi, du soudage et de la provenance avant tout cache
60. **[R-055 3MF Production — références multi-modèles](roadmap/items/R-055-3mf-production-references-multi-modeles.md)** · Idée · jalon post-v1
   Étendre l’import au-delà de 3MF Core ; les références Production restent actuellement refusées.
61. **[R-084 C2b — reconstruire une révolution à profil cubique](roadmap/items/R-084-c2b-reconstruire-une-revolution-a-profil-cubique.md)** · À tester · jalon post-v1
   Un profil lisse cubique borné, deux disques source et un tour complet ; preuve continue, STEP contrôlé et famille Révolution explicite.

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
- [ ] [A-031](roadmap/actions/A-031-examiner-la-figure-de-la-bride-percee-de-fabstep.md) Examiner la figure de la bride percée de FabSTEP (tester)
- [ ] [A-033](roadmap/actions/A-033-verifier-la-correction-contextuelle-et-la.md) Vérifier la correction contextuelle et la lisibilité de FabSTEP (tester, R-042)
- [ ] [A-034](roadmap/actions/A-034-choisir-la-portee-de-la-selection-pour-tore.md) Choisir la portée de la sélection pour Tore (decider, R-050)
- [ ] [A-035](roadmap/actions/A-035-essayer-plan-dans-la-correction-guidee.md) Essayer Plan dans la correction guidée (tester, R-056)
- [ ] [A-036](roadmap/actions/A-036-verifier-le-step-apres-correction-plane-dans.md) Vérifier le STEP après correction plane dans Fusion (tester, R-056)
- [ ] [A-037](roadmap/actions/A-037-essayer-la-selection-rapide-et-la-correction.md) Essayer la sélection rapide et la correction automatique (tester, R-057)
- [ ] [A-038](roadmap/actions/A-038-verifier-tore-et-le-step-automatique-dans-fusion.md) Vérifier Tore et le STEP automatique dans Fusion (tester, R-057)
- [ ] [A-039](roadmap/actions/A-039-essayer-la-selection-de-faces-en-maintenant-le.md) Essayer la sélection de faces en maintenant le clic (tester, R-057)
- [ ] [A-041](roadmap/actions/A-041-verifier-les-raccords-de-rainures-dans-fusion.md) Vérifier les raccords de rainures dans Fusion (tester, R-058)
- [ ] [A-042](roadmap/actions/A-042-examiner-les-chaines-de-chanfreins-de-meme-signe.md) Examiner les chaînes de chanfreins de même signe (tester, R-032)
- [ ] [A-043](roadmap/actions/A-043-reimporter-une-chaine-conique-dans-fusion.md) Réimporter une chaîne conique dans Fusion (tester, R-032)
- [ ] [A-044](roadmap/actions/A-044-verifier-l-inspecteur-et-la-correction.md) Vérifier l’inspecteur et la correction simplifiés (tester, R-042)
- [ ] [A-046](roadmap/actions/A-046-essayer-les-deux-cones-directement-raccordes.md) Essayer les deux cônes directement raccordés (tester, R-060)
- [ ] [A-047](roadmap/actions/A-047-verifier-les-cones-raccordes-dans-fusion.md) Vérifier les cônes raccordés dans Fusion (tester, R-060)
- [ ] [A-048](roadmap/actions/A-048-examiner-les-deux-arrondis-toriques.md) Examiner les deux arrondis toriques (tester, R-061)
- [ ] [A-049](roadmap/actions/A-049-verifier-les-deux-tores-dans-fusion.md) Vérifier les deux tores dans Fusion (tester, R-061)
- [ ] [A-050](roadmap/actions/A-050-examiner-le-bac-ouvert-et-ses-arrondis.md) Examiner le bac ouvert et ses arrondis (tester, R-062)
- [ ] [A-051](roadmap/actions/A-051-verifier-le-bac-ouvert-dans-fusion.md) Vérifier le bac ouvert dans Fusion (tester, R-062)
- [ ] [A-052](roadmap/actions/A-052-essayer-le-lancement-manuel-du-step.md) Essayer le lancement manuel du STEP (tester, R-064)
- [ ] [A-053](roadmap/actions/A-053-verifier-le-suivi-de-reconstruction.md) Vérifier le suivi de reconstruction (tester, R-065)
- [ ] [A-054](roadmap/actions/A-054-verifier-le-compte-du-stl-trop-dense.md) Vérifier le compte du STL trop dense (tester, R-066)
- [ ] [A-056](roadmap/actions/A-056-essayer-et-annuler-un-gros-stl.md) Essayer et annuler un gros STL (tester, R-067)
- [ ] [A-057](roadmap/actions/A-057-verifier-un-step-issu-du-mode-gros-stl.md) Vérifier un STEP issu du mode gros STL (tester, R-067)
- [ ] [A-058](roadmap/actions/A-058-verifier-le-passage-des-contours-du-gros-stl.md) Vérifier le passage des contours du gros STL avec G2 (tester, R-068)
- [ ] [A-059](roadmap/actions/A-059-verifier-le-gros-stl-apres-l-optimisation-g3.md) Vérifier le gros STL après l’optimisation G3 (tester, R-069)
- [ ] [A-060](roadmap/actions/A-060-verifier-le-gros-stl-apres-g4.md) Vérifier le gros STL après G4 (tester, R-070)
- [ ] [A-061](roadmap/actions/A-061-verifier-flansch-apres-l-optimisation-des-arcs.md) Vérifier Flansch après l’optimisation des arcs complexes (tester, R-071)
- [ ] [A-063](roadmap/actions/A-063-verifier-l-annulation-apres-la-correction-du.md) Vérifier l’annulation après la correction du suivi mémoire (tester, R-073)
- [ ] [A-065](roadmap/actions/A-065-examiner-le-raccord-tore-cone-cylindre-dans.md) Examiner le raccord tore–cône–cylindre dans FabSTEP (tester, R-075)
- [ ] [A-066](roadmap/actions/A-066-verifier-le-raccord-tore-cone-cylindre-dans.md) Vérifier le raccord tore–cône–cylindre dans Fusion (tester, R-075)
- [ ] [A-067](roadmap/actions/A-067-examiner-le-bilan-certifie-des-percages-secants.md) Examiner le bilan certifié des perçages sécants (valider, R-063)
- [ ] [A-069](roadmap/actions/A-069-examiner-les-nouveaux-raccords-de-revolution.md) Examiner les nouveaux raccords de révolution (tester, R-076)
- [ ] [A-070](roadmap/actions/A-070-verifier-les-nouveaux-raccords-dans-fusion.md) Vérifier les nouveaux raccords dans Fusion (tester, R-076)
- [ ] [A-071](roadmap/actions/A-071-examiner-le-bac-perce-dans-fabstep.md) Examiner le bac percé dans FabSTEP (tester, R-077)
- [ ] [A-072](roadmap/actions/A-072-verifier-le-bac-perce-dans-fusion.md) Vérifier le bac percé dans Fusion (tester, R-077)
- [ ] [A-073](roadmap/actions/A-073-examiner-les-chaines-coniques-adjacentes-et.md) Examiner les chaînes coniques adjacentes et mixtes (tester, R-078)
- [ ] [A-074](roadmap/actions/A-074-verifier-les-chaines-adjacentes-dans-fusion.md) Vérifier les chaînes adjacentes dans Fusion (tester, R-078)
- [ ] [A-075](roadmap/actions/A-075-examiner-le-coin-local-a-trois-arrondis.md) Examiner le coin local à trois arrondis (tester, R-081)
- [ ] [A-076](roadmap/actions/A-076-verifier-le-coin-a-trois-arrondis-dans-fusion.md) Vérifier le coin à trois arrondis dans Fusion (tester, R-081)
- [ ] [A-077](roadmap/actions/A-077-examiner-le-raccord-a-rayon-lineaire-dans.md) Examiner le raccord à rayon linéaire dans FabSTEP (tester, R-082)
- [ ] [A-078](roadmap/actions/A-078-verifier-les-raccords-elliptiques-dans-fusion.md) Vérifier les raccords elliptiques dans Fusion (tester, R-082)
- [ ] [A-079](roadmap/actions/A-079-examiner-les-percages-secants-dans-fabstep.md) Examiner les perçages sécants dans FabSTEP (tester, R-083)
- [ ] [A-080](roadmap/actions/A-080-verifier-les-percages-secants-dans-fusion.md) Vérifier les perçages sécants dans Fusion (tester, R-083)
- [ ] [A-081](roadmap/actions/A-081-examiner-les-revolutions-cubiques-dans-fabstep.md) Examiner les révolutions cubiques dans FabSTEP (tester, R-084)
- [ ] [A-082](roadmap/actions/A-082-verifier-les-revolutions-cubiques-dans-fusion.md) Vérifier les révolutions cubiques dans Fusion (tester, R-084)

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

### post-v1 : Géométries après v1

- [R-079 D2–G0 — qualifier les raccords locaux et le rayon linéaire](roadmap/items/R-079-d2-g0-qualifier-les-raccords-locaux-et-le-rayon.md) · livrée le 2026-11-17
  Établir les preuves et le contrat minimum pour un coin à trois arrondis et une enveloppe à rayon linéaire avant intégration.
- [R-074 Qualifier les grandes zones courbes de Flansch](roadmap/items/R-074-qualifier-les-grandes-zones-courbes-de-flansch.md) · livrée le 2026-11-17
  Mesurer les quatre profils courbes encore refusés, identifier les familles géométriques et les preuves manquantes, puis définir une extension bornée sans relâcher les tolérances.
- [R-072 Qualifier les résultats très facettisés des gros STL](roadmap/items/R-072-qualifier-les-resultats-tres-facettises-des-gros.md) · livrée le 2026-11-17
  Flansch franchit désormais la reconnaissance avec 4 cylindres et 8 cônes, mais conserve 501 900 plans, soit 501 912 faces au total. Le suivi du résultat est limité à 100 000 faces. Identifier les zones qui restent facettisées et les réductions géométriquement prouvées possibles ; examiner les coûts CAO, mémoire et inspection avant toute proposition de changement de limite ou de simplification.
- [R-058 Raccords ouverts cylindre–chanfrein](roadmap/items/R-058-raccords-ouverts-cylindre-chanfrein.md) · livrée le 2026-11-17
  Reconstruire ensemble les parois cylindriques ouvertes et leurs chanfreins coniques partiels, avec contours partagés et validation du solide entier.
