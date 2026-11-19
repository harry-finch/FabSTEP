# État — 19 novembre 2026

**V1 mécanique 1.0.0 acceptée ; extensions post-v1 dans le build de développement.** Rust/OCCT, Svelte 5/Tauri, GPL-3.0-or-later. Branche de développement : `fabboard/mise-d-equerre` ; nettoyage local sur `fabboard/nettoyage`, sans modification de la branche par défaut ni du paquet release. Historique et mesures détaillés dans [DEVLOG](DEVLOG.md) et [VALIDATION](docs/VALIDATION.md), suivi utilisateur dans [ROADMAP](ROADMAP.md).

## Version et validation courantes

- `artifacts/release/1.0.0/FabSTEP.app` reste le paquet autonome signé ad hoc, accepté sur un autre Mac et dans Fusion le 9 novembre (V-1115). Pas de signature Developer ID ni notarisation revendiquée.
- `artifacts/FabSTEP.app` est le build local de développement, dépendant du SDK installé. Ce paquet comprend les congés de fond de poche R-086 à R-090 et les perçages sécants R-091/R-092, y compris le bloc incliné en STL binaire (V-1219/V-1220). R-093 y ajoute le trou traversant d’une plaque inclinée en mode Auto (V-1221). Les essais de la butée oblique, des congés opposés et du coin à deux congés figurent dans V-1213, V-1214, V-1216 et V-1218.
- Seize tests opt-in workspace restent ignorés par la suite ordinaire. Les essais graphiques/Fusion des extensions restent ouverts dans FabBoard ; aucun succès automatique ne vaut acceptation utilisateur. Le pilotage du dialogue d’ouverture avait été bloqué, mais l’utilisateur a confirmé l’ouverture manuelle.
- Nettoyage A+B approuvé en [A-097](roadmap/actions/A-097-choisir-ce-qui-se-nettoie-dans-fabstep.md) : 40,8 Gio de caches et résidus déplacés à la Corbeille, projet ramené à 2,6 Gio. Les deux exécutables FabSTEP et le helper sont inchangés (SHA-256). Aucun fichier suivi retiré ; formatage Rust et contrôle des espaces Git réussis. Les tests et builds après nettoyage sont reportés au prochain développement sur choix explicite de l’utilisateur ([A-098](roadmap/actions/A-098-choisir-quand-recompiler-fabstep-apres-nettoyage.md)).

## Extensions géométriques de développement

R-094 à R-105 ajoutent la pointe de révolution, la méridienne cubique par morceaux, les maillages irréguliers, les perçages décentrés ou obliques, plusieurs types de poche et deux congés à rayon variable sur la même pièce. R-038 ajoute un relief bicubique explicite. R-050 couvre la correction sphérique depuis une face STEP. Les contrôles par famille figurent dans V-1222 à V-1235 et dans les actions FabBoard de chaque lot.

## Capacités développées

L’import STL/3MF Core conserve les corps et placements ; les STEP sont réimportés et contrôlés avant export. Le STEP est un BRep sans historique d’esquisses. L’atelier propose lancement manuel, progression native, sélection source/STEP et provenance, corrections Plan/Cylindre/Cône/Tore dans leurs domaines prouvés, profils et sections DXF exactes (ellipses comprises), coupe source polygonale. Les erreurs ou changements de requête invalident l’export.

Les familles mécaniques qualifiées comprennent extrusions, cylindres et cônes locaux, chaînes coniques, sphères/calottes/zones, tores et raccords toriques, boîtiers arrondis et bacs ouverts. Dernières tranches :

- **R-076 / C2** : profils toriques composés et arcs obtus, anneaux et raccords démontrés.
- **R-078 / C1b2** : deux à huit cônes adjacents ou mélangés à des portées, témoins intérieurs et découpage unique ; repli Auto historique préservé hors domaine.
- **R-077 / E2** : bac ouvert à coins arrondis avec perçage traversant du fond, sans nouveau moteur de production.
- **R-081 / D2** : coin de bloc orthogonal à trois cylindres partiels et un octant sphérique. Champs/ABI additifs approuvés (ADR-0036/A-068), sources transformées, fidélité, provenance et documents à deux corps contrôlés. Repli Auto facettisé conservé si la preuve analytique échoue ; correction locale couplée refusée. Domaine limité à 32 000 triangles, 8 192 régions planes et six supports dominants.

