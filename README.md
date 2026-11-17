# FabSTEP

FabSTEP transforme des maillages STL et 3MF Core en solides STEP pour poursuivre le travail dans un logiciel de CAO. Il appartient à **FabApps** et reprend l'architecture modulaire et l'interface d'atelier de FabWeave. Licence **GPL-3.0-or-later**.

**Version 1.0.0 — reconstruction mécanique locale.** Le mode **Plans** regroupe les triangles coplanaires en faces CAO. Le mode explicite **Cylindres** reconnaît les extrusions à section constante et les parois cylindriques locales, notamment celles des trous borgnes à fond plat, des bossages et des encoches ouvertes dont les contours et raccords sont démontrés. Le mode explicite **Cônes et cylindres** reconnaît également des parois coniques complètes et des chanfreins circulaires lorsque leurs bandes et raccords sont prouvés. Le mode explicite **Surfaces arrondies** reconnaît des corps sphériques complets, des calottes sphériques, petites ou grandes, avec anneau plan et pôle source, des tores entiers, des congés toriques quart de cercle entre cylindre et plan, ainsi que des boîtiers convexes dont les douze arêtes et huit coins ont le même rayon de congé. Les bandes et raccords sont contrôlés avant export. Le corpus indépendant, les exports natifs et les deux sources de la deuxième pièce réelle ont été réimportés dans FreeCAD ; les mesures et limites sont consignées dans [VALIDATION](docs/VALIDATION.md). FabSTEP contrôle le solide réimporté et fournit un audit échantillonné de la fidélité en modes analytiques. Le résultat est un STEP BRep sans historique d'esquisses. L’import 3MF est limité au format Core : les références multi-modèles de l’extension Production, courantes dans les fichiers de slicers, ne sont pas prises en charge. La compatibilité avec les 3MF en général n’est pas promise.

## Développement après la v1

La [roadmap détaillée](ROADMAP.md) et la [recherche technique](docs/POST_V1_RESEARCH.md) organisent les extensions de révolution, arrondis et géométries complexes. Le build de développement ajoute une première combinaison en **Surfaces arrondies** : un cône complet et un cylindre ouvert distincts, tous deux prouvés, même sans sphère ni tore. Le corpus associe une fraisure et une encoche latérale. Le développement ajoute aussi les zones sphériques comprises entre deux anneaux circulaires plans source, sans pôle, convexes ou concaves lorsque toutes les parois nécessaires sont présentes ; voir [ADR-0014](docs/adr/0014-spherical-zones.md). Les modes Cônes et cylindres et Surfaces arrondies ajoutent aussi les chaînes de trois à huit cônes de pentes alternées, séparés par des portées cylindriques résolues avec des anneaux source intérieurs. Voir [ADR-0013](docs/adr/0013-cone-open-cylinder-combinations.md), [ADR-0015](docs/adr/0015-resolved-conical-chains.md) et les résultats dans [VALIDATION](docs/VALIDATION.md). Le paquet 1.0.0 décrit ci-dessous reste celui déjà qualifié ; cette extension n’y est pas incorporée.

### Reconstruire et corriger dans le build de développement

**Ouvrir STL / 3MF** affiche le maillage. **Reconstruire le STEP**, dans l’inspecteur, lance **Automatique** : le moteur recherche les familles compatibles, conserve les régions prouvées et garde les zones incertaines en facettes. Il ne modifie pas les tolérances en silence. Le document entier doit passer les contrôles source, solide réimporté, dimensions, volume et fidélité avant export. Un défaut du maillage, des corps en contact ou une limite de calcul dépassée restent bloquants.

La vue passe au **Résultat STEP** après conversion. **Double-cliquer sur une face** sélectionne sa paroi prouvée, sans propager la sélection à tout le corps ; **Maj-clic** ajoute ou retire une face. En mode sélection STEP, **maintenir le clic gauche et glisser** ajoute les faces visibles survolées à la sélection ; repasser dessus ne les retire pas. Le bouton droit tourne la pièce, le bouton du milieu déplace la vue et la molette zoome. Sans paroi prouvée, la sélection reste limitée à une face ou un refus est expliqué. Les commandes de voisinage Agrandir/Réduire sont dans Avancé de la sélection. Le calcul de sélection est une lecture : il ne retire pas un export valide. Une demande active et la dernière demande en attente bornent les double-clics successifs.

