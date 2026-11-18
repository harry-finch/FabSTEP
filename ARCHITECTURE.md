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

## Repères de code et de jalons

Les identifiants M0–M6, A/B/C1, I1–I3 et UX désignent les [jalons de la roadmap](ROADMAP.md), qui centralise leur état et leurs liens vers les [ADR](docs/adr/). Le registre [INT-AUDIT-01](docs/INT_AUDIT_01.md) suit les décisions d'intégration de l'audit ; il ne remplace pas les preuves de validation.

- `fabstep-core/src/partial/short.rs` : arcs courts contextuels (ADR-0022) ; `rounded/zone.rs` : zones sphériques (ADR-0014) ; `revolved.rs` : chaînes coniques (ADR-0015).
- `fabstep-app/src/document.rs` : documents, provenance et indications (I1c/I2a/I2b) ; `parameter_search.rs` : recherche radiale explicite (UX2, ADR-0023).
- `ui/src/lib/selection.worker.ts` : inspection source (I1) ; `step-selection.worker.ts` : sélection native et préparation de l'affichage des faces (I1b, UX3).
- `fabstep-desktop/src/main.rs` : commandes `selection_topology`, `result_faces`, `search_parameters`, `guided_convert`, `remove_guidance` et `recompute_guidance`, en complément de l'import, de la conversion et de l'export.

## Parcours des données

STL ou 3MF en octets → unité STL choisie ou unité 3MF déclarée/par défaut → instances et placements en mm → aperçu des maillages source → séparation des composantes → soudage et contrôles topologiques → contrôle CAO de séparation source → reconstruction par corps → STEP à une ou plusieurs racines → réimport, contrôle du résultat et de la séparation → export seulement si tous les corps sont validés.

L’import 3MF Core suit les items du build et leurs composants, conserve les placements et ne fusionne pas les corps. Les références multi-modèles de l’extension Production ne sont pas prises en charge ; les extensions requises inconnues sont refusées et les métadonnées facultatives ignorées signalées. Les limites globales comprennent 32 corps et 100 000 triangles après placement. Le STEP multi-racines conserve les solides et leurs placements, sans constituer un assemblage XCAF. Les noms produits peuvent être normalisés et comptés ; les noms source restent dans le rapport.

Les tolérances de soudage, de planéité et d'ajustement radial restent distinctes. Aucun seuil n’est adapté automatiquement à un refus ou à une précision supposée du fichier. `ProcessingSettings` enregistre le mode et les quatre seuils de la requête ; les seuils de reconnaissance analytique sont actifs dans les modes analytiques, y compris Surfaces arrondies. Ce snapshot est retourné par l’analyse et la conversion de document, la conversion STL historique et les rapports JSON CLI. L’unité effective demeure dans `DocumentSource`, car la préférence STL ne s’applique pas au 3MF. `precision.custom_tolerances` signale une différence des seuils actifs par rapport aux valeurs initiales. L’inspecteur lit les réglages retournés par le moteur, distincts des réglages de la prochaine requête et des mesures de fidélité.

Les limites de taille et de complexité déclenchent des refus explicites. Le compteur de travail d’un reconnaisseur est plafonné à 10 millions d’unités ; les contrôles de contours ont leur limite de 5 millions et l’audit de distance celle de 50 millions. Les recherches, raffinements et attributions débitent leurs compteurs ; aucun nouvel interpréteur ne relève les seuils pour faire réussir un cas.

G2 filtre les couples d’arêtes des contours planaires par un arbre déterministe de boîtes 2D fermées (`planar/broad_phase.rs`). Seules les boîtes strictement séparées sont écartées ; les prédicats de contact et de croisement restent ceux du parcours exhaustif. Jusqu’à 64 segments, ou hors du domaine numérique démontré pour le filtre, le parcours historique est conservé. Le tri, la construction et les visites débitent le budget partagé de cinq millions ; le stockage est linéaire et les nouvelles réservations sont faillibles. L’inclusion des trous reste exhaustive et bornée. Aucun sommet, contour, seuil de planéité ou contrôle CAO n’est modifié.

G3 réutilise la préparation topologique globale dans l’analyse mono-corps uniquement lorsque les indices source forment exactement `0..N`. Le maillage soudé, ses statistiques et l’ordre original d’adjacence passent à la même finition volume/orientation ; les composantes multiples gardent leur préparation locale historique. Dans `rounded::classify`, la recherche d’arêtes horizontales parcourt l’union des arêtes des triangles du patch lorsque sa borne de coût est inférieure au scan global. Les incidences restent celles du maillage entier, l’ordre BTree et les prédicats sont conservés ; construction et parcours locaux débitent le budget existant. Aucun cache global ni contrat public supplémentaire.

R-071 conserve les fenêtres et les ajustements des arcs ouverts. Pour leur maximalité, un contour à sommets source uniques possède une bijection entre positions et arêtes non orientées ; les chaînes candidates deviennent des intervalles cycliques strictement plus courts que le tour complet. L’inclusion se calcule alors sur les indices, sans opérations géométriques supplémentaires. Une copie faillible et un tri par tas explicitement débité vérifient l’unicité ; les contours non éligibles gardent les ensembles historiques. Ordre, départage, fusion et preuve des unions restent inchangés. Le garde peut augmenter le coût des cas peu candidats ; un épuisement ne permet jamais de revenir à un autre algorithme avec un budget neuf.

