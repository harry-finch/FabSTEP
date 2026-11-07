# FabSTEP

FabSTEP transforme des maillages STL et 3MF Core en solides STEP pour poursuivre le travail dans un logiciel de CAO. Il appartient à **FabApps** et reprend l'architecture modulaire et l'interface d'atelier de FabWeave. Licence **GPL-3.0-or-later**.

**Version 1.0.0-rc.1 — candidate mécanique, qualification finale en cours.** Le mode **Plans** regroupe les triangles coplanaires en faces CAO. Le mode explicite **Cylindres** reconnaît les extrusions à section constante et les parois cylindriques locales, notamment celles des trous borgnes à fond plat, des bossages et des encoches ouvertes dont les contours et raccords sont démontrés. Le mode explicite **Cônes et cylindres** reconnaît également des parois coniques complètes et des chanfreins circulaires lorsque leurs bandes et raccords sont prouvés. Le mode explicite **Surfaces arrondies** reconnaît des corps sphériques complets, des calottes sphériques, petites ou grandes, avec anneau plan et pôle source, des tores entiers, des congés toriques quart de cercle entre cylindre et plan, ainsi que des boîtiers convexes dont les douze arêtes et huit coins ont le même rayon de congé. Les bandes et raccords sont contrôlés avant export. Le corpus indépendant, les exports natifs et les deux sources de la deuxième pièce réelle ont été réimportés dans FreeCAD ; les mesures et limites sont consignées dans [VALIDATION](docs/VALIDATION.md). FabSTEP contrôle le solide réimporté et fournit un audit échantillonné de la fidélité en modes analytiques. Le résultat est un STEP BRep sans historique d'esquisses. L’import 3MF est limité au format Core : les références multi-modèles de l’extension Production, courantes dans les fichiers de slicers, ne sont pas prises en charge. La compatibilité avec les 3MF en général n’est pas promise.

## Essayer sur ce Mac

Ouvrir le paquet autonome qualifié indiqué ci-dessous, puis **Charger le cube exemple** ou **Ouvrir STL / 3MF**. Choisir l’unité STL, absente du fichier ; pour 3MF, l’unité déclarée ou celle par défaut du format est indiquée et verrouillée. Utiliser **Exporter STEP** lorsque le résultat est validé. Le cube exemple mesure 20 mm et produit six faces. **Charger le cylindre exemple** sélectionne le mode Cylindres et produit trois faces, dont une cylindrique. **Charger la pièce mixte** montre un bossage et un trou borgne, avec huit plans, deux cylindres et les distances échantillonnées dans les deux sens. **Charger le cylindre chanfreiné** sélectionne le nouveau mode avec un exemple synthétique. **Charger le congé torique** sélectionne le mode Surfaces arrondies sur un exemple synthétique. **Charger les deux cubes 3MF** illustre deux corps placés séparément.

Pendant le traitement, un indicateur animé affiche la phase réelle (attente, lecture, conversion) et le temps écoulé. La barre est indéterminée : aucun pourcentage ni délai restant n’est estimé. L’aperçu reste manipulable et l’indicateur disparaît lorsque la demande courante réussit ou échoue.

Le paquet autonome candidat est construit dans `artifacts/release/1.0.0-rc.1/FabSTEP.app` pour Apple Silicon sous macOS 26 ou ultérieur. Il embarque le noyau CAO et ses bibliothèques ; les conversions de contrôle ont été exécutées avec les lectures Homebrew bloquées. Le bundle `artifacts/FabSTEP.app` reste un build de développement distinct. Voir [construction et qualification du paquet](docs/RELEASE.md). `artifacts/` est généré et exclu de Git.

Les quatre pièces du [corpus mécanique indépendant](fixtures/complex_v1/README.md) ont été converties puis réimportées dans FreeCAD : perçages sur trois axes, double chanfrein, carter mêlant cinq familles de surfaces et boîtier arrondi. Les formes libres, intersections de perçages et congés généraux à rayon variable restent à développer. La qualification du parcours natif de cette candidate est en cours.

## Construire

Prérequis : Rust 1.88+, compilateur C++17, Node compatible avec Vite 8 et SDK OpenCASCADE 7.9. Le poste de validation utilise Rust 1.88, Node 26 et OCCT 7.9.3 sur macOS Apple Silicon.