Dans la barre contextuelle, choisir **Auto, Plan, Cylindre, Cône ou Tore**, puis **Essayer**. En mode Automatique, la correction conserve les régions reconnues indépendantes, sans étape de retour général aux facettes. Les indications peuvent être retirées et recalculées ; toute mutation revalide le document entier. Un refus conserve la session acceptée mais ne republie pas l’ancien export. Les faces du résultat restent colorées et délimitées par leurs contours.

**Tore** couvre les raccords circulaires pris en charge et leur portée cylindrique nécessaire ; le double-clic sélectionne ce groupe. Les chaînes avec voisin conique ne sont pas admises dans ce premier guidage torique. Un tore entier est également reconnu. **Plan** fusionne une sélection polygonale contiguë sans déplacer les coordonnées source ; un faible relief peut être simplifié dans la tolérance et son résidu reste signalé. Les frontières courbes de la fusion plane sont refusées.

**Avancé** conserve les stratégies historiques (facettes, cylindres, cônes et surfaces arrondies), les tolérances et la recherche de paramètres. La première ouverture du nouveau parcours sélectionne Auto tout en conservant unité et tolérances enregistrées ; les choix explicites suivants restent mémorisés. Les anciens résultats sans provenance exacte proposent encore une préparation explicite vers les facettes. **Détails techniques** rassemble les diagnostics de routine, sans cacher les erreurs ni les avertissements géométriques.

Limites : Auto ne reconnaît pas les formes libres ; certains arcs ouverts partageant des contours ambigus restent facettisés par prudence. Le reconnaisseur spécifique des boîtiers arrondis reste dans la stratégie avancée Surfaces arrondies : son modèle dédié n’entre pas encore dans la correction locale. Voir [ADR-0029](docs/adr/0029-automatic-local-reconstruction.md), [R-057](roadmap/items/R-057-ux5-reconstruction-automatique-et-selection-des.md) et [VALIDATION](docs/VALIDATION.md).

### Essayer un gros STL — build de développement

Un STL binaire de 100 001 à 600 000 triangles affiche un avertissement et son compte exact. **Essayer quand même** lance un processus séparé ; **Annuler** l’arrête et conserve le fichier chargé. L’accord est remis à zéro à chaque nouveau fichier. La limite de 64 Mio demeure et aucune simplification automatique n’est effectuée. Le processus est également arrêté après 30 minutes ou au-delà de 4 Gio de mémoire résidente observée.

L’essai ne garantit pas la conversion : la validité géométrique, les budgets de travail et les plafonds du résultat restent obligatoires. En Automatique, le suivi source du résultat reste limité à 100 000 faces : un maillage admis à l’entrée peut donc être refusé si trop de surfaces restent facettisées. Ce refus indique le nombre de faces reconstruites, distinct du nombre de triangles importés. Dans cette première tranche, correction guidée, recherche de paramètres et coupe source sont indisponibles pour ces nouveaux fichiers denses. Les fonctions habituelles restent disponibles sous le seuil. Voir [ADR-0034](docs/adr/0034-isolated-large-stl-worker.md) et [R-067](roadmap/items/R-067-g1-gros-maillages-sur-accord-et-calcul-annulable.md).

Le script `scripts/build-macos-dev.sh` construit le bureau et son helper séparément et copie les deux dans `artifacts/FabSTEP.app`. Ne pas activer `large-stl` sur le bureau ordinaire ni réunir leurs builds Cargo. Le paquet autonome v1 n’est pas modifié.

### Exporter une face en DXF — build de développement

Dans **Résultat STEP**, sélectionner une seule face plane : son profil 2D apparaît dans l’inspecteur. Vérifier les dimensions, les trous et le repère, puis choisir **Exporter DXF**. Les contours natifs sont exportés en millimètres à l’échelle 1:1, avec leurs lignes, arcs, cercles et ellipses exactes. Les faces courbes et les contours contenant d’autres types de courbes ne sont pas projetés ou approximés. Un changement de sélection ou de résultat invalide le profil en cours.

Le DXF utilise le plan XY local affiché ; il ne conserve pas le placement 3D du solide. La coupe selon un plan du STEP est disponible dans le build de développement, comme décrit ci-dessous. Voir [ADR-0024](docs/adr/0024-planar-face-dxf.md) et les validations dans [VALIDATION](docs/VALIDATION.md).

### Couper le STEP par un plan — build de développement