Les modes partagent l'analyse topologique. **Plans** construit un `PlanarModel` et préserve la représentation facettisée des courbes. **Cylindres** essaie d'abord `AnalyticExtrusion`, une extrusion complète à profils polygonaux et circulaires dont la paroi entière est vérifiée. Il vérifie aussi `MixedModel` pour privilégier les arcs ouverts lorsqu’ils sont prouvés. Si l’extrusion échoue avec `recognition.unsupported`, il utilise la reconnaissance locale ; les autres erreurs, dont les refus de budget ou de preuve d’un arc candidat, sont bloquantes.

**Cônes et cylindres** utilise `RevolvedRecognition` et son modèle mixte enrichi de cônes. Une bande et ses raccords doivent être prouvés ; une pointe de rayon nul provient de la source. Seul `recognition.unsupported`, en l’absence de candidat, autorise le repli vers la reconnaissance cylindrique historique ; `recognition.conic_unproven` et les budgets restent bloquants. Des bandes coniques consécutives doivent démontrer une même loi affine de rayon, sauf la paire à rupture unique prouvée de R-060 décrite plus bas. La limite initiale d’une portion conique par chaîne admet désormais une exception précise : deux cônes de pentes opposées séparés par une portée cylindrique résolue au regard des tolérances. Le build de développement admet aussi deux à huit cônes par chaîne, avec signes libres et portées cylindriques résolues (ADR-0031). Deux anneaux intérieurs distincts, séparés au-delà de l’incertitude, sont exigés dans chaque nouvelle portion ; toutes les fenêtres cône–portée–cône reprennent le garde historique. Des cylindres terminaux sont possibles. Les séparateurs ambigus et les chaînes générales sans portée restent hors périmètre ; R-060 ajoute uniquement la paire de cônes adjacents à rupture unique (ADR-0032). Voir [ADR-0015](docs/adr/0015-resolved-conical-chains.md). Ce parcours conserve sa correspondance entre anneaux ; les cônes partiels restent exclus. Il ne promeut pas les arcs cylindriques ouverts dans un corps comportant un cône ; le mode Surfaces dispose de leur preuve mixte. Voir la décision initiale [ADR-0006](docs/adr/0006-cones-et-chanfreins-explicites.md) et son amendement [ADR-0010](docs/adr/0010-resolved-revolutions.md).

**Surfaces arrondies** commence par `RoundedRecognition`, qui enrichit le modèle de révolution de portions sphériques et toriques. Il reconnaît des calottes sphériques avec anneau plan et pôle source, convexes ou concaves, ainsi que des congés toriques quart de cercle entre cylindre et plan. La preuve paramétrique des triangles ne demande pas des anneaux échantillonnés de manière identique. Elle traite aussi le double chanfrein historique et les nouvelles chaînes C1a à trois à huit cônes alternés avec portées résolues et témoins intérieurs, même sans sphère ni tore ; les anciens cas à un cône conservent leur parcours. Hors équateur, une calotte exige une courbure intérieure distincte d’un cône ; la couverture géodésique et la frange du contour circulaire sont contrôlées séparément. Les arcs cylindriques ouverts peuvent compléter les patchs prouvés. Depuis la reprise post-v1, un candidat à exactement un cône complet peut aussi entrer dans ce parcours sans sphère ni tore, uniquement après preuve effective d’un cylindre ouvert. La preuve UV différée et les orientations finales précèdent tout retour du modèle ; sans arc prouvé, le repli historique reste applicable. Les preuves partagent le même budget et leurs erreurs restent bloquantes. Voir [ADR-0013](docs/adr/0013-cone-open-cylinder-combinations.md). Les comptes de tores et de congés sont distincts. Voir [ADR-0007](docs/adr/0007-rounded-surfaces.md), [ADR-0008](docs/adr/0008-spherical-caps.md) et [ADR-0010](docs/adr/0010-resolved-revolutions.md).

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

Les **zones sphériques sans pôle** disposent d’une preuve annulaire dans `rounded/zone.rs`. Deux anneaux plans source déterminent un candidat normalisé ; des échantillons intérieurs doivent démontrer une courbure distincte d’un cône ou cylindre. La projection est contrainte aux deux latitudes, avec déplacement vectoriel compté. La preuve vérifie topologie, orientation, tours uniques, séparation des contours géodésiques et couverture par différence d’éventails ; elle ajoute les deux franges au budget. Le pôle auxiliaire utilisé dans l’intégrale d’aire n’appartient ni à la source ni au modèle. Le constructeur natif utilise les deux anneaux positifs du `MixedSphere` existant, sans nouvelle ABI. Une zone concave exige ses autres parois source et reste un solide à une coque, pas une cavité enfermée imbriquée. Voir [ADR-0014](docs/adr/0014-spherical-zones.md).

Les **corps sphériques complets** conservent le contrat de l’[ADR-0009](docs/adr/0009-complete-spheres.md). `MixedSphere.rings` vaut `Some([i, j])` pour une calotte ou une zone à deux anneaux et `None` (`null` en JSON) pour une sphère complète sans frontière source. Ce dernier cas contient une seule sphère extérieure, aucun plan ni anneau, conserve les sommets et attribue tous les triangles à la sphère. Les régions planes significatives sont exclues ; de petites facettes ou des quadrilatères cosphériques restent indiscernables d’une tessellation sphérique. La preuve combine topologie, résidus, orientation, couverture 4π et borne des intérieurs triangulaires.

