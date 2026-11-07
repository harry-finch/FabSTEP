# Architecture FabSTEP

FabSTEP reconstruit une représentation CAO à partir d'un maillage. Le fichier source ne contient pas l'intention de conception : toute reconnaissance doit fournir son erreur et sa provenance.

## Frontières

| Module | Responsabilité | Dépendance externe |
| --- | --- | --- |
| `fabstep-import` | Lecture bornée 3MF Core/OPC, unités, instances et transformations | ZIP/XML |
| `fabstep-core` | Maillages en mémoire, soudage, topologie et reconnaissance analytique bornée avec attribution des triangles source | Serde |
| `fabstep-cad` | Construction et contrôle BRep, écriture et réimport STEP, tessellation pour l'audit | OpenCASCADE via ABI C |
| `fabstep-verify` | Comparaison bidirectionnelle échantillonnée des maillages | core |
| `fabstep-app` | Pipeline commun, contrôles des dimensions, du volume et de la fidélité | core, cad, verify |
| `fabstep-cli` | Arguments, lecture bornée, rapport, écriture du fichier | app |
| `fabstep-desktop` | Fenêtre Tauri, tâches de calcul, résultat validé en mémoire, dialogue d'export | app, Tauri |
| `ui` | Présentation, préférences, aperçu 3D et commandes utilisateur | Svelte 5, Three.js |

Le workspace principal comprend import, core, cad, verify, app et cli. Le bureau reste exclu comme dans FabWeave pour que le moteur puisse être compilé et testé indépendamment du frontend. La représentation du maillage et son audit restent en Rust pur.

## Parcours des données

STL ou 3MF en octets → unité STL choisie ou unité 3MF déclarée/par défaut → instances et placements en mm → aperçu des maillages source → séparation des composantes → soudage et contrôles topologiques → contrôle CAO de séparation source → reconstruction par corps → STEP à une ou plusieurs racines → réimport, contrôle du résultat et de la séparation → export seulement si tous les corps sont validés.

L’import 3MF Core suit les items du build et leurs composants, conserve les placements et ne fusionne pas les corps. Les références multi-modèles de l’extension Production ne sont pas prises en charge ; les extensions requises inconnues sont refusées et les métadonnées facultatives ignorées signalées. Les limites globales comprennent 32 corps et 100 000 triangles après placement. Le STEP multi-racines conserve les solides et leurs placements, sans constituer un assemblage XCAF. Les noms produits peuvent être normalisés et comptés ; les noms source restent dans le rapport.

Les tolérances de soudage, de planéité et d'ajustement radial restent distinctes. Aucun seuil n’est adapté automatiquement à un refus ou à une précision supposée du fichier. `ProcessingSettings` enregistre le mode et les quatre seuils de la requête ; les seuils de reconnaissance analytique sont actifs dans les modes analytiques, y compris Surfaces arrondies. Ce snapshot est retourné par l’analyse et la conversion de document, la conversion STL historique et les rapports JSON CLI. L’unité effective demeure dans `DocumentSource`, car la préférence STL ne s’applique pas au 3MF. `precision.custom_tolerances` signale une différence des seuils actifs par rapport aux valeurs initiales. L’inspecteur lit les réglages retournés par le moteur, distincts des réglages de la prochaine requête et des mesures de fidélité.

Les limites de taille et de complexité déclenchent des refus explicites. Le compteur de travail d’un reconnaisseur est plafonné à 10 millions d’unités ; les contrôles de contours ont leur limite de 5 millions et l’audit de distance celle de 50 millions. Les recherches, raffinements et attributions débitent leurs compteurs ; aucun nouvel interpréteur ne relève les seuils pour faire réussir un cas.

Les modes partagent l'analyse topologique. **Plans** construit un `PlanarModel` et préserve la représentation facettisée des courbes. **Cylindres** essaie d'abord `AnalyticExtrusion`, une extrusion complète à profils polygonaux et circulaires dont la paroi entière est vérifiée. Il vérifie aussi `MixedModel` pour privilégier les arcs ouverts lorsqu’ils sont prouvés. Si l’extrusion échoue avec `recognition.unsupported`, il utilise la reconnaissance locale ; les autres erreurs, dont les refus de budget ou de preuve d’un arc candidat, sont bloquantes.

**Cônes et cylindres** utilise `RevolvedRecognition` et son modèle mixte enrichi de cônes. Une bande et ses raccords doivent être prouvés ; une pointe de rayon nul provient de la source. Seul `recognition.unsupported`, en l’absence de candidat, autorise le repli vers la reconnaissance cylindrique historique ; `recognition.conic_unproven` et les budgets restent bloquants. Des bandes coniques consécutives doivent démontrer une même loi affine de rayon. La limite initiale d’une portion conique par chaîne admet désormais une exception précise : deux cônes de pentes opposées séparés par une portée cylindrique résolue au regard des tolérances. Les pentes de même signe, les séparateurs presque cylindriques ambigus et les autres chaînes multiples restent refusés. Ce parcours conserve sa correspondance entre anneaux ; les cônes partiels restent exclus. Il ne promeut pas les arcs cylindriques ouverts dans un corps comportant un cône ; le mode Surfaces dispose de leur preuve mixte. Voir la décision initiale [ADR-0006](docs/adr/0006-cones-et-chanfreins-explicites.md) et son amendement [ADR-0010](docs/adr/0010-resolved-revolutions.md).