Dans **Résultat STEP**, ouvrir **Coupe du STEP**, choisir XY/XZ/YZ ou le plan d’une face plane sélectionnée, puis régler le décalage et les corps concernés. Dans la vue 3D, **Déplacer** affiche une flèche pour translater le plan selon sa normale ; **Tourner** affiche deux anneaux pour l’incliner autour du centre de la pièce projeté sur le plan. Le décalage numérique reste disponible pour un réglage précis. Le profil est recalculé au relâchement ; Échap annule le geste. Choisir XY/XZ/YZ ou une face réinitialise l’orientation. Le profil calculé se prévisualise en 2D et s’exporte en DXF, en mm à l’échelle 1:1. Le solide reste intact.

Un corps non traversé est indiqué vide. Une coupe entièrement vide ne s’exporte pas ; un refus sur un corps choisi bloque tout l’export. Les coupes coplanaires, tangentes ou ambiguës, les splines et les autres courbes non prises en charge sont refusées explicitement. Certains refus sont conservateurs, notamment aux raccords entre faces et aux niveaux critiques de surfaces support tronquées et pour les plans passant par le sommet support d’un cône. Voir [ADR-0025](docs/adr/0025-step-section-dxf.md). Les coupes inclinées de cylindres prennent en charge les ellipses complètes ou tronquées, sans approximation polygonale. Les coutures périodiques internes prouvées ne bloquent plus une coupe régulière. La coupe directe du maillage source est décrite ci-dessous.

### Couper le maillage source — build de développement

Dans **Source**, ouvrir **Coupe du maillage**. L’outil prépare et valide les corps source, puis permet de déplacer ou tourner le plan, choisir les corps et exporter leur section polygonale en DXF 1:1, en millimètres. Il reste accessible après un échec de reconnaissance STEP, dès que le traitement est terminé. Les corps et les unités proviennent de la préparation native ; une source ouverte ou des corps en contact, imbriqués ou chevauchants sont refusés sans export partiel.

Les segments suivent les triangles du maillage préparé selon le soudage courant ; aucune courbe n’est reconstruite. **Prétraitement de la source** rassemble les nombres de sommets soudés et les diagnostics d’orientation. Le STEP existant reste intact. Les passages réguliers par un sommet ou une arête sont pris en charge ; les faces coplanaires, tangences et cas numériquement ambigus restent refusés. Le nom proposé se termine par `-mesh-section.dxf` pour distinguer cette sortie de la section STEP. Voir [ADR-0026](docs/adr/0026-source-mesh-section-dxf.md).

## Essayer sur ce Mac

Ouvrir le paquet autonome qualifié indiqué ci-dessous, puis **Charger le cube exemple** ou **Ouvrir STL / 3MF**. Choisir l’unité STL, absente du fichier ; pour 3MF, l’unité déclarée ou celle par défaut du format est indiquée et verrouillée. Utiliser **Exporter STEP** lorsque le résultat est validé. Le cube exemple mesure 20 mm et produit six faces. **Charger le cylindre exemple** sélectionne le mode Cylindres et produit trois faces, dont une cylindrique. **Charger la pièce mixte** montre un bossage et un trou borgne, avec huit plans, deux cylindres et les distances échantillonnées dans les deux sens. **Charger le cylindre chanfreiné** sélectionne le nouveau mode avec un exemple synthétique. **Charger le congé torique** sélectionne le mode Surfaces arrondies sur un exemple synthétique. **Charger les deux cubes 3MF** illustre deux corps placés séparément.

Pendant le traitement, un indicateur animé affiche la phase réelle (attente, lecture, conversion) et le temps écoulé. La barre est indéterminée : aucun pourcentage ni délai restant n’est estimé. L’aperçu reste manipulable et l’indicateur disparaît lorsque la demande courante réussit ou échoue.

Le paquet autonome est disponible dans `artifacts/release/1.0.0/FabSTEP.app` pour Apple Silicon sous macOS 26 ou ultérieur. Il embarque le noyau CAO et ses bibliothèques ; les conversions de contrôle ont été exécutées avec les lectures Homebrew bloquées. Le bundle `artifacts/FabSTEP.app` reste un build de développement distinct. Voir [construction et qualification du paquet](docs/RELEASE.md). `artifacts/` est généré et exclu de Git.

Les quatre pièces du [corpus mécanique indépendant](fixtures/complex_v1/README.md) ont été converties puis réimportées dans FreeCAD : perçages sur trois axes, double chanfrein, carter mêlant cinq familles de surfaces et boîtier arrondi. Les formes libres, intersections de perçages et congés généraux à rayon variable restent à développer. L’aperçu et l’export du boîtier ont été confirmés par l’utilisateur sur la candidate ; la version finale conserve le même code. Les essais automatisés et manuels sont distingués dans la validation.