`MixedModel` associe des faces planes et des parois cylindriques complètes ou partielles. Dans le parcours cylindrique historique, chaque anneau circulaire lie exactement un plan et un cylindre ; les modèles enrichis ajoutent leurs propres incidences démontrées entre surfaces. Les anneaux reprennent les contours démontrés du STL, y compris le contour du fond plan d'un trou borgne. Pour les portions ouvertes, deux arcs plans correspondants et leurs raccords latéraux sont prouvés ; les contours mixtes conservent segments droits et arêtes circulaires. Les arcs de la recherche historique ont au moins huit points, un balayage de 30° à 330° et un pas maximal de 45°. Un profil en D exige une unique ouverture angulaire d’au moins 90° et des autres pas d’au plus 45° ; les contours cocirculaires faiblement échantillonnés dont plusieurs intervalles sont larges ne sont pas interprétés comme arcs ouverts. Des arcs bruités ou très denses peuvent être refusés par la recherche locale ou son budget de travail. Chaque triangle source est attribué à une face reconnue ou conservée. OCCT réutilise les mêmes arêtes circulaires sur les deux faces adjacentes et contrôle la fermeture, les incidences et les auto-interférences.

Dans les modes analytiques, la segmentation interne utilise `max(plane, fit)` pour absorber une partie du bruit de quantification du STL. La source garde ses sommets et chaque triangle soudé est contrôlé dans une BRep native avant reconstruction. Ce contrôle source ne sérialise aucun STEP intermédiaire. Le STEP final réimporté doit préserver les types et nombres de surfaces, le volume analytique attendu et des dimensions cohérentes avec le budget.

Le build de développement complète la preuve des cylindres ouverts pour les triangulations décalées entre hauteurs. Après échec de la couverture par cordes extrudées, les sommets doivent respecter le seuil radial, la projection cylindrique doit garder un jacobien orienté et la frontière doit être exactement les deux arcs et deux génératrices. La portée angulaire de tous les triangles sélectionnés entre dans la borne. Les fragments de candidats chevauchants sont réunis seulement si leur union complète respecte l'ajustement et les limites ; cette récupération bornée ne constitue pas une recherche exhaustive des arcs bruités. Deux parois prouvées peuvent partager une même chaîne source de génératrice, avec attribution disjointe et incidence exactement deux. Le CAD réutilise alors les sommets et l'arête native, ajoute sa pcurve sur le second cylindre et contrôle les parcours opposés. Les arcs restent incidents à un plan et un cylindre ; les intersections à calculer sont exclues. Voir [ADR-0021](docs/adr/0021-staggered-open-cylinder-walls.md).

Pour l'audit de fidélité, OCCT tesselle le STEP réimporté. `fabstep-verify` mesure les distances aux triangles avec un arbre de boîtes englobantes, en prenant les sommets, milieux d'arêtes et barycentres de chaque triangle dans les deux sens. Le rapport donne les maxima mesurés, les nombres d'échantillons et la déflexion demandée. Le budget d'audit tient compte de cette déflexion et d'une marge numérique. La borne composée de reconnaissance et les deux maxima échantillonnés sont distincts. Les preuves analytiques bornent leurs domaines admis ; les deux maxima d’audit restent des mesures échantillonnées, et non une certification globale de Hausdorff du STEP natif. Les optimisations de parcours de l’arbre conservent les distances exactes aux triangles et les mêmes échantillons ; elles ne remplacent pas des branches par une approximation pour respecter le budget.

`partial/short.rs` ajoute un second passage contextuel après preuve des arcs historiques. Il examine uniquement les intervalles source entiers de quatre à sept points, jusqu'à 45°, entre deux parois distinctes déjà prouvées. Deux témoins intérieurs séparés et résolus, un intervalle borné des centres admissibles de largeur au plus 10 % du rayon ajusté et une compatibilité des tangentes voisines conditionnent la candidature. Les deux bouchons doivent désigner les mêmes deux parois d'appui ; les nouveaux arcs courts ne servent pas d'appuis récursifs. Le cercle conserve les extrémités source et n'est pas déplacé pour imposer la tangence. Après candidature, la preuve complète de paroi, les incidences, les bornes et les refus bloquants restent partagés avec le parcours historique. Un intervalle non candidat reste facettisé. Modèle, ABI, paramètres et parcours de guidage manuel sont inchangés. Voir [ADR-0022](docs/adr/0022-contextual-short-arcs.md).

## Concurrence et export

L’inspection I1 suit un parcours distinct : partition topologique source → correspondance exacte des composantes → adjacence source → sélection dans l’aperçu. `selection_topology` travaille hors fil UI et lit le cache natif sans l’invalider ; source, requête et époque sont vérifiées avant et après calcul. Le worker UI calcule les anneaux d’adjacence et le classement d’après les seules attributions exhaustives du résultat courant. Les gardes de sélection rejettent aussi les réponses reçues après effacement. Une nouvelle source ou unité STL efface la zone ; une modification des options conserve ses indices source mais invalide topologie et classement. Cette inspection ne modifie pas la reconstruction et n’autorise jamais l’export. Voir [ADR-0016](docs/adr/0016-source-zone-inspection.md).

L’inspection I1b part des octets STEP validés en cache. `result_faces` réimporte le document, tesselle les faces natives et expose leurs familles, plages de triangles et voisins par arête ; les identifiants désignent uniquement cette révision du STEP, sans identité source déduite de leur ordre. Le snapshot est en lecture seule et contrôle aussi l’identité du résultat après calcul. Un refus d’affichage ne supprime pas un export valide courant. La vue STEP se charge à la demande ; un worker valide le graphe et traite les sélections multiples et leur extension. En mode Plans, une API compagnon conserve la partition source exacte et suit les faces à travers la couture et les tables de transfert STEP. Le cache contient un `Arc<DocumentStep>` opaque liant les octets à leur provenance privée. Les étiquettes de faces relues ne sont utilisables qu’avec cette provenance du même résultat. Une perte de suivi rend le lien source indisponible sans invalider un solide validé. Le worker vérifie la partition complète avant de proposer le surlignage source multi-corps ; les autres modes restent sans correspondance. Voir [ADR-0017](docs/adr/0017-step-face-inspection.md) et [ADR-0018](docs/adr/0018-planar-step-source-provenance.md).

