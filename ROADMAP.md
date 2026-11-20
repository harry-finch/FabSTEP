<!-- Généré par FabBoard depuis roadmap/ : ne pas modifier à la main. -->

Les jalons décrivent des critères de sortie, pas des délais garantis. La v1 mécanique et son paquet macOS local ont été acceptés ; les extensions post-v1 livrées appartiennent au build de développement. Le 19 novembre 2026, l’utilisateur a validé en bloc les tests en attente et les lots présentés à la recette. Le volet Flansch de R-085 reste en développement.

L’[ancienne feuille de route](docs/ROADMAP-avant-fabboard.md) est conservée intégralement pour l’histoire. Elle contient les tableaux, critères et décisions d’origine. Les textes de chaque lot sont aussi conservés dans les éléments. Le [registre INT-AUDIT-01](docs/INT_AUDIT_01.md) garde autorité sur les suites de l’audit ; la [recherche post-v1](docs/POST_V1_RESEARCH.md) détaille les contrats géométriques.

Avant la distribution (R-019), les nouveaux travaux de reconnaissance commencent avec les deux premiers témoins du corpus public R-122 et la fusion plane R-123. R-124 à R-129 étendent ensuite la reconnaissance à plusieurs familles. R-130 et R-131 se déclenchent si les mesures justifient de nouveaux essais du contrôle OCCT. A-136 conserve le plafond de 30 minutes dans tous les modes et clôt R-132.

Le prochain essai de Flansch aura lieu après la fin des développements de reconnaissance géométrique. Les qualifications intermédiaires utilisent de petits solides indépendants. Les prochains lots portent sur une famille de formes utilisable sur plusieurs pièces. Pour une amélioration issue de Flansch, vérifier aussi une pièce indépendante via la conversion STL → STEP de l’application. Suivre le résultat de ces essais dans le chantier concerné (chaînes coniques, révolutions courbes, congés, boîtiers ou perçages).

**Avancement : 102 livrés sur 131** (78 %)

## Restant à faire

1. **[R-115 Récupérer les plans fragmentés par la quantification du STL](roadmap/items/R-115-recuperer-les-plans-fragmentes-par-la.md)** · À tester · jalon v1.0
   Le bloc tourné et la pièce pliée donnent respectivement six et sept faces planes en Auto. Les 96 triangles de chaque STL conservent leur attribution après relecture du STEP.
2. **[R-116 Estimer les normales sur copie Taubin en lecture seule](roadmap/items/R-116-estimer-les-normales-sur-copie-taubin-en-lecture.md)** · À tester · jalon v1.0
   Calculer des normales lissées sur une copie temporaire sans déplacer les sommets source, uniquement pour stabiliser la segmentation et la découverte de candidats sur les STL quantifiés ou bruités.
3. **[R-119 Isoler une zone interne par boîte de découpe visuelle 3D](roadmap/items/R-119-isoler-une-zone-interne-par-boite-de-decoupe.md)** · À tester · jalon atelier
   Ajouter une boîte de découpe visuelle manipulable dans la vue 3D (Source et Résultat STEP) pour inspecter, sélectionner et corriger des cavités internes sans modifier le solide ni l'export.
4. **[R-120 Proposer une correction guidée sur les parois similaires](roadmap/items/R-120-proposer-une-correction-guidee-sur-les-parois.md)** · À tester · jalon atelier
   Après une correction guidée acceptée, rechercher en lecture seule les parois géométriquement analogues du document, les prévisualiser puis appliquer celles qui passent les preuves strictes après accord explicite.