## Ouvrir et reconstruire

Dans le build de développement, ouvrir un STL ou 3MF affiche d’abord le maillage source. Vérifier l’unité du STL puis cliquer sur **Reconstruire le STEP** dans le panneau de droite. Le résultat validé s’affiche ensuite automatiquement. Chaque nouveau fichier attend son propre lancement ; après ce premier calcul, les modifications de réglages conservent leur recalcul habituel. Le suivi affiche les étapes réelles du moteur, le corps en cours et la durée écoulée. Les opérations natives sans mesure interne gardent un indicateur d’activité ; aucun pourcentage de temps restant n’est estimé.

Un STL refusé pour dépassement du plafond affiche son nombre exact de triangles et la limite de 100 000 (binaire de longueur cohérente ou ASCII syntaxiquement valide). Ce plafond est une protection de l’application pour borner stockage et travail avant reconnaissance, construction CAO et contrôles ; ce n’est pas une limite du format STEP ni une garantie de durée pour tout modèle inférieur au plafond. La limite reste inchangée.

## Construire

Prérequis : Rust 1.88+, compilateur C++17, Node compatible avec Vite 8 et SDK OpenCASCADE 7.9. La construction contrôle les macros de version du SDK sélectionné et refuse les familles autres que 7.9.x, sans imposer un correctif exact. Le poste de validation utilise Rust 1.88, Node 26 et OCCT 7.9.3 sur macOS Apple Silicon.

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

La CLI conserve son défaut historique `--mode planar`. `--mode auto` active la reconstruction automatique locale du bureau ; les modes explicites restent stricts et ne changent pas de contrat. Le mode planaire conserve les facettes des courbes. `--mode cylinders` cherche une extrusion complète et des cylindres locaux ; il privilégie le modèle local lorsqu’un arc ouvert y est prouvé. `--mode primitives` essaie les cônes et cylindres prouvés, puis la reconnaissance cylindrique historique uniquement en l’absence de candidat conique ; un candidat non prouvé ou un budget dépassé reste bloquant. Si aucune reconstruction n'est prouvée, le refus est explicite. `--mode surfaces` reconnaît ces portions arrondies ; les reconnaissances de boîtier arrondi, tore entier puis sphère entière sont essayées dans cet ordre uniquement si le parcours historique Surfaces → Primitives → Cylindres conclut à `recognition.unsupported`. Le mode Surfaces peut aussi prouver deux chanfreins opposés séparés par une portée cylindrique suffisamment longue. Un succès historique, une preuve échouée ou un budget dépassé reste prioritaire. `--fit` fixe la tolérance d'ajustement radial (défaut `0.00001` mm) et `--max-deviation` le budget de la borne composée (défaut `0.05` mm). Les modes analytiques interprètent des profils suffisamment circulaires en cercles et, en mode primitives, des bandes admissibles en cônes : le maillage ne contient pas l'intention de conception.

## Précision des fichiers réels

Les seuils initiaux restent inchangés. Dans **Avancé**, la **Tolérance radiale** limite le résidu radial d’ajustement des surfaces reconnues ; le **Budget d’écart** limite la borne composée, qui inclut notamment la flèche des cordes. Les modifier est un choix explicite, sans garantie de reconnaissance. L’inspecteur distingue le résidu radial mesuré, la borne composée, les distances échantillonnées et les **Réglages appliqués**. Les rapports JSON `check` et `convert` conservent ces réglages dans `settings`, séparément de l’unité source ; `precision.custom_tolerances` signale les seuils actifs personnalisés.

Dans le build de développement, **Rechercher un réglage** compare jusqu'à sept tolérances radiales autour de la valeur courante. Chaque essai valide le document entier ; le mode, l'unité, le soudage, la planéité et le budget d'écart restent fixes. La proposition privilégie le moins de faces, puis le seuil le plus strict à égalité, parmi ces seuls essais. Les essais et leurs refus restent consultables. **Appliquer** change explicitement le seuil et relance une conversion complète ; la recherche seule ne remplace pas le STEP courant. Cette action est disponible dans les modes analytiques, hors session d'indications manuelles. Une tolérance plus large peut reconnaître moins de surfaces. Voir [ADR-0023](docs/adr/0023-assisted-radial-search.md).

Le 3MF du premier fichier réel, réexporté par FreeCAD à six chiffres significatifs, est refusé au seuil radial initial de 0,00001 mm. Avec **0,0001 mm choisi manuellement**, il produit 14 faces (11 planes et 3 cylindriques), réimportées et validées dans FreeCAD. Le budget reste à 0,05 mm ; ce réglage n’est pas une recommandation universelle pour tout fichier arrondi.