I2a ajoute `recognize_guided_cylinder` : une sélection de triangles d’analyse désigne un seul cylindre complet contenant toute la sélection. Le reconnaisseur partage la preuve des parois et des anneaux avec le parcours mixte, mais conserve les autres régions facettisées et n’active pas les cylindres ouverts. La segmentation analytique peut regrouper des facettes selon `max(plane, fit)` ; leur résidu est inclus dans la borne. Aucun contour ou fond absent n’est ajouté.

`DocumentStep` suivi conserve aussi les octets source immuables, le format et les options. Le bureau partage l’`Arc` de l’import pour éviter une seconde copie. `guided_convert` consomme atomiquement le résultat de départ, avance la révision et retire l’export avant calcul. Seuls les identifiants des faces STEP sont reçus ; la correspondance source est résolue côté natif. Le pipeline commun valide le corps guidé, conserve les autres corps en Plans, puis recombine et contrôle tout le document. Le rapport `guidance` distingue triangles sélectionnés et paroi entière reconstruite ; les paramètres et l’audit figurent dans le résultat mixte du corps. Un refus conserve l’aperçu source, pas l’export précédent. Voir [ADR-0019](docs/adr/0019-guided-complete-cylinder.md).

I2b conserve la provenance des plans et cylindres complets du résultat guidé, au moyen de jetons explicitement associés aux tableaux du modèle mixte. `recognize_guided_cylinders` prouve toutes les indications d’un corps dans une seule recherche à budget partagé. Le plafond est de 16 indications par document ; doublons et recouvrements de parois sont refusés. Chaque indication garde un identifiant natif stable et ses indices source ; les numéros de faces du résultat initial sont uniquement des traces historiques.

Le cache distingue la session native des indications acceptées du résultat exportable. Une tentative retire l’export ; un refus conserve la session sans republier de STEP. `remove_guidance` et `recompute_guidance` peuvent repartir de cette session après refus, et revalident entièrement le document. L’invalidation ordinaire, le remplacement de source et les conflits de téléversement effacent aussi la session. Le panneau Avancé avertit de cette réinitialisation lorsqu’une session guidée est active ou refusée ; modifier une tolérance ne rejoue pas les indications. Le rapport `guidance_hints` est toujours sérialisé, même vide ; le champ historique `guidance` conserve la dernière indication restante. Voir [ADR-0020](docs/adr/0020-consecutive-cylinder-guidance.md).

I2c ajoute une famille explicite cylindre/cône au guidage. Les troncs de cône exigent deux contours circulaires portés par des plans source ; la preuve de révolution est réutilisée sur la seule paroi indiquée, avec budget partagé et attribution exhaustive. Le parcours cylindrique seul conserve son contrat historique. La construction native suivie ajoute des jetons coniques associés aux faces réelles avant couture et transfert STEP ; l’application conserve la partition source dans l’ordre explicite plans/cylindres/cônes. Les indications des deux familles partagent la même session et les mêmes contrôles du document entier. Voir [ADR-0027](docs/adr/0027-guided-surface-families.md).

L’interface sérialise l’aperçu et la conversion sous une même révision et abandonne les réponses obsolètes. L’aperçu source importé reste disponible si une conversion CAO échoue ; il n’autorise jamais l’export. Chaque changement invalide immédiatement le cache natif avec une révision monotone ; les messages IPC arrivant en retard ne peuvent remplacer ni supprimer une révision plus récente. Tauri exécute le calcul sur une tâche bloquante séparée et garde les octets STEP du résultat validé. L'export reçoit l'identifiant de la conversion, pas du texte STEP arbitraire. Le résultat est revérifié après le dialogue de sauvegarde ; l'écriture utilise un fichier temporaire dans le dossier de destination.

Les variantes contrôlées du pipeline app exposent des points de contrôle nommés avant et après les phases coûteuses et entre les corps, sans changer les DTO persistants ou les API ordinaires. Le bureau vérifie brièvement source/révision/époque à ces frontières, sans retenir son mutex pendant un appel OCCT. Une erreur du callback interrompt toute la recherche de paramètres, sans être classée comme essai refusé. Les gardes de publication finale restent obligatoires. Un appel natif en cours, ou déjà engagé dans l'attente du verrou natif, n'est pas interrompu en son milieu ; l'annulation est coopérative entre étapes.

`DocumentStep` conserve dans un `OnceLock` privé la correspondance source résolue après une inspection native réussie. Le guidage réutilise cette table lorsqu'elle existe ; sans inspection préalable, il suit le parcours d'inspection habituel. Le cache ne conserve ni maillage ni objet OCCT, ne mémorise pas les erreurs natives et repart vide pour chaque nouveau résultat. L'aperçu public continue de demander sa tessellation ; aucun changement de provenance par supposition d'ordre des faces n'est introduit. Voir [INT-AUDIT-01](docs/INT_AUDIT_01.md).

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

## Recherche assistée de paramètres

Le lot UX2 ajoute une recherche dans `fabstep-app`, sans modification des reconnaisseurs ni de l'ABI CAD. Au plus sept conversions complètes sont effectuées séquentiellement avec des tolérances radiales distinctes ; toutes les autres options sont conservées. Les STEP des essais ne sont pas retenus. Le classement compare les nombres de faces des documents validés, puis les seuils à égalité. Les refus gardent leurs codes stables.

