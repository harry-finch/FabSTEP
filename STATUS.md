# État — 19 novembre 2026

**V1 mécanique 1.0.0 acceptée ; extensions post-v1 dans le build de développement.** Rust/OCCT, Svelte 5/Tauri, GPL-3.0-or-later. Branche de développement : `fabboard/mise-d-equerre` ; nettoyage local sur `fabboard/nettoyage`, sans modification de la branche par défaut ni du paquet release. Historique et mesures détaillés dans [DEVLOG](DEVLOG.md) et [VALIDATION](docs/VALIDATION.md), suivi utilisateur dans [ROADMAP](ROADMAP.md).

## Version et validation courantes

- `artifacts/release/1.0.0/FabSTEP.app` reste le paquet autonome signé ad hoc, accepté sur un autre Mac et dans Fusion le 9 novembre (V-1115). Pas de signature Developer ID ni notarisation revendiquée.
- `artifacts/FabSTEP.app` est le build macOS de développement du 19 novembre. Il inclut les lots géométriques jusqu’à R-111, le relief bicubique, la correction sphérique et le constructeur de zones toriques du helper. Les 51 tests du bureau avec le helper de ce paquet passent. Les essais manuels des nouvelles familles figurent dans FabBoard.
- La suite workspace a réussi 706 tests et ignoré 18 tests optionnels. Les 238 tests UI et les 51 tests du bureau avec le helper du paquet passent. Les essais graphiques/Fusion des extensions restent ouverts dans FabBoard ; aucun succès automatique ne vaut acceptation utilisateur. Le pilotage du dialogue d’ouverture avait été bloqué, mais l’utilisateur a confirmé l’ouverture manuelle.
- Nettoyage A+B approuvé en [A-097](roadmap/actions/A-097-choisir-ce-qui-se-nettoie-dans-fabstep.md) : 40,8 Gio de caches et résidus déplacés à la Corbeille, projet ramené à 2,6 Gio. Les deux exécutables FabSTEP et le helper sont inchangés (SHA-256). Aucun fichier suivi retiré ; formatage Rust et contrôle des espaces Git réussis. Les tests et builds après nettoyage sont reportés au prochain développement sur choix explicite de l’utilisateur ([A-098](roadmap/actions/A-098-choisir-quand-recompiler-fabstep-apres-nettoyage.md)).

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

Ces lots sont à tester dans FabBoard. Les détails de portée et de preuve sont dans V-1201/V-1203/V-1204/V-1205/V-1206/V-1208/V-1211/V-1213/V-1214/V-1216. Les compteurs de la v1 ne sont pas confondus avec ces extensions.

## Limites et fichier Flansch

Import ordinaire : 32 corps, 64 Mio et 100 000 triangles après placement. Le mode gros STL accepté utilise un helper séparé `large-stl`, source jusqu’à 600 000 triangles et 1 800 000 sommets, annulation, surveillance RSS/empreinte physique à 4 Gio et délai de 30 minutes. Les plafonds résultat/tessellation et 3MF restent inchangés ; le bureau n’active jamais cette feature (ADR-0034).

Le helper Auto reconstruit la rampe publique de 194 694 triangles en 659 faces. Sur Flansch, la reconnaissance directe attribue 493 542 triangles à 43 surfaces toriques et conserve les autres facettes. Le solide de laboratoire est relu avec 46 671 faces et remesuré avec 98 576 triangles. Écarts bidirectionnels de 0,011553 mm et 0,010018 mm (V-1252/V-1253). Le superviseur du bureau reçoit `cad.invalid_solid` après 23 min 0,14 s sur ce fichier. OCCT refuse le solide au contrôle des auto-intersections, le processus est récupéré et aucun résultat ne devient exportable (V-1256).

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

Les domaines spécifiés de C2/D/E/F/G sont **à tester**. La validation manuelle reste portée par chaque sous-jalon. Les limites dans la dernière colonne décrivent la portée géométrique démontrée.

**Reprise Flansch — R-085.** V-1237 à V-1253 donnent les commandes de qualification du STEP fermé, du maillage partagé, de la reconnaissance directe et du certificat de source. V-1256 donne le verdict du helper sur le fichier original. La prochaine intervention concerne les sous-formes impliquées dans le premier refus du contrôle d’interférences OCCT, sous les plafonds ADR-0034.

Les commandes de validation/build sont dans README/AGENTS et `scripts/check.sh` / `scripts/build-macos-dev.sh`. Les preuves F1/C2b/R-085 sont consignées dans V-1205/V-1206/V-1208/V-1209/V-1210 et ADR-0038/0039/0040. Les journaux locaux sous `artifacts/f-next/`, `artifacts/c2-next/` et `artifacts/r085-*/` sont ignorés par Git ; les tests, générateurs et corpus nécessaires à leur reproduction sont versionnés. Ne pas considérer ces journaux locaux comme une dépendance de reprise sur un autre poste.