5. **[R-121 Afficher la carte de chaleur d'écart 3D sur le résultat STEP](roadmap/items/R-121-afficher-la-carte-de-chaleur-d-ecart-3d-sur-le.md)** · À tester · jalon atelier
   Superposer dans la vue Résultat STEP une coloration par écart échantillonné au maillage source avec légende en millimètres, calculée à la demande en lecture seule sans affecter l'export.
6. **[R-024 Préserver les caractères composés dans les noms de produits tronqués](roadmap/items/R-024-preserver-les-caracteres-composes-dans-les-noms-de-produits-tronques.md)** · À tester · jalon audit
   AUD-011 — Reproduire les cas Unicode utiles ; conserver le nom source et éviter une dépendance disproportionnée pour ce seul besoin
7. **[R-022 Réduire le coût de démarrage du frontend](roadmap/items/R-022-reduire-le-cout-de-demarrage-du-frontend.md)** · À tester · jalon audit
   AUD-009 — Mesurer le démarrage réel de la WebView ; retenir un chargement différé ou un découpage uniquement si le gain est démontré
8. **[R-049 I3 — éditer les indications](roadmap/items/R-049-i3-editer-les-indications.md)** · À tester · jalon atelier
   Modifier ou retirer une indication, annuler/rétablir et comparer le résultat calculé au maillage source
9. **[R-025 Découper les gros modules et mutualiser les helpers au besoin](roadmap/items/R-025-decouper-les-gros-modules-et-mutualiser-les-helpers-au-besoin.md)** · À tester · jalon audit
   AUD-021 — Intervenir par module lors d’un changement fonctionnel justifié, avec tests ; aucune refonte globale fondée uniquement sur le nombre de lignes
10. **[R-023 Durcir les permissions locales Tauri](roadmap/items/R-023-durcir-les-permissions-locales-tauri.md)** · À tester · jalon audit
   AUD-014 — Définir le manifeste applicatif et tester les commandes autorisées/refusées, fenêtres et origines ; ne pas ajouter un fichier de capacités supposé suffisant
11. **[R-117 Proposer des cylindres, cônes et sphères par normales sur bandes résiduelles](roadmap/items/R-117-proposer-des-cylindres-cones-et-spheres-par.md)** · À tester · jalon v1.0
   Utiliser les rayons de normales et l'angle de normale constant sur les chaînes de bandes non planes pour découvrir des parois interrompues, en conservant toutes les preuves géométriques existantes.
12. **[R-113 Qualifier les contours obliques et les parois de révolution interrompues](roadmap/items/R-113-qualifier-les-contours-obliques-et-les-parois-de.md)** · À tester · jalon v1.0
   La lèvre torique est qualifiée à deux densités. Sur trois petits STL d’arbres percés, le prototype mesure deux cylindres et produit un STEP valide à quatre faces.
13. **[R-114 Composer deux reconnaissances locales sur un même solide](roadmap/items/R-114-composer-deux-reconnaissances-locales-sur-un.md)** · À tester
   Qualifier un témoin mêlant deux familles déjà reconnues et un témoin à revendications incompatibles. Définir la propriété des triangles et des arêtes communes avant de modifier Auto.
14. **[R-085 Reconstruire les raccords toriques partiels du pourtour de Flansch](roadmap/items/R-085-reconstruire-les-raccords-toriques-partiels-du.md)** · En cours · jalon v1.0
   Les 46 671 faces passent le contrôle géométrique en laboratoire. Le rejeu de Flansch est reporté après les développements de reconnaissance pour limiter la RAM.
15. **[R-122 Mesurer la reconnaissance sur un corpus public reproductible](roadmap/items/R-122-mesurer-la-reconnaissance-sur-un-corpus-public.md)** · À tester · jalon post-v1
   G7 · Établir une référence sur une douzaine de pièces de plusieurs familles, puis la réutiliser à chaque extension.
16. **[R-123 Fusionner les faces planes voisines des raccords toriques](roadmap/items/R-123-fusionner-les-faces-planes-voisines-des-raccords.md)** · En cours · jalon post-v1
   F1 · Mesurer puis réduire les faces résiduelles et les chaînes de petits raccords sur deux pièces publiques.
17. **[R-126 Reconstruire une révolution à méridienne mixte](roadmap/items/R-126-reconstruire-une-revolution-a-meridienne-mixte.md)** · En cours · jalon post-v1
   G4 · Reconstituer sur un axe commun des portées, des gorges circulaires et des portions de profil cubiques.
18. **[R-128 Proposer des axes et des motifs de parois répétées](roadmap/items/R-128-proposer-des-axes-et-des-motifs-de-parois.md)** · En cours · jalon post-v1
   G6 · Accélérer la recherche des familles et le guidage en utilisant les normales et la répétition des formes.
19. **[R-129 Reconstruire des congés sur arête courbe et des coins à trois rayons](roadmap/items/R-129-reconstruire-des-conges-sur-arete-courbe-et-des.md)** · En cours · jalon post-v1
   G5 · Qualifier d’abord le congé de rayon constant sur arête courbe, puis le raccord entre trois congés inégaux.
20. **[R-127 Reconstruire une extrusion droite à profil libre](roadmap/items/R-127-reconstruire-une-extrusion-droite-a-profil-libre.md)** · En cours · jalon post-v1
   G3 · Reconnaître un contour extrudé composé de segments, d’arcs et de portions cubiques.
21. **[R-130 Qualifier deux contrôles d’interférence simultanés](roadmap/items/R-130-qualifier-deux-controles-d-interference.md)** · En cours · jalon audit
   F2 · Mesurer le temps et la mémoire cumulée du helper avant d’adopter la concurrence entre groupes.
22. **[R-124 Composer des régions qui partagent un bord vérifié](roadmap/items/R-124-composer-des-regions-qui-partagent-un-bord.md)** · Spécifiée · jalon post-v1
   G1 · Décrire les voisinages et les triangles de chaque région afin de combiner plusieurs reconnaissances sur un même corps.
23. **[R-125 Étendre l’atlas aux parois analytiques interrompues](roadmap/items/R-125-etendre-l-atlas-aux-parois-analytiques.md)** · Spécifiée · jalon post-v1
   G2 et suite de F1 · Partager des contours vérifiés entre tores, plans et parois cylindriques, puis étendre famille par famille.
24. **[R-131 Réévaluer sur preuve le découpage et le filtrage des groupes OCCT](roadmap/items/R-131-reevaluer-sur-preuve-le-decoupage-et-le-filtrage.md)** · Spécifiée · jalon audit
   F3 et F4 · Rouvrir séparément les deux essais de V-1276 si un groupe chaud représentatif permet de démontrer un gain.
25. **[R-019 Distribution — Windows et Linux](roadmap/items/R-019-distribution-windows-et-linux.md)** · Idée · jalon distribution
   Construire et qualifier la distribution sur Windows et Linux.
26. **[R-021 Étudier la réutilisation entre instances 3MF](roadmap/items/R-021-etudier-la-reutilisation-entre-instances-3mf.md)** · Idée · jalon audit
   AUD-006 — Démontrer un gain sur plusieurs instances ; établir un contrat tenant compte des transformations, de l’arrondi, du soudage et de la provenance avant tout cache
27. **[R-055 3MF Production — références multi-modèles](roadmap/items/R-055-3mf-production-references-multi-modeles.md)** · Idée · jalon post-v1
   Étendre l’import au-delà de 3MF Core ; les références Production restent actuellement refusées.
28. **[R-118 Exporter un rapport Markdown en CLI et exposer une passerelle MCP locale](roadmap/items/R-118-exporter-un-rapport-markdown-en-cli-et-exposer.md)** · Idée · jalon post-v1
   Ajouter une sortie Markdown lisible à fabstep-cli (--report) et un wrapper MCP optionnel appelant la CLI locale, sans modifier le moteur ni l'atelier bureau.
29. **[R-018 Distribution — signature Developer ID et notarisation](roadmap/items/R-018-distribution-signature-developer-id-et-notarisation.md)** · Idée · jalon distribution
   Signer et notariser le paquet macOS ; ces étapes restent ouvertes.

## Actions à faire

- [ ] [A-124](roadmap/actions/A-124-verifier-les-six-et-sept-faces-des-deux-stl.md) Vérifier les six et sept faces des deux STL tournés (tester, R-115)
- [ ] [A-126](roadmap/actions/A-126-controler-le-toit-quantifie-a-six-faces-dans.md) Contrôler le toit quantifié à six faces dans Fusion (tester, R-116)
- [ ] [A-127](roadmap/actions/A-127-isoler-une-paroi-interne-avec-la-boite-de.md) Isoler une paroi interne avec la boîte de découpe (tester, R-119)
- [ ] [A-128](roadmap/actions/A-128-lire-les-ecarts-sur-le-bossage-borgne.md) Lire les écarts sur le bossage borgne (tester, R-121)
- [ ] [A-129](roadmap/actions/A-129-propager-une-correction-aux-trois-trous-de-meme.md) Propager une correction aux trois trous de même rayon (tester, R-120)
- [ ] [A-130](roadmap/actions/A-130-essayer-l-historique-des-corrections-sur-une.md) Essayer l’historique des corrections sur une pièce (tester, R-049)
- [ ] [A-131](roadmap/actions/A-131-verifier-les-dialogues-natifs-d-ouverture-et-d.md) Vérifier les dialogues natifs d’ouverture et d’export (tester, R-023)
- [ ] [A-132](roadmap/actions/A-132-verifier-le-nom-3mf-tronque-dans-la-cao.md) Vérifier le nom 3MF tronqué dans la CAO (tester, R-024)
- [ ] [A-133](roadmap/actions/A-133-verifier-les-deux-parois-de-l-arbre-perce.md) Vérifier les deux parois de l’arbre percé (tester, R-113)
- [ ] [A-134](roadmap/actions/A-134-verifier-le-conge-et-les-percages-sur-le-meme.md) Vérifier le congé et les perçages sur le même bloc (tester, R-114)
- [ ] [A-135](roadmap/actions/A-135-verifier-l-octant-spherique-et-le-secteur.md) Vérifier l’octant sphérique et le secteur conique (tester, R-117)

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

- [R-112 Intégrer les recommandations NEON vérifiées](roadmap/items/R-112-integrer-les-recommandations-neon-verifiees.md) · livrée le 2026-11-19
  Suivi des seize constats et des reports dans docs/INT_AUDIT_01.md, section NEON. Contrôle atlas restant lié à R-085.
- [R-026 Qualifier le garde de cocircularité des anneaux fermés](roadmap/items/R-026-qualifier-le-garde-de-cocircularite-des-anneaux-fermes.md) · livrée le 2026-11-19
  Le défaut de conditionnement a été reproduit puis corrigé dans R-070. Les anneaux clairsemés et les profils en D sont couverts par les tests de V-1194.
- [R-051 PERF1 — preuves source réutilisées](roadmap/items/R-051-perf1-preuves-source-reutilisees.md) · livrée le 2026-11-10
  Cache privé et exact à chaque recherche ; mesures avant/après et équivalence des résultats consignées.
- [R-020 INT-AUDIT-01 — intégration initiale](roadmap/items/R-020-int-audit-01-integration-initiale.md) · livrée le 2026-11-10
  Cinq jalons validés ; huit recommandations traitées, preuves et suites dans le registre.

### post-v1 : Géométries après v1

- [R-111 C2 — méridiennes cubiques à retour axial](roadmap/items/R-111-c2-meridiennes-cubiques-a-retour-axial.md) · livrée le 2026-11-19
  Reconnaître un retour axial sur un profil de rayon monotone et conserver les deux disques mesurés.
- [R-110 Quatre congés reliés autour du fond d’une poche](roadmap/items/R-110-quatre-conges-relies-autour-du-fond-d-une-poche.md) · livrée le 2026-11-19
  Reconstituer le circuit fermé de quatre cylindres autour d’un fond rectangulaire mesuré.
- [R-109 Deux congés variables sur des arêtes gauches](roadmap/items/R-109-deux-conges-variables-sur-des-aretes-gauches.md) · livrée le 2026-11-19
  Reconstruire deux congés à rayon affine portés par deux arêtes disjointes non parallèles du même bloc.
- [R-108 Perçages obliques décalés suivant la normale commune](roadmap/items/R-108-percages-obliques-decales-suivant-la-normale.md) · livrée le 2026-11-19
  Reconstruire deux perçages dont les axes sont inclinés et décalés latéralement, à quatre bouches circulaires.
- [R-107 Trois congés en U au fond d’une poche](roadmap/items/R-107-trois-conges-en-u-au-fond-d-une-poche.md) · livrée le 2026-11-19
  Reconstruire trois congés cylindriques reliés par deux ellipses au fond de la même poche.
- [R-106 Révolution à deux cubiques sur un maillage irrégulier](roadmap/items/R-106-revolution-a-deux-cubiques-sur-un-maillage.md) · livrée le 2026-11-19
  Reconstruire une méridienne à deux segments tangents dont le raccord se trouve entre deux hauteurs du STL.
- [R-105 F — Deux perçages sécants à axes obliques](roadmap/items/R-105-f-deux-percages-secants-a-axes-obliques.md) · livrée le 2026-11-19
  Reconstruire les deux intersections transverses de perçages dont les axes se croisent dans un prisme à quatre bouches circulaires.
- [R-104 C2 — Révolution cubique sur une tessellation irrégulière](roadmap/items/R-104-c2-revolution-cubique-sur-une-tessellation.md) · livrée le 2026-11-19
  Reconstruire une méridienne cubique lorsque le maillage FreeCAD ne présente pas d’anneaux intérieurs complets.
- [R-103 E — Rebord circulaire arrondi des deux côtés](roadmap/items/R-103-e-rebord-circulaire-arrondi-des-deux-cotes.md) · livrée le 2026-11-19
  Reconstruire les deux congés d’un rebord de récipient ouvert, y compris le quart de tore qui rejoint le cylindre par sa branche radiale intérieure.
- [R-102 G — Deux congés à rayon variable sur un bloc](roadmap/items/R-102-g-deux-conges-a-rayon-variable-sur-un-bloc.md) · livrée le 2026-11-19
  Reconstruire deux congés de lois linéaires distinctes sur des arêtes parallèles opposées.
- [R-101 D — Congés adjacents à angle aigu ou obtus](roadmap/items/R-101-d-conges-adjacents-a-angle-aigu-ou-obtus.md) · livrée le 2026-11-19
  Reconstruire deux congés de poche de même rayon partageant une arête elliptique sur la bissectrice du coin.
- [R-100 E — Bac à rebord et deux cavités ouvertes](roadmap/items/R-100-e-bac-a-rebord-et-deux-cavites-ouvertes.md) · livrée le 2026-11-19
  Conserver le rebord en saillie et deux cavités de profondeur différente séparées par une cloison.
- [R-099 C2 — Révolution cubique terminée sur un sommet axial](roadmap/items/R-099-c2-revolution-cubique-terminee-sur-un-sommet.md) · livrée le 2026-11-19
  Reconstruire une méridienne cubique entre un disque et une pointe présente dans le maillage.
- [R-098 E — Bac arrondi percé sur le fond et la paroi](roadmap/items/R-098-e-bac-arrondi-perce-sur-le-fond-et-la-paroi.md) · livrée le 2026-11-19
  Qualifier un bac ouvert combinant trois perçages traversants et huit congés verticaux.
- [R-097 D/E — Congé de poche entre deux butées obliques](roadmap/items/R-097-d-e-conge-de-poche-entre-deux-butees-obliques.md) · livrée le 2026-11-19
  Reconstruire une paroi cylindrique concave tangente et ses deux bords elliptiques dans une poche borgne.
- [R-096 F — perçages traversants orthogonaux à axes décentrés](roadmap/items/R-096-f-percages-traversants-orthogonaux-a-axes.md) · livrée le 2026-11-19
  Reconstruire les deux intersections partagées lorsque le petit perçage est décentré par rapport au grand.
- [R-095 C2 — révolution à deux méridiennes cubiques tangentes](roadmap/items/R-095-c2-revolution-a-deux-meridiennes-cubiques.md) · livrée le 2026-11-19
  Reconstruire une paroi de révolution dont la méridienne comporte deux segments cubiques à tangente commune.
- [R-094 G — congé à rayon variable maillé entre ses deux extrémités](roadmap/items/R-094-g-conge-a-rayon-variable-maille-entre-ses-deux.md) · livrée le 2026-11-19
  Retrouver le congé linéaire sur une paroi dont les sommets appartiennent tous aux deux bords elliptiques.
- [R-093 Auto — reconnaître le trou traversant d’une plaque inclinée](roadmap/items/R-093-auto-reconnaitre-le-trou-traversant-d-une-plaque.md) · livrée le 2026-11-19
  Sur la plaque inclinée de 268 triangles, le mode Automatique reconstruit six faces planes et un cylindre de Ø6 mm. Sélectionner la paroi retrouve ses 126 triangles source.
- [R-092 F3 — reconnaître les perçages du bloc incliné en STL binaire](roadmap/items/R-092-f3-reconnaitre-les-percages-du-bloc-incline-en.md) · livrée le 2026-11-19
  Deux poses et deux densités de STL binaire inclinés donnent six plans et trois cylindres aux réglages par défaut. Vérification Fusion ouverte.
- [R-091 F2 — perçages orthogonaux de rayons plus proches](roadmap/items/R-091-f2-percages-orthogonaux-de-rayons-plus-proches.md) · livrée le 2026-11-19
  Deux trous orthogonaux de rayons plus proches reconstruits. Sept STEP contrôlés dans FreeCAD, y compris quatre maillages indépendants. Essai FabSTEP et Fusion ouvert.
- [R-090 D/E — congés adjacents de rayons différents](roadmap/items/R-090-d-e-conges-adjacents-de-rayons-differents.md) · livrée le 2026-11-19
  Deux congés adjacents de rayons distincts dans une poche rectangulaire à butées droites. Auto produit deux faces cylindriques sélectionnables et conserve l’arc de prolongement en DXF.
- [R-089 D/E — jonction de deux congés adjacents de même rayon](roadmap/items/R-089-d-e-jonction-de-deux-conges-adjacents-de-meme.md) · livrée le 2026-11-19
  Dans un coin de poche à 90°, Auto reconstruit les deux congés de même rayon et conserve leur arête elliptique commune.
- [R-088 D/E — poche à deux congés de fond opposés](roadmap/items/R-088-d-e-poche-a-deux-conges-de-fond-opposes.md) · livrée le 2026-11-19
  Reconstruire et mesurer séparément les deux arrondis parallèles d’une poche borgne, y compris lorsque leurs rayons diffèrent.
- [R-087 D/E — congé de poche arrêté par une paroi oblique](roadmap/items/R-087-d-e-conge-de-poche-arrete-par-une-paroi-oblique.md) · livrée le 2026-11-19
  Auto reconstruit le congé cylindrique entre une butée droite et une paroi oblique. Les profils de face conservent le cercle et l’ellipse.
- [R-086 D/E — poche borgne à congé de fond arrêté](roadmap/items/R-086-d-e-poche-borgne-a-conge-de-fond-arrete.md) · livrée le 2026-11-19
  Auto reconstruit un congé intérieur à deux butées, même quand ses extrémités comportent des nombres de facettes différents.
- [R-084 C2b — reconstruire une révolution à profil cubique](roadmap/items/R-084-c2b-reconstruire-une-revolution-a-profil-cubique.md) · livrée le 2026-11-19
  Un profil lisse cubique borné, deux disques source et un tour complet ; preuve continue, STEP contrôlé et famille Révolution explicite.
- [R-083 F1 — reconnaître les perçages sécants depuis le STL](roadmap/items/R-083-f1-reconnaitre-les-percages-secants-depuis-le.md) · livrée le 2026-11-19
  Prouver deux perçages centrés orthogonaux, leurs limites communes et leur reconstruction STEP, dans le domaine déjà autorisé par A-045.
- [R-082 G1 — reconstruire un raccord à rayon linéaire](roadmap/items/R-082-g1-reconstruire-un-raccord-a-rayon-lineaire.md) · livrée le 2026-11-19
  Reconnaître l’enveloppe conique inclinée à extrémités elliptiques d’un arrondi linéaire, sans la confondre avec des sections circulaires empilées.
- [R-081 D2 — reconstruire un coin local à trois arrondis](roadmap/items/R-081-d2-reconstruire-un-coin-local-a-trois-arrondis.md) · livrée le 2026-11-19
  Intégrer l’octant sphérique et ses trois raccords cylindriques à partir des frontières STL, avec preuve continue et STEP validé.
- [R-080 Qualifier les 231 584 plans restants de Flansch](roadmap/items/R-080-qualifier-les-231-584-plans-restants-de-flansch.md) · livrée le 2026-11-19
  Après les quatre grandes zones reconstruites, Flansch conserve 231 584 faces planes et 24 surfaces analytiques. Identifier les zones encore facettisées et leurs familles possibles avant toute nouvelle extension ou proposition de plafond ; la validation native dense reste un obstacle séparé.
- [R-078 C1b2b — chaînes coniques adjacentes et mixtes](roadmap/items/R-078-c1b2b-chaines-coniques-adjacentes-et-mixtes.md) · livrée le 2026-11-19
  Reconstruire deux à huit cônes adjacents ou mêlés à des portées, avec une décomposition unique prouvée par les anneaux source.
- [R-077 E2 — bac ouvert avec perçage du fond](roadmap/items/R-077-e2-bac-ouvert-avec-percage-du-fond.md) · livrée le 2026-11-19
  Qualifier un bac à coins arrondis avec un trou traversant le fond, sans remplir la cavité ni perdre les parois.
- [R-076 C2b — chaînes toriques composées et arcs au-delà de 90°](roadmap/items/R-076-c2b-chaines-toriques-composees-et-arcs-au-dela.md) · livrée le 2026-11-19
  Reconstruire les trois autres zones courbes mesurées sur Flansch : raccord à deux cônes et arcs de 100°, avec frontières source et preuves continues.
- [R-075 Reconstruire un raccord tore–cône–cylindre prouvé](roadmap/items/R-075-reconstruire-un-raccord-tore-cone-cylindre.md) · livrée le 2026-11-19
  Première extension issue de R-074 : reconstruire la zone de 64 080 triangles par trois surfaces, avec raccord tangent prouvé et bandes à deux anneaux. Contrat de données inchangé ; règle de preuve approuvée pour implémentation, sans promesse de convertir Flansch entier.
- [R-073 Qualifier la surveillance mémoire des gros STL sur macOS](roadmap/items/R-073-qualifier-la-surveillance-memoire-des-gros-stl.md) · livrée le 2026-11-19
  La sonde R-072 rapporte une empreinte physique de 6.9G malgré un RSS maximal échantillonné de 2,35 Gio. Comparer les métriques macOS et qualifier une surveillance adaptée avant toute augmentation de capacité ; préserver arrêt et nettoyage du helper.
- [R-071 Accélérer les fenêtres d’arcs des contours complexes](roadmap/items/R-071-accelerer-les-fenetres-d-arcs-des-contours.md) · livrée le 2026-11-19
  Après G4, Flansch franchit le cercle dense mais épuise encore le budget sur la face 7 278, contour de 300 points réellement non circulaire. Étudier une réduction prouvée du coût des fenêtres, sans supposer la monotonie des ajustements ni augmenter le budget ; une nouvelle preuve numérique exige une qualification distincte.
- [R-070 G4 — borner la recherche des arcs ouverts denses](roadmap/items/R-070-g4-borner-la-recherche-des-arcs-ouverts-denses.md) · livrée le 2026-11-19
  Après G3, le fichier Flansch de 540 170 triangles s’arrête dans partial::candidates sur le budget partagé de reconnaissance. Mesurer la consommation avant cette recherche et ses fenêtres, puis étudier une réduction prouvée du travail ; les ajustements contraints par les extrémités ne sont pas monotones. Ne pas sauter des candidats ni relever le budget sans preuve.
- [R-069 G3 — réduire le coût de reconnaissance des maillages denses](roadmap/items/R-069-g3-reduire-le-cout-de-reconnaissance-des.md) · livrée le 2026-11-19
  Préparation mono-corps réutilisée et recherche d’arêtes limitée aux patchs, sans relâcher les preuves. Sur le STL de 540 170 triangles, analyse 99→51 s avec égalité bit à bit ; essai 111→66 s, encore refusé dans la recherche d’arcs ouverts (R-070), sans STEP. Build et tests réussis.
- [R-068 G2 — accélérer la vérification des contours denses](roadmap/items/R-068-g2-accelerer-la-verification-des-contours-denses.md) · livrée le 2026-11-19
  Filtrage spatial borné des contours intégré et testé, avec prédicats inchangés. Le fichier de 540 170 triangles franchit ce contrôle puis atteint le budget de reconnaissance (R-069) ; aucun STEP produit. Build de développement mis à jour, qualification utilisateur ouverte.
- [R-067 G1 — gros maillages sur accord et calcul annulable](roadmap/items/R-067-g1-gros-maillages-sur-accord-et-calcul-annulable.md) · livrée le 2026-11-19
  Autoriser un essai explicite au-delà de 100 000 triangles, avec avertissement, limites distinctes et arrêt du calcul ; qualifier d’abord le STL de 540 170 triangles.
- [R-063 F0 — qualifier les courbes des perçages sécants](roadmap/items/R-063-f0-qualifier-les-courbes-des-percages-secants.md) · livrée le 2026-11-19
  Vérifier dans un laboratoire natif borné les courbes et paramètres de deux cylindres orthogonaux après réimport STEP, avant intégration du contrat R-036.
- [R-062 D1–E1 — arêtes arrondies et bac ouvert](roadmap/items/R-062-d1-e1-aretes-arrondies-et-bac-ouvert.md) · livrée le 2026-11-19
  Qualifier les arrondis cylindriques locaux et les bacs ouverts à coins verticaux arrondis, avec fond et vide source conservés.
- [R-061 C2a — deux raccords toriques sur une portée](roadmap/items/R-061-c2a-deux-raccords-toriques-sur-une-portee.md) · livrée le 2026-11-19
  Reconstruire un profil exact arc–ligne–arc, avec deux congés toriques et une portée cylindrique commune.
- [R-060 C1b2a — deux cônes directement raccordés](roadmap/items/R-060-c1b2a-deux-cones-directement-raccordes.md) · livrée le 2026-11-19
  Reconstruire deux cônes adjacents lorsque leur rupture unique et leurs parois sont prouvées par les anneaux source.
- [R-059 C1b2 — chaînes coniques sans portée intermédiaire](roadmap/items/R-059-c1b2-chaines-coniques-sans-portee-intermediaire.md) · livrée le 2026-11-19
  Deux à huit cônes adjacents ou mêlés à des portées sont couverts par R-060 et R-078. Les essais de la chaîne complète sont suivis en A-073 et A-074.
- [R-038 H — surfaces libres](roadmap/items/R-038-h-surfaces-libres.md) · livrée le 2026-11-19
  Patch bicubique de type hauteur, quatre limites source sur un solide fermé
- [R-037 G — congés variables](roadmap/items/R-037-g-conges-variables.md) · livrée le 2026-11-19
  Un ou deux congés à rayon affine sur arêtes parallèles ou gauches, avec les deux extrémités contrôlées.
- [R-036 F — perçages sécants](roadmap/items/R-036-f-percages-secants.md) · livrée le 2026-11-19
  Deux perçages sécants, quatre bouches circulaires et deux branches certifiées, y compris avec un décalage latéral.
- [R-035 E — boîtiers creux ouverts et percés](roadmap/items/R-035-e-boitiers-creux-ouverts-et-perces.md) · livrée le 2026-11-19
  Bacs ouverts percés, parois et fonds, rebords et congés intérieurs vérifiés.
- [R-034 D — congés partiels constants](roadmap/items/R-034-d-conges-partiels-constants.md) · livrée le 2026-11-19
  Congés constants de fond de poche avec butées, coins, U ou circuit fermé.
- [R-033 C2 — profils de révolution courbes](roadmap/items/R-033-c2-profils-de-revolution-courbes.md) · livrée le 2026-11-19
  Révolutions toriques ou cubiques, une ou deux travées, pointe axiale, maillage irrégulier et retour axial vérifié.
- [R-032 C1b1 — chaînes coniques de même signe avec portées](roadmap/items/R-032-c1b-chaines-sans-portee-ou-de-meme-signe.md) · livrée le 2026-11-19
  Reconnaître deux à huit cônes de signes libres séparés par des portées cylindriques prouvées ; conserver les garde-fous contre les fausses ruptures.
- [R-031 C1a — chaînes coniques alternées](roadmap/items/R-031-c1a-chaines-coniques-alternees.md) · livrée le 2026-11-19
  Trois à huit cônes alternés et portées résolues ; réimport indépendant et parcours graphique à qualifier.
- [R-030 Petits raccords cylindriques contextuels](roadmap/items/R-030-petits-raccords-cylindriques-contextuels.md) · livrée le 2026-11-19
  Intervalle entier de 4–7 points entre deux parois prouvées ; rayon résolu et tangentes compatibles
- [R-029 Consolidation M2c — arcs décalés et adjacents](roadmap/items/R-029-consolidation-m2c-arcs-decales-et-adjacents.md) · livrée le 2026-11-19
  Recherche bornée d'arcs fragmentés, angles de triangulation différents selon la hauteur, génératrices source partagées par deux parois
- [R-028 B — zones sphériques](roadmap/items/R-028-b-zones-spheriques.md) · livrée le 2026-11-19
  Zone entre deux anneaux plans sans pôle, puis variante concave
- [R-027 A — cône et encoche ouverte](roadmap/items/R-027-a-cone-et-encoche-ouverte.md) · livrée le 2026-11-19
  Fraisure complète et cylindre ouvert distincts dans un même corps, en Surfaces
- [R-079 D2–G0 — qualifier les raccords locaux et le rayon linéaire](roadmap/items/R-079-d2-g0-qualifier-les-raccords-locaux-et-le-rayon.md) · livrée le 2026-11-17
  Établir les preuves et le contrat minimum pour un coin à trois arrondis et une enveloppe à rayon linéaire avant intégration.
- [R-074 Qualifier les grandes zones courbes de Flansch](roadmap/items/R-074-qualifier-les-grandes-zones-courbes-de-flansch.md) · livrée le 2026-11-17
  Mesurer les quatre profils courbes encore refusés, identifier les familles géométriques et les preuves manquantes, puis définir une extension bornée sans relâcher les tolérances.
- [R-072 Qualifier les résultats très facettisés des gros STL](roadmap/items/R-072-qualifier-les-resultats-tres-facettises-des-gros.md) · livrée le 2026-11-17
  Flansch franchit désormais la reconnaissance avec 4 cylindres et 8 cônes, mais conserve 501 900 plans, soit 501 912 faces au total. Le suivi du résultat est limité à 100 000 faces. Identifier les zones qui restent facettisées et les réductions géométriquement prouvées possibles ; examiner les coûts CAO, mémoire et inspection avant toute proposition de changement de limite ou de simplification.
- [R-058 Raccords ouverts cylindre–chanfrein](roadmap/items/R-058-raccords-ouverts-cylindre-chanfrein.md) · livrée le 2026-11-17
  Reconstruire ensemble les parois cylindriques ouvertes et leurs chanfreins coniques partiels, avec contours partagés et validation du solide entier.

### atelier : Inspection, guidage et atelier

- [R-066 UX8 — préciser le nombre de triangles refusés](roadmap/items/R-066-ux8-preciser-le-nombre-de-triangles-refuses.md) · livrée le 2026-11-19
  Afficher le nombre exact de triangles d’un STL trop dense et le maximum autorisé.
- [R-065 UX7 — suivre les étapes réelles de reconstruction](roadmap/items/R-065-ux7-suivre-les-etapes-reelles-de-reconstruction.md) · livrée le 2026-11-19
  Afficher les étapes observées du moteur et le corps en cours, sans pourcentage ni temps restant inventés.
- [R-064 UX6 — lancer le STEP depuis l’inspecteur](roadmap/items/R-064-ux6-lancer-le-step-depuis-l-inspecteur.md) · livrée le 2026-11-19
  Ouvrir le fichier en aperçu source puis lancer sa reconstruction avec un bouton explicite dans le panneau de droite.
- [R-057 UX5 — reconstruction automatique et sélection des parois](roadmap/items/R-057-ux5-reconstruction-automatique-et-selection-des.md) · livrée le 2026-11-19
  Ouvrir et obtenir le meilleur STEP prouvé, sélectionner une paroi en un geste et corriger localement sans perdre les surfaces reconnues.
- [R-056 I2d — fusion plane guidée](roadmap/items/R-056-i2d-fusion-plane-guidee.md) · livrée le 2026-11-19
  Ajouter Plan aux familles guidées : fusionner une sélection contiguë presque plane, sans déplacer ses sommets, après contrôle du solide.
- [R-050 I2 — élargir les familles guidées](roadmap/items/R-050-i2-elargir-les-familles-guidees.md) · livrée le 2026-11-19
  Après Plan (R-056), ajouter le raccord torique guidé puis étudier les sphères.
- [R-048 I2c — choisir la famille guidée](roadmap/items/R-048-i2c-choisir-la-famille-guidee.md) · livrée le 2026-11-19
  Choisir Cylindre ou Cône sur les facettes sélectionnées ; enchaîner les indications
- [R-047 I2b — enchaîner les cylindres guidés](roadmap/items/R-047-i2b-enchainer-les-cylindres-guides.md) · livrée le 2026-11-19
  Ajouter depuis le STEP courant, retirer une indication et récupérer la session après refus
- [R-046 I2a — première reconstruction guidée](roadmap/items/R-046-i2a-premiere-reconstruction-guidee.md) · livrée le 2026-11-19
  À partir de la sélection, indiquer une hypothèse de surface parmi les familles prises en charge et relancer une conversion contrôlée
- [R-045 I1c — relier le STEP à la source](roadmap/items/R-045-i1c-relier-le-step-a-la-source.md) · livrée le 2026-11-19
  Retrouver les triangles source des faces STEP sélectionnées sans correspondance approximative
- [R-044 I1b — inspecter le STEP](roadmap/items/R-044-i1b-inspecter-le-step.md) · livrée le 2026-11-19
  Afficher le résultat réimporté, sélectionner une ou plusieurs faces natives et étendre aux voisines ; conserver la sélection source en complément
- [R-043 I1 — sélectionner et comprendre](roadmap/items/R-043-i1-selectionner-et-comprendre.md) · livrée le 2026-11-19
  Cliquer sur la surface visible, surligner une zone contiguë du corps choisi, ajuster/effacer la sélection et afficher ce qui est reconnu ou refusé
- [R-042 UX4 — simplifier l’atelier](roadmap/items/R-042-ux4-simplifier-latelier.md) · livrée le 2026-11-19
  Donner la priorité à la pièce et aux actions courantes ; retirer les exemples de l’écran principal ; options et données techniques à la demande
- [R-041 UX3 — rendre les faces lisibles](roadmap/items/R-041-ux3-rendre-les-faces-lisibles.md) · livrée le 2026-11-19
  Couleurs par face STEP, contours contrastés et sélection identifiable
- [R-040 UX2 — recherche assistée des réglages](roadmap/items/R-040-ux2-recherche-assistee-des-reglages.md) · livrée le 2026-11-19
  Comparer au plus sept tolérances radiales et appliquer explicitement la proposition
- [R-039 UX1 — choix compréhensibles](roadmap/items/R-039-ux1-choix-comprehensibles.md) · livrée le 2026-11-19
  Mettre en avant conserver les facettes ou reconnaître les surfaces courbes ; regrouper les reconnaissances ciblées ; aide brève selon le choix, limites détaillées à la demande

### dxf : Profils et sections DXF

- [R-054 DXF2b — coupe du maillage source](roadmap/items/R-054-dxf2b-coupe-du-maillage-source.md) · livrée le 2026-11-19
  Couper le maillage préparé sans exiger un STEP réussi ; profils polygonaux et export DXF distinct.
- [R-053 DXF2a — coupe du STEP et ellipses exactes](roadmap/items/R-053-dxf2a-coupe-du-step-et-ellipses-exactes.md) · livrée le 2026-11-19
  Placer un plan XY/XZ/YZ ou reprendre le plan d’une face plane, régler son décalage et son orientation, voir la coupe et exporter la section DXF
- [R-052 DXF1 — exporter une face plane](roadmap/items/R-052-dxf1-exporter-une-face-plane.md) · livrée le 2026-11-19
  Sélectionner une face du STEP validé, prévisualiser son contour dans son plan, puis **Exporter le profil DXF**

## Écarté

- [R-132 Qualifier un délai prolongé sur demande pour les gros STL](roadmap/items/R-132-qualifier-un-delai-prolonge-sur-demande-pour-les.md)
  F5 · Option CLI conditionnée au choix A-136 si le budget ordinaire demeure insuffisant après les travaux géométriques.