PERF1 ajoute `source_proof_cache.rs`, privé à une seule recherche. Il retient uniquement les maillages exacts et deux types de preuves réussies : validité de chaque corps, séparation du document ordonné. La comparaison porte sur les bits des coordonnées et les indices/ordres des triangles ; aucun hash approximatif, objet OCCT ni preuve fournie par le client. La rétention est bornée à 32 corps, 100 000 triangles et 300 000 sommets. Une différence impose un nouveau contrôle ; un échec n’établit jamais de preuve. La validation reste paresseuse après reconnaissance, et les bornes d’annulation entourent aussi les réutilisations. Analyse, segmentation, reconnaissance, construction, réimport, séparation du résultat et audit de fidélité restent propres à chaque candidat. Les conversions ordinaires et guidées ne partagent pas ce cache.

La commande bureau `search_parameters` prend un snapshot en lecture seule de la source courante, travaille hors fil UI et vérifie révision, source et époque entre les essais. Elle n'installe aucun STEP. La proposition UI est invalidée au changement de contexte ; son application passe par le changement d'options et la conversion ordinaires. Les indications guidées sont exclues de cette recherche. Voir [ADR-0023](docs/adr/0023-assisted-radial-search.md).

## Lisibilité des faces STEP

La préparation du résultat STEP calcule dans le worker UI une attribution déterministe de couleurs par face et les segments de bord de chaque plage de triangles. Les couleurs sont définies par des jetons CSS dans les deux thèmes ; elles ne codent ni les familles de surfaces ni un niveau de validation. Le rendu utilise les couleurs par sommet sans modifier l'ordre des triangles ou leur identité de sélection. Les contours sont issus de la tessellation d'affichage, avec suppression des arêtes intérieures à chaque face ; ils ne constituent pas un nouvel export de courbes BRep exactes. Des coutures périodiques peuvent rester visibles selon les indices de la tessellation native. La visibilité des contours suit le contrôle existant, avec test de profondeur pour masquer les arêtes derrière la pièce.

## Orchestration UI testable (UX4)

`ui/src/lib/workflow.ts` exécute l’import, l’invalidation, l’aperçu puis la conversion explicitement lancée, les mutations de guidage et l’export. `App.svelte` lui fournit une façade d’accesseurs sur ses runes actives et les fonctions d’inspection ; aucune seconde copie d’état n’est maintenue. La recherche conserve le même verrou `busy` et la file `queued`. Les timers et le pont IPC sont injectés pour tester les véritables commandes avec des promesses différées, sans DOM ni dépendance supplémentaire.

L’invalidation retire immédiatement le résultat exportable ; le dernier travail demandé reprend lorsque l’ancien appel libère le verrou. Le guidage conserve son avancée atomique native via `baseRequestId`, sans pré-invalidation qui détruirait la session. La destruction du composant invalide les lectures et réponses en vol. Les workers, l’historique des réglages et la sélection restent dans App : l’extraction est limitée à la logique nécessitant ces tests. Les tests du contrôleur ne remplacent pas une qualification du composant monté et du parcours graphique natif.

Le callback `onResultReady` est déclenché après publication d’une conversion ou mutation guidée courante. App lance alors `showStepResult`, sans attendre la tessellation pour libérer le verrou de conversion. L’aperçu conserve ses jetons de requête/source et de sélection : la vue STEP est activée seulement après préparation réussie du worker ; revenir à Source annule la demande d’affichage en vol. Un échec d’inspection ne retire pas le résultat exportable. La recherche de paramètres ne publie pas de résultat et ne déclenche pas ce callback.


## Profils DXF de faces planes (DXF1)

`fabstep-core/src/profile.rs` décrit un repère 3D et des contours 2D de lignes/arcs/cercles/ellipses, et écrit un DXF ASCII pur en mm. `fabstep-cad::planar_face_profile` extrait les arêtes natives avec le même parcours d’identités que l’inspection STEP ; il conserve les trous, vérifie la fermeture complète et refuse les autres courbes. `DocumentStep` expose cette lecture de son résultat immuable.

Le bureau lie un profil natif à la sélection monotone et à l’identité exacte du résultat. Les commandes `set_face_profile_context`, `face_profile` et `export_face_dxf` ne reçoivent jamais de géométrie arbitraire à exporter ; les gardes restent actives après le dialogue de sauvegarde et pendant l’écriture atomique. L’interface présente l’aperçu et l’export dans le contexte d’une seule face plane. Le worker prépare les courbes SVG et leurs extrema analytiques, sans modifier les courbes exportées. Voir [ADR-0024](docs/adr/0024-planar-face-dxf.md) pour limites, repère, format et refus.


## Sections du STEP (DXF2a)

Les DTO et l’exporteur purs de `fabstep-core::profile` prennent en charge plusieurs corps et composantes, avec chaque corps demandé représenté même vide. `fabstep-cad::section_profile` calcule les arêtes natives et leur graphe fermé, sans soudage ou approximation ; les niveaux critiques des surfaces support, les familles d’intersection admises et les budgets conditionnent l’acceptation. `DocumentStep` expose cette lecture et la résolution du plan d’une face indépendamment de ses contours DXF1.

Le module bureau `section.rs` conserve un contexte indépendant des profils de faces, lié à l’identité immuable du résultat et à une époque de plan/corps. L’interface invalide immédiatement par une commande légère et borne toute la chaîne à un calcul actif et une dernière demande en attente. Le repère résolu pilote un plan 3D d’affichage ; les courbes 2D sont préparées par le worker, sans modification du STEP. L’export relit uniquement le profil natif du contexte courant, avec contrôle jusqu’au remplacement atomique. Voir [ADR-0025](docs/adr/0025-step-section-dxf.md), notamment pour les restrictions et la distinction avec la future coupe du maillage source.