La seconde pièce réelle contient un cône, une sphère et un tore : ses STL et 3MF produisent chacun 19 faces (12 plans, 4 cylindres, 1 cône, 1 sphère et 1 tore), réimportées dans FreeCAD. Les volumes sont respectivement 54 445,27801611677 et 54 445,278047030406 mm³, contre 54 445,27799128441 mm³ pour le STEP de référence. Les dimensions sont 60,5 × 59,5 × 28 mm et les plans sont situés à Z = 0, 14, 18 et 28 mm ; la sphère a un rayon de 10 mm et le tore un rayon mineur de 2 mm. Les tangences ont été vérifiées dans FreeCAD. Le mode **Plans**, avec une planéité explicitement resserrée à **0,0000001 mm**, produit un solide fermé facettisé (8 211 faces pour le STL, 8 360 pour le 3MF), validé dans FreeCAD. Un seuil plus strict conserve davantage de petites faces ; aucune projection des sommets n’est ajoutée. Les différences avec le volume du STEP d’origine viennent déjà de son maillage. Voir les mesures et les limites dans la [validation](docs/VALIDATION.md) et la [comparaison locale du STL](artifacts/m5/verified/body2-surfaces-stl-comparison.json).

## Périmètre et limites

- Jusqu’à 32 corps distincts, chacun fermé et orientable ; les défauts topologiques sont refusés, sans fermeture inventée. Une inversion globale est corrigée et signalée. Tout contact, chevauchement ou imbrication entre corps est refusé à la source et au résultat, sans fusion ni export partiel.
- Jusqu’à 64 Mio en entrée et 100 000 triangles au total après placements, avec budgets supplémentaires d’import et de validation des contours. Ce plafond n'est pas une promesse de performance sur toute géométrie.
- Les déplacements dus au soudage sont comptés ; les sommets ne sont pas projetés sur les plans.
- OCCT exige une planéité native d'environ `0.0000001` mm sur les contours des plans reconstruits ou conservés. La version STL binaire f32 d'une pièce mixte tournée peut fragmenter un bouchon à la reconnaissance. Une tolérance de segmentation plus large peut rétablir la reconnaissance sans rendre ses plans acceptables par OCCT.
- La vue **Source** montre les maillages importés placés et les frontières reconnues ; elle reste disponible après un échec de conversion. La vue **Résultat STEP**, propre au build de développement, montre une tessellation des faces natives du résultat validé courant. Son chargement ou son refus ne modifie pas le STEP exportable. La correspondance exacte avec les triangles source est disponible en mode **Conserver les facettes** et sur les résultats cylindriques guidés, lorsque le suivi complet à travers la construction et le transfert STEP est vérifié. Elle reste indisponible pour les autres reconstructions analytiques ; aucun surlignage approximatif n’est présenté comme exact. Voir [ADR-0018](docs/adr/0018-planar-step-source-provenance.md).
- Préférences persistées et annuler/rétablir des options ; la reprise du fichier source après fermeture reste à développer.
- Les cylindres locaux complets ou partiels doivent être non intersectants et délimités par deux contours plans source correspondants. La reconnaissance exige un échantillonnage cyclique compatible, au moins 12 points par cercle et un intervalle angulaire maximal de 45°. Un fond borgne doit être plan et présent dans le STL. Les régions sans candidat restent planes ou facettisées. La recherche historique des arcs ouverts exige au moins huit points, un balayage de 30° à 330°, un pas maximal de 45°, deux contours plans correspondants et des raccords latéraux démontrés. Un profil en D est distingué par une unique ouverture angulaire d’au moins 90° et des autres pas d’au plus 45° ; le STL ne prouve pas l’intention de conception. Des arcs bruités ou très denses peuvent être refusés par les contrôles conservateurs ou le budget de travail. Les intersections de perçages et les congés généraux restent hors du périmètre prouvé.
- Les cônes complets exigent des anneaux coaxiaux et une variation radiale affine prouvée ; une pointe doit être présente dans la source. Les cônes partiels ne sont admis que dans les raccords ouverts spécifiques du build de développement (ADR-0030). La limite historique d’une portion conique par chaîne est étendue au cas Cône–Cylindre–Cône : pentes opposées, portée et changements de pente résolus par rapport aux tolérances. Le build de développement admet deux à huit cônes de signes libres avec portées résolues et témoins intérieurs (ADR-0031), ainsi que deux cônes directement adjacents à rupture source unique prouvée (ADR-0032). Les successions plus générales sans portée restent hors périmètre. Les contrats historiques sont conservés ; voir [ADR-0015](docs/adr/0015-resolved-conical-chains.md). En mode Cônes et cylindres, dans un corps comportant un cône reconnu, les arcs cylindriques ouverts ne sont pas reconstruits analytiquement ; en absence de candidat conique, le repli historique peut les reconnaître. Les chanfreins droits restent des plans. Les congés généraux restent hors périmètre. Voir [ADR-0010](docs/adr/0010-resolved-revolutions.md).
- Un corps sphérique complet est reconnu sans anneau ni pôle imposé dans la source, après contrôle du maillage fermé entier. Les régions planes significatives sont exclues. Des petites facettes ou des quadrilatères cosphériques restent indiscernables d’une tessellation sphérique : le maillage ne révèle pas l’intention de conception. Voir [ADR-0009](docs/adr/0009-complete-spheres.md).
- Un tore entier doit être un corps annulaire fermé, sans auto-intersection, avec couverture des deux directions périodiques démontrée. Un boîtier arrondi doit comporter six appuis plans rectangulaires, douze quarts de cylindre et huit octants sphériques à rayon commun ; la reconnaissance ne déduit pas la forme de sa seule boîte englobante. Ce contrat ne couvre pas encore les boîtiers creux, les arrondis partiels ou des perçages ajoutés au boîtier. Voir [ADR-0011](docs/adr/0011-uniform-rounded-box.md).
- Les calottes sphériques exigent un anneau circulaire plan, un pôle source et une courbure intérieure suffisamment échantillonnée et distincte d’un cône. Le build de développement reconnaît aussi les zones sphériques entre deux anneaux sans pôle, sous le contrat annulaire de l’[ADR-0014](docs/adr/0014-spherical-zones.md). Les formes dont la hauteur, la couverture ou la courbure ne sont pas résolues ne sont pas promues en sphères. Le bord hors équateur fait l’objet d’une borne supplémentaire ; voir [ADR-0008](docs/adr/0008-spherical-caps.md).
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