- **R-082 / G1** : raccord convexe à rayon linéaire entre deux plans orthogonaux, deux limites elliptiques exactes et un cône incliné. Auto/Surfaces, profils DXF et deux corps séparés contrôlés. Le parcours des cylindres constants est préservé par un test de variation préalable ; les déplacements des quatre extrémités reconstruites sont comptés.

- **R-083, R-091 et R-092 / F** : deux trous traversants centrés orthogonaux, quatre anneaux, six plans et trois parois cylindriques. Le rapport petit/grand rayon reste strictement sous deux tiers, avec marge sur les deux mesures. Partition source, courbes communes réimportées, volume, provenance et deux corps séparés contrôlés. Correction locale couplée et DXF non conique refusés.

- **R-093** couvre le cas d’une plaque à un trou traversant perpendiculaire aux faces d’entrée. Sur le [témoin incliné](fixtures/inclined_plate_hole.stl), les six plans et le cylindre de Ø6 mm sont relus dans FreeCAD. La sélection du cylindre retrouve ses 126 triangles. L’essai dans Fusion est suivi par [A-096](roadmap/actions/A-096-verifier-le-trou-de-la-plaque-inclinee-dans.md).

- **R-084 / C2b** ajoute une révolution complète à profil cubique positif et deux disques source. Auto/Surfaces, profils bombés, étranglés et à inflexion, placements, miroir, fidélité et provenance contrôlés. Une tessellation paramétrique du domaine natif certifié corrige les triangles trop éloignés du mailleur générique sur ces profils. Les splines multisegments, pôles et raccords à d’autres familles restent hors domaine.

- **R-085** : Auto et Surfaces reconstruisent le congé extérieur d’un secteur de couronne fermé par deux plans radiaux. Famille Tore, sept faces. Essais A-084/A-085. Le volet Flansch reste en cours.

- **R-086 / D–E** : Auto reconstruit un congé intérieur de fond de poche qui s’arrête sur deux parois perpendiculaires à son axe. Les deux extrémités peuvent avoir des nombres de segments différents. Le changement s’applique également aux congés extérieurs.

- **R-087 / D–E** : une poche borgne à congé de rayon constant peut se terminer sur une butée droite et une paroi inclinée. Auto conserve le cercle et l’ellipse sur leurs faces respectives. Le domaine et ses limites sont décrits dans ADR-0041.

**R-088 / D–E** qualifie deux congés de fond opposés, y compris avec deux rayons et des subdivisions différentes. Les deux cylindres sont sélectionnables séparément dans le STEP. Une déformation du congé droit laisse le gauche reconnu sur le témoin.

**R-089 / D–E** reconstruit deux quarts de cylindre de même rayon qui se rejoignent dans un coin à 90°. Le STEP partage une seule arête elliptique entre les deux faces. La sélection distingue les deux bandes de triangles source.

Ces lots sont à tester dans FabBoard. Les détails de portée et de preuve sont dans V-1201/V-1203/V-1204/V-1205/V-1206/V-1208/V-1211/V-1213/V-1214/V-1216. Les compteurs de la v1 ne sont pas confondus avec ces extensions.

## Limites et fichier Flansch

Import ordinaire : 32 corps, 64 Mio et 100 000 triangles après placement. Le mode gros STL accepté utilise un helper séparé `large-stl`, source jusqu’à 600 000 triangles et 1 800 000 sommets, annulation, surveillance RSS/empreinte physique à 4 Gio et délai de 30 minutes. Les plafonds résultat/tessellation et 3MF restent inchangés ; le bureau n’active jamais cette feature (ADR-0034).