La manipulation du plan utilise les contrôles Three.js déjà installés. Le repère personnalisé est un DTO scalaire validé côté bureau (orthonormal, direct, fini, origine bornée à ±1e9 mm), sans objet OCCT transmis. Le plan visuel local reste disponible pendant les calculs ; le début du geste invalide le profil exportable, et son relâchement relance la file bornée. Le pivot est capturé au début du geste, près du modèle ; orbite et sélection sont suspendues pendant la manipulation.

Les ellipses partagent le DTO de profil : centre, vecteur de demi-grand axe, rapport des axes et domaine paramétrique signé. Le DXF écrit des entités ELLIPSE exactes ; le worker affiche des arcs SVG elliptiques et calcule leurs extrema analytiques. Les boîtes natives conservatrices excluent les supports sans intersection possible ; seules les coutures périodiques identifiées par topologie exacte sur une unique face peuvent remplacer un ancêtre absent dans une section. Les arêtes entre faces et les vraies tangences restent refusées.


## Sections polygonales source (DXF2b)

La préparation topologique pure est extraite du pipeline historique avant segmentation plane. `PreparedMesh` contient le maillage soudé/orienté, ses bornes, volume et diagnostics ; `prepare_mesh_bodies_for_sections` ne valide pas la géométrie spatiale. `fabstep-app::PreparedSourceSection` n’est construit qu’après validation native de chaque corps et séparation globale du document. Ses maillages restent privés et immuables, sans STEP reconstruit ni objet OCCT conservé.

`fabstep-core::mesh_section` forme les segments et cycles par identités d’arêtes/sommets, contrôle contacts et inclusions avec un budget global de cinq millions d’unités de travail et refuse les prédicats numériques indéterminés. Les sections réutilisent les DTO et le writer DXF existants, avec des lignes uniquement. Préparation et extraction disposent de points de contrôle d’annulation.

Le bureau lie une époque de préparation à la source, aux options et à la révision. Le contexte de section distingue source/STEP et vérifie l’identité exacte de la préparation jusqu’à la sauvegarde atomique. La préparation en lecture seule n’efface pas un STEP valide. L’UI réutilise le panneau et les manipulateurs, avec une file bornée de préparation, puis la file de section existante ; les métadonnées natives fournissent les corps et bornes. Voir [ADR-0026](docs/adr/0026-source-mesh-section-dxf.md).


## Fusion plane guidée (I2d)

`GuidanceFamily::Plane` passe par `guided_plane.rs`. La sélection est une union complète de faces polygonales adjacentes par arête. Un plan déterministe, pondéré par les aires, est vérifié sur les sommets source ; les contours, trous et coordonnées sont conservés. Le résidu intérieur est borné par la tolérance explicite et les contours par une garde conservatrice de 1e-7 mm, sans assouplir le contrôle natif. Un petit relief intérieur peut être simplifié : le résidu participe à la borne et reste signalé.

Les demandes Cylindre/Cône sont d’abord prouvées par le parcours existant ; seules les faces planes entièrement sélectionnées sont ensuite fusionnées. Les faces à contours courbes sont refusées dans ce lot. Les indices de provenance sont remappés après fusion. Sans demande courbe, aucune reconnaissance circulaire n’est lancée ; le modèle plan est construit via l’API native planaire suivie, tout en gardant le rapport et l’audit de fidélité du parcours guidé. La session, les gardes de révision, le retrait et le recalcul restent communs. Voir ADR-0028.


## Reconstruction automatique locale et sélection de paroi (UX5)

Le mode additif Auto passe par `automatic.rs` et une politique locale du reconnaisseur arrondi. Il retient les candidats démontrés et facettise les refus locaux avant assemblage, avec limites de travail et contrôles source/natifs inchangés. Les modèles plans non constructibles sont subdivisés avant attribution ; aucun sommet n’est déplacé. `RoundedRecognition` unifie le résultat suivi, avec jetons plans/cylindres/partiels/cônes/sphères/tores réellement transférés par OCCT.

La session peut partir d’Auto et rejouer ses indications sans perdre les régions reconnues indépendantes. Le groupe torique comprend sa portée cylindrique nécessaire ; les chaînes coniques adjacentes restent refusées en guidage. `suggest_region` est une lecture du résultat immuable, protégée par identité/révision/source/époque ; il mappe une région source vers des faces STEP entières. Le worker UI valide la sélection groupée et les double-clics partagent une file bornée à un calcul actif et une dernière attente. Les stratégies historiques et leurs défauts API restent compatibles. Décision, limites et migration des préférences : [ADR-0029](docs/adr/0029-automatic-local-reconstruction.md).


## Raccords ouverts cylindre–chanfrein

`partial/chain.rs` complète Auto par une preuve conjointe de bandes semi-circulaires cylindriques et coniques entre supports plans. Les arcs intermédiaires sont communs aux deux surfaces voisines ; chaque triangle et chaque génératrice source restent attribués. Le précontrôle porte sur tous les contours des supports, y compris les trous circulaires déjà reconstruits. Les cônes partiels s’ajoutent à `RoundedModel` sans remplacer les modèles historiques ; leur groupe de jetons suit les six groupes existants dans la construction native suivie. Les contrôles de réimport et d’audit restent inchangés. Le guidage et la sélection Auto étendent une bande à ses surfaces dépendantes. Périmètre, compatibilité et budgets : [ADR-0030](docs/adr/0030-open-cylinder-conical-chains.md).

Un échec de mutation guidée courante est présenté comme « Correction non appliquée », avec le recalcul de la session conservée directement sous l’erreur. Les échecs de conversion ordinaires gardent leur titre distinct ; aucune ancienne révision ne redevient exportable.