```sh
brew install opencascade
npm --prefix ui ci
sh scripts/build-macos-dev.sh
```

Le script produit `artifacts/FabSTEP.app` en build de développement. Les bibliothèques Homebrew de ce poste ciblent macOS 26 ; aucune compatibilité avec des versions antérieures, Windows ou Linux n'est encore certifiée. Sur un autre emplacement de SDK, définir `OCCT_ROOT` vers un dossier contenant `include/opencascade/` et `lib/`.

Pour travailler sur l'interface et le bureau ensemble :

```sh
npm --prefix ui run desktop:dev
```

Le port de développement est **5181**, distinct de FabWeave et FabTrace. L’interface ouverte seule dans le navigateur peut prévisualiser un STL ; l’aperçu 3MF et la conversion nécessitent le moteur natif Tauri.

## Ligne de commande

```sh
cargo run -p fabstep-cli -- check fixtures/cube.stl --unit mm --json
cargo run -p fabstep-cli -- convert fixtures/rectangular_hole.stl -o /tmp/fabstep-hole.step --unit mm
cargo run -p fabstep-cli -- convert fixtures/m2_plate_hole.stl -o /tmp/fabstep-round-hole.step --mode cylinders --max-deviation 0.05
cargo run -p fabstep-cli -- convert fixtures/m2b_boss_blind.stl -o /tmp/fabstep-mixed.step --mode cylinders --json
cargo run -p fabstep-cli -- convert fixtures/m2c_open_arc.stl -o /tmp/fabstep-open-arc.step --mode cylinders --json
cargo run -p fabstep-cli -- convert fixtures/m4_chamfered_cylinder.stl -o /tmp/fabstep-chamfer.step --mode primitives --json
cargo run -p fabstep-cli -- convert fixtures/m5b_concave_cap.stl -o /tmp/fabstep-cavity.step --mode surfaces --json
cargo run -p fabstep-cli -- convert fixtures/m5c_icosphere.stl -o /tmp/fabstep-sphere.step --mode surfaces --json
cargo run -p fabstep-cli -- convert fixtures/m5d_ring_torus.stl -o /tmp/fabstep-torus.step --mode surfaces --json
cargo run -p fabstep-cli -- convert fixtures/m6a_rounded_box.stl -o /tmp/fabstep-rounded-box.step --mode surfaces --json
cargo run -p fabstep-cli -- convert fixtures/m3_instances.3mf -o /tmp/fabstep-bodies.step --json
```

`check` analyse les composantes sans valider les solides CAO ni leur séparation spatiale. `convert` effectue le pipeline complet. `--force` autorise explicitement le remplacement d'une sortie existante ; un échec de conversion ne remplace pas le fichier. Les écritures passent par un fichier temporaire. `--weld` et `--plane` sont exprimés en millimètres, avec une valeur initiale de `0.000001`. `--unit` ne s’applique qu’au STL et est refusé pour 3MF : le document donne son unité, ou millimètre par défaut.

Le mode par défaut `--mode planar` conserve les facettes des courbes. `--mode cylinders` cherche une extrusion complète et des cylindres locaux ; il privilégie le modèle local lorsqu’un arc ouvert y est prouvé. `--mode primitives` essaie les cônes et cylindres prouvés, puis la reconnaissance cylindrique historique uniquement en l’absence de candidat conique ; un candidat non prouvé ou un budget dépassé reste bloquant. Si aucune reconstruction n'est prouvée, le refus est explicite. `--mode surfaces` reconnaît ces portions arrondies ; les reconnaissances de boîtier arrondi, tore entier puis sphère entière sont essayées dans cet ordre uniquement si le parcours historique Surfaces → Primitives → Cylindres conclut à `recognition.unsupported`. Le mode Surfaces peut aussi prouver deux chanfreins opposés séparés par une portée cylindrique suffisamment longue. Un succès historique, une preuve échouée ou un budget dépassé reste prioritaire. `--fit` fixe la tolérance d'ajustement radial (défaut `0.00001` mm) et `--max-deviation` le budget de la borne composée (défaut `0.05` mm). Les modes analytiques interprètent des profils suffisamment circulaires en cercles et, en mode primitives, des bandes admissibles en cônes : le maillage ne contient pas l'intention de conception.

## Précision des fichiers réels