**Surfaces arrondies** commence par `RoundedRecognition`, qui enrichit le modèle de révolution de portions sphériques et toriques. Il reconnaît des calottes sphériques avec anneau plan et pôle source, convexes ou concaves, ainsi que des congés toriques quart de cercle entre cylindre et plan. La preuve paramétrique des triangles ne demande pas des anneaux échantillonnés de manière identique. Elle traite aussi la nouvelle chaîne à deux cônes opposés et portée résolue, même sans sphère ni tore ; les anciens cas à un cône conservent leur parcours. Hors équateur, une calotte exige une courbure intérieure distincte d’un cône ; la couverture géodésique et la frange du contour circulaire sont contrôlées séparément. Les arcs cylindriques ouverts peuvent compléter les patchs prouvés. Les comptes de tores et de congés sont distincts. Voir [ADR-0007](docs/adr/0007-rounded-surfaces.md), [ADR-0008](docs/adr/0008-spherical-caps.md) et [ADR-0010](docs/adr/0010-resolved-revolutions.md).

Le [dispatch app](crates/fabstep-app/src/lib.rs) conserve les succès et les refus bloquants historiques. En mode Surfaces seulement, l’ordre complet est :

```text
Surfaces par patchs → Primitives → Cylindres
                                      ↓ absence de candidat dans tout le parcours
                              Boîtier arrondi complet
                                      ↓ absence de candidat
                                  Tore complet
                                      ↓ absence de candidat
                                 Sphère complète
```

Chaque flèche exige exactement `recognition.unsupported`. Un candidat non prouvé, une topologie invalide, un budget dépassé ou une déviation excessive interdit le repli. Aucun de ces essais n’active implicitement le mode Plans. Le boîtier exige six supports plans dominants ; le tore exige χ = 0. Leur absence permet l’essai terminal suivant, sans effacer un refus de preuve.

Les **corps toriques complets** utilisent un `RoundedModel` isolé contenant un seul `MixedTorus`. `rings: None` signifie qu’aucun contour source n’est imposé ; `axis: Some(direction)` fournit l’axe unitaire démontré. Les congés existants gardent `rings: Some([i, j])` et `axis: None`, sans modification de leur JSON. Le core ajuste le tore dans des coordonnées normalisées, affine sa distance géométrique par un calcul déterministe borné, puis prouve les résidus, l’orientation et la couverture UV de degré un. Les tores cornus, auto-intersectants, elliptiques ou insuffisamment résolus restent hors périmètre. Aucun anneau, axe nominal ou pôle n’est inventé à partir d’une boîte englobante. Voir [ADR-0010](docs/adr/0010-resolved-revolutions.md).

Les **boîtiers convexes entièrement arrondis à rayon uniforme** disposent d’un contrat séparé `RoundedBoxRecognition` / `RoundedBoxModel`. Six rectangles plans source fixent le repère, les dimensions et le retrait commun ; toutes les parois doivent ensuite prouver les 6 plans, 12 quarts de cylindre et 8 octants sphériques. Les sommets source sont conservés, les coutures partagent une projection unique, et chaque triangle est attribué à un domaine. Le contrôle natif construit le boîtier, réimporte ses 26 faces et vérifie les supports, domaines, aires, rayons, volume, dimensions et 48 raccords tangents. La permission des octants reste propre à ce solide : le regroupement multi-racines conserve son repère et ses comptes de surfaces avant et après réimport. Il n’autorise pas des sphères partielles arbitraires dans les anciens modèles. Ce contrat n’est pas une reconnaissance générale des congés ; voir [ADR-0011](docs/adr/0011-uniform-rounded-box.md).

Les **corps sphériques complets** conservent le contrat de l’[ADR-0009](docs/adr/0009-complete-spheres.md). `MixedSphere.rings` vaut `Some([i, j])` pour une calotte et `None` (`null` en JSON) pour une sphère complète sans frontière source. Ce dernier cas contient une seule sphère extérieure, aucun plan ni anneau, conserve les sommets et attribue tous les triangles à la sphère. Les régions planes significatives sont exclues ; de petites facettes ou des quadrilatères cosphériques restent indiscernables d’une tessellation sphérique. La preuve combine topologie, résidus, orientation, couverture 4π et borne des intérieurs triangulaires.

