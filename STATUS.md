# État — 20 novembre 2026

**V1 mécanique 1.0.0 acceptée ; extensions post-v1 dans le build de développement.** Rust/OCCT, Svelte 5/Tauri, GPL-3.0-or-later. `main` est la branche de référence pour le développement et les fusions. Elle contient les jalons géométriques et l’intégration NEON (R-112). Les décisions et résultats par lot figurent dans le [registre R-112](docs/INT_AUDIT_01.md#neon--audit-du-19-novembre-2026-r-112). Historique et mesures détaillés dans [DEVLOG](DEVLOG.md) et [VALIDATION](docs/VALIDATION.md), suivi utilisateur dans [ROADMAP](ROADMAP.md).

## Version et validation courantes

Le périmètre fonctionnel v1 est gelé sur `audit/gel-v1`. Les huit constats d’AUDIT-03 et leurs preuves sont consignés dans [le registre](docs/INT_AUDIT_01.md#audit-03--gel-fonctionnel-v1) et [V-1288](docs/VALIDATION.md#v-1288--correctifs-du-gel-fonctionnel-v1). La candidate locale est `artifacts/FabSTEP.app`. Les 17 actions de recette restantes ont été clôturées dans FabBoard le 20 novembre à la demande de l’utilisateur. La signature Developer ID et la notarisation (R-018), ainsi que Windows/Linux (R-019), relèvent de la distribution. Flansch (R-085) et le rapport CLI/MCP (R-118) restent au suivi post-v1.

La candidate intègre R-133. Automatique et Facettes se choisissent dans l’inspecteur. Un réglage manuel verrouille les tolérances. La vue Source et le résultat STEP partagent la navigation. La clôture de la recette sur les deux octants figure dans [A-144](roadmap/actions/A-144-essayer-automatique-facettes-et-la-navigation-3d.md).

La recette précédente sur `atelier/carte-ecarts` couvrait les domaines R-122 à R-129 et les bancs R-130/R-131. Les résultats publics et les refus sont consignés en V-1277 à V-1285. La recette automatisée est détaillée en V-1286 (798 tests workspace réussis). Le rejeu privé V-1287 atteint le délai de trente minutes. Les derniers essais Fusion sont dans les actions A-137 à A-142. A-136 conserve trente minutes et 4 Gio pour le helper. R-132 est abandonné à la suite de cet arbitrage. Le premier jalon de distribution est R-019.

- `artifacts/release/1.0.0/FabSTEP.app` reste le paquet autonome signé ad hoc, accepté sur un autre Mac et dans Fusion le 9 novembre (V-1115). Pas de signature Developer ID ni notarisation revendiquée.
- `artifacts/FabSTEP.app` est le build macOS de développement de cette branche. Ses capacités de reconstruction et les preuves des domaines récents figurent dans ADR-0048. Il comprend aussi les corrections R-112, la préparation de coupe mono-corps et le rapport compact du helper. La candidate autonome est dans `artifacts/audit-neon/candidate/`. Son démarrage et les deux protocoles ont été vérifiés sur ce Mac avec Homebrew bloqué.
- La branche de développement `atelier/carte-ecarts` comprend l’historique des corrections, le chargement de la vue 3D à la première ouverture et les permissions de la fenêtre locale (V-1270/V-1271). L’atelier propose la boîte de découpe en vue Source et Résultat STEP, la recherche de parois similaires après correction guidée et la carte **Écart au maillage** sur un STEP ordinaire. Sur le bossage borgne, le maximum affiché est de 0,00925 mm au seuil de 0,05 mm. Les essais du bureau et du moteur figurent en V-1267/V-1268.
- R-116 récupère six plans sur le toit quantifié. R-117 reconstruit en Auto deux secteurs cylindriques de 22°, dont un congé entre deux pans tangents. Le secteur conique à deux arcs et l’octant de sphère découpé sont aussi admis dans Auto. L’arbre à bossage longitudinal garde son grand arc. Les témoins et limites sont en V-1265/V-1269/V-1274.
- R-115 récupère six et sept faces STEP sur les STL publics du bloc tourné et du bloc plié. Le pli et la provenance sont vérifiés dans les deux modes. La recette FabSTEP et Fusion est cochée dans [A-124](roadmap/actions/A-124-verifier-les-six-et-sept-faces-des-deux-stl.md).
- Qualification R-112 : 710 tests workspace réussis et 18 optionnels ignorés, 55 tests bureau, 11 worker, 241 UI et 31 Python réussis. Le paquet autonome convertit la rampe publique de 194 694 triangles en 659 faces. L’utilisateur a accepté les tests en attente, dont le geste de peinture et les essais Fusion, lors de la recette groupée du 19 novembre.
- Nettoyage direct A+B approuvé en [A-143](roadmap/actions/A-143-choisir-les-fichiers-a-supprimer-directement.md) : 33,5 Gio de caches et résidus supprimés définitivement sur `fabboard/nettoyage-2026-11-20`, projet ramené de 40,5 à 7,0 Gio. Les deux applications et la candidate NEON sont identiques avant/après (empreintes de contenu SHA-256). Aucun fichier suivi retiré ; formatage Rust et contrôle des espaces Git réussis. Les tests et builds ont repris avec AUDIT-03 (V-1288). Les nettoyages précédents sont conservés dans A-097, A-122 et DEVLOG.

Le suivi Argon décrit les accélérations de la segmentation, de l’atlas et du contrôle de fidélité, ainsi que les limites du contrôle natif sur Flansch. R-115 est en recette. R-113 produit quatre faces STEP sur l’arbre à perçage transversal en Auto. R-125 intègre la lèvre torique oblique et les bandes coniques et sphériques qualifiées en V-1282 et V-1284. R-114 produit dix faces sur le bloc à perçages sécants et congé longitudinal. Voir ADR-0047 et V-1272/V-1273. [Résultats des essais](docs/VALIDATION.md#v-1259--contre-expertise-du-contrôle-dinterférence-docct-793)

## Extensions géométriques de développement

R-094 à R-111 couvrent les révolutions cubiques régulières et irrégulières, les perçages obliques et décalés, les poches à congés connectés et les congés variables sur arêtes parallèles ou gauches. R-038 ajoute un relief bicubique explicite. R-050 étend la correction guidée à la sphère. Les résultats reproductibles figurent dans V-1222 à V-1251 et les essais Fusion dans les actions des sous-jalons.

## Capacités développées

L’import STL/3MF Core conserve les corps et placements ; les STEP sont réimportés et contrôlés avant export. Le STEP est un BRep sans historique d’esquisses. L’atelier propose lancement manuel, progression native, sélection source/STEP et provenance, corrections Plan/Cylindre/Cône/Tore/Sphère dans leurs domaines prouvés, profils et sections DXF exactes (ellipses comprises), coupe source polygonale. Les erreurs ou changements de requête invalident l’export.

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

Ces lots ont été acceptés lors de la recette groupée dans FabBoard. Les détails de portée et de preuve sont dans V-1201/V-1203/V-1204/V-1205/V-1206/V-1208/V-1211/V-1213/V-1214/V-1216. Les compteurs de la v1 ne sont pas confondus avec ces extensions.

## Limites et fichier Flansch

Import ordinaire : 32 corps, 64 Mio et 100 000 triangles après placement. Le mode gros STL accepté utilise un helper séparé `large-stl`, source jusqu’à 600 000 triangles et 1 800 000 sommets, annulation, surveillance RSS/empreinte physique à 4 Gio et délai de 30 minutes. Les plafonds résultat/tessellation et 3MF restent inchangés ; le bureau n’active jamais cette feature (ADR-0034).

Depuis R-123, le helper Auto convertit la rampe publique de 194 694 triangles en 442 faces (659 auparavant) et le secteur à deux bords décalés de 166 848 triangles en 583 faces (962 auparavant). FreeCAD confirme les deux tores et le volume de 304,172709 mm³ de ce second témoin. Sur Flansch, la reconnaissance attribue 493 542 des 540 170 triangles à 43 tores. Le banc du helper retrouve les 46 671 faces et leurs identités après transfert. Le solide relu passe le contrôle géométrique. Sur les deux captures, le banc couvre les 13 556 groupes d’intersections, tous acceptés. Les écarts échantillonnés sont de 0,011552 mm du STL au STEP et de 0,010018 mm en retour. Après R-122 à R-131, le helper du paquet de développement atteint encore 1 800,07 s au contrôle des interférences avant écriture. Le pic observé vaut 2,78 Gio. Aucun STEP du fichier original n’est disponible à l’export. Voir V-1262, V-1275 et V-1287.

## Reprise après cette session

Lire les consignes du projet, le tableau ci-dessous et les actions FabBoard pour reprendre. Les choix de domaine figurent dans les ADR, les validations automatiques et leurs commandes dans `docs/VALIDATION.md`.

| Chantier | Déjà développé | Travail restant avant généralisation |
| --- | --- | --- |
| C1b2 | R-078 : chaînes adjacentes et mixtes de deux à huit cônes | Extensions hors des témoins et limites actuellement prouvés ; essais A-073/A-074 |
| C2 / R-033 | R-095, R-099, R-104, R-106 et R-111. Une ou deux cubiques, pointe axiale et méridienne à retour axial mesuré | Retours multibranches et autres conditions de raccord |
| D / R-034 | R-097, R-101, R-107 et R-110. Doubles butées, coins, U et circuit fermé | Autres angles, rayons et topologies |
| E / R-035 | R-098, R-100, R-103 et R-110. Trous du fond et de paroi, deux cavités avec cloison et rebord, lèvre circulaire à deux congés | Réseaux de congés et lèvres hors des domaines publiés |
| F / R-036 | R-096, R-105 et R-108. Axes obliques et décentrage suivant la normale commune | Intersections multiples et bouches elliptiques |
| G / R-037 | R-094, R-102 et R-109. Deux lois affines sur des arêtes parallèles ou gauches | Raccords qui se rejoignent et lois non linéaires |

Les domaines spécifiés de C2/D/E/F/G sont **acceptés** dans les limites de chaque sous-jalon. Les limites dans la dernière colonne décrivent la portée géométrique démontrée.

**R-085.** La qualification du contrôle mono-forme à deux threads sur la rampe et le secteur publics figure en V-1275. Les recettes publiques des nouveaux domaines et les mesures du contrôle inter-groupes figurent en V-1277 à V-1285. Le dernier rejeu privé est consigné dans R-085 et V-1287. Pour reprendre, privilégier un témoin public de groupe coûteux à colliers non coplanaires.

Les commandes de validation/build sont dans README/AGENTS et `scripts/check.sh` / `scripts/build-macos-dev.sh`. Les preuves F1/C2b/R-085 sont consignées dans V-1205/V-1206/V-1208/V-1209/V-1210 et ADR-0038/0039/0040. Les journaux locaux sous `artifacts/f-next/`, `artifacts/c2-next/` et `artifacts/r085-*/` sont ignorés par Git ; les tests, générateurs et corpus nécessaires à leur reproduction sont versionnés. Ne pas considérer ces journaux locaux comme une dépendance de reprise sur un autre poste.