## Chaînes coniques de signe libre avec portées

`resolved_conical_chain` admet deux à huit cônes avec portées, sans condition de signe, uniquement avec deux anneaux source intérieurs par portion. Le garde partagé conserve les marges d’incertitude de pente ; les appels historiques déjà prouvés ne repassent pas par la preuve étendue et gardent leur budget. Primitives et Surfaces utilisent cette règle commune, Auto réutilisant Surfaces. Cette tranche avec portées ne découpe pas les cônes adjacents ; l’extension distincte à deux cônes est décrite dans ADR-0032. Aucun changement CAD/DTO : [ADR-0031](docs/adr/0031-same-sign-conical-chains.md).

## Deux cônes adjacents à rupture unique

R-060 ajoute une recherche bornée de rupture uniquement après échec de la loi affine historique. Chaque portion possède des témoins source intérieurs ; les intervalles de pente sont séparés avec marge et un second candidat entraîne un refus. L’anneau partagé est conservé comme géométrie unique, puis les preuves de paroi et la validation native existantes sont appliquées. Aucun DTO/CAD supplémentaire : [ADR-0032](docs/adr/0032-adjacent-conical-pair.md). Les chaînes plus longues sans portée restent R-059.

## Profil exact à deux raccords toriques

R-061 admet la chaîne extérieure Tore–Cylindre–Tore aux deux extrémités planes, avec deux témoins intérieurs sur la portée. Le rayon cylindrique démontré avant le premier ajustement sert à canonicaliser ensemble les deux quarts de tore. Toutes les sections repassent ensuite par les preuves vectorielles/UV et la borne composée ; les déplacements des cercles terminaux restent comptés. Le parcours à un tore conserve son calcul historique, les nouvelles variantes concaves sont refusées. Modèle et ABI inchangés : [ADR-0033](docs/adr/0033-double-toroidal-fillet.md).

R-062 qualifie le modèle mixte existant sur une ou deux arêtes arrondies disjointes et un bac ouvert à huit coins cylindriques verticaux. Les boucles source, le fond et le rebord forment une frontière fermée de matière ; aucune sémantique de corps imbriqués ni de cavité enfermée n’est ajoutée. Les tangences sont vérifiées sur le corpus, sans nouvelle classification générale des congés.


## Observation de la reconstruction — UX7

Le bureau conserve un seul instantané des checkpoints réellement reçus pendant la conversion ordinaire ou guidée. `get_conversion_progress` est en lecture seule, protégé par révision, source, époque et génération d’opération. La garde de commande efface l’observation à la sortie ; un ancien observateur ne peut ni publier ni effacer la nouvelle opération. L’indice du corps vient directement de l’analyse ; son total est inconnu dans cette version du contrat (`null`), sans déduction depuis les appels répétés. Le précontrôle d’une correction guidée est distingué de l’inspection finale.

L’interface interroge cet instantané seulement pendant la conversion, au plus une requête en vol, avec 350 ms entre réponse et interrogation suivante. Les réponses périmées et erreurs d’observation n’affectent pas le calcul. Trois étapes documentaires et la sous-opération courante sont affichées ; aucune fréquence de checkpoint n’est convertie en pourcentage de travail ou de temps. Les appels natifs demeurent indivisibles et non interruptibles.


## Gros STL — processus isolé

`fabstep-worker`, exclu du workspace comme le bureau, compile la capacité source `large-stl` séparément. Le bureau conserve les limites ordinaires et supervise un seul processus d’essai explicite. Le protocole privé et versionné borne entrées, progression, rapport et STEP ; publication seulement après sortie réussie et contrôle de l’identité courante. Annulation et invalidation arrêtent le processus, puis récupèrent sa fin avant nettoyage. Le résultat transféré ne contient pas d’objet OCCT et ne revendique pas de provenance guidée. Voir [ADR-0034](docs/adr/0034-isolated-large-stl-worker.md) pour limites, portée et qualification.


### Surveillance mémoire du helper macOS — R-073

Le superviseur lit `proc_pid_rusage(RUSAGE_INFO_V0)` chaque seconde : mémoire résidente et empreinte physique. L’une ou l’autre atteignant 4 Gio déclenche `worker.memory_limit`. La lecture invalide reste un refus `worker.monitor_failed`, avec vérification de la sortie concurrente du processus. Le nettoyage existant tue et récolte l’enfant avant de supprimer son dossier privé ; aucun résultat n’est publié sur ces erreurs. Le SDK local fixe la disposition du tampon et des assertions Rust contrôlent taille, alignement et offsets. Le seuil est observé, sans garantie de plafond entre échantillons. Les autres plateformes restent non qualifiées ; aucune limite de géométrie ou de durée n’est changée.


### Chaîne tore–cône–cylindre — R-075

`rounded/torus_cone.rs` intervient après un refus géométrique du classement historique et restaure les mesures des anneaux avant sa recherche. Il exige une décomposition unique en arc torique terminal, cône tangent et portée cylindrique, sans réinitialiser le budget de travail. Les paramètres du raccord sont déterminés conjointement depuis les anneaux source et le plan. La preuve existante de couverture UV et d’écart continu s’applique à chaque triangle ; l’ordre des anneaux du tore suit les angles croissants, même quand l’ordre axial est opposé. DTO, ABI et plafonds restent inchangés ; voir ADR-0035.

Le parcours des composantes ne réinsère plus le triangle courant depuis ses propres arêtes : après validation de la topologie fermée, seule l’autre incidence est un voisin. Les composantes restent identiques à l’ancien parcours ; les visites réellement supprimées libèrent du budget sans modifier les prédicats géométriques ni la recherche d’arcs.