`MixedModel` associe des faces planes et des parois cylindriques complètes ou partielles. Dans le parcours cylindrique historique, chaque anneau circulaire lie exactement un plan et un cylindre ; les modèles enrichis ajoutent leurs propres incidences démontrées entre surfaces. Les anneaux reprennent les contours démontrés du STL, y compris le contour du fond plan d'un trou borgne. Pour les portions ouvertes, deux arcs plans correspondants et leurs raccords latéraux sont prouvés ; les contours mixtes conservent segments droits et arêtes circulaires. Les arcs admis ont au moins huit points, un balayage de 30° à 330° et un pas maximal de 45°. Un profil en D exige une unique ouverture angulaire d’au moins 90° et des autres pas d’au plus 45° ; les contours cocirculaires faiblement échantillonnés dont plusieurs intervalles sont larges ne sont pas interprétés comme arcs ouverts. Des arcs bruités ou très denses peuvent être refusés par la recherche locale ou son budget de travail. Chaque triangle source est attribué à une face reconnue ou conservée. OCCT réutilise les mêmes arêtes circulaires sur les deux faces adjacentes et contrôle la fermeture, les incidences et les auto-interférences.

Dans les modes analytiques, la segmentation interne utilise `max(plane, fit)` pour absorber une partie du bruit de quantification du STL. La source garde ses sommets et chaque triangle soudé est contrôlé dans une BRep native avant reconstruction. Ce contrôle source ne sérialise aucun STEP intermédiaire. Le STEP final réimporté doit préserver les types et nombres de surfaces, le volume analytique attendu et des dimensions cohérentes avec le budget.

Pour l'audit de fidélité, OCCT tesselle le STEP réimporté. `fabstep-verify` mesure les distances aux triangles avec un arbre de boîtes englobantes, en prenant les sommets, milieux d'arêtes et barycentres de chaque triangle dans les deux sens. Le rapport donne les maxima mesurés, les nombres d'échantillons et la déflexion demandée. Le budget d'audit tient compte de cette déflexion et d'une marge numérique. La borne composée de reconnaissance et les deux maxima échantillonnés sont distincts. Les preuves analytiques bornent leurs domaines admis ; les deux maxima d’audit restent des mesures échantillonnées, et non une certification globale de Hausdorff du STEP natif. Les optimisations de parcours de l’arbre conservent les distances exactes aux triangles et les mêmes échantillons ; elles ne remplacent pas des branches par une approximation pour respecter le budget.

## Concurrence et export

L’interface sérialise l’aperçu et la conversion sous une même révision et abandonne les réponses obsolètes. L’aperçu source importé reste disponible si une conversion CAO échoue ; il n’autorise jamais l’export. Chaque changement invalide immédiatement le cache natif avec une révision monotone ; les messages IPC arrivant en retard ne peuvent remplacer ni supprimer une révision plus récente. Tauri exécute le calcul sur une tâche bloquante séparée et garde les octets STEP du résultat validé. L'export reçoit l'identifiant de la conversion, pas du texte STEP arbitraire. Le résultat est revérifié après le dialogue de sauvegarde ; l'écriture utilise un fichier temporaire dans le dossier de destination.

L’indicateur de traitement suit la révision courante, distincte du verrou d’exécution d’un ancien calcul. Il passe de l’attente à la lecture d’aperçu, puis à la conversion sur les appels réellement lancés. Le chronomètre monotone inclut l’attente et est réinitialisé à chaque demande ; une réponse obsolète ne peut pas l’effacer. Le noyau ne publie pas de progression fine : la barre reste indéterminée.

Le pipeline et le classement des régions doivent être reproductibles. Les métadonnées horodatées d'OCCT ne sont pas une promesse de STEP identique octet pour octet.

## Évolutions prévues

Élargir l’import au-delà de 3MF Core et des corps spatialement séparés, reconnaître d’autres formes de révolution et confronter les résultats à un corpus utilisateur. L'historique paramétrique reste hors du périmètre actuel.

Voir [ADR-0001](docs/adr/0001-bureau-rust-et-occt.md), [ADR-0002](docs/adr/0002-extrusions-cylindriques-explicites.md), [ADR-0003](docs/adr/0003-pieces-mixtes-et-audit-distance.md), [ADR-0004](docs/adr/0004-arcs-cylindriques-ouverts.md) et [audit du document de référence](docs/REFERENCE_REVIEW.md).

Voir aussi [ADR-0005 — documents 3MF et corps distincts](docs/adr/0005-documents-3mf-et-corps-distincts.md).


## Stabilisation et paquet local

Tous les modes font valider les triangles source avant de construire le solide de sortie, y compris Plans. Une simplification des régions planes ne peut donc pas masquer une auto-intersection refusée sur le maillage d’origine.

`FabError` conserve `code` et le message CLI historique ; le champ optionnel `body { id, name, detail }` transporte le contexte sans imposer à l’interface de découper un nom utilisateur dans du texte. Les noms STL générés sont traduits selon leur identifiant, les noms source 3MF restent littéraux.

Le paquet macOS est construit séparément des builds de développement. Il relocalise les bibliothèques dans `Contents/Frameworks`, supprime les chemins de recherche externes, contrôle la cible minimale de chaque Mach-O puis signe les copies. Sa qualification compare les dépendances constatées au manifeste et exécute des conversions avec Homebrew interdit en lecture. `TKFillet`, utilisé pour le boîtier, fait partie de la fermeture de dépendances contrôlée du paquet qui embarque cette capacité. Aucun objet natif ne traverse l’ABI et aucune dépendance OCCT n’est ajoutée au core. La qualification de chaque paquet reste distincte des tests du build de développement.