Les seuils initiaux restent inchangés. Dans **Avancé**, la **Tolérance radiale** limite le résidu radial d’ajustement des surfaces reconnues ; le **Budget d’écart** limite la borne composée, qui inclut notamment la flèche des cordes. Les modifier est un choix explicite, sans garantie de reconnaissance. L’inspecteur distingue le résidu radial mesuré, la borne composée, les distances échantillonnées et les **Réglages appliqués**. Les rapports JSON `check` et `convert` conservent ces réglages dans `settings`, séparément de l’unité source ; `precision.custom_tolerances` signale les seuils actifs personnalisés.

Le 3MF du premier fichier réel, réexporté par FreeCAD à six chiffres significatifs, est refusé au seuil radial initial de 0,00001 mm. Avec **0,0001 mm choisi manuellement**, il produit 14 faces (11 planes et 3 cylindriques), réimportées et validées dans FreeCAD. Le budget reste à 0,05 mm ; ce réglage n’est pas une recommandation universelle pour tout fichier arrondi.

La seconde pièce réelle contient un cône, une sphère et un tore : ses STL et 3MF produisent chacun 19 faces (12 plans, 4 cylindres, 1 cône, 1 sphère et 1 tore), réimportées dans FreeCAD. Les volumes sont respectivement 54 445,27801611677 et 54 445,278047030406 mm³, contre 54 445,27799128441 mm³ pour le STEP de référence. Les dimensions sont 60,5 × 59,5 × 28 mm et les plans sont situés à Z = 0, 14, 18 et 28 mm ; la sphère a un rayon de 10 mm et le tore un rayon mineur de 2 mm. Les tangences ont été vérifiées dans FreeCAD. Le mode **Plans**, avec une planéité explicitement resserrée à **0,0000001 mm**, produit un solide fermé facettisé (8 211 faces pour le STL, 8 360 pour le 3MF), validé dans FreeCAD. Un seuil plus strict conserve davantage de petites faces ; aucune projection des sommets n’est ajoutée. Les différences avec le volume du STEP d’origine viennent déjà de son maillage. Voir les mesures et les limites dans la [validation](docs/VALIDATION.md) et la [comparaison locale du STL](artifacts/m5/verified/body2-surfaces-stl-comparison.json).

## Périmètre et limites