### Raccords de rainures chanfreinées (développement)

En mode Automatique, FabSTEP peut reconstruire ensemble les bandes cylindriques ouvertes et leurs chanfreins coniques partiels aux extrémités demi-circulaires de rainures. Sélectionner une bande puis essayer sa famille inclut les surfaces raccordées nécessaires ; elles ne sont pas détachées du reste du solide. Cette prise en charge reste limitée aux chaînes et contours prouvés, sans augmenter les tolérances. En cas de correction refusée, « Correction non appliquée » propose directement un recalcul des indications conservées, ou sans indication si aucune n’a été acceptée. Qualification : R-058 dans la [feuille de route](ROADMAP.md).

### Chaînes de chanfreins de même sens (développement)

Les modes Automatique, Cônes et cylindres et Surfaces arrondies reconnaissent aussi deux à huit cônes complets dont les rayons évoluent dans le même sens, lorsqu’ils sont séparés par des portées cylindriques démontrées et suffisamment échantillonnés. Sans portée, seuls deux cônes à rupture unique et témoins intérieurs prouvés sont ajoutés par R-060 ; les autres profils ou cas ambigus restent refusés ou facettisés selon le mode. Les tolérances ne changent pas. Exemples indépendants et mesures attendues : `fixtures/same_sign_chains/manifest.json`.

### Révolutions et pièces creuses — développement

Deux cônes directement raccordés sont reconstruits lorsqu’une unique rupture et les anneaux intérieurs des deux parois sont prouvés. Exemple : `fixtures/adjacent_cones/increasing.stl`. Les chaînes plus longues sans portée restent hors du premier lot.

Automatique et Surfaces arrondies prennent en charge deux quarts de tore extérieurs reliés par une portée cylindrique résolue, avec des rayons d’arrondi éventuellement différents : `fixtures/double_torus/nominal.stl`. Ce contrat n’est pas une révolution à profil libre.

Le corpus `fixtures/partial_fillets` qualifie aussi les arêtes droites arrondies disjointes et un bac ouvert à coins verticaux arrondis, en Automatique ou Cylindres. Le fond et la cavité sont conservés ; les congés du plancher/rebord, coins à trois branches et rayons variables restent distincts. Les paramètres et limites sont dans les README des corpus et V-1186 de [VALIDATION](docs/VALIDATION.md).