### Chaînes composées — R-076/R-078

`rounded/torus_cone/terminal.rs` ajoute les grammaires toriques terminales et le dépliage source de leurs domaines ; `revolved/adjacent_chain.rs` cherche une décomposition unique de portions affines témoins et partage sa règle entre Primitives et Surfaces/Auto. Les données mesurées sont conservées avant les essais historiques et les déplacements finaux restent couverts par les preuves de paroi. Les types CAD ne changent pas. Voir ADR-0037 pour les conditions et la correction issue de revue.

L’index privé `RimIndex` conserve l’égalité complète des ensembles d’arêtes et les doublons de supports, tout en supprimant les reconstructions répétées de ces ensembles. Le préfiltre sphérique facture désormais ses points effectivement visités, avec charge préalable à chaque visite et plafond commun inchangé. Ce changement de décompte n’est pas présenté comme une réduction de durée.


### Octant sphérique local — R-081

`RoundedModel.octants` ajoute le domaine convexe borné approuvé dans ADR-0036 : centre/rayon, repère direct et trois usages orientés d’arcs source. Le champ absent se lit comme une liste vide ; les propriétaires `SphericalOctant` constituent le huitième groupe de provenance, sans renumérotation des familles précédentes. Le détecteur `rounded/octant.rs` exige dix disques source et leurs limites partagées, puis des cartes orientées et des bornes continues. La contribution de volume est intégrée par flux ; Auto préserve le groupe et refuse les indications locales incompatibles.

La nouvelle entrée CAO partage les arêtes et les sommets des cylindres avec la face sphérique. La validation native distingue le coin à dix faces du boîtier complet à vingt-six ; elle revérifie les supports, tangences, arrêts et volume après transfert STEP, y compris pour plusieurs racines. Les anciennes entrées restent utilisables pour leurs modèles historiques. L’application ajoute le groupe de jetons et l’interface reconnaît son propriétaire en utilisant le libellé de famille sphérique existant. Les preuves exécutées et les limites sont consignées dans VALIDATION, indépendamment des essais Fusion.


### Raccord à rayon linéaire — R-082

Le contrat G approuvé dans ADR-0036 ajoute `MixedModel.elliptic_arcs`, des usages orientés `MixedSegment::EllipticArc` et `RoundedModel.linear_blend`. Ce dernier est optionnel, alloué séparément pour ne pas grossir toutes les variantes de reconnaissance ; la représentation JSON reste un objet ou un champ absent. Les anciens documents se lisent sans ces champs. `LinearBlend` constitue le neuvième groupe de provenance, de famille native Cône.

`rounded/block.rs` partage avec D la recherche bornée des six supports ; leurs graphes de voisinage distincts orientent les essais sans multiplier les recherches. `rounded/linear_blend.rs` exige sept disques source, leurs frontières, une carte orientée et un écart continu par triangle. Les quatre sommets de raccord projetés sont comptés et leur déplacement entre dans la borne ; l’analyse source reste intacte. L’intégrale de volume prend le sommet du cône comme origine, où sa contribution de flux est nulle, et conserve les aires elliptiques des plans. Les indications locales sur le groupe couplé sont refusées.

La CAO partage les deux arêtes elliptiques 3D exactes et les deux génératrices. Leur représentation paramétrique sur le cône utilise une interpolation d’Hermite bornée à 128 intervalles, contrôlée par l’estimation numérique OCCT de l’écart, sous `Precision::Confusion()/8`. Cette estimation n’est pas un certificat continu ; elle ne remplace ni la preuve source ni les contrôles de solide et de réimport. Les seuils d’export ne sont pas augmentés. Le précontrôle des ellipses borne aussi leur tessellation.

La partition source exhaustive est obligatoire. Le suivi des étiquettes natives conserve la politique historique tout-ou-rien d’ADR-0018 : une correspondance indisponible n’est jamais inventée. Les corpus G vérifient une correspondance native exacte, y compris deux corps séparés, sans promettre que tout transfert STEP conserve ses étiquettes. Aucun changement de cette politique n’est introduit par G.


### Perçages sécants centrés — R-083

`RoundedModel.cross_drill` ajoute un domaine optionnel `CrossDrill` sans changer les anciens documents. Son repère, ses quatre anneaux, ses deux cycles de sommets bruts et son résidu mesuré traversent la frontière scalaire ; aucun objet OCCT ne la traverse. Les trois propriétaires `RoundedFaceRef::CrossDrill` forment le dixième groupe de provenance. Les wrappers historiques restent utilisables. Voir ADR-0038 pour le domaine et les limites.

`cross_drill.rs` prouve la partition orientée en six plans et trois parois, les cycles, les cartes cylindriques et leurs bornes continues. Avant de prendre priorité sur les reconnaisseurs historiques, une jonction source entre les deux parois cylindriques est exigée : quatre anneaux de trous borgnes ne suffisent pas. La réserve native de 4e−7 mm entre dans le budget demandé, sans relever les tolérances.

La CAO construit conjointement les courbes spatiales et paramétriques, puis certifie leurs données effectivement réimportées et leurs cycles. `cross_drill_verify.cpp` compile séparément le certificat partagé avec le laboratoire F0, sans fast-math ni contraction FMA. Les budgets d’intervalles, le volume indépendant, les sondes matière et le contrôle du solide entier s’appliquent aussi aux documents multisolides. L’exception de tessellation pour les branches certifiées n’autorise pas les splines arbitraires. La provenance native conserve la politique tout-ou-rien ADR-0018 ; les corrections locales couplées et les profils DXF non coniques restent refusés.