Le helper de l’application refuse Flansch à **231 608 faces**. Le laboratoire réimporte une coque ouverte de **2 083 faces** couvrant **460 044 des 540 170 triangles** du STL. Les **80 126 triangles restants** forment trois nappes et trois triangles isolés, avec 77 396 propriétaires plans. La partition du document entier compterait **79 479 faces** en les conservant. Voir les contrôles et la commande de reprise dans [V-1210](docs/VALIDATION.md#v-1210--r-085-perçages-épaulements-et-marquages-de-flansch).

## Reprise après cette session

Le secteur annulaire de R-085 est intégré sous ADR-0040. R-086 couvre le cas d’un congé droit de poche entre deux butées planes perpendiculaires. R-087 ajoute une butée oblique (accord A-087). R-088 qualifie deux congés opposés partageant le fond plan. R-089 intègre le coin à deux congés adjacents de même rayon (ADR-0042). R-090 intègre la jonction de deux rayons différents dans une poche rectangulaire (A-092, ADR-0043). Les conversions depuis STL sont vérifiées dans V-1218. Les réseaux plus étendus et les congés du rebord restent ouverts. La couture des 267 facettes sur Flansch reste, à ce stade, une preuve de laboratoire. La reprise ultérieure ne nécessite pas l’historique de conversation : lire les consignes et ce document, les réponses de `fabboard action list`, puis les éléments ci-dessous. Les accords A-045 (F) et A-068 (D/G) sont acquis ; ils ne valent pas acceptation des essais manuels.

| Chantier | Déjà développé | Travail restant avant généralisation |
| --- | --- | --- |
| C1b2 | R-078 : chaînes adjacentes et mixtes de deux à huit cônes | Extensions hors des témoins et limites actuellement prouvés ; essais A-073/A-074 |
| C2 / R-033 | R-095, R-099 et R-104. Deux travées tangentes, pointe axiale, mono-travée sans anneaux complets | Retours axiaux, familles voisines et autres conditions de raccord |
| D / R-034 | R-097 et R-101. Doubles butées inclinées et coins à deux congés de 65° ou 115° | Réseaux à plusieurs jonctions |
| E / R-035 | R-098, R-100 et R-103. Trous du fond et de paroi, deux cavités avec cloison et rebord, lèvre circulaire à deux congés | Réseaux de congés et lèvres hors des domaines publiés |
| F / R-036 | R-096 et R-105. Décentrage orthogonal et axes obliques dans un prisme à lèvres circulaires | Intersections multiples, bouches elliptiques et décentrage oblique combiné |
| G / R-037 | R-094 et R-102. Bandes sans rangée intermédiaire et deux lois affines indépendantes | Raccords qui se rejoignent, axes non parallèles et lois non linéaires |

Les parents C2/D/E/F/G sont **en cours** dans FabBoard : des sous-domaines sont développés, pas la généralisation entière. Les essais utilisateur restent dans leurs actions existantes, sans les dupliquer ni les marquer acceptés.

**Reprise Flansch — R-085.** La coque de V-1210 sert d’entrée au laboratoire de V-1237. Le solide fermé réimporté contient désormais 13 802 faces et le graphe attribue les 540 170 triangles du STL. À 0,005 mm, le maillage OCCT ordinaire consomme 215 486 triangles. La suite porte sur la tessellation commune des bords et la découverte complète depuis le STL dans le helper. Le témoin public et quatre cas de refus se rejouent sans la pièce privée.

Les commandes de validation/build sont dans README/AGENTS et `scripts/check.sh` / `scripts/build-macos-dev.sh`. Les preuves F1/C2b/R-085 sont consignées dans V-1205/V-1206/V-1208/V-1209/V-1210 et ADR-0038/0039/0040. Les journaux locaux sous `artifacts/f-next/`, `artifacts/c2-next/` et `artifacts/r085-*/` sont ignorés par Git ; les tests, générateurs et corpus nécessaires à leur reproduction sont versionnés. Ne pas considérer ces journaux locaux comme une dépendance de reprise sur un autre poste.