- Jusqu’à 32 corps distincts, chacun fermé et orientable ; les défauts topologiques sont refusés, sans fermeture inventée. Une inversion globale est corrigée et signalée. Tout contact, chevauchement ou imbrication entre corps est refusé à la source et au résultat, sans fusion ni export partiel.
- Jusqu’à 64 Mio en entrée et 100 000 triangles au total après placements, avec budgets supplémentaires d’import et de validation des contours. Ce plafond n'est pas une promesse de performance sur toute géométrie.
- Les déplacements dus au soudage sont comptés ; les sommets ne sont pas projetés sur les plans.
- OCCT exige une planéité native d'environ `0.0000001` mm sur les contours des plans reconstruits ou conservés. La version STL binaire f32 d'une pièce mixte tournée peut fragmenter un bouchon à la reconnaissance. Une tolérance de segmentation plus large peut rétablir la reconnaissance sans rendre ses plans acceptables par OCCT.
- L’aperçu central montre les **maillages source placés** et les frontières reconnues. Il est conservé en cas d’échec de conversion après import ; ce n’est pas une tessellation du solide exporté.
- Préférences persistées et annuler/rétablir des options ; la reprise du fichier source après fermeture reste à développer.
- Les cylindres locaux complets ou partiels doivent être non intersectants et délimités par deux contours plans source correspondants. La reconnaissance exige un échantillonnage cyclique compatible, au moins 12 points par cercle et un intervalle angulaire maximal de 45°. Un fond borgne doit être plan et présent dans le STL. Les régions sans candidat restent planes ou facettisées. Les arcs ouverts exigent au moins huit points, un balayage de 30° à 330°, un pas maximal de 45°, deux contours plans correspondants et des raccords latéraux démontrés. Un profil en D est distingué par une unique ouverture angulaire d’au moins 90° et des autres pas d’au plus 45° ; le STL ne prouve pas l’intention de conception. Des arcs bruités ou très denses peuvent être refusés par les contrôles conservateurs ou le budget de travail. Les intersections de perçages et les congés généraux restent hors du périmètre prouvé.
- Les cônes complets exigent des anneaux coaxiaux et une variation radiale affine prouvée ; une pointe doit être présente dans la source. Les cônes partiels sont refusés. La limite historique d’une portion conique par chaîne est étendue au cas Cône–Cylindre–Cône : pentes opposées, portée et changements de pente résolus par rapport aux tolérances. Les successions arbitraires de cônes restent refusées. En mode Cônes et cylindres, dans un corps comportant un cône reconnu, les arcs cylindriques ouverts ne sont pas reconstruits analytiquement ; en absence de candidat conique, le repli historique peut les reconnaître. Les chanfreins droits restent des plans. Les congés généraux restent hors périmètre. Voir [ADR-0010](docs/adr/0010-resolved-revolutions.md).
- Un corps sphérique complet est reconnu sans anneau ni pôle imposé dans la source, après contrôle du maillage fermé entier. Les régions planes significatives sont exclues. Des petites facettes ou des quadrilatères cosphériques restent indiscernables d’une tessellation sphérique : le maillage ne révèle pas l’intention de conception. Voir [ADR-0009](docs/adr/0009-complete-spheres.md).
- Un tore entier doit être un corps annulaire fermé, sans auto-intersection, avec couverture des deux directions périodiques démontrée. Un boîtier arrondi doit comporter six appuis plans rectangulaires, douze quarts de cylindre et huit octants sphériques à rayon commun ; la reconnaissance ne déduit pas la forme de sa seule boîte englobante. Ce contrat ne couvre pas encore les boîtiers creux, les arrondis partiels ou des perçages ajoutés au boîtier. Voir [ADR-0011](docs/adr/0011-uniform-rounded-box.md).
- Les calottes sphériques exigent un anneau circulaire plan, un pôle source et une courbure intérieure suffisamment échantillonnée et distincte d’un cône. Les zones sphériques entre deux anneaux sans pôle et les formes dont la hauteur ou la courbure ne sont pas résolues sont refusées. Le bord hors équateur fait l’objet d’une borne supplémentaire ; voir [ADR-0008](docs/adr/0008-spherical-caps.md).
- Les modes analytiques contrôlent toute la paroi et valident les triangles source avant de construire le solide. La borne composée additionne notamment soudage, résidus de reconnaissance et flèche des cordes. L'audit de fidélité compare le maillage source à une tessellation du STEP réimporté dans les deux sens. Les maxima mesurés portent sur des échantillons, avec la déflexion demandée indiquée dans le rapport.
- Le moteur vérifie pour chaque corps le volume, les dimensions et la validité BRep. Un document multi-corps est exporté en STEP à racines multiples, pas en assemblage XCAF : les placements sont conservés, mais pas une hiérarchie d’assemblage paramétrique. Les noms de produits adaptés pour le noyau sont signalés ; les noms source restent dans le rapport.
- M3 est une première prise en charge 3MF Core, pas une compatibilité générale avec les projets de slicers. Les extensions requises non prises en charge, dont les références Production multi-modèles, sont refusées. Les propriétés ou métadonnées facultatives ignorées sont signalées. Les contrôles automatisés et réimports indépendants sont consignés dans la [validation](docs/VALIDATION.md).

Un premier STL réel utilisateur, conservé localement, comprend une encoche circulaire ouverte. En mode Cylindres, la conversion mesurée produit 14 faces : 11 planes et 3 cylindriques, pour un volume de 52 480,08325658634 mm³. Le STEP d’origine fourni ensuite, le STL et le STEP reconstruit situent tous le plan principal à Z = 18 mm depuis la base. Une arête intermédiaire du petit trou, à Z ≈ 17,13256 mm dans l’original, pourrait expliquer la mesure de 17,13 mm rapportée ; c’est une hypothèse, et l’écart de hauteur n’a pas été reproduit. [Mesures locales](artifacts/body-test-height-validation.json).

## Vérifier

```sh
sh scripts/check.sh
```

Le bureau est hors workspace. Les résultats et le réimport FreeCAD sont consignés dans [docs/VALIDATION.md](docs/VALIDATION.md).

Lire [STATUS](STATUS.md), [ROADMAP](ROADMAP.md), [ARCHITECTURE](ARCHITECTURE.md), l'[audit du brainstorming](docs/REFERENCE_REVIEW.md) et le [rapport de délégation](docs/DELEGATION.md). Le document de brainstorming d'origine est conservé intact.
