# Validation — historique et qualification courante

**Les premières tables ci-dessous décrivent le jalon 0.6.** Les résultats de chaque jalon sont conservés dans leurs sections datées. Les essais ont été exécutés sur macOS Apple Silicon avec Rust 1.88, Open CASCADE 7.9.3 Homebrew, FreeCAD 1.1.1 et Node 26.10.0. Les contrôles encore ouverts sont indiqués séparément.

## Automatisation au jalon M4

| Contrôle | Résultat |
| --- | --- |
| `cargo test --workspace --offline` | 128 tests réussis (52 core, 26 CAD, 28 app, 7 CLI, 11 import, 4 verify). [Journal final](../artifacts/m4/workspace-tests-final.log) |
| `cargo clippy --workspace --all-targets --offline -- -D warnings` | Réussi |
| `cargo fmt --all -- --check` | Réussi |
| `cargo check --manifest-path crates/fabstep-desktop/Cargo.toml --offline` | Réussi |
| Formatage, Clippy strict et tests du manifeste desktop séparé | Réussis ; 1 test de révisions IPC |
| `npm --prefix ui run check` | 0 erreur, 0 avertissement |
| `npm --prefix ui run build` et `sh scripts/build-macos-dev.sh` | Réussis. Application locale avec frontend intégré. Vite signale un bundle JS minifié supérieur à 500 ko. [Build final](../artifacts/m4/desktop-build.log) |

Les tests couvrent lecture ASCII/binaire, en-tête binaire `solid`, fichiers tronqués, limite de taille/nombre, coordonnées invalides, incidences, contours et trous, régularité des sommets, orientation, composants multiples, retessellation et sommets colinéaires partagés, soudage de cellules voisines, bornes numériques, plans sans dérive, unités, rotation et translation. Les tests CLI vérifient aussi que ni un fichier existant ni une sortie après échec ne sont écrasés implicitement.

## M1 — plans (6 novembre 2026)

Validation initiale de `0.1.0-alpha.1` avec 25 tests Rust, Clippy strict, compilation du bureau et contrôles Svelte.

Le générateur indépendant `fixtures/generate.py` définit les volumes attendus. Les exports sont dans `artifacts/` et reproductibles par la CLI.

| ID | Source | Triangles | Faces STEP | Solides | Volume mm³ | Dimensions mm | FreeCAD Part |
| --- | --- | ---: | ---: | ---: | ---: | --- | --- |
| V-001 | Cube de test | 12 | 6 | 1 | 1 | 1 × 1 × 1 | Valide, fermé, surfaces Plane |
| V-002 | Prisme concave | 20 | 8 | 1 | 6 | 2 × 2 × 2 | Valide, fermé, surfaces Plane |
| V-003 | Plaque à trou rectangulaire traversant | 32 | 10 | 1 | 24 | 4 × 4 × 2 | Valide, fermé, surfaces Plane |
| V-004 | Cube intégré à l'interface, export via dialogue natif | 12 | 6 | 1 | 8 000 | 20 × 20 × 20 | Valide, fermé, surfaces Plane |

Chaque conversion est relue par le lecteur STEP d'OCCT avant succès. Les sorties V-001/002/003 ont également été relues dans un processus `DRAWEXE` séparé ; V-004 a été relue après sauvegarde depuis l'application. Les quatre sorties sont ensuite lues par le module `Part` de **FreeCAD 1.1.1**. Rapport de l'exécution locale : `artifacts/freecad-validation.json`.

Reproduction sur ce Mac :

```sh
PYTHONPATH=/Applications/FreeCAD.app/Contents/Resources/lib \
  /Applications/FreeCAD.app/Contents/Resources/bin/python \
  scripts/validate-freecad.py artifacts/rectangular_hole.step \
  --faces 10 --volume 24 --dimensions 4 4 2
```

La comparaison comporte validité BRep, auto-interférences, coque unique fermée, incidences des arêtes, volume positif, nombre de faces et dimensions. Elle ne remplace pas une future mesure bidirectionnelle complète entre source et résultat.

### Interface et bureau M1

- Aperçu local inspecté dans le navigateur en sombre et clair, à 1440 et 1100 px. Labels et champs numériques ajustés après détection d'une valeur tronquée. Traduction anglaise vérifiée à l'écran.
- Dans le navigateur, l'export reste désactivé et la nécessité du moteur natif est explicitée ; aucune conversion simulée.
- Application Tauri ouverte depuis `artifacts/FabSTEP.app`, chargement du cube, calcul natif, six faces et mesures affichées, dialogue de sauvegarde, fichier effectivement créé et relu.
- L'essai d'import graphique dans FreeCAD a été interrompu par un crash dont la pile se situe dans `QMacAccessibilityElement`. Il ne constitue pas un succès manuel d'import graphique ; les quatre validations par l'API FreeCAD ont réussi ensuite.
- Fusion, Windows/Linux, installation autonome, fichiers utilisateur réels et grands maillages : **non testés**.

## Tolérances et distribution

Les tolérances core sont distinctes du seuil de planéité natif OCCT (~10⁻⁷ mm). Sans projection de sommets, une région approximativement plane peut être refusée au stade CAD, même si elle est reconnue au stade core. C'est un refus explicite.

Le paquet local dépend des bibliothèques Homebrew installées, dont le minimum macOS de ce build est 26. Le conditionnement des bibliothèques, la signature et les notices complètes des dépendances doivent être achevés avant distribution à d'autres postes.


## M2a — surfaces cylindriques (6 novembre 2026)

Validation de `0.2.0-alpha.1` avec 44 tests workspace (27 core, 8 CAD, 6 app, 3 CLI), un test desktop, formatage, Clippy strict, contrôle Svelte et build. Vite signalait un chunk Three.js d'environ 699 ko minifié.

Générateur indépendant : `fixtures/generate_m2.py`, dimensions nominales dans `fixtures/m2_manifest.json`. Les sorties suivantes sont réimportées avec le module Part de FreeCAD 1.1.1 dans des processus séparés. Toutes contiennent un solide fermé valide ; les types de surfaces, rayons et volumes sont contrôlés.

| ID | Source | Faces | Plans | Cylindres | Rayons mm | Volume mm³ |
| --- | --- | ---: | ---: | ---: | --- | ---: |
| V-101 | Cylindre R10 H20, 64 segments | 3 | 2 | 1 | 10 | 6 283,185307 |
| V-102 | Même cylindre, subdivisions axiales | 3 | 2 | 1 | 10 | 6 283,185307 |
| V-103 | Cylindre tourné et translaté | 3 | 2 | 1 | 10 | 6 283,185307 |
| V-104 | Tube R10/r5 H20 | 4 | 2 | 2 | 5 ; 10 | 4 712,388980 |
| V-105 | Plaque 30 × 20 × 5, trou R3 | 7 | 6 | 1 | 3 | 2 858,628331 |
| V-106 | Exemple intégré, export par dialogue macOS | 3 | 2 | 1 | 10 | 6 283,185307 |

L'axe de V-103 est aussi comparé à la rotation de référence, au signe près. Rapports locaux : `artifacts/m2-freecad-validation.json` et `artifacts/m2-conversions.json`. V-106 est sauvegardé dans `artifacts/m2-cylinder-interface.step` puis relu avec FreeCAD ; dimensions 20 × 20 × 20 mm et rayon 10 mm confirmés.

Le STL roté converti en binaire f32 est testé de bout en bout dans Rust ; un export supplémentaire est également réimporté dans DRAWEXE avec trois faces et un solide valide. Le chemin Plans peut refuser ses faces fusionnées légèrement non planes ; le chemin Cylindres valide les triangles exacts avant reconstruction.

Les tests négatifs permanents couvrent cône, étranglement intermédiaire, budget trop faible, saillie radiale de 1 mm dans la tranche de tolérance d'un bouchon, cercles incomplets/aberrants, contact entre profils, repère invalide et sortie existante protégée après refus. La revue indépendante vérifie aussi un trou supérieur translaté de 0,25 mm (refus), un déplacement de 8 µm (accepté avec résidu accru), des bouchons 64/63 points ou déphasés (refus conservateur).

La reconnaissance exige une extrusion complète et des contours correspondants. Le budget `max_deviation_mm` porte sur la borne composée de résidus vérifiés et de flèche des cordes. Il ne constitue pas une garantie de distance globale bidirectionnelle. Une fermeture topologique et des mesures proches ne suffisent pas : la validation native source/résultat reste obligatoire.

UI M2a revue en sombre et clair à 1440 × 900 et 1100 × 800, en français et en anglais. Le navigateur reste un aperçu. Un test couvre les réponses et invalidations du cache natif arrivant dans le désordre.

La saisie native a été revérifiée par l'API d'accessibilité. Un budget de 0,001 mm bloque le cylindre et vider Ajustement bloque la conversion. Passer en Plans produit 66 faces et restaurer les valeurs valides redonne trois faces. Sur ce poste, l'outil de contrôle quitte l'application à l'envoi de `super+a`, y compris hors calcul. La saisie par `setValue` fonctionne. La cause de la traduction clavier reste indéterminée.

## M2b — pièces mixtes et trous borgnes (6 novembre 2026)

Sept cas sont produits par le générateur Python. Le huitième provient d'un modèle de référence créé avec FreeCAD Part, puis tessellé en STL. Il comporte deux perçages borgnes orthogonaux. Les conversions mesurées en debug ont pris de 1,46 à 4,33 s sur ce poste. [Générateur Python](../fixtures/generate_m2b.py) · [Manifeste](../fixtures/m2b_manifest.json) · [Générateur FreeCAD](../fixtures/generate_freecad_m2b.py)

Les huit exports contiennent chacun un seul solide fermé valide dans FreeCAD Part. Les types de surfaces, les rayons et les profondeurs ou hauteurs attendus sont vérifiés, ainsi que les axes tournés et orthogonaux. Les mesures complètes figurent dans [le rapport FreeCAD](../artifacts/m2b-freecad-validation.json). Les deux dernières colonnes donnent les maxima échantillonnés en millimètres pour une déflexion demandée de 0,005 mm.

| ID | Source | Triangles STL | Plans + cylindres | Volume mm³ | Source → résultat mm | Résultat → source mm |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| V-201 | Trou borgne R3, profondeur 6 | 512 | 7 + 1 | 5 830,353997 | 0,011856 | 0,012743 |
| V-202 | Même trou, paroi subdivisée | 896 | 7 + 1 | 5 830,353997 | 0,011856 | 0,012743 |
| V-203 | Même trou, pièce inclinée et translatée | 896 | 7 + 1 | 5 830,353997 | 0,012408 | 0,012850 |
| V-204 | Trou borgne R3 sur 6 mm et traversant R3 sur 10 mm | 1 728 | 7 + 2 | 5 547,610658 | 0,010096 | 0,010221 |
| V-205 | Bossage R3, hauteur 5 | 896 | 7 + 1 | 6 141,371669 | 0,011856 | 0,012743 |
| V-206 | Bossage et trou borgne | 1 728 | 8 + 2 | 5 971,725666 | 0,010096 | 0,010221 |
| V-207 | Même pièce mixte, inclinée et translatée | 1 728 | 8 + 2 | 5 971,725666 | 0,009703 | 0,010193 |
| V-208 | Pièce FreeCAD à trous orthogonaux R2 et R3 | 3 364 | 8 + 2 | 5 767,522144 | 0,002497 | 0,002494 |

Les rapports CLI indiquent aussi le nombre d'échantillons dans chaque sens. Voir [les sept cas Python](../artifacts/m2b-conversions.json) et [la pièce FreeCAD](../artifacts/m2b_orthogonal_freecad-conversion.json).

L'audit mesure des distances aux triangles depuis les sommets, milieux d'arêtes et barycentres des deux maillages. Les maxima portent sur ces échantillons et sur la tessellation du STEP réimporté. La déflexion est une consigne de tessellation, sans borne certifiée sur la surface continue. Une preuve globale de Hausdorff reste ouverte. Un dépassement du budget de calcul ou du seuil d'audit provoque un refus sans rapport partiel.

Les cas négatifs du corpus couvrent un fond conique, une paroi conique, un bossage bombé et un cercle échantillonné sur huit points. Tous sont refusés en mode Cylindres. Les tests natifs vérifient aussi le refus de deux trous qui se croisent sans arêtes d'intersection. Une régression numérique couvre la mesure à l'intérieur des triangles très fins.

La conversion binaire f32 du cas incliné V-207 reste refusée. Au réglage par défaut, la segmentation fragmente un bouchon. Avec `--plane 0.00002`, la reconnaissance aboutit, puis OCCT refuse un contour de plan conservé au-delà de sa tolérance d'environ 0,0000001 mm. La source reste intacte après le soudage déclaré et l'opération ne produit aucun STEP.

### Interface et bureau M2b

L'exemple mixte a été chargé dans l'application macOS. Le résultat natif affiché contient dix faces, dont huit plans et deux cylindres, pour un volume de 5 971,725666 mm³. L'inspecteur présente environ 0,010096 mm de la source vers le résultat et 0,010221 mm du résultat vers la source.

**V-209 — export natif.** Le nouveau bundle a été lancé depuis le Finder. L'application a enregistré l'exemple mixte via le dialogue macOS et confirmé la sauvegarde. FreeCAD Part a relu le fichier comme un solide fermé valide de dix faces, dont huit plans et deux cylindres de rayon 3 mm et de longueurs 5 et 6 mm. Volume de 5 971,725666 mm³, dimensions de 30 × 20 × 15 mm. C'est le neuvième réimport M2b, en plus des huit sorties CLI du tableau. [STEP exporté](../artifacts/m2b-mixed-interface.step) · [Rapport FreeCAD](../artifacts/m2b-interface-freecad-validation.json)

Pendant l'automatisation du dialogue, le bouton Enregistrer est resté désactivé après saisie de la valeur par l'API d'accessibilité et navigation vers le dossier. La sélection du nom complet puis sa saisie au clavier ont activé le bouton. Aucun changement du code n'a été requis pour cet essai.

Revue visuelle M2b effectuée par l'architecte dans le navigateur intégré : exemple mixte à 1440 × 900 et 1100 × 800, sombre en anglais et clair en français. Les commandes, tolérances, descriptions et panneaux restent lisibles sans chevauchement observé. L'export y demeure désactivé et le besoin du moteur natif est affiché ; les mesures de fidélité ont été vérifiées dans l'application native. Le sous-agent UI n'avait pas accès à ce navigateur ; sa tentative de lancer un navigateur externe a été interrompue, puis la vérification a été reprise par l'architecte.

Fusion, les STL utilisateur, Windows/Linux, les grands maillages et la distribution autonome restent à tester.

## M2c — arcs ouverts et premier fichier utilisateur (6 novembre 2026)

Sept références synthétiques indépendantes et le STL fourni par l’utilisateur ont été convertis automatiquement par la CLI, puis réimportés par FreeCAD Part. Les huit sorties sont des solides fermés valides. Les références contrôlent nombres et types de faces, volumes, rayons, hauteurs et balayages angulaires ; `scripts/validate-freecad.py` accepte désormais `--angles-deg`. Les temps observés en debug sont de 1,4 à 6 s sur ce poste, sans garantie sur des géométries plus grandes.

| ID | Source | Triangles | Plans + cylindres | Volume mm³ | Source → résultat mm | Résultat → source mm |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| V-301 | `m2c_open_arc.stl` | 600 | 6 + 1 | 53857.346583 | 0.010538 | 0.011030 |
| V-302 | `m2c_open_arc_subdivided.stl` | 1218 | 6 + 1 | 53857.346583 | 0.010538 | 0.011030 |
| V-303 | `m2c_open_arc_rotated.stl` | 1218 | 6 + 1 | 53857.346583 | 0.010538 | 0.011030 |
| V-304 | `m2c_open_arc_hole.stl` | 1858 | 6 + 2 | 52952.567899 | 0.010538 | 0.011030 |
| V-305 | `m2c_open_arc_mixed.stl` | 2114 | 7 + 3 | 53345.266981 | 0.010538 | 0.011030 |
| V-306 | `m2c_open_arc_mixed_rotated.stl` | 2114 | 7 + 3 | 53345.266981 | 0.010538 | 0.011030 |
| V-307 | `m2c_arc_other_angle.stl` | 600 | 6 + 1 | 51282.205487 | 0.010715 | 0.011394 |
| V-308 | `body-test.stl` | 2150 | 11 + 3 | 52480.083257 | 0.016419 | 0.017193 |

Les maxima sont échantillonnés avec une déflexion demandée de 0,005 mm, et ne constituent pas une preuve globale de distance. [Rapports des conversions](../artifacts/m2c-conversions.json) · [Contrôles FreeCAD](../artifacts/m2c-freecad-validation.json) · [Manifeste synthétique](../fixtures/m2c_manifest.json).

### Comparaison au STEP d’origine et hauteur

Le fichier utilisateur reste exclu de Git. Son ancien résultat contenait 49 faces (47 plans, 2 cylindres). Le creux ouvert occupait 36 facettes ; il devient une seule surface cylindrique partielle, portant le résultat à 14 faces (11 plans, 3 cylindres). Le STEP d’origine fourni possède 15 faces : le petit trou y est divisé en deux faces cylindriques adjacentes, fusionnées dans le résultat.

| Mesure | STEP d’origine fourni | Résultat FabSTEP 0.4 |
| --- | ---: | ---: |
| Base → plan principal | 18 mm | 18 mm |
| Rayon du creux ouvert | 17,613914953809 mm | 17,613915009037 mm |
| Balayage du creux | 2,514231927642 rad | 2,514231894625 rad |
| Volume | 52 480,083212 mm³ | 52 480,083257 mm³ |
| Dimensions totales | 60,5 × 59,5 × 28 mm | 60,5 × 59,5 × 28 mm |

L’écart de hauteur rapporté (17,13 contre 18 mm) n’a pas été reproduit dans les fichiers fournis. Une lecture directe du STL binaire trouve uniquement des triangles horizontaux à Z = 0, 18 et 28 mm. Z = 17,13256073 apparaît dans les subdivisions des parois, sans triangle horizontal. Le STEP original contient lui aussi une arête intermédiaire à Z = 17,13256124 sur la paroi du petit trou ; cette arête peut expliquer la mesure observée dans la CAO, mais le lien avec la sélection utilisateur reste une hypothèse. Aucun niveau de plan ni sommet source n’a été modifié pour imposer une cote.

Le volume du résultat diffère de l’original de 0,0000444 mm³. Tous les rayons correspondent à moins de 0,00001 mm ; l’écart angulaire du creux multiplié par son rayon reste inférieur à 0,00001 mm. Ces comparaisons au STEP original sont distinctes de l’audit échantillonné du STL. [Mesures de hauteur et empreintes](../artifacts/body-test-height-validation.json) · [Comparaison détaillée](../artifacts/body-original-comparison.json) · [STEP corrigé](../artifacts/body-test-fabstep-0.4.step).

### Revue contradictoire et limites

La revue indépendante a reproduit un contournement de la politique d’échantillonnage : un contour co-circulaire à dix points et deux intervalles de 50° pouvait devenir un cylindre partiel et deux facettes. La détection de co-circularité est désormais indépendante du seuil par intervalle ; les trois variantes contrôlées sont refusées sans fichier. Les régressions testent plusieurs rotations du contour. Un profil en D avec ou sans sommet collinéaire sur la corde produit désormais la même géométrie et le même volume, à l’arrondi près. Les budgets couvrent également la maximalité des candidats et les remplacements de chaînes ; les génératrices sont préindexées et strictement monotones.

Les parois coniques, bombées et déphasées sont refusées avec `recognition.partial_unproven`. Une extrusion M2a déjà reconnue peut être enrichie par un arc ; une régression vérifie qu’un budget insuffisant ne fait pas revenir silencieusement à la version facettisée. Les arêtes et sommets partagés, les chaînes invalides et les incidences des génératrices sont testés dans le pont natif.

La recherche reste conservatrice : une chaîne bruitée peut être refusée si un préfixe échoue à l’ajustement, même lorsque la chaîne entière respecterait la tolérance. Un essai à 512 segments d’arc dépasse le budget de recherche et renvoie `recognition.too_complex`. Les limites de planéité native, les cylindres intersectants, les congés et les surfaces libres restent hors du périmètre démontré.

### Interface et export M2c

Le nouveau bundle 0.4 a chargé `body-test.stl` via le dialogue natif. Il affiche 14 faces, 11 plans, 3 cylindres, le volume attendu et les distances 0,0164194 / 0,0171932 mm (9 675 / 6 332 échantillons). Les contours de l’arc et de ses raccords ont été inspectés. Le fichier `body-test-interface-0.4.step` a été enregistré depuis l’application puis validé dans FreeCAD : il constitue le neuvième réimport M2c. [Rapport de cet export](../artifacts/m2c-interface-freecad-validation.json).

Les textes et panneaux ont été revus à 1440 × 900 et 1100 × 800 dans le navigateur, en sombre anglais et clair français. Aucun chevauchement observé ; le navigateur explique toujours la nécessité du moteur natif et ne simule pas de conversion. La vue native du fichier utilisateur a également été vérifiée. Fusion, un corpus utilisateur plus large, Windows/Linux et la distribution autonome restent à tester.

## M3 — 3MF Core et corps séparés (6 novembre 2026)

Version `0.5.0-alpha.1` : 105 tests workspace et un test desktop passent, ainsi que formatage et Clippy strict sur les deux manifestes. Le contrôle Svelte retourne zéro erreur et zéro avertissement ; l'application locale est construite avec son frontend intégré. Le bundle JS minifié mesure 1 013,37 ko ; l'avertissement Vite de taille demeure. [Journal des tests](../artifacts/m3-workspace-tests.log).

Le corpus utilise le générateur ZIP/XML Python standard et un export indépendant FreeCAD Mesh de deux objets placés. Les neuf cas positifs sont convertis par la CLI puis relus avec FreeCAD Part 1.1.1 : chaque corps est fermé, valide et possède six plans ; volumes et boîtes mondiales sont comparés individuellement au manifeste. Le cas affine combine échelle non uniforme et cisaillement. [Manifeste](../fixtures/m3_manifest.json) · [Générateur Python](../fixtures/generate_m3.py) · [Générateur FreeCAD](../fixtures/generate_freecad_m3.py) · [Mesures FreeCAD](../artifacts/m3-freecad-validation.json).

| ID | Source 3MF | Corps | Volume total mm³ | Point vérifié |
| --- | --- | ---: | ---: | --- |
| V-401 | `m3_single_cube` | 1 | 1 000 | Core minimal |
| V-402 | `m3_default_unit` | 1 | 1 000 | Millimètres par défaut explicite |
| V-403 | `m3_instances` | 2 | 2 000 | Même ressource, deux placements |
| V-404 | `m3_nested_components` | 1 | 1 000 | Ordre des transformations imbriquées |
| V-405 | `m3_inches` | 1 | 16 387,064 | Pouces, translation comprise |
| V-406 | `m3_affine` | 1 | 3 000 | Échelle non uniforme et cisaillement |
| V-407 | `m3_mirror` | 1 | 1 000 | Déterminant négatif, orientation préservée |
| V-408 | `m3_disconnected_mesh` | 2 | 2 000 | Deux composantes dans un maillage |
| V-409 | `m3_freecad_instances` | 2 | 2 | Objets placés par FreeCAD |

Les conversions de ces petits cas prennent environ 0,06 à 0,11 s en debug sur ce poste, sans garantie de performance générale. Les huit négatifs du corpus sont refusés : contact, chevauchement, inclusion, cycle de composants, indice invalide, maillage ouvert, extension requise et partie ZIP de 70 Mio. Les tests app vérifient aussi qu'une coque STL intérieure inversée ne devient pas silencieusement un corps rempli, et que plusieurs corps analytiques conservent leurs surfaces et leurs audits. Les API STL monocomposants gardent leur refus historique des composants multiples. [Rapports CLI](../artifacts/m3-conversions.json).

### Contrôles de lecture et revue croisée

La revue indépendante a trouvé puis fait corriger trois défauts : structures Core/OPC malformées acceptées, descripteurs ZIP incohérents et travail disproportionné du parseur lors de redéclarations d'espaces de noms. Les limites XML sont désormais contrôlées avant construction du DOM, avec plafond de portée et budgets cumulés. Descripteurs ZIP32 signés et non signés sont vérifiés. Les régressions permanentes couvrent cas valides et refus.

L'architecte a contre-vérifié les fichiers de revue avec la CLI finale. Les trois structures malformées et les deux descripteurs incohérents sont refusés ; le descripteur correct passe. Le cas à 4 000 redéclarations qui prenait 4,16 s est refusé en environ 0,036 s, avant le DOM. Les références Production cachées, chemins Unicode contradictoires et attributs excessifs sont aussi refusés. Un élément sans espace de noms ajouté aux ressources est ignoré avec `threemf.metadata_ignored` : l'importeur ne prétend pas effectuer une validation XSD complète. [Rapport de contre-vérification](../artifacts/m3-import-review.json).

### Pièce utilisateur et précision d'export

**V-410** — Le STL utilisateur reconverti avec le pipeline document conserve un solide, 14 faces, 11 plans et 3 cylindres, volume 52 480,08325658634 mm³ et dimensions 60,5 × 59,5 × 28 mm. FreeCAD le confirme. [STEP](../artifacts/m3/body-test-stl.step) · [Contrôle](../artifacts/m3-body-stl-freecad-validation.json).

Une réexportation du STL par `FreeCAD Mesh.write` vers 3MF arrondit ici les coordonnées à six chiffres significatifs : les 1 075 sommets correspondent exactement au STL arrondi par cette règle. L'écart maximal entre sommets est 0,00008001154635 mm. Le fichier reste topologiquement identique, mais la reconnaissance Cylindres est refusée au réglage strict. Un ajustement circulaire indépendant mesure des résidus d'environ 0,0000455 à 0,0000634 mm sur les trois contours, au-delà du seuil de 0,00001 mm ; les coordonnées exactes donnent environ 0,000002 mm.

**V-411** — Test isolant : conserver le même paquet 3MF, les mêmes indices et le même ordre, et remplacer uniquement les coordonnées arrondies par les représentations décimales exactes des f32 source. La conversion retrouve les 14 faces et le volume 52 480,083256587255 mm³ ; FreeCAD valide le résultat et les mêmes dimensions. Aucun seuil ni sommet source n'a été ajusté dans le moteur. L'expérience établit le rôle de la quantification sur ce cas, sans identifier l'ordre exact des prédicats internes. Les dérivés du fichier utilisateur restent privés dans `artifacts/`. [Comparaison indépendante](../artifacts/body-test-quantization.json) · [STEP du témoin exact](../artifacts/m3/body-test-exact-f32.step) · [Contrôle FreeCAD](../artifacts/m3-body-exact-freecad-validation.json).

### Interface et export M3

Les quatre dispositions navigateur ont été revues : sombre/clair à 1440 × 900 et 1100 × 800, français et anglais. L'aperçu STL fonctionne sans moteur ; le 3MF affiche qu'il nécessite l'application native. Aucun résultat de conversion n'est simulé. Les textes d'accueil ont été corrigés pendant la revue pour mentionner les deux formats.

Dans le bundle natif 0.5, les deux cubes 3MF affichent 24 triangles, 16 sommets, 12 plans, deux solides, 2 000 mm³ et 40 × 15 × 10 mm. Sélectionner le second corps affiche six plans, 1 000 mm³ et 10 × 10 × 10 mm. Les instances homonymes sont numérotées. L'unité 3MF est verrouillée et son origine déclarée est affichée.

**V-412** — Export par le dialogue macOS alors que le second corps est sélectionné dans l'inspecteur : le fichier contient bien le document complet avec deux solides. FreeCAD vérifie validité, fermeture, 12 plans, volume 2 000 mm³ et dimensions 40 × 15 × 10 mm. [STEP exporté](../artifacts/m3-two-cubes-interface.step) · [Rapport](../artifacts/m3-interface-freecad-validation.json). Avec V-401 à V-411, douze sorties distinctes ont été réimportées.

Le cas `m3_touching.3mf` a été ouvert par le dialogue de fichiers : les corps sont refusés, le maillage source reste visible, les mesures STEP sont effacées et Exporter STEP est désactivé. Recharger l'exemple valide rétablit la conversion. Français/sombre et anglais/clair ont été vérifiés sur le résultat natif. Après fermeture du dialogue de fichiers, l'outil d'accessibilité a temporairement conservé une fenêtre périmée ; une nouvelle sélection de l'application a rétabli les observations. Un échantillon du processus montrait la boucle UI en attente normale, sans calcul CAO bloqué.

Production multi-modèles, union de corps en contact, assemblages XCAF, précision variable des exports, Fusion, Windows/Linux et distribution autonome restent hors de la validation de ce jalon.

## Consolidation M3 — pièces utilisateur et réglages explicites (6 novembre 2026)

Les premières mesures ont été faites avec la CLI 0.5.0-alpha.1, puis les STEP ont été contrôlés dans des processus FreeCAD Part 1.1.1 séparés. Les sources privées, leurs témoins et références STEP, les résultats d'essai et les empreintes restent dans `artifacts/`, exclu de Git.

### V-501 — export FreeCAD 3MF arrondi de la première pièce

Avec les réglages par défaut en mode Cylindres, le STL et le témoin 3MF à coordonnées f32 exactes passent (V-410 et V-411). Le 3MF arrondi est refusé avec `recognition.unsupported`. Sur ce fichier, choisir explicitement `--fit 0.0001` suffit pour valider et exporter un solide de 14 faces, dont 11 plans et trois cylindres. La planéité, le soudage et le budget d'écart gardent leurs valeurs par défaut. Le résidu radial maximal mesuré vaut 0,00007358 mm et la borne composée 0,01778451 mm pour un budget de 0,05 mm. Les maxima échantillonnés sont 0,01643823 mm de la source vers le résultat et 0,01721296 mm dans l'autre sens, pour une déflexion demandée de 0,005 mm. [Rapport de conversion](../artifacts/consolidation/precision-rounded-fit-0.0001.stdout.json)

| Mesure FreeCAD | STEP de référence de la première pièce | STEP du 3MF arrondi avec ajustement explicite |
| --- | ---: | ---: |
| Dimensions mm | 60,5 × 59,5 × 28 | 60,5 × 59,5 × 28 |
| Niveaux des faces horizontales, mm | 0, 18, 28 | 0, 18, 28 |
| Rayon de l'encoche, mm | 17,613914954 | 17,613895252 |
| Balayage de l'encoche, rad | 2,514231928 | 2,514235729 |
| Rayon du trou traversant, mm | 5,315072906 | 5,315069827 |
| Rayon du bossage, mm | 8,514693183 | 8,514697619 |
| Volume, mm³ | 52 480,083212164 | 52 480,086308697 |

Les longueurs axiales de l'encoche, du trou traversant et du bossage restent de 18, 18 et 10 mm. Le résultat représente le trou traversant sur une seule face cylindrique, contre deux dans la référence. Le plus grand écart de rayon est de 0,00001970 mm et l'écart de volume de +0,00309653 mm³. Ces comparaisons à la référence CAO sont distinctes de l'audit échantillonné du maillage. Le réglage a été choisi pour ce fichier à partir de la comparaison de précision M3. [Réimport du résultat](../artifacts/consolidation/precision-rounded-fit-0.0001-freecad.json) · [Mesures des références](../artifacts/consolidation/step-reference-comparison.json)

### V-502 et V-503 — second jeu SolidCraft

Le 3MF contient un seul modèle Core, un objet `Body1` et un item sans transformation. Son unité déclarée est le millimètre. Le paquet ne requiert aucune extension. Le STL binaire porte un en-tête SolidCraft et a été lu en millimètres. Les deux maillages comptent 10 990 triangles et 5 495 sommets distincts. Arrondir les coordonnées 3MF en f32 reproduit les triangles du STL dans le même ordre, avec un déplacement maximal de 0,00000253 mm. Les plans horizontaux maillés se situent à Z = 0, 14, 18 et 28 mm. [Inspection directe ZIP, XML et STL avec empreintes](../artifacts/consolidation/body-test-2-source-inspection.json)

Le STEP AP242 de référence contient un solide fermé valide de dimensions 60,5 × 59,5 × 28 mm et de volume 54 445,277991 mm³. Ses 20 faces comprennent 12 plans, cinq cylindres, un cône, une sphère et un tore. Le trou borgne R4 a un fond à Z = 14 mm et une profondeur de 4 mm. Le bossage R8,514693 présente une paroi cylindrique de Z = 20 à 26 mm, un raccord conique à 45° sous cette paroi et un congé torique de rayon mineur 2 mm au-dessus. La surface sphérique R10 s'étend de Z = 18 à 28 mm. Ces mesures proviennent du STEP fourni. [Réimport de la référence](../artifacts/consolidation/body-test-2-step-freecad.json) · [Détail des surfaces](../artifacts/consolidation/step-reference-comparison.json)

Aux réglages par défaut, les deux maillages passent le contrôle topologique en mode Plans, puis le noyau refuse la conversion avec `cad.invalid_model` pour dépassement de sa tolérance de planéité. Le mode Cylindres est refusé avec `recognition.unsupported`, aussi lors des essais d'ajustement à 0,0001 et 0,001 mm. Les trois types de surfaces courbes supplémentaires sont hors du périmètre de reconstruction M3.

En mode Plans, choisir explicitement `--plane 0.0000001` resserre le regroupement des facettes. Les deux exports obtenus ont été réimportés dans FreeCAD comme un solide fermé valide chacun, avec les dimensions de la référence.

| ID | Source | Faces planes du résultat | Volume maillé, mm³ | Volume STEP converti, mm³ |
| --- | --- | ---: | ---: | ---: |
| V-502 | `body-test-2.stl` | 8 211 | 54 448,914717 | 54 448,914716 |
| V-503 | `body-test-2.3mf` | 8 360 | 54 448,914591 | 54 448,914591 |

Les surfaces courbes restent facettisées. L'écart de volume de l'ordre de +3,637 mm³ par rapport à la CAO de référence est déjà présent dans les maillages. L'écart entre chaque volume maillé et sa conversion est inférieur à 0,000002 mm³. [Réimport du STL converti](../artifacts/consolidation/body-test-2-stl-planar-freecad.json) · [Réimport du 3MF converti](../artifacts/consolidation/body-test-2-3mf-planar-freecad.json)

### V-504 et V-505 — corpus synthétique de précision

Le corpus reprend le maillage synthétique V-305, avec une encoche R18, un trou traversant R4 et un bossage R5. Les variantes 3MF exacte et arrondie à six chiffres significatifs partagent les indices de leurs 1 057 sommets et 2 114 triangles. L'arrondi déplace les sommets d'au plus 0,00006194 mm. Les tests de régression vérifient la reconnaissance avec les réglages par défaut sur le témoin exact, le refus du maillage arrondi au seuil strict, puis son acceptation à 0,0001 mm de tolérance radiale. La variante avec un sommet déplacé de 0,002 mm reste refusée avec cette tolérance. Le maillage arrondi est également refusé avec un budget d'écart de 0,001 mm. Le STL ASCII arrondi vérifie le même comportement de précision pour l'autre format. [Manifeste](../fixtures/precision_manifest.json) · [Générateur](../fixtures/generate_precision.py)

FreeCAD confirme un solide fermé valide de 10 faces, dont sept planes et trois cylindriques, dans chaque STEP. Les dimensions sont 60 × 60 × 23 mm et les longueurs axiales respectives 18, 18 et 5 mm. Le volume nominal indépendant est de 53 345,266980878 mm³. [Manifeste M2c](../fixtures/m2c_manifest.json)

| ID | 3MF et ajustement radial | Rayons encoche, trou et bossage, mm | Volume réimporté, mm³ | Source → résultat / résultat → source, mm |
| --- | --- | --- | ---: | ---: |
| V-504 | Exact à 0,00001 mm | 18 · 4 · 5 | 53 345,266980876 | 0,01053830 / 0,01102975 |
| V-505 | Arrondi à 0,0001 mm | 18,000049434 · 3,999996654 · 4,999990428 | 53 345,260015110 | 0,01055128 / 0,01104196 |

Sur le témoin arrondi, l'écart du volume au nominal vaut −0,00696577 mm³. Le résidu radial maximal mesuré est de 0,00007473 mm et la borne composée de 0,01113749 mm. Les deux audits ont demandé une déflexion de 0,005 mm. [Réimport du témoin exact](../artifacts/consolidation/precision_exact-freecad.json) · [Réimport du témoin arrondi](../artifacts/consolidation/precision_rounded-freecad.json)

### Contre-vérification avec la CLI 0.5.1-alpha.1

V-501 et V-503 ont été rejoués avec la nouvelle CLI. Les rapports JSON consignent le mode et les tolérances de la requête dans `settings`, avec le diagnostic `precision.custom_tolerances`. FreeCAD confirme à nouveau un solide fermé valide dans les deux exports, avec les mêmes surfaces, dimensions et volumes. Le réimport V-501 vérifie également les trois rayons et les longueurs axiales.

```sh
cargo run --offline -p fabstep-cli -- convert artifacts/body-test-freecad.3mf --mode cylinders --fit 0.0001 --json -o /tmp/fabstep-rounded.step
cargo run --offline -p fabstep-cli -- convert artifacts/body-test-2.3mf --mode planar --plane 0.0000001 --json -o /tmp/fabstep-solidcraft-planar.step
```

[Rapport V-501](../artifacts/consolidation/v051-precision-rounded-fit-0.0001.json) · [Réimport V-501](../artifacts/consolidation/v051-precision-rounded-fit-0.0001-freecad.json) · [Rapport V-503](../artifacts/consolidation/v051-body-test-2-3mf-planar.json) · [Réimport V-503](../artifacts/consolidation/v051-body-test-2-3mf-planar-freecad.json)

### Interface et bureau 0.5.1-alpha.1

La disposition a été revue dans le navigateur à 1440 × 900 et 1100 × 800, en clair et en sombre, en français et en anglais. Les libellés débordants ont été corrigés puis vérifiés. Dans le bundle natif, l'exemple cylindre affiche trois faces, un résidu radial et la section des réglages appliqués.

L'ouverture du 3MF FreeCAD arrondi dans l'application native avec les réglages stricts a produit le refus `recognition.unsupported`. L'aperçu de 2 150 triangles et 1 075 sommets reste visible, et l'export est désactivé. L'outil d'accessibilité a expiré après le dialogue de fichiers lors d'une tentative. Le processus échantillonné était dans sa boucle normale d'attente. Après redémarrage de l'application, la navigation vers le chemin puis l'ouverture en deux actions séparées ont permis de vérifier le refus.

**V-506 — export natif du 3MF arrondi.** En français et en thème sombre, saisir 0,0001 mm dans Ajustement rétablit la conversion. L'inspecteur affiche 14 faces, 11 plans et trois cylindres, un volume de 52 480,086309 mm³ et un résidu radial de 0,0000735827 mm. Les réglages appliqués indiquent un ajustement de 0,0001 mm, un budget de 0,05 mm, et un soudage et une planéité de 0,000001 mm. Le fichier enregistré par le dialogue macOS est relu par FreeCAD comme un solide fermé valide de 60,5 × 59,5 × 28 mm, avec le même volume, les mêmes rayons et les mêmes longueurs axiales que V-501. Rétablir l'ajustement strict invalide immédiatement le résultat et désactive l'export. [STEP exporté](../artifacts/precision-interface-0.5.1.step) · [Réimport FreeCAD](../artifacts/consolidation/precision-interface-0.5.1-freecad.json)


### V-507 — reprise native de la seconde pièce et traduction réactive

Dans le build final 0.5.1, le 3MF SolidCraft a été ouvert en mode Plans avec les seuils initiaux. Le refus de planéité `cad.invalid_model` s’affiche en anglais ; le basculement vers le français traduit immédiatement le même diagnostic sans relancer le calcul. La saisie manuelle de `0.0000001` mm dans Planéité produit ensuite le résultat validé : 8 360 faces planes, zéro cylindre, un solide, volume 54 448,914591 mm³ et dimensions 60,5 × 59,5 × 28 mm. Le repli Réglages appliqués affiche `1e-7 mm`, le diagnostic de tolérances personnalisées est présent et l’export est disponible. Ce contrôle vérifie l’interface et le pipeline natif ; aucun nouvel export de cette seconde pièce n’a été effectué par le dialogue, les réimports FreeCAD correspondants restent V-503.

Le navigateur a été contrôlé à 1440 × 900 et 1100 × 800, en français et anglais, sombre et clair ; les champs gardent leurs unités visibles après correction du débordement. Le résultat natif arrondi et ses réglages ont été inspectés en français sombre, la seconde pièce en anglais puis français clair. Le serveur de développement a été arrêté et les onglets temporaires fermés. L’application finale reste ouverte sur la seconde pièce, en Plans, avec la planéité personnalisée de `1e-7` mm visible.

Lors d’un premier essai d’import, l’outil d’accessibilité a cessé de répondre après fermeture du dialogue. L’échantillonnage du processus montrait une boucle principale au repos ; seule l’instance de test lancée pour cette session a été redémarrée. Les imports, calculs et sauvegarde ont ensuite été vérifiés avec des observations séparées après fermeture des dialogues. Ce comportement de l’outil n’est pas assimilé à une preuve de blocage du moteur.


## M4a — cônes et chanfreins circulaires (6 novembre 2026)

Le mode explicite `primitives` reconnaît des parois coniques complètes, y compris une pointe démontrée dans la source, et leurs raccords avec des plans ou cylindres complets. Les chanfreins droits restent des plans. Les seuils initiaux sont inchangés. L’[ADR-0006](adr/0006-cones-et-chanfreins-explicites.md) décrit la preuve et les refus conservateurs.

Le [générateur Python indépendant](../fixtures/generate_m4.py) et son [manifeste analytique](../fixtures/m4_manifest.json) définissent les dimensions, rayons, angles et volumes attendus. La CLI convertit chaque cas avec les seuils initiaux, sauf le budget insuffisant volontaire. Chaque résultat positif est relu par OCCT dans le pipeline, puis par un processus FreeCAD Part séparé. Le script [validate-m4-freecad.py](../scripts/validate-m4-freecad.py) contrôle types de surfaces, rayons aux deux extrémités, demi-angles, longueurs axiales, axes au signe près, boîtes mondiales, volumes, fermeture et validité de chaque solide.

| ID | Cas | Triangles | Plans / cylindres / cônes STEP | Volume nominal mm³ | Résultat FreeCAD |
| --- | --- | ---: | --- | ---: | --- |
| V-601 | Cône tronqué R7 → R3, H8 | 192 | 2 / 0 / 1 | 661,828852356 | Valide et fermé |
| V-602 | Même cône tourné et translaté | 384 | 2 / 0 / 1 | 661,828852356 | Axe et boîte mondiale conformes |
| V-603 | Cône à pointe R6, H9 | 192 | 1 / 0 / 1 | 339,292006588 | Pointe conservée, valide et fermé |
| V-604 | Cylindre R5 H8, chanfrein à 45° H2 vers R3 | 192 | 2 / 1 / 1 | 730,943890735 | Valide et fermé |
| V-605 | Bossage chanfreiné sur plaque | 896 | 7 / 1 / 1 | 6 495,324441716 | Valide et fermé |
| V-606 | Fraisure à 45°, perçage R3 | 896 | 6 / 1 / 1 | 5 604,159325648 | Valide et fermé |
| V-607 | Trou borgne R3 avec fond conique source | 768 | 6 / 1 / 1 | 5 839,778774667 | Fond H2, demi-angle 56,309932474° |
| V-608 | Chanfrein droit, mode Plans | 20 | 7 / 0 / 0 | 5 955 | Valide et fermé |
| V-609 | V-604 en 3MF, translation (11, −7, 4) | 192 | 2 / 1 / 1 | 730,943890735 | Boîte (6, −12, 4) → (16, −2, 14) |
| V-610 | Deux instances 3MF de V-601 | 384 | 4 / 0 / 2 | 1 323,657704712 | Deux solides, placements conservés |

Les dix fichiers contiennent onze solides au total. Les écarts de volume satisfont une tolérance relative de 1e-8 ; les boîtes sont contrôlées à 1e-7 mm plus tolérance relative dans FreeCAD, et à 1e-6 mm dans les tests app. Chaque corps analytique fournit une attribution exhaustive des triangles et un audit échantillonné bidirectionnel inférieur au budget de 0,05 mm. Les valeurs nominales sont des références du générateur ; les mesures réelles figurent dans les rapports. [Rapport CLI final](../artifacts/m4/final/corpus-cli-summary.json) · [Réimports FreeCAD finaux](../artifacts/m4/final/corpus-freecad.json).

### Refus et contre-revue

Sept cas sont refusés sans STEP : anneau de huit points (`recognition.unsupported`), pointe décentrée, paroi intermédiaire à trois lobes, deux pentes coniques contiguës, méridienne sphérique, cônes séparés par de très courts tronçons presque cylindriques (`recognition.conic_unproven`), et deux corps tangents (`cad.bodies_not_separated`). Un huitième essai, le cône tronqué avec un budget de 0,001 mm, échoue avec `recognition.deviation_exceeded`. La CLI garde la sortie précédente intacte même avec `--force` lorsque la conversion échoue.

Le cas à pentes alternées a été découvert par la contre-revue après les premiers succès. De petites variations de rayon sous le seuil radial pouvaient être interprétées comme cylindres et séparer artificiellement plusieurs cônes. M4a impose désormais **une seule plage conique par chaîne connectée**, même lorsque des cylindres séparent les portions candidates. Cela peut refuser une pièce à plusieurs chanfreins réels sur la même chaîne ; aucune généralisation au-delà de ce périmètre n’est annoncée. Le contre-exemple est reproduit dans un test core et dans `m4_alternating_taper.stl`, puis refusé par la CLI finale.

L’audit natif a également révélé un triangle effondré à l’apex dans la tessellation OCCT. Le pont ne supprime pas arbitrairement les triangles dégénérés : il identifie la pointe conique topologique, réunit uniquement ses copies à moins de 1e-7 mm dans le maillage d’audit et exige que les triangles restants couvrent la pointe. Source, BRep et STEP restent inchangés. Les tests app vérifient les pointes extérieure et de perçage ; deux sondes supplémentaires tournées et translatées, avec pointe en haut ou en bas, ont passé l’audit.

### Compatibilité et fichiers réels

Les modes Plans et Cylindres gardent leurs contrats. Les tests app vérifient le refus du cône en Cylindres, sa représentation facettisée en Plans, et les chemins historiques cylindre simple, pièce mixte et arc ouvert sans cône en mode Primitives. La reconnaissance conique ne promeut pas les arcs cylindriques ouverts présents dans le même corps ; la couverture analytique peut donc rester partielle ou être refusée.

**V-611 et V-612** — La première pièce utilisateur a été convertie en mode Primitives depuis le STL et le 3MF FreeCAD arrondi (ajustement explicitement conservé à 0,0001 mm pour ce dernier). Les réimports FreeCAD confirment 14 faces, 11 plans, trois cylindres, aucun cône et des dimensions 60,5 × 59,5 × 28 mm. Volumes respectifs 52 480,08325658634 et 52 480,08630869694 mm³. [STL](../artifacts/m4/body-test-stl-primitives-freecad.json) · [3MF arrondi](../artifacts/m4/body-test-freecad-3mf-primitives-freecad.json).

La seconde pièce utilisateur STL et 3MF est refusée avec `recognition.conic_unproven` ; le rapport identifie le triangle 302 hors des anneaux ou d’une pointe coaxiale source. Ce refus n’attribue pas chaque facette à une sphère ou un tore : la nature de ces surfaces provient du STEP original mesuré lors de V-502/V-503. Aucune réussite analytique complète ni correction du maillage n’est annoncée. Les résultats Plans antérieurs restent documentés dans la consolidation.

### Interface, export et portée des contrôles

Le navigateur a été inspecté à 1440 × 900 et 1100 × 800, en français sombre et anglais clair. Le sélecteur de mode tronquait le libellé anglais ; sa largeur a été corrigée et vérifiée. L’exemple chanfreiné charge 192 triangles et choisit le nouveau mode. Le navigateur seul garde l’export désactivé et indique le besoin du moteur natif.

**V-613 — export natif du chanfrein.** L’application 0.6 affiche quatre faces (deux planes, une cylindrique, une conique), un volume de 730,943891 mm³ et des dimensions 10 × 10 × 10 mm. Les réglages appliqués correspondent aux seuils initiaux. Le fichier enregistré via le dialogue macOS est relu par FreeCAD : cylindre R5 H8 et cône R5 → R3 H2 à 45°, solide valide et fermé. [STEP natif](../artifacts/m4-interface-0.6.step) · [Réimport indépendant](../artifacts/m4/interface-freecad.json). Avec V-601 à V-612, treize fichiers distincts ont été réimportés pour M4a.

Réduire le budget à 0,001 mm efface les mesures, désactive l’export et affiche `recognition.deviation_exceeded`, avec borne 0,024077367 mm. L’aperçu source importé reste dans le modèle de vue ; le refus se traduit immédiatement FR/EN. Rétablir 0,05 mm rétablit le résultat.

Une première capture native montrait un aperçu vide, aussi sur le cube. Une instrumentation temporaire a établi : contexte WebGL valide, dimensions non nulles, 192 triangles chargés, zéro frame et document masqué. Lorsque la fenêtre est redevenue visible, une frame a été produite et l’aperçu chanfreiné avec ses anneaux est apparu. L’instrumentation a été retirée sans modifier le rendu. Un menu resté inaccessible à l’outil a nécessité le redémarrage de la seule instance de test ; aucune panne géométrique n’en a été déduite.

Les essais ne certifient ni Fusion, ni Windows/Linux, ni un paquet autonome sans Homebrew. Les cônes partiels, plusieurs plages coniques par chaîne, sphères, tores, congés, révolutions générales et aides manuelles restent hors de cette validation. L’audit de distance reste échantillonné, sans garantie globale de Hausdorff.

Contrôle complémentaire du dialogue : la tentative d’ouverture du témoin à pentes alternées via l’accessibilité n’a pas abouti (fichier sélectionné, bouton Open resté désactivé, puis expiration de l’observation après annulation). Un échantillonnage du processus montrait la boucle principale en attente normale ; seule l’instance de test a été redémarrée. Ce cas est validé par core/CLI, pas déclaré testé dans l’interface. Les conversions de l’exemple intégré, l’invalidation par le budget, les traductions et l’export natif V-613 ont bien été observés.

Le build final sans instrumentation a été rouvert et l’exemple chanfreiné reconverti : quatre faces, un cône, un cylindre, volume 730,943891 mm³ et export disponible. L’application reste ouverte sur cet exemple en français sombre, avec les seuils initiaux. Le serveur de développement a été arrêté et l’onglet navigateur temporaire fermé.

## M5a — hémisphères et congés toriques (6 novembre 2026)

Le mode explicite `surfaces` ajoute les hémisphères avec équateur et pôle source, et les congés toriques en quart de cercle entre cylindre et plan. Il combine ces surfaces avec les plans, cônes et cylindres complets ou partiels du même corps. La reconnaissance utilise uniquement le maillage. Les seuils initiaux sont conservés ; les limites et preuves sont décrites dans l'[ADR-0007](adr/0007-rounded-surfaces.md).

Le [générateur indépendant](../fixtures/generate_m5.py) produit notamment une demi-sphère à triangulation irrégulière. Le [manifeste](../fixtures/m5_manifest.json) définit les références analytiques. Chaque STEP positif a été réimporté par OCCT puis par FreeCAD dans un processus distinct ; le [script](../scripts/validate-m5-freecad.py) mesure surfaces, rayons, domaines, axes, volumes, boîtes et normales aux jonctions toriques.

| ID | Cas synthétique | Faces planes / cylindriques / coniques / sphériques / toriques | Volume nominal mm³ | Résultat |
| --- | --- | --- | ---: | --- |
| V-701 | Hémisphère R6 | 1 / 0 / 0 / 1 / 0 | 452,389342117 | Solide valide fermé |
| V-702 | Même hémisphère tourné et translaté | 1 / 0 / 0 / 1 / 0 | 452,389342117 | Axe, boîte et pôle conformes |
| V-703 | Cylindre R8, congé torique R6/R2 | 2 / 1 / 0 / 0 / 1 | 1 567,756663669 | Deux jonctions tangentes |
| V-704 | Même pièce tournée et translatée | 2 / 1 / 0 / 0 / 1 | 1 567,756663669 | Placement et tangences conformes |
| V-705 | Cône, cylindre, congé et encoche ouverte | 7 / 2 / 1 / 0 / 1 | 55 936,135652066 | Encoche analytique conservée |
| V-706 | Deux instances 3MF du congé | 4 / 2 / 0 / 0 / 2 | 3 135,513327338 | Deux solides séparés, placements conservés |

Ces six fichiers contiennent sept solides. Les quatre sources déformée sphérique, déformée torique, trop peu échantillonnée et non tangente sont refusées, ainsi que le 3MF mettant les corps en contact. Un budget de 0,001 mm refuse également l'hémisphère. Les tests CLI vérifient qu'une conversion refusée garde une sortie existante intacte, même avec `--force`. Les modes précédents et les refus coniques historiques sont conservés. [Rapport CLI final](../artifacts/m5/verified/corpus-cli-summary.json) · [Réimports FreeCAD finaux](../artifacts/m5/verified/corpus-freecad.json).

Deux sondes supplémentaires inversent le placement selon Z dans un 3MF : hémisphère inférieur et congé torique inférieur. Elles sont converties et réimportées dans FreeCAD, avec volumes nominaux et tangences conservées. Elles restent des sondes privées sous `artifacts/m5/mirror-probes/`.

### V-707 et V-708 — deuxième pièce utilisateur

Le STL et le 3MF SolidCraft de 10 990 triangles passent en mode Surfaces avec les réglages initiaux : soudage et planéité à 1e-6 mm, ajustement à 1e-5 mm, budget à 0,05 mm. Chacun produit un solide fermé valide de **19 faces : 12 planes, quatre cylindriques, une conique, une sphérique et une torique**. Le STEP original en compte 20 : la reconstruction regroupe une paroi cylindrique scindée dans la référence, sans perte de géométrie.

FreeCAD confirme la sphère R10, le rayon mineur du tore R2 et ses deux tangences orientées avec le cylindre et le plan. Les dimensions sont **60,5 × 59,5 × 28 mm** et les niveaux des plans horizontaux **0, 14, 18 et 28 mm**. Le plan principal est donc à 18 mm, comme dans le STEP de référence. Les fichiers de référence ne sont jamais lus par le convertisseur ; leurs empreintes sont restées identiques.

| Source | Volume STEP mm³ | Écart au volume original mm³ | Borne composée mm | Audit source → résultat / résultat → source mm |
| --- | ---: | ---: | ---: | --- |
| V-707 — STL | 54 445,278016117 | +0,000024832 | 0,036382919 | 0,016419410 / 0,017193231 |
| V-708 — 3MF | 54 445,278047030 | +0,000055746 | 0,036384702 | 0,016419054 / 0,017192882 |

Le volume original vaut 54 445,277991284 mm³. La déflexion demandée pour les audits est de 0,005 mm. Les distances sont échantillonnées : elles ne certifient pas une distance globale entre surfaces continues. Les erreurs de rayons et de centres mesurées contre la référence sont consignées séparément. [STL final](../artifacts/m5/verified/body2-surfaces-stl.step) · [Comparaison STL](../artifacts/m5/verified/body2-surfaces-stl-comparison.json) · [3MF final](../artifacts/m5/verified/body2-surfaces-3mf.step) · [Comparaison 3MF](../artifacts/m5/verified/body2-surfaces-3mf-comparison.json).

### Contre-revue et corrections

La construction sphérique a d'abord confondu l'équateur et une arête méridienne ayant le même centre et le même rayon. Le contrôle des anneaux compare désormais aussi leur axe ; la version corrigée passe les variantes tournées et les réimports indépendants. La revue des preuves a ensuite relevé que les déplacements imposés à l'équateur et aux pôles devaient être inclus dans le résidu vectoriel. Une régression interne couvre ces contraintes, sans augmentation des tolérances. Les recherches sur les contours, patchs et niveaux consomment le budget de travail explicite.

Les comptes sphériques et toriques sont conservés au réimport multi-racines. La tessellation native traite les pôles topologiques prouvés, contrôle leur couverture et borne le travail de maillage dans les deux directions de courbure avant d'appeler OCCT.

### Contrôles d'intégration

Les **142 tests workspace** passent : 57 core, 29 CAD, 33 app, huit CLI, onze import et quatre audit. Le formatage workspace, Clippy strict et le contrôle Svelte passent. Les logs sont sous `artifacts/m5/`. Les contrôles du bureau et l'export par dialogue natif sont consignés ci-dessous.

Le navigateur a été contrôlé à 1100 × 800 en anglais clair et à 1440 × 900 en français sombre. Les six boutons d'exemple, le nouveau mode, les compteurs et les réglages sont accessibles ; la colonne défile à petite hauteur. L'exemple torique affiche son maillage source, avec export désactivé puisque le navigateur ne charge pas le noyau natif.

### V-709 — deuxième pièce dans l’application native

Le build final 0.7.0-alpha.1 ouvre `body-test-2.3mf` en mode Surfaces arrondies, avec les seuils initiaux. L’aperçu source et ses frontières sont visibles ; l’inspecteur affiche 19 faces, une sphère, un tore, un congé torique prouvé, un solide, un volume de 54 445,278047 mm³ et des dimensions 60,5 × 59,5 × 28 mm. Le résidu mesuré vaut 0,00000424249 mm et la borne composée 0,0363847 mm.

Le dialogue macOS a enregistré [m5-interface-0.7.step](../artifacts/m5-interface-0.7.step). Son [réimport FreeCAD indépendant](../artifacts/m5/interface-freecad.json) et la [comparaison à la référence](../artifacts/m5/interface-comparison.json) passent : solide fermé valide, mêmes mesures que le STEP de la CLI 3MF finale, et normales orientées concordantes aux deux jonctions du tore. Les neuf exports principaux V-701 à V-709 sont donc réimportés, en plus des sondes miroirs et des deux essais directs du pont natif.

Saisir un budget de 0,001 mm invalide immédiatement le résultat, efface ses mesures et désactive l’export. `recognition.deviation_exceeded` indique une borne de 0,036384702 mm, puis se traduit immédiatement de l’anglais vers le français sans relancer le calcul. Le maillage source reste disponible après ce refus.

Lors du premier import, la sélection automatique obtenue par « Aller au dossier » n’a pas activé Open et une interaction d’accessibilité a expiré. L’échantillonnage du processus montrait la boucle principale au repos. Seule l’instance de test a été redémarrée ; un clic dans la liste à partir d’une capture a ensuite activé Open et permis l’import. Pour Save, une observation complète a retrouvé le bouton actif après des observations différentielles périmées. Ces difficultés de pilotage ne sont pas présentées comme des défauts géométriques ; l’import, le calcul et l’enregistrement ont finalement été observés.

Le test desktop de révision passe, ainsi que `cargo check`, Clippy strict et le formatage du bureau. Le build local réussit avec le même avertissement Vite de taille du module JS ; l’exemple STL embarqué porte ce module à environ 1 457,5 ko minifiés. Le bundle dépend encore du SDK OCCT local. Fusion, Windows/Linux et la distribution autonome n’ont pas été testés. Les sphères ou tores complets, autres calottes, congés non toriques et rayons variables restent hors du périmètre reconnu.

Rétablir 0,05 mm dans le build final reconvertit la pièce : 19 faces, volume 54 445,278047 mm³ et export de nouveau actif. L’application reste ouverte sur ce résultat en français sombre, avec les seuils initiaux. Le serveur Vite a été arrêté et l’inventaire confirme la fermeture de l’onglet temporaire ; la taille d’écran imposée pour le test a été réinitialisée.


## 0.7.1 — indicateur de calcul (6 novembre 2026)

Le composant `ProcessingIndicator` affiche un cube discret, une barre animée indéterminée, la phase et le temps monotone écoulé depuis la demande courante, attente comprise. Il ne donne aucun pourcentage ni estimation de fin. L’annonce accessible concerne la phase ; le chronomètre reste hors des annonces répétées. `prefers-reduced-motion` neutralise les deux animations ; cette règle a été relue, sans modifier la préférence système ni réaliser de test VoiceOver.

**V-710 — interface et révisions, IPC simulé.** Un banc temporaire monte le véritable `App.svelte` avec des promesses d’aperçu/conversion résolues à la demande. Il a permis d’observer la lecture, son passage à la conversion et la continuité du chronomètre, en français sombre à 1440 × 900 puis en anglais clair à 1100 × 800. L’indicateur reste lisible et ne recouvre pas la pièce au centre. Un changement de fichier pendant la conversion affiche l’attente ; le refus du travail obsolète est ignoré et la lecture du nouveau fichier démarre. Un refus 3MF avant aperçu retire l’indicateur, affiche l’erreur et rend l’unité indisponible. Une saisie vide pendant la lecture invalide immédiatement l’état ; la réponse d’aperçu obsolète ne le restaure pas. L’export reste désactivé pendant ces états.

Le banc utilise des réponses simulées, pas OCCT : il ne constitue pas une validation géométrique. Ses deux fichiers sont conservés sous `artifacts/progress-ui/` et ont été retirés de `ui/` avant le build final. Le navigateur normal, sans pont natif, montre l’aperçu du congé torique et le besoin du moteur de bureau, sans faux indicateur de calcul. L’onglet temporaire a été fermé, la taille imposée réinitialisée et Vite arrêté.

**V-711 — bureau et pièce réelle.** Le build 0.7.1-alpha.1 ouvre `body-test-2.3mf` en mode Surfaces arrondies, aux seuils initiaux. Le maillage de 10 990 triangles est visible pendant le calcul, avec l’indicateur « Conversion en cours », le chronomètre et l’export désactivé. La [capture native](../artifacts/progress-ui/conversion-native.jpg) montre cet état. Modifier le budget à 0,001 mm pendant ce calcul affiche « Calcul en attente » ; après la fin du travail précédent, la nouvelle demande est refusée par `recognition.deviation_exceeded`. L’indicateur disparaît, l’aperçu source demeure et l’export reste désactivé. Rétablir 0,05 mm relance la lecture puis la conversion ; à la réussite, l’indicateur disparaît et l’export redevient disponible. L’inspecteur retrouve 19 faces, 54 445,278047 mm³ et 60,5 × 59,5 × 28 mm. L’application reste ouverte sur ce résultat en français sombre, aux seuils initiaux.

**Contrôles exécutés.** Formatage workspace, 142 tests workspace, Clippy strict et `cargo check` du bureau réussis. Le test desktop de révisions passe. Svelte : zéro erreur et zéro avertissement. Le build UI et le bundle macOS réussissent ; l’avertissement Vite préexistant de module supérieur à 500 ko demeure. Logs dans `artifacts/progress-ui/`. Aucun changement du moteur, nouvel export ou réimport FreeCAD n’est nécessaire pour cette évolution d’affichage ; les résultats M5 restent consignés dans la section précédente.

## M5b1 — calottes sphériques (7 novembre 2026)

La version **0.8.0-alpha.1** étend le mode Surfaces arrondies aux petites et grandes calottes convexes ou concaves. Un anneau circulaire plan, un pôle source et une courbure intérieure résolue sont exigés. Les seuils initiaux, les contrats sérialisés et l’ABI native restent inchangés. La [décision ADR-0008](adr/0008-spherical-caps.md) décrit la couverture géodésique, le contrôle du contour et sa frange hors équateur.

### Corpus et réimports indépendants

Le [générateur Python](../fixtures/generate_m5b.py) produit des triangulations non uniformes, indépendantes du code Rust. Le [manifeste](../fixtures/m5b_manifest.json) définit les dimensions et volumes attendus. Le [script FreeCAD](../scripts/validate-m5b-freecad.py) réimporte chaque STEP et contrôle fermeture, surfaces, rayon, centre, domaine sphérique, orientation, dimensions et volume. Des références construites séparément avec `Part.makeSphere` et réimportées confirment ces mesures, y compris la boîte de la pièce tournée.

| ID | Cas | Faces planes / sphériques | Dimensions mm | Volume nominal mm³ | Résultat |
| --- | --- | --- | --- | ---: | --- |
| V-801 | Petite calotte R6, hauteur 2 | 1 / 1 | 8,94427191 × 8,94427191 × 2 | 67,020643277 | Solide fermé valide |
| V-802 | Grande calotte R6, hauteur 10 | 1 / 1 | 12 × 12 × 10 | 837,758040957 | Équateur intérieur et limites conformes |
| V-803 | Grande calotte tournée et translatée | 1 / 1 | Boîte mondiale du manifeste | 837,758040957 | Placement conservé |
| V-804 | Bloc avec creux sphérique R6, profondeur 2 | 6 / 1 | 20 × 20 × 10 | 3 932,979356723 | Surface intérieure conforme |
| V-805 | Deux petites calottes 3MF, dont un miroir | 2 / 2 | Placements du manifeste | 134,041286553 | Deux solides séparés |

Les cinq STEP contiennent six solides. [Rapport CLI final](../artifacts/m5b1/verified/corpus-cli-summary.json) · [Réimports FreeCAD](../artifacts/m5b1/verified/corpus-freecad.json) · [Comparaison aux références Part](../artifacts/m5b1/verified/part-reference-comparison.json).

| Cas | Audit source → résultat mm | Audit résultat → source mm | Borne composée mm |
| --- | ---: | ---: | ---: |
| Petite calotte | 0,013994603 | 0,014130117 | 0,018428952 |
| Grande calotte | 0,016004719 | 0,016625505 | 0,021127755 |
| Grande calotte tournée | 0,016027952 | 0,016550950 | 0,021127755 |
| Creux sphérique | 0,019780023 | 0,019505424 | 0,027571096 |

La déflexion native demandée est de 0,005 mm. Les audits portent sur des échantillons ; ces chiffres ne certifient pas une distance globale de Hausdorff. Les faibles différences numériques avec les références analytiques ne constituent pas une garantie sur la précision d’un STL réel.

Les sphéroïdes, pôles décalés et zones sphériques sans pôle sont refusés par `recognition.surface_unproven`. Le cas trop peu échantillonné est refusé par `recognition.unsupported`. Les corps en contact sont refusés par `cad.bodies_not_separated` et un budget de 0,001 mm par `recognition.deviation_exceeded`. Aucune sortie STEP n’est produite pour ces six refus. Les tests couvrent aussi le maintien d’un cône très plat comme cône, sans promotion en sphère.

Deux maillages supplémentaires produits directement par FreeCAD à 0,008 mm, de 11 006 et 29 890 triangles, passent la reconnaissance `check --mode surfaces`. Ces sondes sous `artifacts/m5b1/freecad-mesh-probes/` n’ont pas été converties en STEP : elles ne s’ajoutent pas aux exports validés.

### Non-régression de la deuxième pièce utilisateur

Les STL et 3MF ont été reconvertis avec la CLI finale 0.8, puis réimportés séparément dans FreeCAD. Ils conservent un solide fermé valide, **19 faces : 12 planes, quatre cylindriques, une conique, une sphérique et une torique**. Les dimensions restent **60,5 × 59,5 × 28 mm**, avec les plans horizontaux à 0, 14, 18 et 28 mm. Le rayon sphérique R10, la hauteur sphérique 10, le rayon mineur torique R2 et les deux tangences sont conservés. Les volumes restent exactement ceux des rapports M5a : 54 445,27801611677 mm³ depuis STL et 54 445,278047030406 mm³ depuis 3MF. Les sources sont restées intactes.

[Rapport de non-régression](../artifacts/m5b1/verified/real-regression.json) · [Comparaison STL](../artifacts/m5b1/verified/body2-surfaces-stl-comparison.json) · [Comparaison 3MF](../artifacts/m5b1/verified/body2-surfaces-3mf-comparison.json). Les paramètres et rapports numériques de reconnaissance core sont également inchangés, avec un écart nul mesuré dans [la comparaison core](../artifacts/m5b1/body2-core-regression.json).

### Contrôles d’intégration et limite de l’essai natif

Les **152 tests workspace** passent : 63 core, 30 CAD, 36 app, huit CLI, onze import et quatre audit. Le test desktop de révisions passe. Formatage workspace, Clippy strict, Svelte (zéro erreur et zéro avertissement), build UI, `cargo check` du bureau et build du bundle macOS réussissent. Les logs finaux sont sous `artifacts/m5b1/`. L’avertissement Vite préexistant de taille du module JS demeure ; le bundle local dépend toujours du SDK OCCT installé.

Le bureau final 0.8 a importé `m5b_large_cap.stl` en mode Surfaces arrondies. L’état accessible observé confirme **2 604 triangles, deux faces dont une sphérique, un solide, 837,758041 mm³, 12 × 12 × 10 mm**, une borne de 0,0211278 mm, le statut « Résultat validé » et le bouton Exporter STEP actif. Le nouveau texte français décrivant les calottes est présent.

L’essai s’arrête à cette conversion : l’enregistrement par dialogue macOS n’a pas été effectué ni confirmé. Les captures et lectures suivantes ont expiré avec `timeoutReached`, malgré reconnexion et réinitialisation du pilotage. Une tentative antérieure d’import avait aussi nécessité le redémarrage de la seule instance de test ; l’échantillonnage de ce processus montrait une boucle principale au repos. Ces observations ne démontrent pas un défaut du moteur. Aucune capture M5b1 n’a été enregistrée, ni nouvelle revue visuelle navigateur réussie. **L’export par dialogue et la revue visuelle M5b1 restent à effectuer manuellement.** Les validations natives et visuelles de 0.7/0.7.1 ci-dessus restent des essais historiques, sans être comptées comme nouvel essai de 0.8.

Fusion, Windows/Linux, la distribution autonome, les sphères et tores complets, les zones sphériques sans pôle et les congés généraux restent hors de cette validation.

## M5b2 — sphères entières (7 novembre 2026)

La version **0.9.0-alpha.1** ajoute la reconnaissance terminale d’un corps sphérique complet au mode Surfaces arrondies. Les interprétations historiques réussies et leurs refus bloquants restent prioritaires. La preuve exige notamment un maillage fermé connexe, une projection radiale orientée et une couverture 4π ; elle ne crée aucun anneau ni pôle source. Le noyau construit et réimporte une seule face sphérique. Voir [ADR-0009](adr/0009-complete-spheres.md).

### Corpus indépendant et refus

Le [générateur M5c](../fixtures/generate_m5c.py) et son [manifeste](../fixtures/m5c_manifest.json) sont indépendants du code Rust. L’icosphère irrégulière R6 contient 1 280 triangles et 642 sommets, sans paire antipodale exacte : la distance minimale à l’antipode d’un autre sommet vaut environ 0,00239210 mm. La sphère UV compte 3 968 triangles et conserve les quadrilatères coplanaires habituels de cette tessellation. Les références séparées sont construites avec FreeCAD `Part.makeSphere`, puis réimportées. Le [script de contrôle](../scripts/validate-m5c-freecad.py) compare solides, fermeture, surfaces, orientation, centres, rayons, latitudes, dimensions et volumes.

| ID | Cas | Solides / faces | Dimensions mm | Volume nominal mm³ | Résultat |
| --- | --- | --- | --- | ---: | --- |
| V-901 | Icosphère irrégulière R6 | 1 / 1 | 12 × 12 × 12 | 904,778684234 | Sphère entière valide |
| V-902 | Même maillage tourné et translaté | 1 / 1 | 12 × 12 × 12 | 904,778684234 | Centre et placement conservés |
| V-903 | Sphère UV R6 | 1 / 1 | 12 × 12 × 12 | 904,778684234 | Quadrilatères de tessellation admis |
| V-904 | Deux instances 3MF, dont un miroir | 2 / 2 | Placements du manifeste | 1 809,557368468 | Deux racines séparées |
| V-905 | Échelle uniforme 3MF ×1,5, R9 | 1 / 1 | 18 × 18 × 18 | 3 053,628059289 | Rayon et placement conformes |
| V-906 | Témoin tronqué cosphérique avec bouchon plan | 1 / 2 | 8,94427191 × 8,94427191 × 2 | 67,020643277 | Calotte et plan conservés |

Les six fichiers contiennent sept solides, dont six sphères entières. Leurs réimports FreeCAD passent. [CLI finale](../artifacts/m5b2/verified/corpus-cli-summary.json) · [Mesures FreeCAD](../artifacts/m5b2/verified/corpus-freecad.json) · [Comparaison aux références Part](../artifacts/m5b2/verified/part-reference-comparison.json). L’égalité numérique de mesures avec les références synthétiques ne certifie pas la précision d’un maillage utilisateur.

| Cas | Borne composée mm | Audit source → résultat mm | Audit résultat → source mm |
| --- | ---: | ---: | ---: |
| Icosphère | 0,027331523 | 0,026692909 | 0,027161798 |
| Icosphère tournée | 0,027331523 | 0,026864608 | 0,027262097 |
| Sphère UV | 0,014429472 | 0,013373291 | 0,014233189 |
| Sphère agrandie | 0,040996785 | 0,039715715 | 0,040633038 |

La déflexion native demandée reste de 0,005 mm ; l’audit porte sur des échantillons. Les tests core couvrent aussi 3 840 points intérieurs source et 401 points analytiques répartis sur toute la sphère, les échelles ×0,01/×1,5, une translation de 1e8 mm et la répétabilité. Ce dernier contrôle est un essai de reconnaissance core, pas une certification d’export natif à cette translation. Une translation de 1e10 mm est refusée par le garde-fou de précision.

Sept essais négatifs passent : sphéroïde, sommet déprimé de 0,04 mm et sphère trop peu échantillonnée (`recognition.surface_unproven`), coque ouverte (`mesh.open`), budget de 0,001 mm (`recognition.deviation_exceeded`) et deux contacts (`cad.bodies_not_separated`). Aucune sortie STEP n’est écrite.

Le contact source utilise une sphère UV allégée à 1 472 triangles par corps pour permettre une preuve sous le budget de travail inchangé. Dans le second témoin, les maillages restent séparés de 0,000049079877 mm suivant X, mais les sphères reconstruites sont tangentes. La conversion indépendante en Plans produit bien deux solides facettisés ; le mode Surfaces refuse leur résultat analytique. Cette [sonde facettisée](../artifacts/m5b2/contact-probes/result-touch-faceted.json) est distincte des six exports analytiques du corpus. Les tests app vérifient les deux étapes de séparation, et le test CLI conserve un STEP existant lorsque la nouvelle demande est refusée avec `--force`.

### V-907 et V-908 — exports dans l’application macOS

Le bureau final 0.9 a importé l’icosphère en mode Surfaces arrondies aux seuils initiaux. La conversion affiche une seule face sphérique, un solide, 904,778684 mm³ et 12 × 12 × 12 mm. La [capture native](../artifacts/m5b2/interface-sphere.jpg) montre le maillage source sans frontière circulaire inventée. Le dialogue macOS enregistre [le STEP de sphère](../artifacts/m5b2-interface-sphere-0.9.step), dont le [réimport FreeCAD](../artifacts/m5b2/interface-sphere-freecad.json) confirme R6, les latitudes −90° à 90° et les mêmes mesures que la CLI.

Saisir 0,001 mm invalide les mesures, désactive l’export et termine par `recognition.deviation_exceeded`. L’indicateur disparaît et l’aperçu source reste visible. Le [refus en anglais clair](../artifacts/m5b2/interface-budget-en.jpg) a été observé, puis le budget initial 0,05 mm et le thème français sombre ont été rétablis.

La grande calotte M5b1 a ensuite été importée et exportée depuis le même build 0.9. Le [STEP natif de calotte](../artifacts/m5b2-interface-cap-0.9.step) est [réimporté dans FreeCAD](../artifacts/m5b2/interface-cap-freecad.json) : deux faces, sphère R6 de hauteur 10 mm, volume 837,7580409573093 mm³, dimensions 12 × 12 × 10 mm. **Cet essai clôt la vérification d’export M5b1 restée ouverte en 0.8**, sans modifier le statut historique des tentatives précédentes.

Au début du contrôle, l’ancienne instance native répondait encore par des expirations de pilotage. Seule cette instance de test a été fermée pour charger le build 0.9. Les dialogues ont ensuite été pilotés avec remise au premier plan et nouvelle sélection de l’application après leur fermeture ; imports, conversions et deux messages « Fichier STEP exporté » ont été observés. Aucun contournement n’a été ajouté au produit. L’application reste ouverte sur la grande calotte validée, en français sombre, avec les seuils initiaux.

Les vues natives sombre/français et claire/anglais ont été examinées à 1440 × 900 (captures Retina 2880 × 1800). L’agent UI a également contrôlé les textes et l’exemple torique dans Vivaldi à environ 1512 px, puis fermé son onglet et arrêté Vite. La largeur exacte de 1100 px n’a pas été vérifiée dans ce jalon ; aucune nouvelle validation responsive à cette largeur n’est revendiquée.

### V-909 et V-910 — deuxième pièce utilisateur et intégration

Les deux sources privées STL et 3MF ont été reconverties en 0.9, puis réimportées dans FreeCAD. Chacune conserve un solide fermé valide de **19 faces, 60,5 × 59,5 × 28 mm**, la sphère R10, le rayon mineur torique R2 et les deux tangences. Les volumes sont strictement identiques aux rapports précédents : 54 445,27801611677 mm³ depuis STL et 54 445,278047030406 mm³ depuis 3MF. Les empreintes des sources et de la référence restent intactes. [Non-régression](../artifacts/m5b2/verified/real-regression.json) · [Comparaison STL](../artifacts/m5b2/verified/body2-surfaces-stl-comparison.json) · [Comparaison 3MF](../artifacts/m5b2/verified/body2-surfaces-3mf-comparison.json).

Les **165 tests workspace** passent : 68 core, 33 CAD, 40 app, neuf CLI, onze import et quatre audit. Le test desktop de révisions passe. Formatage et Clippy strict workspace, Svelte (zéro erreur et zéro avertissement), build UI, check desktop et build du bundle macOS réussissent. Logs sous `artifacts/m5b2/`. Les dix sorties V-901 à V-910 ont été réimportées indépendamment, hors références et sonde facettisée.

Les limites conservées comprennent le SDK OCCT Homebrew local, l’avertissement Vite préexistant de taille du module, Fusion et Windows/Linux non testés, les zones sphériques sans pôle, les tores complets et les congés généraux. Les petites facettes ou quadrilatères cosphériques restent une ambiguïté d’intention explicitée dans ADR-0009 ; aucun historique de conception n’est annoncé.


## Stabilisation 0.10 — préparation de la v1 (7 novembre 2026)

Le contrôle des triangles source est maintenant exécuté avant toute construction native, y compris en Plans. Le témoin synthétique de 22 triangles dont le relief traverse la face inférieure était auparavant transformé en pavé valide par le regroupement des plans ; il est refusé pour auto-intersection. Trois tests app, un CLI STL/3MF avec conservation de la sortie et un test direct du budget d’audit complètent les régressions. Le témoin voisin valide reste accepté. La source utilisateur de 10 990 triangles reste valide en Plans ; le contrôle natif source seul prend environ 7,7 s sur ce poste. [Générateur](../fixtures/generate_v1_source_validation.py)

`sh scripts/check.sh` a réussi : **170 tests workspace, un test desktop et quatre tests UI**, formatages, Clippy strict, vérifications desktop et Svelte, build frontend. La définition CI existe mais n’a pas été exécutée à distance. Le verrou npm initial référençait FabWeave ; il a été remplacé par un verrou de registre autonome, avec versions directes exactes, et vérifié par une installation `npm ci` neuve. [Journal complet](../artifacts/v1/check.log)

L’interface groupe les éditions numériques au commit, ne persiste aucune valeur invalide, conserve l’aperçu importé pour le même fichier et la même unité, et expose les erreurs de corps de manière structurée. L’aide est accessible au clavier ; les boutons de thème/langue ont un état de sélection. Les tests UI vérifient les snapshots, les valeurs invalides, les noms utilisateur et les erreurs contextualisées. Revue navigateur FR sombre/EN clair à environ 1512 px, fermeture Échap et valeur invalide non persistée observées ; 1100 px non revérifié.

Le bundle autonome 0.10 a affiché les conversions natives du cube (six faces, 8 000 mm³) et des deux cubes 3MF (douze faces, deux solides, 2 000 mm³). Une planéité nulle désactive l’export et conserve les 24 triangles de l’aperçu ; restaurer 1e−6 rétablit la conversion. L’aide affiche bien 0.10.0-alpha.1. La capture native montre toutefois un viewport vide et les interactions de fenêtre ont ensuite échoué dans l’outil (`noWindowsAvailable`/`cgWindowNotFound`) : **le rendu visuel natif et l’export par dialogue de ce paquet restent à confirmer**. Le titre HTML conservait par erreur 0.9 ; il a été remplacé par le nom stable FabSTEP dans les sources suivantes.

La qualification du ZIP, la fermeture des 27 bibliothèques et les conversions STL/3MF avec Homebrew bloqué sont détaillées dans [RELEASE](RELEASE.md). Ces essais CLI ne remplacent pas le parcours d’export de l’interface.

## Candidate mécanique — 7 novembre 2026

La candidate `1.0.0-rc.1` reprend les contrats historiques et ajoute trois reconnaissances délimitées. Les campagnes géométriques ci-dessous ont été effectuées sur les builds de développement précédant le changement de numéro ; les rapports gardent leur numéro et leur empreinte d’origine. Elles ne sont pas attribuées rétroactivement à un binaire final différent.

### V-1101 à V-1104 — corpus mécanique indépendant

Les quatre références proviennent de FreeCAD Part, indépendamment du moteur Rust. Le [manifeste](../fixtures/complex_v1/manifest.json) fixe les rayons, volumes, dimensions, domaines et points de matière/vide. Les sorties 01–03 sont dans la [campagne d’audit](../artifacts/complex-v1/audit-optimization/cli-replay/measurements.json), la sortie 04 dans le [rapport dédié](../artifacts/m6a/final/cas04-freecad.json). L’ancien rapport commun garde son échec 04 historique ; le rapport dédié constate sa résolution.

| ID | Pièce | Résultat FreeCAD |
| --- | --- | --- |
| V-1101 | Bloc de distribution 48 × 36 × 18 mm, trois axes | 8 plans, 3 cylindres, traversant et deux fonds borgnes conservés |
| V-1102 | Bague à deux chanfreins opposés, 30 × 30 × 17 mm | 2 plans, 2 cylindres, 2 cônes à 45°, volume 9 598,612754269 mm³ ; mode Surfaces |
| V-1103 | Carter mixte de 41 290 triangles, 50 × 40 × 24 mm | 8 plans, 3 cylindres, 1 cône, 1 sphère, 1 tore ; 32 855,662328855 mm³ |
| V-1104 | Boîtier 28 × 20 × 14 mm, congés R2 | 6 plans, 12 quarts de cylindre, 8 octants sphériques ; 7 637,828852356 mm³ |

Les quatre STEP contiennent chacun un solide fermé valide et les surfaces attendues. Les points matière/vide et les raccords du manifeste passent. Les 48 raccords du boîtier sont tangents : écart angulaire maximal mesuré 0,000000854° aux échantillons. L’alésage elliptique reste refusé en mode analytique ; son export Plans conserve 296 faces. Le bloc ouvert reste refusé par `mesh.open` dans les deux modes. Les négatifs ne produisent aucun STEP.

### V-1105 — coût de l’audit, sans modification du résultat

Le BVH est séparé suivant l’étendue des centres des triangles. Les boîtes individuelles évitent les distances inutiles et un cache exact réutilise les milieux d’arêtes identiques. Tous les échantillons logiques restent comptés ; chaque boîte examinée consomme le budget. Les régressions comparent aussi aux distances par recherche exhaustive, notamment sur des triangles très fins et des coordonnées translatées.

Sur V-1103, le travail passe de 195 080 991 à **43 587 709 opérations**, sous le budget produit inchangé de 50 millions. Les deux distances restent identiques bit à bit : **0,008463496943941362 / 0,008868497224693708 mm**, pour **185 805 / 50 625** échantillons. La mesure de l’ancien parcours utilise une copie instrumentée au budget temporairement relevé, jamais la configuration du produit. [Comparaison détaillée](../artifacts/complex-v1/audit-optimization/work-comparison.json)

### V-1106 — tores entiers

Le [corpus indépendant M5d](../fixtures/m5d_manifest.json) couvre le tore R6/r2, une rotation/translation quantifiée f32 et deux instances 3MF dont un miroir. Les **trois STEP et quatre solides** ont été réimportés dans FreeCAD, avec une face torique par solide, axes/rayons/dimensions et volume contrôlés. [CLI](../artifacts/m5d/refined/corpus-cli-summary.json) · [FreeCAD](../artifacts/m5d/refined/freecad.json)

Les volumes STL valent 473,741011612034 et 473,741013884412 mm³, contre 48π² ≈ 473,741011252289 mm³ nominalement. Le raffinement géométrique après l’ajustement quartique corrige un biais de paramètres f32 sans augmenter les seuils. La borne composée inclut les résidus vectoriels, le soudage et la courbure dans les deux directions. Les témoins déformés, ouverts, insuffisamment échantillonnés, les contacts source/résultat et le budget insuffisant sont refusés. Les preuves core couvrent aussi une double couverture et un repli local dont les sommets satisfont pourtant l’ajustement.

### V-1107 — boîtiers convexes arrondis

Le [corpus M6a](../fixtures/m6a_manifest.json) comprend la boîte nominale binaire, une variante tournée f32 et deux instances 3MF séparées, dont un miroir. Ces trois exports et V-1104 représentent **quatre STEP, cinq solides**, tous réimportés avec succès. [Conversions](../artifacts/m6a/final/corpus-cli-summary.json) · [Réimports](../artifacts/m6a/final/freecad.json)

Le vérificateur contrôle les six appuis plans uniques, douze cylindres partiels, huit octants et 48 raccords, avec rayons, domaines, positions et points matière/vide. La variante tournée a un volume de 7 637,829026775005 mm³ ; le 3MF totalise 15 275,657704712452 mm³. Pour la variante f32 seulement, l’oracle indépendant tient compte de la quantification des coordonnées : l’écart maximal de rayon est de 0,0000006355 mm, soit **0,6355 nm**, et l’ULP de coordonnées de 0,0000038147 mm. Le seuil produit n’est pas relâché ; le volume CAD reste contrôlé contre les paramètres reconstruits. Les attentes nominales f64 conservent leur seuil strict.

Le contrat est celui d’une boîte convexe complète à rayon uniforme. Les boîtiers creux ou percés, congés partiels, rayons variables et raccords arbitraires ne sont pas annoncés. La revue native a vérifié le refus de rayons sphériques falsifiés et d’un octant isolé, et la combinaison correcte d’une boîte avec une sphère entière séparée. La permission des huit octants ne s’étend pas aux autres corps.

### V-1108 — régressions privées

Les six conversions positives ont été réimportées dans FreeCAD ; les six sources et références privées conservent leurs empreintes. Le 3MF de la première pièce reste refusé à la tolérance initiale et accepté avec `fit=0.0001` choisi explicitement. La première pièce garde 14 faces et ses niveaux à 0, 18 et 28 mm. La deuxième garde exactement ses rapports géométriques et audits de 0.9 : 19 faces, dimensions 60,5 × 59,5 × 28 mm, sphère R10, tore de rayon mineur 2 et tangences conformes. Ses sorties Plans à `plane=1e-7` conservent 8 211 / 8 360 faces. [Conversions et empreintes](../artifacts/complex-v1/private-regression/conversion-summary.json) · [FreeCAD](../artifacts/complex-v1/private-regression/freecad-summary.json) · [Comparaison historique](../artifacts/complex-v1/private-regression/m5b2-regression.json)

### Interface et intégration de la candidate

Le transport source utilise désormais un corps IPC binaire et une identité mise en cache. Les options seules ne retransmettent pas le fichier. Révision, identité source et époque protègent l’aperçu, le résultat et l’export ; les noms 3MF fournis sont conservés et seuls les noms générés se traduisent. Les cinq tests bureau et sept tests UI passent ; le contrôle dans WKWebView réel reste à consigner. La limite de 64 Mio est appliquée après réception Tauri et avant copie dans le cache, pas avant toute allocation de transport. Voir [ADR-0012](adr/0012-transport-source-binaire.md).

**V-1109 — contrôles globaux RC1.** `scripts/check.sh` a terminé avec succès : **204 tests workspace, cinq tests bureau et sept tests UI** ; formatage et Clippy strict workspace/bureau réussis, Svelte sans erreur ni avertissement et build UI réussi. Le seul avertissement du build concerne le module JavaScript minifié supérieur à 500 ko (environ 1 469,56 ko), incluant Three.js et les exemples. [Journal complet](../artifacts/v1/check-rc1.log)

**V-1110 — paquet autonome RC1.** Le [rapport strict indépendant](../artifacts/release/1.0.0-rc.1/verification-stricte.json) confirme 29 bibliothèques embarquées, les signatures ad hoc, l’empreinte du ZIP, l’architecture arm64 et le minimum macOS 26. Quatre conversions / cinq solides passent après extraction avec lectures Homebrew bloquées : sphère STL, deux sphères 3MF, tore entier et boîtier tourné. Les types de surfaces, volumes et dimensions sont vérifiés. Le binaire de contrôle est relocalisé avec les bibliothèques du bundle ; ce test ne remplace pas le parcours de la WebView. [Procédure](RELEASE.md)

**V-1111 — parcours natif RC1, état partiel.** Le cube STL passe par le nouveau transport brut, affiche son aperçu 3D et produit six plans, 8 000 mm³ et 20 × 20 × 20 mm. Le document d’exemple 3MF produit deux solides, douze plans, 2 000 mm³ et 40 × 15 × 10 mm. Le dialogue de sauvegarde a enregistré `artifacts/v1/rc1-two-cubes.step` et l’application a affiché « Fichier STEP exporté ». Son [réimport FreeCAD indépendant](../artifacts/v1/rc1-native-freecad.json) passe : deux solides fermés valides, six plans et 1 000 mm³ chacun, placements [0,0,0]–[10,10,10] et [30,5,0]–[40,15,10].

Après sélection du boîtier tourné dans le dialogue d’ouverture, l’outil ne peut plus relire la fenêtre (`timeoutReached`), y compris après nouvelle liaison et réactivation par Finder. Un redémarrage de la seule instance de test a permis le parcours 3MF et sa sauvegarde ; une nouvelle ouverture STL reproduit la perte d’accès. Les échantillons du processus et du WebContent associé ne montrent ni calcul actif ni fil principal occupé. Ils ne prouvent pas la réussite de l’import et n’établissent pas que l’outil soit seul responsable. Une confirmation manuelle de l’aperçu et de l’export du boîtier a donc été demandée. Aucun succès natif du boîtier n’est revendiqué à ce stade.

## Clôture 1.0.0 — 7 novembre 2026

**V-1112 — confirmation manuelle du parcours du boîtier.** L’utilisateur confirme « Aperçu et export réussis, 26 faces » sur la candidate 1.0.0-rc.1, pour le parcours demandé avec `m6a_rounded_box_rotated.stl` en Surfaces arrondies. Cela clôt la réserve d’usage liée à la perte d’accès de l’outil. Ce résultat est déclaré par l’utilisateur, pas observé par l’automatisation. L’utilisateur a précisé le chemin réel : `artifacts/v1/m6a_rounded_box_rotated.step`. Cet export exact a été réimporté indépendamment dans FreeCAD 1.1.1, sans reconversion ni remplacement. Le [rapport](../artifacts/v1/rc1-rounded-box-freecad.json) confirme un solide fermé valide de 26 faces : 6 plans, 12 cylindres partiels et 8 octants sphériques. Les 48 raccords et les 40 sondes matière/vide passent ; l’écart angulaire maximal échantillonné vaut 0,0000012074°. Le volume de 7 637,829026775005 mm³ est identique à celui de la sortie CLI du même STL tourné. L’empreinte SHA-256 `5c77fe619ab0219b7ab31bc0c8226a8b638c528a3c5e08f3b350968aa5bbf67f` du fichier utilisateur reste inchangée. Ce contrôle clôt le parcours natif du boîtier, distinct des réimports CLI antérieurs.

La version finale ne modifie aucun algorithme, transport ou composant d’interface par rapport au commit RC1 `030fd56`. Seuls les numéros de version des sept manifestes/verrous et les documents évoluent. La validation du paquet final vérifie le binaire reconstruit ; elle ne transforme pas les essais de RC1 en nouveaux essais visuels de 1.0.0.

**V-1113 — paquet final 1.0.0.** Construction et [vérification indépendante stricte](../artifacts/release/1.0.0/verification-stricte.json) réussies. Le ZIP de 19 196 918 octets embarque 29 bibliothèques ; toutes les dépendances Mach-O sont locales ou système, les signatures ad hoc sont valides et chaque image respecte macOS 26 arm64. Après extraction dans un chemin contenant des espaces, les quatre conversions (cinq solides) passent avec lectures Homebrew bloquées et chargements de bibliothèques confirmés : sphère STL, deux sphères 3MF, tore entier et boîtier tourné. Les volumes, boîtes et comptes de surfaces sont vérifiés. Le numéro 1.0.0 est cohérent dans les manifestes, verrous et plist. La revue croisée confirme l’absence de changement fonctionnel depuis RC1. Les essais natifs sur la candidate ne sont pas présentés comme de nouvelles observations visuelles de la finale.

**V-1114 — contrôles finaux et sources.** `scripts/check.sh` a terminé avec succès sur 1.0.0 : **204 tests workspace, cinq tests bureau et sept tests UI**, formatage et Clippy strict workspace/bureau, Svelte sans erreur ni avertissement et build UI. L’avertissement de taille du module JavaScript persiste ; aucune défaillance n’est relevée. [Journal final](../artifacts/v1/check-1.0.0.log). La livraison est enregistrée dans un commit et un tag locaux `v1.0.0` ; son archive des sources exclut les fichiers privés et conserve les verrous. Le manifeste de l’archive est comparé fichier par fichier au commit. Pas d’exécution CI distante, de publication, de notarisation ni d’essai sur un autre Mac revendiqués.


## Reprise — 9 novembre 2026

**V-1115 — validation utilisateur hors poste et Fusion.** L’utilisateur confirme l’installation sur un autre Mac et les STEP dans Fusion. Cette déclaration clôt les essais d’usage correspondants de la roadmap v1. Le matériel, les versions macOS/Fusion et les fichiers testés n’ont pas été précisés ; aucune mesure supplémentaire de volume, dimensions ou fidélité n’est attribuée à cet essai.

**V-1116 — CI distante.** Le workflow « Check macOS » du 7 novembre 2026 est terminé avec succès : https://github.com/harry-finch/FabSTEP/actions/runs/42419762317 (état consulté le 9 novembre). Cette exécution complète les contrôles locaux historiques ; elle ne qualifie pas Windows/Linux ni la notarisation.


## Première tranche post-v1 — cône et encoche ouverte

**V-1117 — contrat et corpus.** L’[ADR-0013](adr/0013-cone-open-cylinder-combinations.md) étend uniquement le mode Surfaces à exactement un cône complet accompagné d’un cylindre ouvert effectivement prouvé. Le [générateur](../fixtures/generate_post_v1_combinations.py) et le [manifeste](../fixtures/post_v1_combinations/manifest.json) sont indépendants du reconnaisseur. Le corpus contient le nominal, la rotation/translation, le miroir, le témoin sans arc, un cône déformé, une paroi bombée, un arc sous-échantillonné, des features qui se coupent et un arc dense dépassant le budget.

Les quatre tests core et les trois tests app dédiés ont réussi. Ils vérifient attribution complète, sommets source, orientations, déterminisme, paramètres, volume, dimensions, audit et codes stables. Les cônes historiques sans arc conservent leur repli, y compris des pointes. Un arc sous-échantillonné sans candidat reconnu reste facettisé dans le parcours historique ; ce cas n’est pas présenté comme refusé globalement ou reconstruit en cylindre. Une paroi candidate bombée est refusée par `recognition.partial_unproven`, le budget dense par `recognition.too_complex` et le budget d’écart trop strict par `recognition.deviation_exceeded`. La recherche d’arcs supplémentaire peut épuiser le budget d’une source complexe avant le repli ; aucun coût historique identique n’est promis.

**V-1118 — trois exports CLI et réimports OCCT.** Les trois positifs sont dans `artifacts/post-v1-combinations/`, avec STEP et rapports JSON. Chacun contient un solide valide de 10 faces : 7 plans, 2 cylindres et 1 cône. Volume nominal : **9 875,132801517426 mm³**, écart maximal observé d’environ **1,44 × 10⁻⁹ mm³**. Les maxima bidirectionnels échantillonnés sont au plus **0,009065 mm**, pour un budget de 0,05 mm et une déflexion de 0,005 mm ; 4 626 échantillons source et 8 096 résultat. Les tests app vérifient aussi les bornes nominales transformées. Il s’agit du pipeline natif OCCT via CLI/app, pas d’un essai de l’interface bureau.

**V-1119 — réimport FreeCAD indépendant non exécuté.** Le [script dédié](../scripts/validate-post-v1-combinations-freecad.py) construit une référence analytique indépendante et contrôle les STEP existants (types, rayons, domaines, aires, volume, dimensions, sondes matière/vide). Son lancement avec le Python de FreeCAD échoue avant les contrôles : `Incompatible processor. This Qt build requires the following features: neon` (code 134). Aucun succès FreeCAD n’est revendiqué pour ces trois exports. Les fichiers et le script restent disponibles pour relance ; journal local `/tmp/fabstep-post-v1-freecad.log`.

La revue indépendante du code, du corpus et de l’oracle n’a relevé aucun défaut bloquant ; elle n’a pas exécuté les tests. Le parcours import/export de l’interface, le cas 3MF dédié avec placements et la qualification d’un nouveau paquet restent ouverts. Les confirmations Mac/Fusion de V-1115 concernent la v1 existante, pas cette extension.

**V-1120 — validation automatisée globale.** `sh scripts/check.sh` a terminé avec code 0 le 9 novembre 2026 sur le correctif final : **211 tests workspace, cinq tests bureau et sept tests UI** réussis ; formatage workspace/bureau, Clippy strict workspace/bureau, contrôle Svelte, build UI et contrôle Cargo bureau réussis. L’avertissement Vite sur la taille du module persiste. [Journal complet local](../artifacts/post-v1-combinations/check.log). `git diff --check` est également sans diagnostic. Ces contrôles ne remplacent pas le réimport FreeCAD bloqué ni les essais d’interface mentionnés ci-dessus.


**V-1121 — lisibilité des choix de reconstruction.** Le sélecteur présente les intentions « Conserver les facettes » et « Reconnaître les formes courbes », puis un groupe de reconnaissances ciblées. L’aide est liée au contrôle par `aria-describedby` ; les limites détaillées sont repliables. Valeurs moteur, préférences, valeur initiale, exemples et pipeline restent inchangés. Traductions françaises et anglaises complètes, aucun style ajouté. `npm --prefix ui run check` réussit sans erreur/avertissement ; `npm --prefix ui run build` réussit (avertissement de taille Vite existant) ; sept tests UI réussissent. Le contrôle visuel à 1100/1440 px, en clair/sombre et FR/EN, n’est pas exécuté : le serveur Vite est refusé avec `EPERM` à l’écoute sur `127.0.0.1:5181`. Aucun rendu ou essai clavier réel n’est revendiqué.


**V-1122 — zones sphériques sans pôle.** L’[ADR-0014](adr/0014-spherical-zones.md) et `rounded/zone.rs` ajoutent une preuve annulaire entre deux anneaux source positifs. Le [générateur analytique](../fixtures/generate_post_v1_zones.py) est indépendant du reconnaisseur ; le [manifeste](../fixtures/post_v1_zones/manifest.json) conserve paramètres, mesures nominales et empreintes. Huit positifs STL couvrent l’équateur, les hémisphères nord/sud, une rotation, une triangulation irrégulière/dense, le binaire f32 et une zone concave traversante dans un bloc. Un 3MF contient deux instances dont une miroir ; le cas de contact reste refusé. La zone concave est une frontière matérielle à une coque, pas une cavité totalement enfermée.

Les cinq tests core dédiés, les deux tests unitaires de projection/conditionnement/budget et les trois tests app ciblés ont réussi lors du développement. Les tests vérifient notamment les deux rabattements de latitude avec leur déplacement vectoriel complet, une borne échantillonnée indépendamment dans les deux sens, les paramètres, la provenance et les refus exacts. `m5b_spherical_zone.stl` passe désormais comme zone prouvée, avec deux plans et une sphère : son ancienne attente de refus est remplacée explicitement par des tests positifs core/app. Les autres refus historiques des calottes sont conservés.

Les négatifs fixent les codes observés : sphéroïde et repli `recognition.surface_unproven`, sous-échantillonnage du corpus `recognition.deviation_exceeded`, triangle manquant `mesh.open`, contact multi-corps `cad.bodies_not_separated`. Une courbure non résolue n’est pas promue en sphère ; elle peut garder une interprétation historique admissible. Il n’y a pas encore de fixture dédiée à une double couverture ni de campagne de bruit de part et d’autre des seuils.

Une revue indépendante du raisonnement, du code, des tests et de l’ADR n’a relevé aucun défaut bloquant ; cette revue n’a pas exécuté les tests. Les résultats globaux finaux sont consignés séparément ci-dessous. Le réimport indépendant FreeCAD et le parcours graphique de ces nouvelles zones restent non exécutés ; le blocage FreeCAD documenté en V-1119 n’a pas fait l’objet de nouvelles tentatives. Le paquet 1.0.0 existant n’est pas reconstruit par ce chantier.

**V-1123 — neuf exports CLI de zones, dix solides.** Les huit STL positifs et le 3MF à deux instances ont été convertis avec `target/debug/fabstep --mode surfaces --json`. Les STEP, rapports JSON et [mesures extraites](../artifacts/post-v1-zones/measures.json) sont conservés sous `artifacts/post-v1-zones/`. Chaque zone convexe donne deux plans et une sphère ; la variante concave donne six plans et une sphère. La zone R10 entre z=-4 et z=6 réimportée mesure 2 848,3773392547755 mm³ ; la variante concave 6 151,622660745116 mm³. Le STL f32 mesure 2 848,3772982142123 mm³, avec ses paramètres reconstruits et le même fit initial, sans adaptation de seuil. Les placements et le miroir du 3MF sont conservés.

Sur les dix solides, le maximum bidirectionnel échantillonné observé est **0,017076591 mm** et la borne composée maximale **0,028359630 mm**, pour le budget de **0,05 mm** et une déflexion d’audit de **0,005 mm**. Ces deux métriques restent distinctes. Les réimports, volumes, dimensions, types et audits appartiennent au pipeline natif OCCT ; ils ne constituent pas un contrôle FreeCAD séparé ni un essai de la WebView.

**V-1124 — contrôles globaux finaux du lot B et de l’interface.** `sh scripts/check.sh` termine avec code 0 le 9 novembre 2026 sur le code final : **221 tests workspace, cinq tests bureau et sept tests UI** réussis ; formatage workspace/bureau, Clippy strict workspace/bureau, contrôle Svelte sans erreur ni avertissement, build UI et contrôle Cargo bureau réussis. Le seul avertissement de build reste la taille du module JavaScript. [Journal complet local](../artifacts/post-v1-zones/check.log). Ces résultats confirment les assertions finales de codes négatifs et le refactor des paramètres après les essais ciblés. `git diff --check` est sans diagnostic. Les essais visuels de V-1121 et les contrôles externes/non automatisés de V-1122 restent ouverts.


**V-1125 — chaînes coniques alternées résolues (C1a).** L’[ADR-0015](adr/0015-resolved-conical-chains.md) étend Primitives et Surfaces à trois à huit cônes par chaîne, séparés par des portées cylindriques résolues ; les témoins intérieurs sont exigés sur chaque nouvelle portion, y compris les cylindres terminaux. L’admission historique exacte de deux cônes opposés demeure prioritaire. Le [générateur](../fixtures/generate_post_v1_chains.py), les six STL, le 3MF miroir et leur [manifeste](../fixtures/post_v1_chains/manifest.json) constituent un corpus analytique indépendant du reconnaisseur.

Les quatre tests core nouveaux, les dix régressions ciblées de révolution/doubles chanfreins et les trois tests app ont réussi pendant le développement. Les tests core couvrent notamment huit cônes admis, neuf refusés, témoins absents dans une portion tardive ou terminale, milieu unique insuffisant, dernière portée ambiguë, non-affinité, transformations et limites d’écart. Les tests app contrôlent douze conversions STL (six fichiers dans chacun des deux modes), les instances miroir 3MF, familles de surfaces, dimensions, volume, provenance et audit après réimport natif. Les tests historiques et leurs attentes ne sont pas modifiés ; `m4_alternating_taper` et `m4_piecewise_taper` restent bloquants.

La recherche des témoins est débitée du compteur de travail fixe existant ; aucun nouveau paramètre de petit budget n’est introduit et aucun essai spécifique d’injection d’un budget réduit n’est revendiqué. Le plafond par chaîne et le budget de déviation sont testés. La revue indépendante du code, des tests, du générateur et de l’ADR ne relève aucun défaut bloquant ; elle n’a pas exécuté de commande Cargo. Les contrôles globaux finaux et exports CLI sont consignés séparément ci-dessous.

Les pentes de même signe, les cônes consécutifs sans portée et les révolutions générales restent hors de ce contrat. Les témoins prouvent une interprétation bornée des échantillons, pas l’intention de conception ni l’impossibilité d’un interpolant lisse. Aucun essai du paquet natif ni réimport FreeCAD indépendant n’est revendiqué pour C1a ; le paquet 1.0.0 existant n’est pas reconstruit.

**Préparation I1 — lecture seule.** La [préparation de la sélection guidée](GUIDED_SELECTION_PLAN.md) décrit la provenance exacte disponible dans `analyze_bodies`, les identifiants d’instances placées, la séparation des états source/analyse et les tests à prévoir. Elle n’ajoute aucun clic de sélection ni hypothèse moteur. Aucun test d’une fonctionnalité I1 n’est revendiqué.

**V-1126 — sept exports CLI C1a, huit solides réimportés.** Les six STL et le 3MF miroir du corpus ont été convertis en mode Surfaces avec le binaire recompilé. STEP, rapports JSON et [mesures extraites](../artifacts/post-v1-chains/measures.json) sont conservés dans `artifacts/post-v1-chains/`. Le nominal à trois cônes possède 8 faces (2 plans, 3 cylindres, 3 cônes), volume réimporté **9 682,388558363778 mm³**. Le profil à quatre cônes possède 10 faces ; la variante à cylindres terminaux en possède également 10 (2 plans, 5 cylindres, 3 cônes). Le binaire f32 conserve les tolérances initiales et mesure **9 682,388587625363 mm³**. Les deux instances 3MF conservent leurs placements et leur miroir.

Le maximum bidirectionnel échantillonné observé est **0,008031148 mm**, et la borne composée maximale **0,008032784 mm**, sous le budget **0,05 mm**, avec déflexion d’audit **0,005 mm**. Les contrôles du pipeline OCCT comprennent source, STEP réimporté, volume, dimensions, surfaces et fidélité. Aucun de ces résultats n’est présenté comme un essai de la WebView ou un réimport FreeCAD indépendant. La limite de huit cônes est couverte par les tests core ; les exports CLI archivés portent sur les profils à trois et quatre cônes du manifeste.

**V-1127 — contrôles globaux finaux C1a.** `sh scripts/check.sh` termine avec code 0 le 9 novembre 2026 : **228 tests workspace, cinq tests bureau et sept tests UI** réussis ; formatage workspace/bureau, Clippy strict workspace/bureau, contrôle Svelte sans erreur ni avertissement, build UI et contrôle Cargo bureau réussis. L’avertissement de taille Vite demeure. [Journal complet local](../artifacts/post-v1-chains/check.log). `git diff --check` est sans diagnostic. Les résultats couvrent le code final de C1a et les régressions des lots A/B ; les essais visuels, du paquet natif et le réimport FreeCAD indépendant restent ouverts.

**V-1128 — provenance et topologie d’inspection I1.** Quatre nouveaux tests core vérifient la correspondance exacte de triangles de composantes entrelacés, le soudage, l’inversion d’orientation, les unités STL, la couverture exhaustive, les bornes et les refus topologiques. Une topologie combinatoire valide reste calculable même si le volume du solide est invalide : cette API ne certifie pas un export. Trois tests app supplémentaires couvrent les identités de composantes, les instances 3MF placées/miroir/composées et l’indépendance vis-à-vis de la reconstruction. Les sept tests bureau passent, dont deux nouveaux contrôles de lecture seule du cache et de rejet des requêtes/sources/époques obsolètes. La revue indépendante du backend ne relève pas de défaut bloquant. Voir [ADR-0016](adr/0016-source-zone-inspection.md).

**V-1129 — sélection UI, worker et gestes.** Seize tests UI passent dans le contrôle ciblé : les tests ajoutés couvrent le clic immobile, les glissements, modificateurs, pan, gestes multipoints et annulation ; les gardes source/requête/effacement ; la croissance et réduction déterministes sans franchissement de composante ; l’attribution exacte des familles et son indisponibilité explicite ; les limites et graphes invalides. Le vrai module worker est exécuté dans un worker Node avec une adaptation de son canal de messages pour vérifier préparation, classement, croissance, refus et ordre FIFO. Un test Three.js vérifie le choix du corps source le plus proche indépendamment de l’ordre de la liste et le clic dans le vide. Ces tests ne remplacent pas les gestes dans la fenêtre native. La revue indépendante UI n’a pas relevé de défaut bloquant. L’analyse Svelte ciblée termine sans erreur ni avertissement et le build réussit, avec l’avertissement de taille JavaScript existant.

**V-1130 — validation intégrée I1.** `sh scripts/check.sh` termine avec code 0 le 9 novembre 2026 : **235 tests workspace, sept tests bureau et seize tests UI** réussissent. Formatage workspace/bureau, Clippy strict workspace/bureau, contrôle Svelte sans erreur ni avertissement, build UI et contrôle Cargo bureau passent. L’avertissement de taille Vite demeure. [Journal complet local](../artifacts/post-v1-selection/check.log). Le build embarqué est reconstruit avec `CARGO_NET_OFFLINE=true sh scripts/build-macos-dev.sh`, code 0 ; [journal](../artifacts/post-v1-selection/build.log). Ce paquet de développement dépend du SDK local et ne remplace pas le paquet autonome qualifié 1.0.0.

**V-1131 — essai graphique natif I1, qualification partielle.** Le paquet local démarre sans serveur Vite. Dans la fenêtre de référence 1440 × 900 (captures Retina 2880 × 1800), le cube est converti en six faces. En français sombre, activation de Sélection, clic sur un triangle, surlignage bleu, croissance de 1 à 4 triangles puis réduction à 1 sont observés ; le cadrage reste inchangé sur ces opérations. Échap efface la zone et un glissement d’orbite n’en crée pas. L’attribution indisponible du mode Plans est explicitement affichée.

Sur le cylindre chanfreiné, le résultat possède quatre faces ; la sélection d’un triangle affiche Plan, puis sa croissance affiche un cône et trois plans. Le passage à l’anglais clair traduit l’inspecteur et ses commandes. Avec un budget abaissé de 0,05 à 0,0001 mm, le moteur refuse avec `recognition.deviation_exceeded`, l’export est désactivé, les quatre triangles restent sélectionnés et le voisinage redevient disponible. Le budget est restauré à 0,05 mm. Le chargement des deux cubes 3MF efface la zone ; le résultat natif annonce deux solides, douze faces, 2 000 mm³ et 40 × 15 × 10 mm. La langue et le thème sont remis en français sombre.

**Limites de cet essai :** une première erreur ScreenCaptureKit est surmontée en relevant la fenêtre. Plus tard, les clics par coordonnées échouent avec `noWindowsAvailable` malgré un arbre accessible ; les captures du canvas deviennent anciennes ou vides, alors que les contrôles et compteurs continuent de répondre. Le rendu du nouveau 3MF et le changement des couleurs du canvas en thème clair ne sont donc pas qualifiés. Les causes de ce défaut d’observation ne sont pas établies. Restent à vérifier en fenêtre active : rendu après changement de source, thème clair du canvas, largeur 1100 px, occlusion multi-corps réelle, pan/multipoint et changement d’unité pendant calcul. Les tests automatisés correspondants ne sont pas comptés comme essais graphiques. Aucun nouvel export par dialogue, essai Fusion ou réimport FreeCAD indépendant n’est revendiqué.

**V-1132 — faces natives du résultat STEP (I1b), contrôles ciblés.** Quatre nouveaux tests CAD réussissent : les six faces du cube couvrent chacune leurs deux triangles sur leur support plan exact ; le cylindre possède trois faces et sa couture périodique ne crée pas de voisin vers lui-même ; les racines placées restent séparées dans le graphe ; un STEP invalide et une discrétisation dépassant le budget sont refusés. Deux tests app passent sur cône, calotte concave, sphère complète, tore complet, boîtier arrondi et instances 3MF placées/miroir. Ils comparent les comptes de familles aux rapports de conversion, la couverture exhaustive des triangles et les positions, et vérifient l’indisponibilité explicite de la provenance source. Deux nouveaux tests bureau contrôlent la lecture seule, la conservation du STEP après un refus d’inspection et le rejet d’un résultat remplacé même sous la même révision. Une régression de tessellation d’audit existante passe également. Voir [ADR-0017](adr/0017-step-face-inspection.md).

**V-1133 — sélection de faces STEP, contrôles UI ciblés.** Vingt-trois tests UI réussissent : les tests supplémentaires couvrent la sélection d’une face entière depuis un triangle de sa tessellation, les modificateurs, l’occlusion, la sélection multiple, la croissance/réduction sans franchissement de solide, la validation des plages et des limites, ainsi que le vrai worker. Analyse Svelte sans erreur ni avertissement et build réussis ; avertissement de taille JavaScript existant. La revue indépendante du contrat natif, du cache et de l’UI ne relève pas de défaut bloquant. Les identifiants de solides STEP sont distincts de ceux des corps source et le sélecteur source est masqué dans la vue STEP pour éviter toute assimilation.

**V-1134 — essai natif I1b, rendu suspendu par la visibilité.** Le build embarqué démarre et convertit le cube (six faces, 8 000 mm³). Le bouton Résultat STEP charge l’aperçu par la vraie commande native, active la sélection de faces et affiche l’inspecteur dédié ; la commande d’export reste disponible. Les détails indiquent la déflexion de 0,01 mm et l’absence de correspondance source démontrée. Le retour Source rétablit l’inspecteur source sans réponse tardive observée ; les libellés et contrôles sont accessibles en français sombre et en anglais clair. Le sélecteur de corps source est absent en vue STEP.

Le canvas reste vide dans cet essai. Un diagnostic temporaire confirme un maillage préparé (1 mesh, 12 triangles), des dimensions non nulles (880 × 765 CSS, canvas 1760 × 1530), un contexte WebGL valide et aucune erreur JavaScript, mais `document.visibilityState = hidden`, une image planifiée et aucune image rendue. Le retour au premier plan via le menu Window et le redimensionnement ne changent pas cet état ; les clics par coordonnées peuvent aussi échouer avec `noWindowsAvailable`. Ce constat explique la suspension de rendu observée, sans qualifier le comportement d’une WebView effectivement visible. Le diagnostic est retiré, et le hash du bundle UI final redevient identique à celui produit avant instrumentation. Aucun contournement forçant le rendu d’une fenêtre masquée n’est ajouté.

La sélection visuelle de faces natives, les modificateurs réels, la croissance/réduction à l’écran, les deux thèmes du canvas, l’occlusion multi-corps et la largeur de 1100 px restent à vérifier en fenêtre visible. Les tests natifs et du worker réussis ne sont pas présentés comme ces essais graphiques. L’instance temporaire est fermée, les préférences FR/sombre restaurées, et le paquet final est reconstruit sans instrumentation (V-1135). Aucun nouvel export par dialogue ni essai Fusion/FreeCAD indépendant n’est revendiqué.

**V-1135 — contrôles globaux finaux I1b.** `sh scripts/check.sh` termine avec code 0 le 9 novembre 2026 : **241 tests workspace, neuf tests bureau et vingt-trois tests UI** réussissent. Formatage workspace/bureau, Clippy strict workspace/bureau, contrôle Svelte sans erreur ni avertissement, build UI et contrôle Cargo bureau passent. L’avertissement de taille Vite reste présent. [Journal complet local](../artifacts/post-v1-step-faces/check.log). Les diagnostics de rendu temporaires sont retirés et les tests UI, l’analyse Svelte et le build ont été relancés sur ce code final. Les limites graphiques de V-1134 restent ouvertes ; le paquet autonome 1.0.0 existant n’est pas remplacé.

Le build de développement final `artifacts/FabSTEP.app` est reconstruit avec `CARGO_NET_OFFLINE=true sh scripts/build-macos-dev.sh`, code 0. [Journal du build final](../artifacts/post-v1-step-faces/build.log). `git diff --check` est sans diagnostic.

**V-1136 — accord utilisateur pour la suite.** Le 9 novembre 2026, l’utilisateur valide le passage à l’étape suivante après I1b. Cet accord autorise le chantier de provenance exacte STEP/source ; aucun détail supplémentaire d’essai graphique ou de fichier testé n’est fourni. Il ne transforme pas les limites d’observation de V-1134 en contrôles exécutés.


**V-1137 — provenance exacte Plans vers STEP (I1c).** L’[ADR-0018](adr/0018-planar-step-source-provenance.md) ajoute la partition source conservée depuis la segmentation et le suivi natif construction → couture → tables de transfert STEP → réimport. Les étiquettes des faces ne sont jamais suffisantes sans la provenance privée liée aux mêmes octets dans `DocumentStep`. Les API historiques restent disponibles. La provenance incomplète est indisponible pour l’ensemble du document ; les autres reconstructions restent explicitement non couvertes.

Les **49 tests CAD**, dont six nouveaux, passent lors du contrôle ciblé. Les nouveaux tests couvrent une couture remplaçant effectivement les faces, la permutation des faces et des jetons, la renumérotation des entités STEP, l’ordre inversé des racines avec noms répétés, les noms manquants/dupliqués/malformés et les collisions de jetons entre racines. Une perte de métadonnées conserve la géométrie inspectable ; les vecteurs de jetons invalides sont refusés.

Les trois nouveaux tests app vérifient STL en millimètres et pouces, trou polygonal, composantes entrelacées, soudage et inversion, ainsi que les instances 3MF placées, imbriquées, miroir et maillages déconnectés. Un oracle géométrique indépendant contrôle les supports plans attribués ; la production ne fait aucun rapprochement par coordonnées. La partition source est exhaustive et exclusive. Un STEP relu sans le compagnon privé reste sans correspondance, même s’il conserve les noms de faces. Les suites de sélection core/app passent également. Les **dix tests bureau** passent, dont le nouveau contrôle d’atomicité provenance/octets, du remplacement par un résultat non suivi ayant les mêmes octets, et de l’invalidation. La revue indépendante core/app/bureau n’a identifié aucun défaut concret ; elle est statique et ne remplace pas ces tests.

**V-1138 — inspection de la source des faces STEP.** **27 tests UI** réussissent, dont quatre nouveaux tests de provenance et du vrai worker : union de plusieurs corps par identifiants explicites, partition complète, croissance/réduction, effacement, rejet des réponses périmées et dégradation vers `step.provenance_invalid` en cas de métadonnées incohérentes. Le graphe de faces STEP demeure utilisable lorsque la provenance est refusée. La revue du principal porte sur le worker, les transitions source/STEP, l’effacement et le surlignage multi-corps.

L’interface propose **Voir la zone source** et **Retour au STEP**, conserve la sélection native et garde l’inspection source I1 indépendante. Les métriques de la vue liée couvrent le document entier ; la sélection d’un corps dans l’inspecteur source ne masque pas une partie de l’union. Les libellés français/anglais précisent la disponibilité actuelle avec **Conserver les facettes**. Aucun nouvel essai graphique n’est exécuté pour ce lot : rendu de la surbrillance, navigation réelle, thèmes, largeur de 1100 px et interactions multi-corps restent à qualifier en fenêtre visible. Le blocage d’observation de V-1134 n’est pas déclaré résolu. Aucun essai Fusion ou réimport FreeCAD indépendant supplémentaire n’est revendiqué.


**V-1139 — contrôles globaux finaux I1c.** `sh scripts/check.sh` termine avec code 0 le 9 novembre 2026 : **250 tests workspace, dix tests bureau et vingt-sept tests UI** réussissent. Formatage workspace/bureau, Clippy strict workspace/bureau, contrôle Svelte sans erreur ni avertissement, build UI et contrôle Cargo bureau passent. L’avertissement existant sur la taille du module JavaScript Vite demeure. [Journal complet local](../artifacts/post-v1-provenance/check.log). Ces tests ne remplacent pas les essais graphiques encore ouverts en V-1138.

Le paquet de développement `artifacts/FabSTEP.app` est reconstruit avec `CARGO_NET_OFFLINE=true sh scripts/build-macos-dev.sh`, code 0 ; [journal](../artifacts/post-v1-provenance/build.log). Il dépend du SDK local et ne remplace pas le paquet autonome 1.0.0. `git diff --check` est sans diagnostic.


**V-1140 — première hypothèse guidée, cylindre complet (I2a).** L’[ADR-0019](adr/0019-guided-complete-cylinder.md) borne l’action à une paroi cylindrique complète contenant tous les triangles sélectionnés, entre deux contours plans source. Le chemin réutilise `prove_patch` sans changer les preuves, les seuils ni le budget de travail ; les autres zones restent facettisées et les arcs ouverts ne sont pas activés. La segmentation analytique peut regrouper des facettes avec `max(plane, fit)` ; le résidu des plans conservés est inclus dans la borne. La sélection est plafonnée avant allocation, y compris si une analyse invalide est fournie directement à l’API core.

Les **six nouveaux tests core et sept régressions mixtes** passent lors des contrôles ciblés. Ils vérifient le choix distinct bossage/trou, la rotation, la reconstruction de la paroi entière depuis une graine, le déterminisme de la sélection, les indices invalides, les sélections de plans ou de plusieurs parois, les corpus négatifs de forme/contours/échantillonnage, le budget d’écart et le résidu des facettes avec seuils distincts. Aucun test d’épuisement des dix millions d’unités de travail ni de construction d’un cas réellement ambigu à plusieurs cylindres prouvés n’est revendiqué. La revue du principal confirme que la preuve historique reste partagée et que le chemin automatique conserve ses gardes.

**V-1141 — conversion guidée native et cache.** Les **cinq nouveaux tests app** passent : choix depuis les identifiants natifs du STEP Plans, un seul cylindre reconstruit, géométrie réimportée et audit de fidélité, rejet des sélections invalides/plusieurs parois, capture immuable des octets et des options, distinction des allocations `Arc` et du format, document STL à deux corps avec conservation du corps non ciblé, et 3MF miroir placé. Le résultat guidé reste sans provenance native analytique ; une seconde indication directe est refusée avec `guidance.provenance_unavailable`.

Le [générateur du cas 3MF](../fixtures/generate_guided_cylinder.py) réutilise le maillage indépendant M2b sans appeler le reconnaisseur. Le [manifeste](../fixtures/guided_cylinder_instances.json) conserve les empreintes et les deux placements : identité, puis miroir X translaté de 100 mm. Le test vérifie l’identité de l’instance miroir sélectionnée, un seul cylindre final, les deux solides et leurs placements, et les faces/volume de l’autre instance conservée en Plans. L’oracle de sélection de ce test a initialement utilisé par erreur le mode Plans avec `recognize_mixed` et obtenu `recognition.disabled` ; seul ce réglage d’oracle a été corrigé en Cylindres avant la réussite du test. La base de conversion reste Plans.

Les **trois nouveaux tests bureau** ciblés passent : consommation atomique du résultat de départ, échec sans restauration de l’export, annulation par révision/options/source ou conflit de téléversement, et rejet d’une allocation source différente même avec les mêmes octets. La revue indépendante app/bureau ne relève aucun défaut concret ; elle reste statique. Le pipeline commun contrôle la source, le solide réimporté, son volume et ses dimensions, puis la fidélité du corps guidé et la séparation du document recombiné. Les paramètres analytiques actifs sont signalés même lorsque le mode de base enregistré demeure Plans.

**V-1142 — parcours interface de l’indication.** **31 tests UI** passent lors des contrôles ciblés : quatre nouveaux tests couvrent l’éligibilité des faces planes avec provenance exacte, l’identité de requête/source, le rejet de plusieurs corps/composantes et les réponses guidées périmées. Le contrôle Svelte ne relève ni erreur ni avertissement et le build réussit, avec l’avertissement de taille Vite existant. La revue du principal contrôle l’enchaînement de la nouvelle requête sans invalidation prématurée de sa base, la file d’attente, la conservation de la sélection source et le retrait de l’indication.

**Essayer un cylindre** expose attente, succès ou refus, puis **Retirer l’indication** recalcule les facettes. L’interface montre les nombres de triangles indiqués et de paroi reconstruite ; les paramètres et mesures détaillés restent dans le rapport du corps. Le clic de sélection seul ne modifie rien. Les libellés et diagnostics sont traduits en français et anglais. Aucun essai graphique du nouveau parcours ni appel manuel à la commande Tauri réelle n’est exécuté pour ce lot ; les tests de cache ne sont pas présentés comme tels. L’observation d’une fenêtre masquée en V-1134 reste une limite non résolue, et aucun nouvel essai Fusion/FreeCAD indépendant n’est revendiqué.


**V-1143 — contrôles globaux finaux I2a.** `sh scripts/check.sh` termine avec code 0 le 9 novembre 2026 : **261 tests workspace, treize tests bureau et trente et un tests UI** réussissent. Formatage workspace/bureau, Clippy strict workspace/bureau, contrôle Svelte sans erreur ni avertissement, build UI et contrôle Cargo bureau passent. Le seul avertissement de build reste la taille du module JavaScript Vite. [Journal complet local](../artifacts/post-v1-guided-cylinder/check.log). La borne de taille de sélection ajoutée à la revue et les libellés finaux font partie de cette exécution. Les essais visuels indiqués en V-1142 restent ouverts.

`CARGO_NET_OFFLINE=true sh scripts/build-macos-dev.sh` termine également avec code 0 et reconstruit `artifacts/FabSTEP.app` ; [journal](../artifacts/post-v1-guided-cylinder/build.log). Ce build dépend du SDK local et ne remplace pas le paquet autonome 1.0.0. `git diff --check` est sans diagnostic.

**V-1144 — accord utilisateur après I2a.** L’utilisateur valide la poursuite du chantier le 9 novembre 2026. Aucun détail supplémentaire de fichier, de geste graphique ou d’essai CAO n’est fourni ; cet accord ne clôt pas à lui seul les contrôles visuels restant ouverts en V-1142.


**V-1145 — provenance exacte du modèle mixte guidé.** L’[ADR-0020](adr/0020-consecutive-cylinder-guidance.md) étend le suivi natif aux plans et cylindres complets. L’API CAD additive reçoit les jetons des plans et des cylindres séparément ; la construction conserve les faces exactes avant couture puis réutilise les tables de transfert STEP. Les modèles à cylindres partiels gardent leur géométrie validée sans cette provenance. Les API historiques restent disponibles.

Les **quatre nouveaux tests CAD** ciblés passent : correspondance des supports avec jetons arbitraires, permutation des plans/cylindres et couture de topologies disjointes, combinaison mixte/planaire avec ordre de racines inversé et noms identiques, métadonnées corrompues, jetons invalides et repli sans provenance pour les cylindres partiels. Les assertions géométriques constituent un oracle de test ; la production n’attribue jamais les faces par proximité ou rang de parcours. La revue du principal n’a identifié aucun défaut dans l’ABI et le suivi.

**V-1146 — indications successives et session native.** Les **sept tests core guidés** passent lors du contrôle ciblé. Le nouveau cas vérifie deux parois distinctes dans le même modèle, les attributions, le refus de deux graines de la même paroi — même différentes — et le plafond de 16 indications. La preuve et son budget sont partagés par corps ; aucun test d’épuisement du budget de dix millions d’unités ni de document comportant 16 parois effectivement admises n’est revendiqué.

Les **six tests app guidés** passent lors du contrôle ciblé. Ils couvrent ajout de deux cylindres depuis les STEP successifs, retrait du premier avec conservation de l’identifiant du second, nouvel ajout sans réutilisation d’identifiant, recalcul, retrait d’un identifiant absent et retour aux facettes après retrait de la dernière indication. Le cas 3MF existant ajoute maintenant une indication sur chacune des deux instances, dont une miroir, puis retire la première ; placements, familles et identité des occurrences restent contrôlés. Les assertions finales supplémentaires de partition exclusive/exhaustive et de cohérence des familles entre faces natives et triangles source sont exécutées dans le contrôle global consigné séparément.

Les **quinze tests bureau** passent, dont deux nouveaux tests de session conservée après refus et d’invalidation. La session acceptée ne constitue jamais un export ; retrait et récupération repartent de la source et des indications privées. Une opération périmée ne republie pas de STEP. La revue indépendante core/app/bureau n’a trouvé aucun défaut de production ; elle a relevé une faiblesse du test d’époque, corrigée avant le dernier passage bureau : la requête testée utilise désormais sa propre époque avant l’invalidation suivante. Cette revue est statique et n’a pas exécuté les tests à la place de leur auteur.

**V-1147 — liste et gestion des indications dans l’interface.** **32 tests UI** passent lors du contrôle ciblé, avec Svelte sans erreur ni avertissement et build réussi. Le nouveau test vérifie l’inspection source d’un cylindre guidé, l’ajout depuis les facettes restantes du dernier STEP et la borne 16. La revue du principal contrôle la liste acceptée non modifiée avant succès, les commandes de retrait/recalcul sans invalidation prématurée, l’effacement lors des changements de source/options et le rejet des réponses obsolètes.

Les indications sont affichées avec identifiant stable, occurrence source et nombres de triangles indiqués/reconstruits. Après refus, la liste acceptée est conservée et **Recalculer les indications conservées** relance une validation ; si elle est vide, l’action recalcule les facettes. Les libellés français et anglais expliquent ces états. Aucun essai graphique du parcours ajouter deux/retirer une/refus/récupération ni appel manuel à la commande Tauri n’est exécuté pour ce lot. La limite d’observation de V-1134 n’est pas déclarée résolue ; aucun nouveau contrôle Fusion/FreeCAD indépendant n’est revendiqué.


**V-1148 — contrôles globaux et dernier garde de recalcul I2b.** `sh scripts/check.sh` termine avec code 0 le 9 novembre 2026 : **267 tests workspace, quinze tests bureau et trente-deux tests UI** réussissent. Formatage workspace/bureau, Clippy strict workspace/bureau, Svelte sans erreur ni avertissement, build UI et contrôle Cargo bureau passent. L’avertissement existant de taille du module JavaScript Vite demeure. [Journal complet local](../artifacts/post-v1-multi-guidance/check.log).

La revue finale du principal renforce ensuite un garde défensif : les résumés `reconstructed_source_triangle_ids` d’une session acceptée sont effacés avant recalcul et doivent être remplis par les nouvelles preuves. Source et options immuables empêchent normalement qu’une composante disparaisse ; ce garde évite néanmoins qu’un résumé historique non vide masque une indication sans composante dans un état interne incohérent. Le test privé injectant précisément cet état refuse avec `guidance.invalid_selection`. Ce changement ne modifie pas les preuves géométriques ni les seuils.

Après ce dernier changement, **douze tests app** réussissent : le nouveau test privé et les suites guidage, inspection native et provenance STEP/source. Les **quinze tests bureau** sont relancés avec succès ; formatage et Clippy strict workspace/bureau réussissent également. [Tests app finaux](../artifacts/post-v1-multi-guidance/final-app.log), [tests bureau finaux](../artifacts/post-v1-multi-guidance/final-desktop.log), [Clippy workspace](../artifacts/post-v1-multi-guidance/final-clippy.log), [Clippy bureau](../artifacts/post-v1-multi-guidance/final-desktop-clippy.log). Les tests sans rapport avec ce garde ne sont pas relancés après la suite globale. Les limites graphiques de V-1147 restent ouvertes.

Le build de développement `artifacts/FabSTEP.app` est reconstruit avec `CARGO_NET_OFFLINE=true sh scripts/build-macos-dev.sh`, code 0 ; [journal](../artifacts/post-v1-multi-guidance/build.log). Il dépend du SDK local et ne remplace pas le paquet autonome 1.0.0. `git diff --check` est sans diagnostic.

## Consolidation des cylindres ouverts — 9 novembre 2026

**V-1149 — reproduction sur une troisième pièce privée.** Le STL comporte 1 830 triangles. L'essai choisit explicitement les millimètres, absents du format. Le mode Plans produit un solide valide à 430 faces planes, volume 2 949,875429127158 mm³. Avant correction, les trois modes analytiques refusent avec `recognition.partial_unproven`. Le diagnostic distingue un ajustement contraint non monotone lors de l'allongement des arcs, une paroi dont les angles de triangulation changent avec la hauteur, puis des génératrices partagées par deux arcs voisins. Certaines chaînes dépassent réellement le seuil radial initial de 0,00001 mm. La source et tous ses dérivés restent privés dans `artifacts/sock-clip/` ; ils ne sont pas redistribués comme fixtures.

**V-1150 — preuves et corpus indépendant.** L'[ADR-0021](adr/0021-staggered-open-cylinder-walls.md) décrit la récupération des unions complètes de candidats chevauchants, la preuve complémentaire des parois décalées et les raccords source partagés. Les critères et seuils historiques restent explicites ; aucun défaut de preuve ou budget dépassé ne déclenche un export facettisé implicite. La portée angulaire de chaque triangle et les dépassements numériques admis du domaine entrent dans la borne.

Le [générateur indépendant](../fixtures/generate_partial_staggered.py) produit cinq positifs et cinq négatifs. Les cinq nouveaux tests core couvrent ajustement non monotone dans les deux sens, triangulation décalée, bandes alignées près des bouchons, rotation 3D, coordonnées f32 translatées, lentille à deux raccords partagés, paroi concave, défauts radiaux/axiaux/repli/ouverture et budget insuffisant. La suite core complète passe avant le dernier ajout du résidu numérique de bord ; la validation globale finale est consignée séparément.

Les six tests CAD ciblés passent, dont deux nouveaux. Huit configurations positives contrôlent lentille convexe et encoche concave divisée, ordre des cylindres permuté et axe d'un cylindre inversé. Les négatifs refusent duplicata, troisième cylindre, deux cylindres plus un plan sur une génératrice, extrémités incohérentes et orientation erronée. Le constructeur partage les sommets et l'arête par indices source, attache la pcurve au second cylindre et contrôle les parcours opposés. `TKG2d`, déjà composant du SDK OCCT, est ajouté à la liaison directe. Une dernière garde de distance entre sommets générés est ajoutée après ces essais CAD ciblés.

Les deux nouveaux tests app passent ensuite, avec cette garde native : quinze conversions positives (cinq sources dans les trois modes analytiques), surfaces réimportées, volumes et boîtes analytiques indépendants, coordonnées source conservées, attribution complète, audit bidirectionnel et réglages personnalisés tracés. Les cinq sources défectueuses et le budget insuffisant restent refusés. [Journal app](../artifacts/sock-clip/app-tests.log). La revue indépendante du core accepte les contrôles de couverture, bornes et incidences ; la revue du principal porte sur le constructeur natif et son paramétrage des arêtes. La recherche initiale des arcs reste conservatrice et non exhaustive ; les deux contours doivent toujours correspondre.

**V-1151 — conversion réelle corrigée.** Après le dernier ajout du résidu de bord, la CLI est reconstruite et la pièce privée est convertie avec `--unit mm --fit 0.00002`, budget d'écart inchangé à 0,05 mm. Cylindres, Primitives et Surfaces produisent chacun un solide réimporté valide à **160 faces : 17 cylindriques et 143 planes**, volume **2 950,4786983269582 mm³**. Toutes les zones ne sont donc pas transformées en courbes analytiques. La borne composée vaut 0,008161612386789188 mm ; le résidu radial maximal vaut 0,000011701047885948057 mm.

L'audit échantillonné mesure 0,008133078482530292 mm de la source vers le résultat et 0,008146582366730087 mm dans l'autre sens, sur 8 237 et 11 070 échantillons, avec une déflexion de tessellation de 0,005 mm. Il ne s'agit pas d'une certification globale de Hausdorff. Les réglages et diagnostics consignent le seuil personnalisé. Aux valeurs initiales, Cylindres et Surfaces restent refusés, comme attendu ; Plans réussit encore avec 430 faces. [Six essais CLI et mesures](../artifacts/sock-clip/corrected/runs.json), [STEP Surfaces local](../artifacts/sock-clip/corrected/surfaces-fit-0.00002.step).

Une tentative supplémentaire de réimport FreeCAD avec le script existant échoue au démarrage du Python de FreeCAD, avant lecture du STEP : `Incompatible processor ... neon`, code processus -6. [Journal](../artifacts/sock-clip/freecad.stderr). Aucun succès FreeCAD indépendant, essai Fusion ni parcours graphique du nouveau build n'est revendiqué pour cette correction.

**V-1152 — contrôles globaux et application reconstruite.** `sh scripts/check.sh` termine avec code 0 : **277 tests workspace, quinze tests bureau et trente-deux tests UI** réussissent. Cette exécution inclut le dernier résidu numérique de bord et la garde native entre sommets générés. Formatage workspace/bureau, Clippy strict workspace/bureau, Svelte sans erreur ni avertissement, build UI et contrôle Cargo bureau passent. L'avertissement existant de taille du module JavaScript Vite reste présent. [Journal global](../artifacts/sock-clip/check.log).

`CARGO_NET_OFFLINE=true sh scripts/build-macos-dev.sh` termine avec code 0 et reconstruit `artifacts/FabSTEP.app` ; [journal](../artifacts/sock-clip/build.log). L'exécutable du bundle et celui du build bureau ont la même empreinte SHA-256. L'application doit être quittée puis relancée pour charger ce nouvel exécutable. Ce build dépend du SDK local et ne remplace pas le paquet autonome 1.0.0. `git diff --check` est sans diagnostic. Les limites de validation indépendante et graphique de V-1151 restent ouvertes.

## Petits raccords cylindriques contextuels — 9 novembre 2026

**V-1153 — retour utilisateur et diagnostic.** L'utilisateur confirme une amélioration sur la pièce privée, puis signale des petits raccords restés facettisés et autorise leur prise en charge. Le diagnostic sur le résultat V-1151 trouve neuf intervalles de quatre points avec un rayon ajusté proche de 0,75 mm, balayant environ 2,2° à 32,8°. Un dixième intervalle de quatre points a un rayon ajusté proche de 1,456 mm. Ces intervalles sont hors de la candidature historique à huit points et 30° minimum, même lorsque leur résidu radial respecte le seuil. Le diagnostic ne prouve ni les rayons nominaux de conception, ni la tangence effective du cercle ajusté.

**V-1154 — contrat, corpus et contrôles ciblés.** L'[ADR-0022](adr/0022-contextual-short-arcs.md) limite la recherche complémentaire aux intervalles entiers de quatre à sept points entre deux parois historiques prouvées. La résolution du centre, les témoins intérieurs et la compatibilité des tangentes conditionnent la candidature ; les deux bouchons doivent désigner les mêmes appuis. Une preuve de paroi échouée reste bloquante. Modèle, ABI, paramètres, seuils initiaux et guidage manuel sont inchangés.

Le [générateur indépendant](../fixtures/generate_short_arcs.py), écrit séparément du reconnaisseur, produit neuf STL et un manifeste. Une génération dans un dossier temporaire reproduit exactement les dix fichiers. Les quatre positifs couvrent raccord convexe, concave, rotation 3D et f32 translaté. Deux contre-exemples conservent un intervalle facettisé : conditionnement insuffisant et arc incompatible avec les tangentes voisines. Trois autres vérifient absence d'appuis, bosse et repli intérieurs, sans changer les contours ni les raccords des appuis pour les deux derniers.

Les **cinq nouveaux tests core** passent, ainsi que neuf tests historiques ciblés et Clippy core strict. Ils ajoutent bruit des témoins, source ouverte, budget insuffisant, attribution complète, volumes et déterminisme des mesures. Aucun nouveau test d'épuisement du budget de travail ni cas 3MF dédié à ces petits arcs n'est revendiqué. La revue indépendante vérifie l'algèbre de l'intervalle des centres, le contrôle des tangentes, l'identité des appuis, les refus bloquants et les limites d'interprétation. Les gardes de finitude et le débit préalable du budget sont intégrés avant la validation finale.

Les **deux nouveaux tests app** passent : dix-huit conversions dans les trois modes analytiques, avec types et comptes de surfaces, volumes et dimensions issus des formules du générateur, coordonnées source conservées, attribution complète et audit bidirectionnel. Les contre-exemples sans candidature gardent leurs facettes ; les parois défectueuses et le budget insuffisant refusent l'export. L'oracle initial du test sans appuis attendait le code core `recognition.unsupported` ; il est corrigé pour le mode Surfaces, dont le dispatch historique poursuit jusqu'au refus terminal `recognition.surface_unproven`. Aucun comportement de production n'est changé pour faire passer cette assertion. [Journal app final](../artifacts/sock-clip/short-arcs/app-tests.log).

**V-1155 — nouveaux STEP de la pièce privée.** La CLI reconstruite teste explicitement les seuils radiaux 0,00001 ; 0,000012 ; 0,000015 ; 0,00002 et 0,00005 mm, sans modifier les préférences de l'application. À **0,00002 mm**, les trois modes analytiques donnent un solide réimporté valide à **123 faces : 25 cylindriques et 98 planes**, volume **2 950,4648808945294 mm³**. Huit des neuf petits raccords proches de 0,75 mm sont reconstruits. L'intervalle d'environ 2,2° reste insuffisamment déterminé ; l'autre intervalle de rayon proche de 1,456 mm reste incompatible avec les tangentes des voisins. D'autres régions restent également facettisées.

La borne composée reste 0,008161612386789188 mm, le résidu radial maximal 0,000011701047885948057 mm. L'audit échantillonné mesure 0,008133078482530292 mm de la source vers le résultat et 0,008146582366730087 mm dans l'autre sens, sur 8 237 et 10 866 échantillons, avec déflexion demandée de 0,005 mm. Les maxima mesurés ne sont pas une certification globale de Hausdorff.

Les seuils explicites 0,000012 et 0,000015 mm donnent aussi 25 cylindres et 123 faces. Le seuil initial 0,00001 mm reste refusé. À 0,00005 mm, l'incertitude admissible exclut un raccord supplémentaire : 24 cylindres et 124 faces. Une tolérance plus large ne garantit donc pas davantage de surfaces reconnues. [Sept essais CLI](../artifacts/sock-clip/short-arcs/runs.json), [nouveau STEP Surfaces](../artifacts/sock-clip/short-arcs/surfaces-fit-0.00002.step). Aucun réglage automatique n'est ajouté. FreeCAD indépendant n'est pas relancé après le blocage documenté en V-1151 ; aucun nouvel essai Fusion ou parcours graphique n'est revendiqué.

**V-1156 — contrôles globaux et nouveau build.** `sh scripts/check.sh` termine avec code 0 : **284 tests workspace, quinze bureau et trente-deux UI** passent. Formatage workspace/bureau, Clippy strict workspace/bureau, contrôle Cargo bureau, Svelte sans erreur ni avertissement et build UI réussissent. L'avertissement existant de taille du module JavaScript Vite demeure. [Journal global](../artifacts/sock-clip/short-arcs/check.log).

`CARGO_NET_OFFLINE=true sh scripts/build-macos-dev.sh` termine avec code 0 ; [journal](../artifacts/sock-clip/short-arcs/build.log). `artifacts/FabSTEP.app` contient l'exécutable reconstruit, dont l'empreinte SHA-256 est identique à celle du build bureau. Quitter puis relancer l'application est nécessaire pour charger ce code. Il s'agit du build dépendant du SDK local ; le paquet autonome 1.0.0 n'est pas remplacé. `git diff --check` est sans diagnostic. Les validations graphiques et indépendantes non exécutées en V-1155 restent ouvertes.

## Recherche assistée de la tolérance radiale — 9 novembre 2026

**V-1157 — contrat et contrôles ciblés.** L'[ADR-0023](adr/0023-assisted-radial-search.md) introduit une action explicite de recherche parmi sept seuils radiaux au maximum. Chaque essai convertit le document entier ; le budget d'écart, l'unité, le mode, le soudage et la planéité restent inchangés. Le classement privilégie le moins de faces validées, puis le seuil le plus strict à égalité. Aucun STEP d'essai n'est installé dans le cache. L'application de la proposition emprunte le parcours ordinaire de changement d'options et de reconversion.

Trois tests unitaires app contrôlent les limites du calendrier, le classement non monotone et l'annulation avant/après un essai refusé, sans proposition partielle. Un test d'intégration réalise sept conversions natives du raccord convexe synthétique : six faces, dont trois cylindres, bornes et audit contrôlés ; sept essais d'une entrée vide ne donnent aucune proposition. Un test bureau contrôle le maintien du cache d'export, les révisions périmées, le conflit de source et le refus des indications guidées, y compris une session conservée après échec. Deux tests UI couvrent l'identité des propositions, les changements de contexte et l'extraction du seuil validé. La revue indépendante ne relève pas de blocage. Ces tests UI ne constituent pas un parcours complet de l'application montée.

**V-1158 — recherche native sur la pièce privée.** Un test local temporaire appelle directement l'API app de recherche, avec le STL sock-clip, l'unité mm et le mode Surfaces, depuis le seuil initial 0,00001 mm. Les sept essais sont exécutés en 16,58 secondes sur ce poste. Les seuils 0,000005 et 0,00001 mm sont refusés avec `recognition.partial_unproven`. Les seuils 0,000012, 0,000015 et 0,00002 mm donnent chacun **123 faces et 25 surfaces cylindriques**. À 0,00005 mm, le résultat compte 124 faces et 24 cylindres ; à 0,0001 mm, 123 faces et 24 cylindres. Le classement retient donc **0,000012 mm**, avec budget inchangé à **0,05 mm**. Aucun seuil universel ni optimum continu n'est déduit de cet essai.

Le test réussit, vérifie le choix, les comptes et le budget, puis est retiré de la suite portable ; sa source et son journal restent dans `artifacts/parameter-search/`. [Rapport des sept essais](../artifacts/parameter-search/sock-clip.json), [journal](../artifacts/parameter-search/sock-clip.log). Le fichier privé n'est pas ajouté aux fixtures. Aucun essai Fusion, réimport FreeCAD indépendant ni parcours graphique de cette nouvelle action n'est revendiqué.

**V-1159 — contrôles globaux et application reconstruite.** `sh scripts/check.sh` termine avec code 0 : **288 tests workspace, seize tests bureau et trente-quatre tests UI** passent. Formatage workspace/bureau, Clippy strict workspace/bureau, contrôle Cargo bureau, Svelte sans erreur ni avertissement et build UI réussissent. L'avertissement habituel de taille du module JavaScript Vite reste présent. [Journal global](../artifacts/parameter-search/check.log).

`CARGO_NET_OFFLINE=true sh scripts/build-macos-dev.sh` termine avec code 0 et reconstruit `artifacts/FabSTEP.app`. [Journal du build](../artifacts/parameter-search/build.log). L'exécutable du bundle et celui du build bureau partagent l'empreinte SHA-256 `089ce72c245b82d2543bf8bb8f470dd551d421f148655d9b39c5223e24bbb348`. Quitter puis relancer l'application charge ce nouveau code. Le paquet autonome qualifié 1.0.0 reste distinct. `git diff --check` ne relève aucun défaut. Le parcours graphique de recherche/application et les essais Fusion/FreeCAD indépendants restent à qualifier ; les essais natifs de V-1158 ne les remplacent pas.

## Lisibilité des faces du résultat STEP — 9 novembre 2026

**V-1160 — couleurs et contours.** À la demande de l'utilisateur, la vue Résultat STEP reçoit une palette de huit teintes par face, définie par jetons CSS dans les deux thèmes. Une attribution déterministe tient compte des voisins déjà colorés ; la palette peut être réutilisée et ne code ni famille ni qualité. La sélection conserve le bleu avec une opacité renforcée. Le contrôle **Contours**, précédemment masqué en vue STEP, y est désormais disponible.

Le worker de sélection validé prépare les couleurs et les segments de bord par face. Deux nouveaux tests vérifient la suppression des diagonales internes, la conservation des limites entre faces coplanaires, le déterminisme, l'absence de mutation du maillage et le bornage de la palette. Les segments suivent la tessellation native d'affichage, pas une exportation de courbes BRep exactes ; certaines coutures périodiques peuvent rester visibles. Le rendu teste la profondeur et décale les surfaces pour limiter la superposition instable. Les buffers de couleur sont réutilisés au changement de thème. Le cache d'export et le noyau géométrique ne sont pas modifiés.

**36 tests UI passent**, Svelte ne signale aucune erreur ni aucun avertissement, et le build UI réussit. L'avertissement Vite habituel de taille du module demeure. [Tests](../artifacts/step-faces/ui-tests.log), [Svelte](../artifacts/step-faces/ui-check.log), [build UI](../artifacts/step-faces/ui-build.log). Les suites Rust géométriques ne sont pas relancées pour ce changement d'affichage ; leur dernier contrôle global reste V-1159.

`CARGO_NET_OFFLINE=true sh scripts/build-macos-dev.sh` termine avec code 0 et reconstruit `artifacts/FabSTEP.app`. [Journal](../artifacts/step-faces/build.log). L'exécutable empaqueté et le binaire bureau ont la même empreinte SHA-256 `814baca1e8bca2190015304434d44fffaab9e9faedfa2434d1970f36b414d9fd`. `git diff --check` est sans diagnostic. Un lancement de Chrome headless avec profil temporaire isolé échoue avec code 134 avant toute capture ; aucune qualification visuelle clair/sombre ou manipulation native de ces couleurs n'est revendiquée. Quitter puis relancer le build de développement est nécessaire pour charger ce rendu ; le paquet autonome 1.0.0 reste distinct.

## INT-AUDIT-01 — intégration raisonnée du rapport technique, 10 novembre 2026

**V-1161 — qualification, référence et corrections ciblées.** Le [registre unique d’intégration](INT_AUDIT_01.md) contient les décisions sur les 22 constats et les jalons 01.1–01.5. Le rapport d’audit original n’est pas réécrit. Une sauvegarde et des empreintes de l’arbre initial préservent les nombreux développements post-v1 préexistants. `scripts/check.sh` exécuté avant modification termine avec code 0 : **288 tests workspace, 16 bureau, 36 UI**, formatage, Clippy strict, Svelte et builds réussis. [Journal de référence](../artifacts/int-audit-01/baseline-check.log).

Les contrôles ciblés et la revue indépendante valident la garde des quatre coins cylindriques en 3D, l’annulation coopérative aux frontières des opérations et le cache privé de correspondance guidée. Le témoin numérique hors tolérance auparavant accepté par le core puis rejeté par OCCT est désormais refusé avec `recognition.partial_unproven`, sans fichier STEP. Son voisin dans la tolérance reste un solide natif valide. Le sock-clip conserve **123 faces, dont 25 cylindres**, volume **2 950,4648808945294 mm³**, au seuil radial explicitement choisi de 0,000012 mm. [Régressions natives](../artifacts/int-audit-01/native-regressions.json).

Les mesures du cache portent sur trois répétitions par fixture, en profil test non optimisé avec OCCT 7.9.3. Le temps médian du guidage passe de 2 344,082 à 2 304,690 ms sur le STL mixte, et de 7 253,398 à 7 140,568 ms sur le 3MF à deux instances. Les six paires de rapports géométriques et de fidélité sont identiques. Les gains observés (1,68 % et 1,56 %) ne sont pas généralisés ; les compteurs des tests démontrent séparément l’inspection native évitée. [Mesures et dispersion](../artifacts/int-audit-01/runtime/benchmark-comparison.json).

La protection d’archive dispose de douze tests Python isolés ; le garde SDK a quatre tests Rust autonomes. Les tests Python ont été rejoués après le dernier ajustement de leur remise à l’état propre, qui utilise des commits de fixtures temporaires plutôt qu’un reset destructif ; aucun index ni historique du dépôt réel n’est modifié. [Dernier journal Python](../artifacts/int-audit-01/package-tests-final.log). L’aide CLI et l’avertissement UI sont vérifiés par compilation et contrôles automatisés. Les limites d’annulation au sein d’un appel natif, de qualification graphique, de tests de composants montés et de réimport FreeCAD/Fusion indépendant restent explicites dans le registre.

**V-1162 — clôture des cinq jalons et nouveau build.** La suite finale `sh scripts/check.sh` termine avec code 0 : **297 tests workspace, seize tests bureau, trente-six tests UI, douze tests Python et quatre tests SDK autonomes** passent. Le benchmark ignoré dans la suite normale a déjà été exécuté explicitement avant et après le cache (V-1161). Formatage, Clippy strict workspace/bureau, contrôle bureau, Svelte sans erreur ni avertissement et build UI réussissent. L’avertissement habituel Vite concernant la taille du module demeure. [Journal final](../artifacts/int-audit-01/final-check.log).

`CARGO_NET_OFFLINE=true sh scripts/build-macos-dev.sh` réussit et reconstruit `artifacts/FabSTEP.app` ; [journal du build](../artifacts/int-audit-01/build.log). Le binaire empaqueté et le binaire bureau partagent l’empreinte SHA-256 `0022c36794f957e775c9e99f0e41399b3c74f15a60f6d4641d6493381aab0207`. `git diff --check` est sans diagnostic. Les modifications de cette intégration sont inventoriées relativement à la sauvegarde du début de tâche, afin de préserver et distinguer le travail post-v1 préexistant. Le rapport d’audit original est inchangé ; aucun commit ni push du dépôt réel. Les limites graphiques/CAO indépendantes de V-1161 restent ouvertes.

## UX4 — atelier simplifié, 10 novembre 2026

**V-1163 — présentation, contrôleur et non-régression.** La vue 3D occupe l’espace principal, avec un inspecteur unique à droite. Les six boutons d’exemple et leur chargement dans App sont retirés, sans supprimer les fixtures. Deux choix courants de reconstruction restent visibles ; les modes ciblés et les tolérances sont sous Avancé, initialement replié avec un nouvel identifiant UX4. Détails du résultat regroupe mesures, provenance, réglages appliqués et diagnostics techniques. Les erreurs, unités, avertissements influant sur le résultat et la remise à zéro du guidage restent visibles ; les outils de correction apparaissent avec la sélection. La recherche est accessible sous Avancé et directement après un refus de reconnaissance.

`workflow.ts` est le contrôleur réellement utilisé par App pour lire/importer, invalider, convertir, guider et exporter. Une façade d’accesseurs conserve les runes et le verrou partagés avec la recherche et les workers. **23 nouveaux tests** avec promesses différées vérifient les lectures inversées, les refus, les réponses périmées aux différentes phases, la reprise de la dernière demande, les réglages invalides, le guidage atomique et sa récupération, l’export et la destruction. Ces tests ne montent pas un DOM et ne simulent pas une réussite géométrique du moteur natif. [Journal ciblé](../artifacts/ux4/workflow-tests.log).

La revue indépendante accepte l’intégration : comparaison aux fonctions avant UX4, contrôle des accesseurs dans le JavaScript compilé Svelte et 23 tests rejoués. Elle relève uniquement deux espaces perdus devant le nombre d’échantillons ; ils sont rétablis avant le dernier build et la fin de la suite globale. Douze clés devenues inutilisées sont retirées ; les clés des composants partagés et l’alias `guidance.complete_cylinder_verified`, toujours émis par le core, sont conservés. Les **353 clés FR/EN** et leurs paramètres concordent. [Parité des langues](../artifacts/ux4/locales.json).

`sh scripts/check.sh` termine avec code 0 : **297 tests workspace, 16 bureau, 59 UI, 12 Python et 4 tests SDK** réussissent, ainsi que formatage, Clippy strict, contrôle bureau, Svelte (0 erreur, 0 avertissement) et build UI. [Journal complet](../artifacts/ux4/check.log). L’avertissement de taille du module Vite demeure. Aucun changement de noyau, d’ABI ou de format de données ; pas de benchmark de performance revendiqué. `git diff --check` est sans diagnostic.

**V-1164 — build et qualification native partielle.** `CARGO_NET_OFFLINE=true sh scripts/build-macos-dev.sh` réussit et reconstruit `artifacts/FabSTEP.app`. Le binaire empaqueté et le binaire bureau ont la même empreinte SHA-256 `56c62f0a1807ba19a21b10ab167a52d521a840fccbb69ea80bc335a70a53b61d`. [Journal du build](../artifacts/ux4/build.log), [manifest des fichiers](../artifacts/ux4/manifest.json).

L’accueil du nouveau build est observé par capture native et arbre d’accessibilité en **français sombre à 1440 × 900 points** (capture 2880 × 1800) : canevas agrandi, inspecteur unique, exemples absents, Avancé et Détails repliés, unité et reconstruction disponibles, export désactivé sans résultat. La capture précède uniquement la restauration des deux espaces dans les détails de fidélité. Ouvrir affiche le sélecteur natif ; après sélection de `m6a_rounded_box_rotated.stl`, le bouton Open reste désactivé, y compris après activation du dialogue et double clic. L’annulation et les appels suivants expirent (`timeoutReached`), même après réinitialisation de l’outil. Le navigateur de contrôle n’est pas disponible. La cause du blocage n’est pas établie ; le filtre de fichier est inchangé par UX4.

**Non validés graphiquement dans ce lot :** import/conversion/export complet, correction par sélection, états succès/refus, clair/anglais, clavier et largeur 1100 points/petits écrans. Les tests du contrôleur et les contrôles natifs automatisés ne remplacent pas ces essais. Aucune nouvelle validation Fusion/FreeCAD indépendante. Le paquet autonome 1.0.0 est inchangé. Aucun commit ni push ; les modifications préexistantes du dépôt sont conservées.

## Bascule automatique vers le résultat STEP — 10 novembre 2026

**V-1165.** Le contrôleur appelle `onResultReady` après publication d’une conversion ou correction guidée courante. App utilise le chargement STEP existant : la vue change à la réception de la préparation worker, pas à la seule réception du rapport de conversion. Les recalculs, ajouts, retraits et reprises de guidage réussis déclenchent ce parcours ; les essais de recherche ne publient aucun STEP et ne le déclenchent pas.

Deux nouveaux tests vérifient le déclenchement une fois par résultat publié, son absence sur réponse périmée/refus et les trois commandes guidées. **61 tests UI, dont 25 du contrôleur, passent** ; Svelte signale zéro erreur/zéro avertissement. [Tests](../artifacts/auto-step/ui-tests.log), [Svelte](../artifacts/auto-step/ui-check.log). La revue indépendante accepte le diff et rejoue les 25 tests du contrôleur. Elle vérifie l’installation du cache natif avant la réponse, l’annulation des jetons au retour Source, le maintien de l’export après échec d’aperçu/worker et la destruction. Ces derniers contrôles sont une revue de code, pas un essai graphique.

`CARGO_NET_OFFLINE=true sh scripts/build-macos-dev.sh` réussit ; [journal](../artifacts/auto-step/build.log). `artifacts/FabSTEP.app` et le binaire bureau partagent SHA-256 `fa56a3cfcc07b7da238e88e72c8ce0be509b9485799b8c3315d576e27ed833c5`. L’avertissement Vite de taille demeure. `git diff --check` est sans diagnostic. Aucune modification Rust ni de contrat natif ; les suites Rust complètes ne sont pas rejouées pour ce callback UI, leur dernière validation reste V-1163.

Le build est relancé via le contrôle natif. L’essai d’import de `guided_cylinder_instances.3mf` rencontre le même blocage que V-1164 : fichier sélectionné, bouton Open désactivé, puis expiration du pilotage lors de l’annulation. **La bascule et le parcours graphique complet ne sont donc pas validés dans l’application.** Aucune attribution de cause sans preuve. Le paquet autonome 1.0.0 est inchangé ; aucun commit ni push, modifications préexistantes préservées.

## Qualification UX4 et diagnostic du sélecteur — 10 novembre 2026

**V-1166 — accueil anglais clair et expérience de filtre.** L’accueil natif est observé en anglais clair à 1440 × 900 points : bascule de langue/thème effective, vue 3D dominante, unité, reconstruction, Avancé et Détails accessibles ; export désactivé sans résultat. Les préférences sont ensuite rétablies en français sombre. Cette observation ne qualifie pas les états avec modèle ni les petits écrans.

Une variante temporaire `accept=".stl,.3mf"`, compilée puis relancée, ne débloque pas la sélection de `guided_cylinder_instances.3mf` : Open demeure désactivé. La lecture du Wry 0.55.1 effectivement verrouillé montre que `run_file_upload_panel` crée un NSOpenPanel et configure fichiers/dossiers/multiplicité, sans configurer de filtre MIME ou d’extensions. Aucun lien causal avec les MIME n’est donc démontré. La variante est **retirée**, App est identique à sa sauvegarde avant expérience, et le build est reconstruit avec le filtre d’origine. Les validations extension/64 Mio du workflow et les contrôles natifs n’ont pas changé. La cause du blocage du sélecteur demeure inconnue ; aucune nouvelle commande d’import ni dépendance n’est ajoutée sur cette seule hypothèse.

[Build expérimental](../artifacts/ux4-native/build-extensions.log), [build restauré](../artifacts/ux4-native/build-restored.log). Le build restauré termine avec code 0 ; SHA-256 de `artifacts/FabSTEP.app/Contents/MacOS/FabSTEP` : `52187b997d2f45c3c4a2efa21bf3b07329be8adfe84a687967d77f53ecadcae6`. L’outil natif ne fournit pas d’API d’activation d’application supplémentaire (`launch_app` indisponible). Le parcours import → conversion → bascule STEP → export reste à qualifier, indépendamment des tests automatisés de V-1165.

## Profilage de la recherche radiale — 10 novembre 2026

**V-1167 — mesure préalable à PERF1 (AUD-018/004/005).** Ajout du diagnostic ignoré par défaut `crates/fabstep-app/tests/parameter_search_profile.rs`, sans dépendance ni changement de production. Il reconstruit les sept conversions des réglages initiaux via `convert_document_controlled`, pour obtenir leurs frontières nommées. Ce n’est pas une instrumentation directe de `search_parameters` : l’égalité complète des résumés, paramètres et classement avec la recherche réelle est vérifiée sur la fixture mixte. Les résultats restent identiques entre répétitions sur chaque cas.

Profil `cargo test` non optimisé, Apple Silicon, macOS 26.5.2, SDK OCCT 7.9.3 observé. Trois répétitions par cas, sans exclusion du premier passage, séquence fixe ; aucune extrapolation au build release. Le test natif termine avec code 0 en **224,89 s** (63 conversions du scénario reconstruit et 7 de comparaison à la recherche réelle, refus attendus inclus). [Journal natif](../artifacts/radial-profile/run.log), [mesures brutes](../artifacts/radial-profile/summary.json), [synthèse et limites](../artifacts/radial-profile/analysis.json).

| Source | Médiane des 7 essais | Min–max | Validation source | Séparation source | Audit comparatif |
| --- | ---: | ---: | ---: | ---: | ---: |
| Mixte, 1 728 triangles | 12,838 s | 12,568–13,002 s | 43,18 % | — | 28,90 % |
| 3MF, 2 instances, 3 456 triangles | 40,648 s | 40,268–40,757 s | 28,20 % | 28,16 % | 19,83 % |
| Sock-clip privé, 1 830 triangles | 17,223 s | 17,208–17,231 s | 27,76 % | — | 47,34 % |

Les pourcentages rapportent la médiane des sommes d’une phase à la médiane du scénario. Analysis (import, soudage, topologie, segmentation initiale et provenance) représente 6,39–10,65 %. AuditTessellation représente 0,93–1,44 %, mais inclut lecture et tessellation sans les distinguer. La construction englobe le réimport/contrôle STEP obligatoire. Les enveloppes Body sont inclusives et exclues de la somme des phases. Une sonde indépendante d’import/aperçu vaut 13,21 / 11,20 / 1,32 ms médians ; elle ne mesure pas une sous-phase de la recherche et ne doit pas être soustraite à Analysis. L’environnement n’est pas un banc de performance isolé.

Sock-clip : deux refus `recognition.partial_unproven`, puis cinq résultats validés ; proposition inchangée **0,000012 mm, 123 faces, 25 surfaces courbes (cylindres dans la qualification native précédente)**. Les trois répétitions ont la même classification et le même classement. La lecture racine des JSON recalcule et confirme les médianes et les parts ci-dessus.

Après la mesure complète, le diagnostic est ajusté : fichier privé uniquement sur variable d’environnement (aucun chemin personnel en source), noyau lu dans les futurs rapports validés, contrôle de distance échantillonnée limité à finitude/positivité pour ne pas imposer un seuil plus strict que le contrat natif, suivi par cas et `progress.json` marqué complet à la fin. **Ces ajustements finaux sont compilés et contrôlés par formatage/Clippy strict, sans second profil natif complet.** Les artefacts bruts conservent la première exécution et le précisent ; aucun temps n’est présenté comme mesuré par cette seconde variante. [Compilation finale](../artifacts/radial-profile/final-compile.log), [Clippy ciblé](../artifacts/radial-profile/clippy.log). L’orchestrateur rejoue formatage workspace et Clippy ciblé strict avec succès. Les suites géométriques complètes et UI ne sont pas rejouées : seul un diagnostic ignoré est ajouté ; références précédentes V-1163/V-1165.

Reproduction sans fichier privé :

```sh
cargo test -p fabstep-app --test parameter_search_profile --offline -- --ignored --nocapture --test-threads=1
```

`FABSTEP_PROFILE_REPETITIONS` accepte 2 ou 3 ; `FABSTEP_PROFILE_PRIVATE_STL` désigne facultativement une source privée. Les sorties vont dans `artifacts/radial-profile/` et les mesures d’une nouvelle exécution remplacent les fichiers de diagnostic précédents : les conserver ailleurs avant une comparaison.

Conclusion : étudier d’abord des preuves source réutilisables dans un contexte privé par recherche, en préservant l’ordre reconnaissance → validation source et les contrôles d’annulation. La séparation des corps source mérite un traitement distinct. Chaque résultat candidat doit garder son propre réimport et audit ; segmentation partagée et fusion FFI restent différées. **Aucune accélération n’a été implémentée ou mesurée avant/après dans ce lot.** Aucun commit ni push ; dépôt préexistant conservé.

**Complément utilisateur du 10 novembre 2026 — ouverture manuelle.** À la question distinguant le défaut produit du blocage de pilotage, l’utilisateur confirme : « Oui, l’ouverture fonctionne ». La sélection et l’ouverture manuelles sont donc validées par déclaration utilisateur ; aucun fichier, format ou build précis n’est indiqué dans cette confirmation. Le blocage observé reste une limite du parcours automatisé, sans défaut d’import produit établi. Cette confirmation ne qualifie pas à elle seule la bascule STEP, les corrections ou l’export.

## V-1168 — PERF1 : preuves source privées et mesure de la recherche réelle

10 novembre 2026. **Équivalence et mesure ciblées PASS ; contrôles globaux suivis en V-1169.** Le cache est neuf à chaque `search_parameters`, privé, borné, sans objet OCCT. Il compare les bits des coordonnées et les triangles ordonnés après unités/placements. Seuls les succès de validité source et de séparation source sont conservés, indépendamment. La reconnaissance garde sa priorité sur la validité du corps ; les callbacks avant/après sont exécutés aussi sur un succès réutilisé. Analyse/segmentation, construction, réimport, contrôles du résultat et audit de fidélité restent exécutés pour chaque candidat. Les parcours ordinaires/guidés ne fournissent aucun cache. Revue indépendante en lecture seule : ACCEPT ; diff également contrôlé par l’orchestrateur.

Les tests natifs comparent les **DocumentReport complets** et les traces de phases avec/sans cache sur les sept réglages : STL public `short_arcs/convex.stl`, 3MF `guided_cylinder_instances.3mf`, puis sock-clip privé. Ces rapports comprennent provenance, dimensions/volumes, reconnaissances et fidélité. Les compteurs privés de test montrent 1 validation au lieu de 7 sur le STL, 2 au lieu de 14 et 1 séparation au lieu de 7 sur le 3MF ; sock-clip : 1 validation au lieu de 5, deux refus de reconnaissance et cinq succès identiques. Chaque candidat accepté conserve ses propres phases Build/Audit et les contrôles associés. Les tests vérifient également unités différentes, géométrie/ordre/indices/bits modifiés, bornes mémoire, nouvelles tentatives après refus natif, priorité des refus et annulation avant/après les accès réussis au cache. Le test privé est ignoré par défaut et exécuté explicitement via `FABSTEP_PROFILE_PRIVATE_STL` ; [journal](../artifacts/perf1/private-full-report-equivalence.log), [tests du cache](../artifacts/perf1/cache-unit-tests.log).

La mesure distincte `tests/search_performance.rs` appelle cette fois la **vraie API search_parameters**, avant et après modification, trois fois par fichier avec les mêmes réglages, séquentiellement sans autre test natif simultané. Profil Rust de développement non optimisé, même Mac arm64 et SDK OCCT 7.9.3 ; pas de suppression de chauffe, ni de prédiction release. La référence utilise le binaire antérieur conservé dans les artefacts locaux. L’environnement n’est pas un banc de performance isolé.

| Source | Médiane avant | Médiane après | Min–max avant | Min–max après | Temps réduit |
| --- | ---: | ---: | ---: | ---: | ---: |
| Mixte `m2b_boss_blind.stl` | 13,057 s | 8,294 s | 12,930–13,140 s | 8,212–8,339 s | 36,5 % |
| 3MF à deux instances | 41,508 s | 21,209 s | 40,776–41,566 s | 21,097–21,280 s | 48,9 % |
| Sock-clip privé | 17,465 s | 13,538 s | 17,402–17,487 s | 13,528–13,576 s | 22,5 % |

Les trois répétitions sont stables en résultat. Settings, sept résumés, refus et classement sont identiques avant/après ; comparaison supplémentaire des JSON par Python avec `parse_float=Decimal`, sans tolérance numérique. Sock-clip conserve 0,000012 mm proposé, 123 faces et 25 surfaces courbes. [Avant](../artifacts/perf1/before.json), [après](../artifacts/perf1/after.json), [comparaison](../artifacts/perf1/comparison.json), [journal avant](../artifacts/perf1/before.log), [journal après](../artifacts/perf1/after.log).

Incidents des outils de validation, corrigés sans changement du moteur : premier essai de mesure arrêté après le STL sur un chemin de sortie relatif au dossier du crate (relancé avec chemin absolu) ; comparaison initiale des artefacts sensible à l’arrondi d’un dernier bit par le parseur JSON Rust par défaut (les valeurs sérialisées étaient identiques ; même parsing des deux côtés puis vérification décimale indépendante) ; deux attentes de test corrigées pour refléter le parcours existant (absence de Combine sur STL unique et code historique `recognition.surface_unproven`). Les comparaisons finales ne relâchent aucune tolérance géométrique.

Reproduire la mesure sur la version courante, avec chemins absolus et destination distincte pour conserver les références :

```sh
FABSTEP_PERF_OUTPUT=/chemin/absolu/apres.json \
FABSTEP_PERF_BASELINE=/chemin/absolu/avant.json \
cargo test -p fabstep-app --test search_performance --offline -- --ignored --nocapture --test-threads=1
```

`FABSTEP_PROFILE_PRIVATE_STL` ajoute facultativement le sock-clip local ; le même ensemble de cas est exigé avant/après. Le fichier privé n’est pas ajouté au dépôt. Ces mesures ne qualifient pas le parcours graphique ni un paquet autonome. La segmentation partagée (AUD-004) et la fusion FFI de lectures STEP (AUD-005) restent différées. AUD-018 est traité par une solution adaptée, sans cache public d’Analysis ni prévalidation modifiant la priorité des erreurs.

## V-1169 — PERF1 : contrôle global et bundle de développement

10 novembre 2026. **PASS.** `sh scripts/check.sh` termine avec code 0 : 12 tests Python de packaging, 4 tests du garde SDK, 304 tests workspace, 16 tests bureau et 61 tests UI ; formatage workspace/bureau, Clippy strict workspace/bureau, vérification TypeScript/Svelte (0 erreur/avertissement), build UI et vérification bureau réussis. Les tests ignorés de profilage ne sont pas comptés comme exécutés par cette suite ; la mesure réelle et le test privé sont exécutés séparément en V-1168. Aucun échec final ni régression détectée. [Journal global](../artifacts/perf1/check.log).

`CARGO_NET_OFFLINE=true sh scripts/build-macos-dev.sh` termine avec code 0 et reconstruit **`artifacts/FabSTEP.app`**. Le binaire du bundle est identique au binaire bureau produit, vérifié par SHA-256 ; [build](../artifacts/perf1/build-macos.log), [empreintes](../artifacts/perf1/bundle-sha256.txt). L’avertissement existant Vite sur la taille du bundle demeure ; ce lot ne prétend pas optimiser le démarrage UI. Fermer puis rouvrir l’application pour charger ce binaire. Le paquet autonome `artifacts/release/1.0.0/FabSTEP.app` est inchangé.

Limites : pas de nouvel essai graphique ni de réimport Fusion/FreeCAD indépendant dans ce lot ; la qualification native automatisée porte sur les API Rust/CAD. Les gains dépendent du modèle et du profil de compilation ; ils ne réduisent pas les contrôles des résultats. Les modifications locales préexistantes sont conservées, sur `codex/v1-mechanical` ; aucun commit, push, réécriture d’historique ni publication. Le diff propre à PERF1 touche trois modules app, le nouveau cache et le diagnostic de mesure, ainsi que README/architecture/état/roadmap/registre/validation. `git diff --check` réussit. Le retour arrière peut retirer uniquement le branchement du cache de la recherche, en préservant les changements antérieurs ; les trois fichiers source avant PERF1 sont conservés dans les artefacts locaux pour comparaison, sans imposer de restauration globale.

## V-1170 — DXF1 : profils natifs et réimport DXF indépendant

10 novembre 2026. **PASS ciblé ; validation globale suivie en V-1171.** Extraction d’une face plane du STEP immuable, identité partagée avec l’inspection des faces, contour extérieur et trous contrôlés exhaustivement. Lignes, arcs et cercles natifs conservés ; pas de discrétisation, projection de face courbe ou fermeture ajoutée. Voir [ADR-0024](adr/0024-planar-face-dxf.md).

Douze tests core couvrent repère, contours, trous, arcs de ±90°/±270°, erreurs numériques, ouverture, budgets de contours/segments et sortie 16 Mio. Neuf tests CAD couvrent identité des six faces, plans obliques, miroir et translation, plusieurs racines, trous polygonaux/circulaires, arcs de 270° et faces opposées, refus des ellipses/faces non planes/identifiants absents, limite d’entrée et STEP à contour ouvert. Les quatre tests d’inspection `result_faces` sont rejoués après partage du lecteur de topologie. Un test app contrôle le lien à `DocumentStep`, l’export des six faces et l’absence de mutation du STEP. Trois tests bureau vérifient sélection A → B → A, messages désordonnés, source/options/résultat remplacé à révision identique et conservation du STEP après échec de profil. Les 19 tests bureau ciblés passent. Les contrôles natifs de saturation à 4 096 contours/100 000 arêtes sont relus dans le code, sans génération d’un STEP atteignant ces deux limites ; le test de contour ouvert échoue dès la lecture du STEP et ne force pas chaque branche interne de WireExplorer.

Le corpus `fabstep-app/tests/face_profile.rs::emit_native_dxf_corpus`, ignoré par défaut, produit **39 profils** provenant de cinq conversions : cube fixture de 1 mm, bossage/trou borgne, arc ouvert oblique, petits arcs et 3MF à deux instances. Les DXF sont relus avec **ezdxf 1.4.4**, installé préalablement dans un environnement Python local distinct de l’application. `scripts/validate-dxf-profiles.py` vérifie version AC1015, unités mm, nombre/type des entités, calques extérieur/trous, coordonnées, rayons, balayages, fermeture des contours importés et dimensions de référence du cube. Chaque document est audité : **0 erreur et 0 réparation**. Cinq références supplémentaires de l’exporteur vérifient les arcs signés/majeurs, cercle et trous, avec le même résultat d’audit.

Preuves : [corpus natif](../artifacts/dxf1/native-corpus/manifest.json), [réimport indépendant](../artifacts/dxf1/native-corpus/independent-validation.json), [journal du lecteur](../artifacts/dxf1/independent-reader.log), [références de l’exporteur](../artifacts/dxf1/core-ezdxf-verification.json), [test app](../artifacts/dxf1/app-profile-tests.log), [tests bureau](../artifacts/dxf1/desktop-tests.log).

Reproduction (chemin absolu pour la sortie du test Cargo ; Python disposant d’ezdxf pour la relecture) :

```sh
FABSTEP_DXF_CORPUS_DIR=/chemin/absolu/corpus \
cargo test -p fabstep-app --test face_profile --offline emit_native_dxf_corpus -- --ignored --nocapture --test-threads=1
python scripts/validate-dxf-profiles.py /chemin/absolu/corpus
```

Les erreurs initiales des outils/tests sont corrigées sans changement du contrat : assertion de dimension du cube revue après lecture de la fixture (1 mm, distinct du modèle d’exemple de 20 mm) ; le cas ellipse utilise les identifiants du STEP d’origine, le tessellateur historique refusant déjà le STEP altéré ; une attente de rejet du worker est corrigée car la requête avait déjà été remplacée. Une tentative d’installation d’ezdxf dans un environnement temporaire échoue faute de résolution réseau ; aucun paquet ni dépendance n’est ajouté au projet, et l’environnement existant est ensuite utilisé. Le réimport FreeCAD est impossible : Qt signale `Incompatible processor … neon` au chargement de son module. Ezdxf qualifie ici le fichier DXF, pas son utilisation dans Fusion.

## V-1171 — DXF1 : interface, contrôle global et bundle

10 novembre 2026. **PASS automatisé ; qualification graphique/Fusion restante.** L’inspecteur propose automatiquement le profil d’une seule face plane du STEP courant. Les tracés SVG et extrema analytiques sont préparés par un worker dédié, avec au plus un calcul actif et une demande suivante remplacée par la plus récente. Les résultats périmés sont rejetés après extraction et après préparation graphique. Repère, dimensions, trous, unités et export sont contextualisés ; français/anglais complets et jetons de thème conservés.

Les 36 nouveaux tests UI couvrent cycle asynchrone et gardes d’identité, changement de vue/source/résultat/sélection, retour A → B → A, export annulé/refusé, réponses et échecs tardifs, destruction, worker actif/remplacé, limites, arcs signés/majeurs et dimensions analytiques. Le worker réel est exercé par les tests ; ils ne montent pas le composant Svelte et ne constituent pas des essais graphiques. La revue indépendante a demandé de retirer le calcul géométrique synchrone initial ; après déplacement dans le worker et suppression du double calcul dans le composant, la revue est **ACCEPT** sur core, CAD, bureau et UI. Le principal a également relu les modifications et vérifié le corpus indépendant.

`sh scripts/check.sh` termine avec code 0 : **326 tests workspace, 19 bureau, 97 UI, 12 Python packaging et 4 SDK** ; formatage workspace/bureau, Clippy strict, Svelte/TypeScript (0 erreur et 0 avertissement), build UI et vérification bureau réussis. Aucun échec final ni régression détectée. Les tests ignorés d’émission de corpus sont exécutés séparément en V-1170, sans les compter dans cette suite. [Journal global](../artifacts/dxf1/check.log), [tests UI ciblés](../artifacts/dxf1/ui-tests.log).

La tentative de qualification visuelle s’arrête dès l’inventaire CUA : aucune surface disponible, `Native pipe startup failed` et `browser access host unavailable`. Aucun navigateur, modèle ni dialogue n’a donc été piloté ; aucune capture ou validation graphique n’est revendiquée. L’ouverture manuelle précédemment confirmée ne valide pas automatiquement ce nouveau profil. Restent à essayer dans l’application : clic sur une face, aperçu dans les thèmes/langues, sauvegarde/annulation, clavier et petits écrans, puis réimport Fusion. FreeCAD est indisponible comme décrit en V-1170 ; la relecture DXF par ezdxf est réellement exécutée.

Le build de développement `artifacts/FabSTEP.app` est reconstruit via `CARGO_NET_OFFLINE=true sh scripts/build-macos-dev.sh` ; le binaire livré est comparé au binaire bureau produit par SHA-256. [Journal du build](../artifacts/dxf1/build-macos.log), [empreintes](../artifacts/dxf1/bundle-sha256.txt). L’avertissement existant Vite sur le bundle principal de plus de 500 ko reste présent ; le worker de profil est livré séparément. Le paquet autonome 1.0.0 n’est pas modifié.

État Git : branche `codex/v1-mechanical`, modifications locales antérieures conservées ; aucun commit, push ou publication. Snapshots avant DXF1 conservés dans `artifacts/dxf1/before` pour comparaison ciblée. `git diff --check` passe. Les seuls ajouts concernent DXF1, ses tests/lecteur indépendant et sa documentation. Aucun nouveau coût de dépendance d’exécution ; la suite réellement utile est la qualification utilisateur du profil, puis DXF2 (coupe selon un plan), sans étendre silencieusement DXF1 aux faces courbes.

## V-1172 — DXF2a : sections natives et relecture indépendante

10 novembre 2026. **PASS ciblé ; suite globale suivie en V-1173.** Première étape DXF2 sur le STEP validé : plans XYZ ou plan repris d’une face, décalage signé, liste explicite des corps, contours fermés et DXF 1:1 en mm. Le solide n’est pas découpé. La coupe directe du maillage source reste une étape distincte, DXF2b. La demande utilisateur de sélectionner un groupe de facettes puis une forme géométrique à rechercher pour corriger le STEP est ajoutée au suivi I2 de la roadmap.

Dix nouveaux tests core couvrent plusieurs corps/composantes/trous, corps vides conservés, refus d’export entièrement vide, identifiants, repère, budgets cumulés et écriture. Les douze tests DXF1 restent réussis ; cinq références DXF1 sont réémises identiques octet pour octet. Deux sections de référence sont relues par ezdxf 1.4.4 sans erreur ni réparation, avec unités, calques, rayon et arc horaire de 270° vérifiés : [preuve core](../artifacts/dxf2/core-ezdxf-verification.json).

Les neuf nouveaux tests CAD couvrent boîte et corps choisi vide, trous circulaires/polygonaux, anneau du tore et deux composantes disjointes, plan oblique, miroir/placement, grand cercle, plan de face indépendant de son contour elliptique, refus coplanaire/tangent/couture/ellipse et absence de résultat partiel. Les niveaux critiques sont contrôlés avant le résultat de l’opération : test de sphère tangente hors couture après un premier corps valide, support tronqué et sommet conique. La revue a demandé de préserver le parcours DXF1 avec contexte de face et de remplacer toute confiance dans trois normales échantillonnées par des critères analytiques globaux et une liste conservative d’intersections admissibles. Les 18 tests CAD ciblés de profils passent après corrections (9 DXF1, 9 DXF2). Les contrôles de normales ponctuels restent complémentaires.

Deux tests app vérifient le contrat `DocumentStep`, l’absence de mutation du STEP après section valide/vide/refusée, et une source STL en pouces : cube de 1 unité converti en profil de 25,4 mm, avec unités DXF mm. Quatre tests bureau couvrent époques indépendantes du profil de face, plan/corps/résultat périmés, ordre des messages, source conflictuelle et limites des décalages ; la suite bureau ciblée passe 23 tests.

Le corpus natif optionnel exporte sept sections de cinq sources : cube (`box-xy`, quadrilatère 1 mm ; `box-oblique`, hexagone), bossage/trou borgne, petits arcs, deux instances 3MF, tore en anneau et tore en deux composantes disjointes. Le même lecteur indépendant que DXF1 vérifie les coordonnées et types, rayons/balayages, fermeture des contours importés, identités/calques de corps et composantes, unités et dimensions de référence. **Sept DXF relus et audités : aucune erreur ni réparation.** L’extension du vérificateur est également rejouée sur les 39 profils DXF1, toujours réussis.

Preuves : [manifestes et profils](../artifacts/dxf2/native-corpus/manifest.json), [relecture indépendante](../artifacts/dxf2/native-corpus/independent-validation.json), [journal du corpus](../artifacts/dxf2/native-corpus.log), [journal du lecteur](../artifacts/dxf2/independent-reader.log), [régression lecteur DXF1](../artifacts/dxf2/dxf1-reader-regression.log), [tests bureau](../artifacts/dxf2/desktop-tests.log). Reproduction :

```sh
FABSTEP_SECTION_CORPUS_DIR=/chemin/absolu/corpus \
cargo test -p fabstep-app --test section_profile --offline emit_native_section_corpus -- --ignored --nocapture --test-threads=1
python scripts/validate-dxf-profiles.py /chemin/absolu/corpus
```

Le Python de relecture doit disposer d’ezdxf ; aucune dépendance n’est ajoutée à l’application. Les STEP/sections ne sont pas réimportés dans Fusion pendant ce lot. Les limites restent celles de [l’ADR-0025](adr/0025-step-section-dxf.md) : les niveaux critiques des surfaces support peuvent être refusés même hors de leur partie tronquée, les plans passant par le sommet support d’un cône peuvent exclure des coupes axiales régulières de troncs de cône, et certaines coutures/tangences/coupes de tore restent refusées. Les ellipses/splines ne sont jamais discrétisées silencieusement. Les plafonds extrêmes de topologie et le budget interne de classement sont contrôlés par lecture, sans prétendre à une saturation native de chaque branche ; les budgets scalaires sont exercés dans core/UI.

## V-1173 — DXF2a : contrôle global, bundle et tentative native

10 novembre 2026. **PASS automatisé ; parcours graphique et Fusion non qualifiés.** `sh scripts/check.sh` termine avec code 0 : **347 tests workspace, 23 bureau, 130 UI, 12 Python packaging et 4 SDK**. Formatage workspace/bureau, Clippy strict, vérification Svelte/TypeScript, build UI et vérification bureau passent ; Svelte signale 0 erreur/avertissement. Les corpus ignorés sont exécutés séparément en V-1172, sans les compter comme exécutés par la suite ordinaire. [Journal global](../artifacts/dxf2/check.log).

Les 33 nouveaux tests UI couvrent la configuration du plan, les corps choisis, les réponses tardives, la fermeture de l’outil, les échecs, la sauvegarde, les géométries/contours partagés et le plan 3D. Un test garde un appel natif long en attente pendant huit éditions : seuls le calcul initial et la dernière demande sont exécutés. La commande légère d’invalidation reste immédiate ; les calculs lourds ont une file limitée à un actif et une dernière demande. Le worker réel est exercé ; les tests ne montent pas les composants Svelte. Le refactoring du rendu partagé et du worker conserve les 97 tests UI précédents. [UI](../artifacts/dxf2/ui-tests.log), [Svelte](../artifacts/dxf2/ui-check.log), [build UI](../artifacts/dxf2/ui-build.log). Revue indépendante finale **ACCEPT**, après corrections du parcours DXF1, de la preuve de transversalité et de la file de calcul ; le principal vérifie également le diff et les réimports.

`CARGO_NET_OFFLINE=true sh scripts/build-macos-dev.sh` termine avec code 0. Le bundle **`artifacts/FabSTEP.app`** contient le binaire bureau produit, vérifié identique par SHA-256 ; [build](../artifacts/dxf2/build-macos.log), [empreintes](../artifacts/dxf2/bundle-sha256.txt). Le paquet autonome 1.0.0 n’est pas modifié. L’avertissement Vite existant sur la taille du bundle principal est conservé, sans prétendre à une optimisation de démarrage.

L’accès CUA natif est redevenu disponible pendant ce lot, mais pas l’hôte navigateur. Un harnais temporaire de composant est abandonné après refus d’écoute locale (`EPERM`, port 5192) et ses trois fichiers sont retirés. L’essai se poursuit sur le vrai bundle DXF2 : accueil FR sombre observé, bouton de coupe présent mais désactivé sans résultat ; mode **Conserver les facettes** choisi pour la fixture cube. Le dialogue sélectionne `fixtures/cube.stl`, mais **Ouvrir reste désactivé** dans l’état d’accessibilité et l’image observée. Aucune conversion, aucun aperçu de coupe et aucun export DXF graphique ne sont donc réalisés.

Après arrêt de ce seul essai d’import, Cancel puis lecture de l’état expirent (`-10005`). Une unique lecture ultérieure de nettoyage expire également : fermeture du dialogue et restauration du mode initial non confirmées. **Dernier mode observé : Plans ; mode initial : Surfaces**, tolérance radiale initiale 0,00002 mm, FR/sombre inchangés. Aucun fichier `ui-section-cube.dxf` n’est créé. Le blocage de pilotage n’établit pas un défaut de l’import produit, précédemment confirmé fonctionnel par l’utilisateur. [Faits observés](../artifacts/dxf2/ui-native-attempt.json). Aucun succès graphique ou Fusion n’est revendiqué ; la relecture native/API et les DXF indépendants de V-1172 restent les preuves automatisées.

Git : branche `codex/v1-mechanical`, modifications locales antérieures conservées ; aucun commit, push ou publication. Snapshots dans `artifacts/dxf2/before`, états Git avant/après conservés, `git diff --check` réussi. Les ajouts concernent la section STEP, les tests/lecteur partagés et leur documentation. Restent la qualification manuelle, puis DXF2b sur le maillage source ; le choix d’une famille géométrique pour corriger un groupe de facettes reste explicitement planifié dans I2, sans être présenté comme implémenté.


## V-1174 — déplacement et rotation du plan de coupe (10 novembre 2026)

Le bureau accepte un repère personnalisé scalaire (`custom_frame`) en plus des plans existants. Deux tests ajoutés vérifient une orientation oblique et son décalage signé, puis les refus de repères manquants, ambigus, indirects, non unitaires, non finis ou hors limites avant/après décalage. Les six tests ciblés du module section passent ; aucun changement au calcul natif d’intersection ni au format DXF. [Tests ciblés](../artifacts/dxf2-manipulation/desktop-tests.log).

L’interface ajoute la translation normale et les deux anneaux de rotation locaux, avec pivot près de la pièce, décalage précis conservé et remise au plan XY central. Le plan reste visible pendant le calcul ; la manipulation invalide immédiatement l’export, puis relance uniquement au relâchement. Échap, annulation de pointeur et perte de focus restaurent le plan capturé. Les tests couvrent les repères, la copie des données, la file bornée et l’API réelle de TransformControls installée ; ils ne constituent pas un essai graphique des composants Svelte.

La qualification manuelle des poignées, des thèmes/langues et de l’export après geste reste à faire. Aucun redémarrage forcé de la session utilisateur ni nouvel essai du dialogue d’import n’est effectué. Les limites de courbes et refus conservateurs de V-1172 restent applicables aux plans inclinés. Les déplacements peuvent recaler l’origine locale DXF sur le pivot, sans changer l’échelle.


Validation finale : `sh scripts/check.sh` termine avec code 0 — **347 tests workspace, 25 bureau, 141 UI, 12 Python et 4 SDK**. Formatage, Clippy strict, Svelte (0 erreur/avertissement), build UI et contrôles bureau passent. [Journal global](../artifacts/dxf2-manipulation/check.log). La revue indépendante est **ACCEPT**, après correction de la détection de poignée sur un clic sans survol préalable : le hit du pointeur courant est établi avant OrbitControls, avec test utilisant le véritable contrôleur Three.js. La capture perdue et les pointeurs secondaires sont protégés.

`CARGO_NET_OFFLINE=true sh scripts/build-macos-dev.sh` réussit après cette correction ; `artifacts/FabSTEP.app` contient le binaire bureau attendu, empreintes SHA-256 identiques (`ba250024163ce9114c0180a42c1eeb0c56d2015d91d0ac9d5ece85adcec2eaa8`). [Build](../artifacts/dxf2-manipulation/build-macos.log), [empreintes](../artifacts/dxf2-manipulation/bundle-sha256.txt). Avertissement Vite de taille du bundle conservé ; aucun gain de performance mesuré ou revendiqué. Le paquet autonome de livraison reste inchangé.

État Git : modifications préexistantes conservées, snapshots avant intervention dans `artifacts/dxf2-manipulation/before`, aucun commit ni push. Diff sans erreur d’espacement. Le périmètre supplémentaire porte sur le repère de section côté bureau, l’interaction UI, ses tests et la documentation.


## V-1175 — ellipses exactes et couverture des sections (12 novembre 2026)

Le retour utilisateur concerne `sock_clip.stl` : une faible rotation du plan horizontal provoque `dxf.unsupported_curve`. La matrice diagnostique sur cylindre confirme les refus elliptiques de 1° à 89° ; un STEP sock-clip conservé de l’audit reproduit les quatre refus à 1°/5° autour de X/Y. Les contrôles des coutures refusent aussi certaines sections centrales régulières, alors qu’un décalage de 0,001 mm les admet. [Diagnostic initial](../artifacts/dxf2-diagnosis/native-before.txt), [sock avant ellipses](../artifacts/dxf2-diagnosis/pre-ellipse.txt), [preuve d’identité des coutures](../artifacts/dxf2-diagnosis/seam-identity.txt). Le diagnostic sur STEP conservé et la reconversion fraîche ci-dessous sont des preuves distinctes ; aucune équivalence binaire entre ces reconstructions n’est revendiquée.

L’utilisateur valide l’extension aux ellipses exactes. Le DTO pur, l’ABI scalaire et les deux lecteurs de profils, le writer DXF et l’aperçu worker sont étendus. Aucune dépendance de production ajoutée ; aucune approximation polygonale. Les garde-fous de source, résultat, plan et sauvegarde restent inchangés. L’exclusion par bornes natives conservatrices évite les refus des supports hors coupe ; les coutures sont reconnues uniquement par identité topologique et appartenance à une face distincte. Les vrais contacts, raccords entre faces et courbes non prises en charge restent refusés, sans export partiel.

Tests ciblés : 27 tests core, 20 tests CAD de profils, 3 tests applicatifs de sections et 151 tests UI passent. Les cas couvrent ellipses complètes/partielles, parcours horaires/antihoraires, repères retournés, trous elliptiques d’une face plane, section conique elliptique, refus d’hyperbole après un premier corps valide, coutures et pôles sphériques, bornes, domaines invalides et budgets. Douze DXF elliptiques produits par le core sont audités indépendamment par ezdxf 1.4.4, sans erreur/réparation ; sept références historiques ligne/arc/cercle restent identiques octet pour octet. [Core indépendant](../artifacts/dxf2-coverage/core-ellipse-ezdxf-verification.json). La fixture native annulaire elliptique est conservée dans `crates/fabstep-cad/tests/fixtures/elliptical_ring.step`.

Qualification du fichier utilisateur : lecture du STL original via `FABSTEP_SECTION_PRIVATE_STL`, reconversion en Surfaces, unité mm, tolérance radiale **0,000012 mm**, budget d’écart 0,05 mm, autres réglages par défaut. Ce réglage est explicite pour le corpus, sans changement des préférences utilisateur ni des valeurs par défaut. Le test opt-in `emit_private_inclined_section_corpus` exporte les plans passant par le centre des bornes, inclinés de 0°, 1°, 5°, 15°, 30°, 60° et 89° autour de X puis Y : **14/14 exportés**. [Résultats et options](../artifacts/dxf2-coverage/sock-corpus/outcomes.json), [empreinte source](../artifacts/dxf2-coverage/source-sha256.txt).

Le lecteur indépendant vérifie unités, couches, centres, axes, rapports, domaines, points intérieurs et raccords de boucles : **14 DXF, 646 LINE, 50 ARC et 226 ELLIPSE**, aucune erreur ni réparation. [Relecture](../artifacts/dxf2-coverage/sock-corpus/independent-validation.json). Les 14 profils natifs passent aussi la préparation d’aperçu UI réelle : [aperçus](../artifacts/dxf2-coverage/sock-preview.json). Les deux tests de corpus ignorés ne sont pas comptés comme exécutés par la suite ordinaire ; le corpus privé est exécuté explicitement. Le fichier privé n’est pas ajouté aux fixtures suivies.

Revue indépendante **ACCEPT** après ajout d’un garde défensif de coplanarité des arêtes elliptiques natives. Le reviewer contrôle le code sans exécuter lui-même les tests ; leurs journaux proviennent des auteurs et du principal. Restent les essais d’interaction/sauvegarde dans l’application native et l’import dans Fusion. Les tests API, worker et lecteur indépendant ne les remplacent pas. Les splines, hyperboles, paraboles et intersections toriques obliques générales restent hors périmètre ; certains raccords entre faces et niveaux critiques de supports tronqués restent refusés par prudence. Aucun plan n’est déplacé automatiquement pour forcer un export.

Application reconstruite par `CARGO_NET_OFFLINE=true sh scripts/build-macos-dev.sh`, code 0 : [build](../artifacts/dxf2-coverage/build-macos.log). Le bundle `artifacts/FabSTEP.app` contient exactement le binaire bureau attendu, SHA-256 `663e76ee1d4f33fe50f3398df8f0fdf670c51a968c4ba88307e79a447c0aa382` : [empreintes](../artifacts/dxf2-coverage/bundle-sha256.txt). Le paquet autonome 1.0.0 reste inchangé. Aucun pilotage graphique ni changement des préférences utilisateur ; il faut relancer l’application pour charger le nouveau code.


Validation globale finale : **355 tests workspace, 25 bureau, 151 UI, 12 Python et 4 SDK** réussis. La première exécution de `scripts/check.sh` passe tous les tests workspace puis s’arrête sur Clippy : boucle à un seul élément dans un test CAD modifié. Le principal remplace cette boucle par l’assertion directe, sans changement de production. Ce test est réexécuté avec succès, puis tous les contrôles restants sont exécutés en séquence : formatage, Clippy strict workspace, tests/check/build UI, formatage/check/test/Clippy bureau. Svelte : 0 erreur/avertissement ; avertissement Vite existant sur la taille du bundle. [Première séquence](../artifacts/dxf2-coverage/check.log), [reprise complète des contrôles restants](../artifacts/dxf2-coverage/check-remaining.log). La suite workspace déjà réussie n’est pas répétée pour cette simplification du seul test.

Git : modifications antérieures conservées, aucun commit/push/publication. Snapshots avant intervention et états Git dans `artifacts/dxf2-coverage`. `git diff --check` réussi. Aucun gain de performance n’est revendiqué ; les bornes sont introduites pour écarter des refus géométriques injustifiés, pas comme un résultat de benchmark.


## V-1176 — sections polygonales du maillage source (12 novembre 2026)

DXF2b ajoute une coupe indépendante de la reconstruction STEP : préparation topologique puis preuve géométrique native de tous les corps et de leur séparation, stockage privé des maillages, extraction pure des intersections et export DXF de lignes. Aucune fermeture par proximité, ajustement de courbe ou omission d’un corps invalide. Les options de soudage et diagnostics d’orientation restent explicites.

Le principal construit deux variantes isolées du core, avant refactoring (fichiers restaurés depuis les snapshots) et après extraction de la préparation. Dix rapports d’analyse complets — cube, prisme concave, tube, sphéroïde et tore, chacun avec orientations normales puis inversées — sont **identiques octet pour octet**. Cette comparaison est répétée après les ajouts de points de contrôle d’annulation : [équivalence](../artifacts/dxf2b/analysis-equivalence.json). Les variantes et JSON complets sont conservés dans `artifacts/dxf2b`, hors sources suivies.

Le test applicatif `source_sections_do_not_depend_on_successful_curve_recognition` établit le refus de reconnaissance Surfaces du sphéroïde, puis prépare ce même STL et exporte sa section polygonale. Un autre test vérifie un cube STL en pouces : profil de 25,4 mm et DXF INSUNITS=4. [Qualification applicative](../artifacts/dxf2b/app-source-qualification.log). Les tests de préparation vérifient également les corps invalides, contacts/placements et la conservation des prétraitements, sans marqueur de validation native encore requise après preuve réussie.

Le corpus opt-in `emit_source_section_corpus` est exécuté séparément avec sept fichiers : cube, prisme concave, tube, sphéroïde, tore, instances 3MF et sock-clip privé. Deux plans par document, horizontal et incliné de 13°, sont définis à hauteur explicite de 37 % du premier corps ; **14/14 DXF exportés**. Ce décalage est un paramètre de corpus, pas un déplacement automatique du plan utilisateur. Le lecteur ezdxf 1.4.4 vérifie unités, entités, couches et fermeture de chaque contour, sans erreur ni réparation : [relecture indépendante](../artifacts/dxf2b/source-corpus/independent-validation.json). Les profils ne contiennent que des LINE. Les 14 passent aussi la préparation UI réelle : [aperçus](../artifacts/dxf2b/source-preview.json). Aucun fichier privé n’est ajouté aux fixtures suivies.

La revue indépendante vérifie la chaîne d’autorisation native, les époques/cache, le maintien du STEP et les files UI. Elle identifie un cas de sous-flux flottant pouvant être pris pour une distance exactement nulle ; les produits trop petits pour garantir le résidu arithmétique sont désormais refusés explicitement et couverts par régression. Les contrôles traversant sommets/arêtes, cycles, inclusions, budgets, annulation et absence d’export partiel restent nécessaires.

Les essais dans la fenêtre native, les gestes réels de coupe Source et le réimport Fusion restent à qualifier. Les tests de contrôleur, de moteur et les relectures DXF ne les remplacent pas. Aucun pilotage CUA ni changement des préférences utilisateur n’est effectué.


Le code final passe les 14 tests ciblés du nouveau noyau de section (54 tests core au total), les 4 tests de préparation applicative, les 2 tests applicatifs de qualification, les 30 tests bureau et les 170 tests UI dans leurs exécutions ciblées. Les journaux se trouvent dans `artifacts/dxf2b/`. La revue indépendante finale est **ACCEPT**, après vérification de la régression numérique ; le reviewer n’a pas exécuté lui-même les tests.

`CARGO_NET_OFFLINE=true sh scripts/build-macos-dev.sh` termine avec code 0. Le binaire de `artifacts/FabSTEP.app` est identique au binaire bureau produit, SHA-256 `8d56969b1cf381781c6bd3d519c0bfbfa5ff9a5993314d52dfd14de84c5ef0e2` : [build](../artifacts/dxf2b/build-macos.log), [empreintes](../artifacts/dxf2b/bundle-sha256.txt). Le paquet autonome 1.0.0 n’est pas modifié.


Validation globale finale : **375 tests workspace, 30 bureau, 170 UI, 12 Python et 4 SDK** réussissent. La première séquence passe tous les tests workspace puis Clippy signale une boucle indexée dans le nouveau test des pouces. Après correction locale et réexécution des deux tests applicatifs, la suite restante passe jusqu’au Clippy bureau, qui demande de nommer un type de retour imbriqué. Un alias privé est ajouté sans changement de comportement ; les 30 tests bureau et Clippy strict sont réexécutés avec succès, ainsi que le formatage bureau. [Séquence initiale](../artifacts/dxf2b/check.log), [contrôles restants](../artifacts/dxf2b/check-remaining.log), [tests bureau finaux](../artifacts/dxf2b/desktop-final-tests.log), [Clippy final](../artifacts/dxf2b/desktop-final-clippy.log). Svelte : 0 erreur/avertissement ; avertissement Vite existant sur la taille du bundle. Le panneau n’affiche pas les identifiants internes des corps ; noms, nombres et diagnostics utiles restent accessibles.

Git : modifications préexistantes préservées, aucun commit/push/publication. Snapshots et états Git avant/après conservés sous `artifacts/dxf2b`. Diff sans erreur d’espacement. Aucun gain de performance revendiqué ; la réutilisation de la préparation pendant le déplacement est vérifiée fonctionnellement, sans benchmark utilisateur.

## V-1177 — I2c : choix de famille et troncs de cône guidés (12 novembre 2026)

**Périmètre.** Choix Cylindre/Cône sur une sélection de facettes STEP à provenance exacte. Les cônes guidés de ce lot ont deux contours circulaires de rayon positif portés par des plans source ; pointes, cônes partiels et raccords directs à des surfaces courbes restent refusés. Les autres parois ne sont pas reconstruites implicitement. Contrat : [ADR-0027](adr/0027-guided-surface-families.md).

**Preuves ciblées observées.**

- Core : huit nouveaux tests d’intégration pour tronc droit/tourné, déterminisme de graines, deux familles indépendantes, deux cônes, mauvaises familles/sélections, recouvrements, budgets/tolérances, cône déformé voisin non sélectionné et équivalence du parcours cylindrique. Un test privé contrôle la propagation du budget de preuve conique. [Tests guidés](../artifacts/i2c/core-guided-surfaces.log), [suite core](../artifacts/i2c/core-tests.log), [Clippy core](../artifacts/i2c/core-clippy.log).
- CAD : deux nouveaux tests dans la suite de 77 vérifient les jetons plans/cylindres/cônes permutés, leurs supports natifs, la combinaison de racines inversées, les jetons invalides et les métadonnées corrompues sans perte du solide. L’ABI historique est conservée ; la nouvelle entrée de provenance est additive.
- App : cinq tests réussissent : tronc droit/tourné avec audit de fidélité, partition source exhaustive et retrait ; deux instances 3MF et indications indépendantes ; refus de mauvaise famille, pointe et raccord courbe ; annulation avant construction ; cône et cylindre dans un même corps, ajout dans les deux ordres, retrait de chaque indication et contrôle du volume attendu/provenance exacte. La source synthétique de ce dernier test transforme linéairement la paroi du bossage de la fixture existante, sans changer le trou indépendant. [Journal app](../artifacts/i2c/app-guided.log).
- Bureau : famille absente = cylindre historique ; famille inconnue refusée après consommation de la révision ; session conique réellement acceptée puis tentative refusée sur un plan, ancien export retiré et recalcul explicite réussi. [Famille](../artifacts/i2c/desktop-guided.log), [session](../artifacts/i2c/desktop-cone-session.log).
- UI : cinq nouveaux tests portent sur les traductions/étiquettes natives, la famille capturée au lancement, le rapport natif faisant autorité et les réponses coniques périmées après changement de source. 175 tests, Svelte sans erreur/avertissement et build réussis lors de la livraison du lot UI ; le contrôle global ci-dessous les reprend.

**Revue.** Revue indépendante du diff par rapport aux copies initiales `artifacts/i2c/before` : ACCEPT, aucun blocage restant. Le relecteur a contrôlé les preuves ciblées, le périmètre des parois, les budgets, la provenance et les gardes de session ; il n’a pas effectué d’essai graphique. Une remarque de style `clone_on_copy` après ajout de `Copy` à l’identité de face a été corrigée et Clippy core repassé.

**Contrôles globaux.** `sh scripts/check.sh` termine avec code 0 : **391 tests workspace, 32 bureau, 175 UI, 12 Python et 4 SDK** réussissent, formatage workspace/bureau, Clippy strict tous targets, Svelte sans erreur/avertissement, build UI et contrôle bureau. L’avertissement de taille du bundle UI reste celui déjà présent. [Journal global](../artifacts/i2c/check.log), [résumé](../artifacts/i2c/validation-summary.json).

**Paquet local.** `CARGO_NET_OFFLINE=true sh scripts/build-macos-dev.sh` termine avec code 0. Le binaire de `artifacts/FabSTEP.app` est identique au binaire compilé, SHA-256 `831bb9dcf5e30c294a2535fe19a8091fc8afbaf1e4894724ccf8055aa8152c68` : [build](../artifacts/i2c/build-macos.log), [empreintes](../artifacts/i2c/bundle-sha256.txt).

**Limites.** Aucun parcours graphique natif ni import Fusion de ce lot n’a été qualifié automatiquement. Le réimport STEP, les types de surfaces, volumes, dimensions et distances sont contrôlés par le pipeline natif et les tests ; cela ne constitue pas une validation indépendante dans Fusion/FreeCAD. Pas de mesure de performance avant/après : ce lot n’est pas une optimisation. Les tests de corpus privés/benchmarks marqués opt-in restent exclus de la suite ordinaire.

**Traçabilité.** État Git initial déjà largement modifié et non suivi, préservé dans `artifacts/i2c/initial-git-status.txt` ; copies des fichiers avant travail dans `before/`, inventaire des fichiers de code de ce lot dans `changed-code-files.txt`. Aucun commit ni push. Le paquet autonome 1.0.0 n’est pas modifié.


## V-1178 — UX4 : correction contextuelle et inspecteur allégé (15 novembre 2026)

Retour utilisateur et deux captures fournis : guidage introuvable, diagnostics trop présents, barre de familles Cutaway comme référence. Correction rattachée à R-042, sans changement Rust/CAD ni extension géométrique.

- Barre `GuidedCorrectionBar.svelte` dans le flux de mise en page sous les outils 3D, hors du canevas de picking : Cylindre/Cône/Essayer sur sélection à provenance exacte ; préparation explicite sinon lorsque le résultat analytique courant le permet. Aucune famille non prise en charge affichée. Famille capturée au clic, refus pendant une sélection en attente et commandes désactivées pendant le travail.
- Préparation via `change({reconstruction:'planar'})` : historique de réglages, invalidation habituelle immédiate de l’export, nouvelle conversion et nouvelle sélection. Le texte annonce le remplacement des formes automatiques ; aucune préservation implicite de la correction locale n’est promise.
- Liste explicite des codes de routine dans `diagnostics.ts`. Les changements géométriques, codes inconnus, avertissements de gravité accrue et erreurs restent visibles. Les diagnostics complets restent dans Détails techniques.
- Révélation opt-in du panneau par identité canonique de sélection ; état partagé conservé lors des démontages. Le repli manuel tient jusqu’à une nouvelle sélection ou un effacement, sans masquer la barre de correction.

**Validation observée** : 181/181 tests UI (six de plus que la base 175), Svelte 0 erreur/0 avertissement, build UI et bundle macOS réussis, `cargo check --manifest-path crates/fabstep-desktop/Cargo.toml --offline` réussi et `git diff --check` propre. [Tests](../artifacts/ux4-clarity/ui-test-final.log), [Svelte](../artifacts/ux4-clarity/ui-check-final.log), [build](../artifacts/ux4-clarity/build-macos.log), [bureau](../artifacts/ux4-clarity/desktop-check.log). Les suites Rust complètes ne sont pas réexécutées pour cette modification exclusivement UI ; leur dernière exécution demeure celle du 13 novembre, distincte de ces contrôles.

Revue indépendante du diff par rapport au snapshot initial : aucun défaut bloquant ; `role="group"` ajouté à la barre nommée. Les tests du contrôleur couvrent la préparation et l’invalidation d’export ; ils ne sont pas une qualification graphique montée.

**Vérifications visuelles impossibles ici** : Vite refuse l’écoute locale avec `EPERM 127.0.0.1:5181` ([journal](../artifacts/ux4-clarity/vite.log)). Une page autonome embarquant le vrai composant et les jetons a été compilée dans `artifacts/ux4-clarity/visual/index.html`, mais l’ouverture par le pilote Chrome échoue avec « The browser access host is unavailable ». Aucun rendu clair/sombre à 1100/1440 px ni parcours natif n’est donc revendiqué. Les actions utilisateur restent dans FabBoard.

Le paquet `artifacts/FabSTEP.app` et le binaire compilé ont la même empreinte SHA-256 `2ad1672a4d2d540be1c48a84809470749101b5961be2d93de35d3e691bff10a9` ([empreintes](../artifacts/ux4-clarity/bundle-sha256.txt)). État Git préexistant préservé, pas de commit ni push ; copies avant modification dans `artifacts/ux4-clarity/before/`.


## V-1179 — Diagnostic de body 26mm.stl (15 novembre 2026)

L’utilisateur confirme que `cad.invalid_model` apparaît à l’ouverture, pendant la conversion automatique. Reproduction CLI sur le fichier privé de 18 248 triangles, SHA-256 `fd51d3360addefa5da284a2493685290717eb4c641bed5b5e731bd8ce139472d`, unité explicitement choisie : mm. Source inchangée, aucun changement de code ni de seuil natif.

À la planéité initiale de 1e-6 mm, les modes planar, primitives et surfaces échouent sur le contrôle natif de planéité. Le regroupement source tolère un écart supérieur à celui accepté pour le contour natif ; le refus protège la validité du résultat. La tolérance d’ajustement participe aussi à la segmentation des modes analytiques : baisser uniquement la planéité ne constitue pas une solution universelle.

**Solution vérifiée sur ce fichier :** mode « Conserver les facettes », planéité 1e-7 mm, fusion 1e-6 mm, autres réglages inchangés. Conversion en 36,967 s dans cette exécution locale : réimport natif validé, un solide, 13 397 faces planes, volume 17 053,71559422933 mm³, dimensions 90 × 50 × 11 mm conservées à la précision numérique. La source mesure 17 053,715580561457 mm³. Le rapport conserve 512 résidus de planéité non nuls ; aucune surface cylindrique n’est revendiquée. Pas d’audit de fidélité échantillonné dans ce parcours planaire.

Les modes primitives et surfaces avec planéité **et** ajustement à 1e-7 mm échouent respectivement avec `recognition.unsupported` et `recognition.surface_unproven`. Aucun résultat partiel n’est proposé. Cette investigation fournit un réglage explicite, pas une correction générale du moteur ni une adaptation automatique.

Preuves locales : [matrice](../artifacts/body26/matrix.json), [rapport réussi](../artifacts/body26/planar-0.0000001.json), [STEP facettisé](../artifacts/body26/planar-0.0000001.step). Les durées sont des observations isolées, pas un benchmark. Compilation CLI hors ligne réussie ; suites globales non relancées, aucun code modifié. Ouverture de ce résultat dans Fusion et reproduction graphique non vérifiées. État Git préexistant conservé ; aucun commit ni push.


## V-1180 — I2d : famille Plan guidée (15 novembre 2026)

Périmètre : R-056, ADR-0028. Ajout Plan au guidage, contours polygonaux seulement, coordonnées source conservées et résidu géométrique borné. Tore reste à développer dans R-050.

Preuves ciblées : six tests core couvrent fusion réelle, déterminisme, conservation des sommets et trous, refus non-plan/disjoint/incomplet/recouvrant, budget et frontière native, coexistence avec cylindre et cône sans reconstruction implicite. Les huit tests guided_surfaces historiques passent. Trois tests applicatifs vérifient réimport natif d’une fusion effective (prisme mince à deux pans presque plans), provenance exhaustive, audit, recalcul/retrait, refus, annulation et coexistence Plan/Cylindre dans les deux ordres avec retrait indépendant. Les tests UI couvrent libellé natif, transmission Plane et réponses périmées ; le parseur bureau accepte plane et conserve le défaut historique cylinder.

Le premier essai applicatif a révélé que le constructeur mixte refuse les modèles entièrement plans : la correction route ces seuls modèles vers le constructeur planaire suivi existant, avec audit et contrôles après construction inchangés. Un échec ultérieur du test de coexistence provenait du montage témoin (mode automatique requis non activé), corrigé sans modification du moteur. Trois tests applicatifs passent ensuite.

Revue indépendante : ACCEPT après correction du diagnostic de résidu, qui compte désormais les faces finales réellement concernées plutôt que les indications sélectionnées. Les traductions décrivent la simplification possible d’un faible relief ; ce warning reste visible. [Core](../artifacts/guided-plane/core-plane-tests.log), [Clippy core](../artifacts/guided-plane/core-clippy.log), [app](../artifacts/guided-plane/app-planes.log).

**Contrôles globaux :** `sh scripts/check.sh` termine avec code 0 : **400 tests workspace, 32 bureau, 184 UI, 12 Python et 4 SDK** réussissent. Formatage workspace/bureau, Clippy strict tous targets, Svelte (0 erreur/0 avertissement), build UI et contrôle bureau passent. Les tests opt-in restent ignorés ; avertissement de taille du bundle Vite existant. [Journal global](../artifacts/guided-plane/check.log). `CARGO_NET_OFFLINE=true sh scripts/build-macos-dev.sh` termine avec code 0 ; le binaire de `artifacts/FabSTEP.app` correspond au binaire compilé (SHA-256 `bb0c0eae6513f233dc054e94301f8e51c1a3e4867b400981cc5e5ffe4035d2c7`). [Build](../artifacts/guided-plane/build-macos.log), [empreintes](../artifacts/guided-plane/bundle-sha256.txt).

Limites de validation graphique : le pilote renvoie « Sky Computer Use native pipe startup failed » et « The browser access host is unavailable ». Aucun parcours graphique clair/sombre ni réimport Fusion revendiqué. Aucun changement des seuils OCCT, aucune dépendance ajoutée, aucun gain de performance revendiqué. Modifications préexistantes préservées ; snapshots des fichiers modifiés et état initial dans `artifacts/guided-plane/before` et `initial-git-status.txt`. Aucun commit/push.


## V-1181 — UX5 : reconstruction automatique, parois et Tore (16 novembre 2026)

Périmètre approuvé en une livraison : R-057, ADR-0029. Auto distingue refus géométrique local et erreur bloquante ; provenance native Rounded exhaustive ; sélection géométrique en lecture seule et correction préservant les régions indépendantes. Les stratégies historiques restent explicites dans Avancé ; défaut API/CLI Planar conservé.

**Tests ciblés observés.** Huit tests core Auto couvrent cylindres, cônes, raccord et tore entier, sphère entière, groupe torique sans plans, candidat déformé conservé en facettes, région plane subdivisée sans déplacement, refus de guidage torique exigeant un voisin conique, et arcs indépendants maintenus après rejet d’un autre arc. Suites rounded/partial/partial_staggered/short_arcs et Clippy core passent ; anciens tests guidés passent. Deux tests CAD couvrent jetons permutés, erreurs de jetons et provenance réelle de cinq modèles avec faces sphériques/toriques/cylindriques partielles.

Cinq tests applicatifs passent : provenance exhaustive du solide réimporté, correction Cylindre conservant l’autre cylindre automatique et retrait, sélection géométrique sans bouchon ni mutation d’export et annulation, Tore en sessions Plans/Auto avec retrait/recalcul, deux corps 3MF avec placements/provenance et refus des contacts/source ouverte. Le backend lie la lecture suggest_region à révision/source/époque/identité du résultat ; l’UI borne les calculs à un actif et une dernière attente. 197 tests UI ciblés passent avec Svelte 0 erreur/0 avertissement et build réussi.

**Revue indépendante : ACCEPT**, y compris le filtrage des arcs partiels avant mutation et la provenance native suivie après couture/transfert STEP. La limite des contours couplés ambigus demeure : une passe partielle peut rester facettisée dans son ensemble, sans acceptation d’un solide invalide. Le constructeur dédié des boîtiers arrondis reste dans le mode Surfaces avancé.

**Fichier privé.** Premier essai Auto de `body 26mm.stl` avec tolérances initiales : succès, 1 solide, 6 cylindres, 3 cônes, 11 965 plans (11 974 faces), volume 17 053,67731144076 mm³, dimensions 90 × 50 × 11 mm. Réimport natif validé, audit de fidélité exécuté. Durée isolée 89,225 s en debug ; aucune accélération revendiquée par rapport au parcours planaire antérieur, qui n’exécutait pas le même audit. Le fichier source reste privé et inchangé. Le chemin historique de sock_clip.stl n’existe plus ; recherche ciblée dans Downloads sans résultat, aucun essai réussi de ce fichier revendiqué.

**Contrôles globaux :** `sh scripts/check.sh` termine avec code 0 : **415 tests workspace, 32 bureau, 197 UI, 12 Python et 4 SDK** réussissent, ainsi que formatage, Clippy strict, Svelte (0 erreur/0 avertissement), build UI et contrôle bureau. Les tests opt-in restent ignorés ; avertissement Vite de taille du bundle préexistant. [Journal global](../artifacts/auto-workflow/check.log), [résumé](../artifacts/auto-workflow/validation-summary.json). Build macOS réussi ; le binaire de `artifacts/FabSTEP.app` correspond au binaire compilé (SHA-256 `b7abd43037c712612ee8093167e5f4094b266721737e7dd347f396ca6ab32c2a`). [Build](../artifacts/auto-workflow/build-macos.log), [empreintes](../artifacts/auto-workflow/bundle-sha256.txt). Répétition finale de body 26mm réussie en 75,336 s avec le binaire final : mêmes 6 cylindres, 3 cônes, 11 965 plans, volume et bornes que le premier essai. Audit échantillonné : source→résultat 0,00246669797302896 mm, résultat→source 0,002508739181881575 mm, déflexion 0,005 mm ; ce n’est pas une distance de Hausdorff certifiée. Empreinte source inchangée `fd51d3360addefa5da284a2493685290717eb4c641bed5b5e731bd8ce139472d`. [Rapport final](../artifacts/auto-workflow/body26-final.log), [STEP final](../artifacts/auto-workflow/body26-final.step), [durée](../artifacts/auto-workflow/body26-final-run.json). Les durées isolées ne constituent pas un benchmark. Aucun contrôle automatisé en échec à la clôture ; `git diff --check` et FabBoard lint passent.

**Limites graphiques.** La tentative de lecture des surfaces du pilote est interrompue avant exécution (« aborted before dispatch ») ; aucun parcours graphique de ce lot ni ouverture Fusion revendiqué. Les vérifications utilisateur restent distinctes des tests automatisés. Snapshots initiaux dans `artifacts/auto-workflow/before` (copies core originales aussi dans `artifacts/automatic-workflow/before`), logs et STEP privés dans `artifacts/auto-workflow`. Aucun commit/push ni modification du paquet autonome 1.0.0.


## V-1182 — UX5 : sélection de faces par balayage

Retour utilisateur : le double-clic sur une zone cylindrique facettisée n’étend pas la sélection. Le parcours géométrique actuel ne propose que les régions prouvées ; le geste manuel demandé est ajouté indépendamment du reconnaisseur. Aucun changement Rust/CAD ni de la conversion.

En mode sélection STEP, cliquer-glisser à gauche ajoute les faces visibles, une seule fois par balayage ; le clic immobile garde son comportement, Maj-clic permet de retirer une face. Capture avant OrbitControls, déplacement minimal 4 px, interpolation écran avec au plus 64 raycasts par trame. Les gestes droite/milieu/molette restent respectivement rotation/déplacement/zoom dans ce mode. Perte de capture, annulation, Échap, perte de focus ou changement du document/vue arrêtent le geste et rétablissent les contrôles. Les faces déjà ajoutées restent sélectionnées lors de l’arrêt.

La file borne les mises à jour à une commande worker active et une union de faces en attente. Les accusés périmés libèrent la file sans publier d’ancienne sélection ; effacement/changement de contexte retire l’attente. L’ajout groupé valide tous les indices avant mutation et reste idempotent. Le double-clic revenant avec une seule face affiche une aide neutre : aucune face supplémentaire sélectionnée, possibilité de cliquer-glisser. Il ne prétend pas que le moteur reconnaît davantage de cylindres.

**Validation observée : 204 tests UI réussis**, Svelte 0 erreur/0 avertissement et build UI réussi. Sept tests supplémentaires couvrent seuil, interpolation/déduplication, limite de raycasts, annulation/identité du pointeur, worker lent et cumul, effacement/réponse tardive, erreur de dispatch et reprise, ajout groupé atomique. Revue par l’orchestrateur du code réellement modifié et de son raccordement aux événements/worker ; pas de revue indépendante supplémentaire pour ce changement UI borné. [Tests](../artifacts/paint-selection/ui-test.log), [Svelte](../artifacts/paint-selection/ui-check.log), [build UI](../artifacts/paint-selection/ui-build.log). Les suites Rust complètes ne sont pas réexécutées pour cette modification exclusivement UI ; leur dernière preuve reste V-1181.

**Limite visuelle :** le pilote retourne « Sky Computer Use native pipe startup failed » et « The browser access host is unavailable » ; aucun geste monté dans le bureau ni rendu clair/sombre n’est déclaré validé. L’interpolation est bornée et ne garantit pas de toucher chaque face subpixel lors d’un mouvement très rapide. Qualification utilisateur ajoutée au suivi R-057. Build macOS réussi ; binaire du bundle identique au build (SHA-256 `fe6db1385f27bdb7894ebf1cf8a8a2cfc82505440c989129247445b683fc5ac0`). [Journal](../artifacts/paint-selection/build-macos.log), [empreintes](../artifacts/paint-selection/bundle-sha256.txt). État Git préexistant conservé, snapshots/logs sous `artifacts/paint-selection`, aucun commit/push.


## V-1183 — Refus guidé et raccords ouverts cylindre–chanfrein (16–17 novembre 2026)

R-057/R-058, décision ADR-0030. Reproduction privée initiale : body 26mm.stl, SHA-256 `fd51d3360addefa5da284a2493685290717eb4c641bed5b5e731bd8ce139472d`, 18 248 triangles, millimètres. Quatre parois supérieures analogues de 144 triangles sont identifiées par centre/rayon/niveaux ; les identifiants exacts du geste dans la capture ne sont pas disponibles. Leurs rayons sont 2,5 mm et leurs centres XY (-72,-25), (-53,-25), (-25,-25), (-5,-25). Leur bord inférieur rejoint un chanfrein conique partiel et une bande cylindrique de rayon 1,25 mm. Le parcours précédent cherchait deux arcs portés par des plans et refusait ces sélections avec `guidance.family_unproven`. La conversion automatique initiale restait native valide, 6 cylindres et 3 cônes. Preuve : `artifacts/cylinder-refusal/private-native-regression.log`, témoins géométriques et indices dans `wall-evidence.json`.

Le nouveau modèle partage les arcs entre cylindres et cônes partiels ; la preuve vérifie les bandes source, leur couverture, les génératrices, les contours complets des supports et les budgets. Aucun sommet projeté ni tolérance augmentée. Le contrôle des supports a été renforcé après revue afin qu’une frontière courbe ne masque pas un contour non plan. Les rayons égaux à l’arrondi numérique sont canonisés seulement après vérification des extrémités et des résidus source.

Tests ciblés : quatre tests core couvrent la chaîne droite/transformée, le groupe dépendant et l’ordre des sélections, le refus d’une bande déformée et d’un support non plan ; trois tests CAD couvrent provenance exacte, volume/types après réimport, rayons croissants/décroissants, rotation et encoche concave, ainsi que frontières/jetons invalides. Un test app indépendant vérifie la demi-pièce de volume exact 25π/6, la partition source complète, le groupe de trois faces, les indications Cylindre et Cône, le recalcul et le retrait. **208 tests UI** passent ; Svelte 0 erreur/0 avertissement et build UI réussis. Le titre de refus guidé et le bouton de reprise sont liés à la révision courante ; aucun ancien résultat n’est réexporté. Le DTO et l’inspection distinguent les cônes partiels, avec compatibilité des rapports sans le champ additif.

Journaux ciblés : `artifacts/cylinder-refusal/core-chain-test.log`, `core-chain-clippy.log`, `artifacts/partial-cone-cad/tests.log`, `artifacts/cylinder-refusal-integration/app-chain.log`, `ui-test.log`, `ui-check.log`, `ui-build.log`. La suite globale et ses compléments sur les derniers ajustements sont consignés ci-dessous.

**Essai privé final réussi** : Auto puis indication Cylindre sur la paroi centrée en (-25,-25) produisent chacun un solide, 10 869 faces (10 848 planes, 14 cylindriques, 7 coniques), volume 17 053,652458875804 mm³ et dimensions 90 × 50 × 11 mm à l’arrondi numérique. Les 144 triangles indiqués étendent la correction aux 432 triangles de la chaîne ; correspondance STEP/source exacte. Les quatre parois analogues sont vérifiées. Audit échantillonné source→résultat 0,0026187957263904826 mm et résultat→source 0,0030518852180759504 mm, déflexion 0,005 mm ; pas de distance de Hausdorff certifiée. Le test de deux conversions termine en 147,19 s, durée isolée sans revendication de performance. [Log](../artifacts/cylinder-refusal/private-native-chain.log), [STEP automatique](../artifacts/cylinder-refusal/body26-chain-auto.step), [STEP guidé](../artifacts/cylinder-refusal/body26-chain-guided.step), rapports JSON associés. Une première exécution avait réussi les deux conversions puis échoué uniquement sur l’écriture des artefacts avec un chemin relatif ; le rejeu avec chemin absolu passe.

Les dernières gardes de chaînes incompatibles et l’indépendance à l’ordre des sélections ont été contrôlées séparément après compilation de la suite globale : tests core finaux, Clippy core et test app du groupe complet réussis (`core-chain-test-final.log`, `core-chain-clippy-final.log`, `artifacts/cylinder-refusal-integration/app-chain-final.log`). Revue indépendante ACCEPT après ces derniers ajustements.

Build macOS réussi, binaire du bundle identique au build : SHA-256 `cb2a166d40451369cb37700e2250160900a5feb2080cf44836cbdae953736183`. Journaux `artifacts/cylinder-refusal-integration/build-macos.log` et `bundle-sha256.txt`. Limite graphique confirmée : « Sky Computer Use native pipe startup failed » et « The browser access host is unavailable ». Aucun parcours graphique ni réimport Fusion déclaré validé ; A-040/A-041 portent ces vérifications. Aucun nouveau paquet autonome, aucune dépendance ajoutée, aucun commit/push.

**Bilan global final :** 423 tests workspace, 32 bureau, 208 UI, 12 Python et 4 SDK réussissent ; 12 tests opt-in restent ignorés dans le passage global, dont le test privé exécuté séparément avec succès. Formatage final, doctests, Clippy strict workspace/bureau, Svelte et builds passent. Le passage initial `scripts/check.sh` a terminé tous les tests exécutables puis s’est arrêté sur `E0463` dans les doctests, après recompilation concurrente de dépendances pendant les compléments ciblés. Rejeu séquentiel de `cargo test --workspace --doc --offline --locked`, puis de toutes les étapes restantes : code 0. Ce n’est donc pas un passage unique du script annoncé vert. [Passage initial](../artifacts/cylinder-refusal-integration/check.log), [fin séquentielle](../artifacts/cylinder-refusal-integration/check-final.log), [résumé](../artifacts/cylinder-refusal-integration/validation-summary.json). Aucun échec non résolu. Avertissement de taille Vite préexistant ; `git diff --check` et FabBoard lint passent. État Git initial conservé, travaux non commités/non poussés.

## V-1184 — C1b1 et inspecteur contextuel — 17 novembre 2026

R-032 admet deux à huit cônes de signes libres séparés par des portées cylindriques résolues. Chaque plage du domaine étendu exige des témoins intérieurs issus de la source ; les paires historiques gardent leur contrat et évitent le travail supplémentaire. Aucun changement de DTO, ABI, dépendance ou tolérance. Les cônes sans portée restent hors domaine (R-059). Décision : ADR-0031.

Le corpus reproductible `fixtures/same_sign_chains` comporte des chaînes croissantes, décroissantes et mixtes, une rotation, un STL binaire et deux instances 3MF. Les valeurs attendues sont calculées depuis les profils nominaux par le générateur indépendant. Les tests app vérifient le réimport, volume, dimensions, familles, provenance exacte et refus des cas sans portée, courbes ou avec portée ambiguë ; Auto conserve alors les facettes. Les tests core couvrent aussi témoins manquants, budgets et compatibilité historique. Revue indépendante acceptée après correction du court-circuit des paires historiques.

R-042 simplifie cinq composants de l’interface : informations techniques et familles repliées, voisinage source sous Avancé, effacement accessible en en-tête, Retour au STEP prioritaire, Essayer en action primaire et historique visible seulement avec indications acceptées. Les unités, alertes, refus, récupération, limite de 16 indications, retrait et exports DXF sont conservés. Le diff contre les snapshots de début de lot a été relu par l’orchestrateur.

Limites : pilotage graphique indisponible (native pipe et browser host), donc aucun essai visuel nouveau ni réimport manuel Fusion revendiqué. Les tests automatisés ne remplacent pas les contrôles FR/EN, clair/sombre et petits écrans. Il ne s’agit pas d’une optimisation mesurée ; aucun gain de performance annoncé.

Contrôles exécutés : `cargo test --workspace --offline --locked` termine avec **433 tests réussis, 0 échec, 12 opt-in ignorés**, doctests inclus. L’analyse statique initiale a relevé `manual_div_ceil` dans un nouveau test ; utilisation de `div_ceil(2)`, rejeu des 8 tests `conical_chains` et Clippy workspace strict réussis. Aucun code de production modifié après la suite globale. Formatage workspace réussi. UI : **208 tests**, Svelte **0 erreur/0 avertissement**, build réussi avec l’avertissement Vite connu >500 kB. Scripts : **12 tests Python et 4 tests SDK** réussis.

Journaux locaux : `artifacts/c1b-integration/workspace-tests.log`, `workspace-clippy.log` (premier refus), `workspace-clippy-final.log`, `core-final.log`, `python-tests.log`, `sdk-tests.log`, et `artifacts/ux-context-cleanup/{tests,check,build}.log`. Les snapshots de début de lot permettent de distinguer ces changements des nombreuses modifications préexistantes ; aucun historique Git réécrit ni publication distante.

Bureau : formatage, `cargo check`, **32 tests** et Clippy strict réussis, suivis séquentiellement du build macOS hors ligne. Paquet mis à jour : `artifacts/FabSTEP.app`, empreinte SHA-256 de l’exécutable identique au binaire construit : `1be8b5f4a64ee012037d7c8c55ad37adc4ca3d05e22dc1fc945d5b83f36c5774`. Journal `artifacts/c1b-integration/desktop-build.log`. Version release inchangée. Essais utilisateur A-042 (corpus), A-043 (Fusion) et A-044 (interface) ouverts ; R-032 et R-042 à tester, sans prétendre les avoir livrés.

## V-1185 — Contrôle natif de l’atelier et consigne STEP — 17 novembre 2026

Le pilotage natif est redevenu accessible après ouverture des permissions de session. Sur le build V-1184, ouverture réelle de `fixtures/same_sign_chains/two_increasing.stl` en mm/Auto : résultat validé et bascule automatique vers le STEP observés. Capture française sombre : faces colorées et contours visibles, Avancé replié. Sélection d’une face plane : barre Auto/Plan/Cylindre/Cône/Tore/Essayer et profil natif 32 × 32 mm, un trou. Aller-retour Voir la zone source → Retour au STEP effectué. Capture anglaise claire : barre Try visible, inspecteur compact ; retour FR/sombre effectué ensuite.

Le pilote a produit des erreurs intermittentes ScreenCaptureKit (-3811/-3812), mais les arbres d’accessibilité suivants ont confirmé les actions effectuées ; aucune conversion impossible observée sur ce parcours. Ni Essayer, ni sauvegarde STEP/DXF, ni Fusion, ni fenêtre étroite, ni balayage/double-clic ne sont qualifiés par cette session. Les actions A-042 à A-044 restent ouvertes.

Une consigne gestuelle identique était affichée dans le viewport et dans l’inspecteur STEP sans sélection. Suppression de la copie dans l’inspecteur ; l’aide source et la consigne du viewport restent intactes. Aucun changement de logique de conversion, de sélection ou d’export. Contrôle après modification : Svelte 0 erreur/0 avertissement, build UI et bundle macOS réussis, avertissement Vite habituel sur la taille du chunk. Exécutable du bundle identique au binaire construit (empreinte dans `artifacts/ux-native-followup/bundle-sha256.txt`). Ce dernier retrait de texte n’a pas fait l’objet d’un second lancement graphique ; la suite Rust globale de V-1184 n’a pas été relancée pour ce changement de présentation.


## V-1186 — Deux cônes adjacents, double raccord torique et pièces creuses — 17 novembre 2026

R-060 / ADR-0032 : exactement deux cônes pleins adjacents, à rayons positifs, séparés par un anneau source complet et une rupture affine unique. Deux témoins intérieurs résolus sont exigés de chaque côté ; une loi affine unique historique garde la priorité. Les chaînes plus longues sans portée ne sont pas couvertes. Le cas `same_sign_chains/no_land` devient positif sans modification de ses octets STL. Le nouveau corpus `adjacent_cones` est indépendant du reconnaisseur ; volume, surfaces, provenance native et transformations sont contrôlés. Tests ciblés : 37 tests core et 9 app réussis ; revue indépendante acceptée. Journaux `artifacts/c1b2/`.

R-061 / ADR-0033 : chaîne extérieure limitée à deux quarts de tore et leur portée cylindrique commune. La canonicalisation conserve un seul rayon source pour la portée et compte les déplacements dans les résidus vérifiés. Une portée avec deux témoins intérieurs est obligatoire ; pas de raccord intérieur, profil libre ou rayon variable. Corpus `double_torus` : sept variantes positives, miroir 3MF et refus ciblés. Quatre tests core, un test privé de budget et quatre tests app passent ; régressions historiques ciblées vertes et revue indépendante acceptée. Les quatre tests app incluent quatorze conversions natives positives. Le cas 3MF en contact est bloqué par `cad.bodies_separation_budget` : ce n’est pas une preuve de détection directe du contact. Journaux `artifacts/double-torus/`.

R-062 : qualification du moteur existant, sans modification de production. Corpus `partial_fillets` : une ou deux arêtes verticales arrondies et bac ouvert avec coins intérieurs/extérieurs cylindriques ; fond et rebord restent à arêtes vives. Quatre tests core et quatre tests app réussis. Les sondes matière/vide par nombre d’enroulement portent sur la source et la tessellation du STEP réimporté ; elles complètent les vérifications de solide, volume, dimensions, tangence et provenance. Une paroi à rayon variable est refusée en mode explicite et conservée facettisée en Auto. Aucun congé général ni coin à trois branches revendiqué. Journaux `artifacts/partial-fillets/`.

**Validation globale réussie en un passage séquentiel** : `scripts/check.sh`, code 0 — 459 tests workspace, 32 bureau, 208 UI, 12 Python et 4 SDK. Douze tests opt-in restent ignorés. Formatage, Clippy strict workspace/bureau, Svelte (0 erreur, 0 avertissement), builds UI/bureau et doctests passent. Avertissement Vite préexistant de taille du bundle. Régénération des quatre corpus : 48 fichiers identiques octet par octet. Aucun échec non résolu. Journaux et résumé : `artifacts/complex-geometry/{check.log,build.log,validation-summary.json,corpus-determinism.txt}`.

Build macOS réussi : `artifacts/FabSTEP.app`, binaire identique au build, SHA-256 `e2e895b72e0e403c225aaccd9fe9a131d0b4b8d24af8d0d964589815706d692b`. Le paquet autonome 1.0.0 reste inchangé. Aucun essai graphique ni réimport Fusion des nouveaux corpus n’est encore déclaré réussi. Pas de mesure comparative de performance revendiquée. Le laboratoire F0 des perçages sécants est séparé et ne modifie pas les chemins de conversion de l’application.


## V-1187 — F0, laboratoire des perçages sécants — 17 novembre 2026

**PARTIAL : aucune activation dans la conversion.** Accord utilisateur A-045 obtenu pour l’extension additive du modèle interne ; R-063 réalise d’abord un laboratoire autonome, R-036 reste à intégrer. `scripts/qualify-cross-drill.sh` compile du C++ isolé avec OCCT 7.9.3 dans `artifacts/cross-drill-lab`. Code de sortie 2 attendu tant que `curves_certified` est faux ; `--inspect-only` n’est jamais un certificat. Aucun nouveau DTO, ABI, chemin de reconnaissance ou dépendance de production.

La référence est un cube de côté 20 mm percé de deux cylindres orthogonaux centrés, rayons 3 et 6 mm. La construction booléenne de laboratoire ne démontre pas une reconstruction depuis un STL. Après export/réimport : un solide, six plans et trois faces cylindriques, 36 sondes matière/vide conformes ; trois arêtes d’intersection B-spline de degré 6. Deux arêtes non périodiques constituent la branche positive, une arête périodique constitue la négative. Incidences opposées, connexité et degrés des cycles contrôlés.

Volume réimporté par intégration adaptative : 5500,895249460223 mm³ ; quadrature indépendante : 5500,895249214532 mm³, **référence numérique non certifiée**. L’appel non adaptatif donne 5500,898895086297 mm³ : cet écart dépasse la garde relative actuelle et doit être traité avant l’intégration de cette famille. Aucun relâchement de garde dans la production.

Le certificat expérimental travaille par intervalles sur les nœuds, pôles et poids réimportés, avec conversion Cox–de Boor vers Bernstein sans transformation flottante non bornée. Il borne les résidus implicites et la correspondance des deux courbes paramétriques de surface, puis contrôle la phase. Les tests élémentaires passent (5983 opérations) : arithmétique, trigonométrie encadrée, spline quadratique connue, petit segment positif à 1e-7 mm et corruptions ciblées. Un premier test a trouvé une borne négative subnormale sur le quotient nul de valeurs absolues ; correction par intersection avec le domaine [0,1] déjà démontré, sans tolérance élargie. Le contrôle d’agrégation de phase possède un témoin positif indépendant et rejette une duplication de tour.

**Le cas natif complet ne passe pas.** À 1e-5 mm, le budget global de dix millions d’opérations s’épuise dans la première arête après 1223 portions localement bornées ; ces portions ne constituent pas un certificat de l’arête. Les autres arêtes ne sont pas parcourues dans le passage global final. La courbe périodique est en outre hors domaine explicite du certificat ; un essai isolé antérieur l’a refusée. À 1e-7 mm, le budget s’épuise également. Les mutations natives sont refusées, mais `adversaries.all_passed=false` : sans cas nominal certifié, ces refus ne sont pas présentés comme une qualification complète.

Suites nécessaires : réduire le coût de la borne continue sans élargir les tolérances ; déplier les splines périodiques à partir de leurs nœuds/pôles originaux avec arrondis bornés ; qualifier le volume ; ensuite seulement établir la preuve source, le contrat core/CAO et le routage R-036. Journaux : `artifacts/cross-drill-lab/{compile.log,native.log,report.json,run-result.json}`. Les bornes de l’agrégation des cycles sont spécifiques à cette référence fixe, pas un contrat pour des dimensions arbitraires. Aucun essai utilisateur ou Fusion revendiqué.

Revue indépendante finale : PARTIAL, acceptable comme laboratoire isolé, sans défaut bloquant supplémentaire trouvé dans les calculs du domaine fixe. Rejeu final par l’orchestrateur : sortie 2, contrôles élémentaires réussis et certificat global refusé comme annoncé (`orchestrator-run.json`).


## V-1188 — UX6, reconstruction déclenchée dans l’inspecteur

R-064 : l’ouverture ne lance que le chargement et l’aperçu ; un bouton explicite lance le premier STEP de chaque source. Avant ce clic, les réglages ne convertissent pas. Après lancement volontaire, leur recalcul historique reste actif. Le changement de source remet cette autorisation à zéro ; le verrou partagé et les révisions empêchent doublons, export ancien et publication de réponses périmées.

215 tests UI réussis, dont 49 du workflow et sept nouveaux scénarios ; Svelte 0 erreur/0 avertissement, build UI et macOS réussis. Un test d’intégration de coupe source attendait encore la conversion automatique : adapté au clic explicite, il vérifie toujours un véritable refus de conversion puis la coupe et l’export source, sans export STEP. La suite complète finale passe. Aucun code Rust changé, suites géométriques non réexécutées ; dernière preuve V-1186. Logs sous `artifacts/manual-reconstruction/`. Empreinte du bundle identique au build : `956a0a3642064ec73e67cb421eaf10926af23fcbcd8937f76cb116dc07ccfc4d`.

Contrôle natif observé : ouverture de `two_increasing.stl`, aperçu sans STEP exportable, bouton visible à droite ; clic, bouton désactivé pendant calcul, puis résultat validé et bascule automatique à la vue STEP. Rendu sombre observé. Une consigne prématurée vers le STEP a été retirée après cette observation et le build refait. L’essai graphique d’un second fichier n’a pas abouti : dialogue natif avec bouton Ouvrir désactivé, puis délais dépassés du pilote y compris pour fermer le dialogue. La réinitialisation au second fichier est couverte automatiquement, pas revendiquée comme essai graphique réussi. Clair/anglais et fenêtre étroite non rejoués. Aucun essai Fusion nécessaire à ce changement de déclenchement.


## V-1189 — UX7, progression native observée

R-065 : suivi de l’analyse, de la reconstruction et de la validation finale, avec sous-opération native, numéro de corps réel et durée. Pas de compteur de zones/triangles ni de total de corps inventé, pas de pourcentage ou de temps restant. Les phases courtes peuvent ne pas apparaître entre deux interrogations ; aucune durée artificielle ne les retient à l’écran. L’activité reste indéterminée à l’intérieur d’un appel natif long. Les corrections guidées distinguent l’inspection préalable du résultat précédent de la validation finale.

38 tests bureau passent, dont six de suivi : phases/répétitions, fraîcheur, invalidation/conflit, succès/refus, génération, panique et précontrôle guidé. 219 tests UI passent ; quatre nouveaux tests vérifient interrogation séquentielle, arrêt/réponse tardive, valeurs périmées ou invalides, récupération après erreur IPC et mapping des étapes. Svelte 0 erreur/0 avertissement, formatage bureau, Clippy strict et build macOS passent. Un premier Clippy signalait l’ancienne méthode de contrôle de fraîcheur devenue inutilisée ; elle a été supprimée, la nouvelle observation réalisant le même contrôle sous verrou. Rejeu final vert. Aucun changement app/core/CAO ; suite géométrique non rejouée (preuve antérieure V-1186).

Journaux dans `artifacts/reconstruction-progress/` : `ui-tests.log`, `ui-check.log`, `desktop-tests.log`, `desktop-clippy.log`, `build.log`, et premier passage `desktop-initial.log`. Binaire du bundle identique au build : SHA-256 `c0f552d8518caee8ffc86a8426050cd7f5dbbf556d0a53305436ddc45a757271`. Contrôle graphique non exécuté : `getApp` du pilote natif expire sur FabSTEP ; navigateur de pilotage indisponible. Clair/sombre, fenêtre étroite et affichage pendant une reconstruction restent à confirmer par l’action utilisateur. Aucune réussite visuelle n’est déduite des tests automatisés.


## V-1190 — Nombre de triangles dans le refus STL

R-066 : diagnostic dynamique sans changement de code d’erreur, de format FabError ou de plafond. Le binaire utilise le nombre d’enregistrements uniquement après concordance exacte de sa longueur. L’ASCII compte toutes les facettes complètes, dans la source déjà limitée à 64 Mio, sans conserver plus de 100 000 triangles ; une syntaxe tardive invalide reste une erreur de syntaxe plutôt qu’un faux total. La capacité réservée est plafonnée et dérivée d’une borne syntaxique minimale, jamais utilisée comme nombre annoncé. L’interface localise les deux nombres ; les anciens messages sans compte gardent leur rendu générique. Le 3MF n’est pas modifié.

Preuve privée : FlanschTurboLuftfilter_REV02.STL, 27 008 584 octets = 84 + 50 × 540 170. Le CLI réellement compilé retourne code 1 attendu et `input.too_many_triangles`, message `STL : 540170 triangles ; limite : 100000.`. Aucun triangle du fichier privé n’est ajouté au dépôt. Tests : cinq nouveaux cas du lecteur, quatorze régressions d’analyse et 220 tests UI réussis ; Svelte 0 erreur/0 avertissement, formatage workspace et Clippy core strict réussis. Pas de reprise des suites géométriques complètes pour ce diagnostic ; dernières preuves V-1186. Logs `artifacts/triangle-count/` ; le refus CLI se trouve dans `private-check.stderr`. Aucun nouveau contrôle graphique revendiqué, pilote natif précédemment bloqué.

Build macOS réussi ; binaire du bundle identique au build, SHA-256 `cd3636a8fbb331a73c47a4d0576457c75eda175c302c6a9421b0472a17c6ad4d`. Paquet release inchangé.

## V-1191 — G1 : gros STL sur accord, calcul isolé et annulable (17 novembre 2026)

**Périmètre :** R-067, ADR-0034, macOS Apple Silicon. Accord du processus séparé reçu et consigné dans A-055. STL binaires de 100 001 à 600 000 triangles, 64 Mio maximum ; parcours ordinaire/3MF inchangés. Le helper compile `large-stl` séparément ; limites source et résultat distinctes. Aucun audit, budget de reconnaissance ou contrôle de fermeture supprimé.

**Implémentation et revue :** avertissement avec compte exact, accord lié au fichier, Annuler et rejet des réponses tardives. Supervision native avec contrôle des identités, fichiers bornés, marqueur final et sortie réussie avant cache exportable. Arrêt/récupération avant nettoyage sur erreur, annulation, invalidation ou fermeture normale de l’application. Surveillance macOS `libproc` à 4 Gio de RSS toutes les secondes et délai de 30 minutes ; refus si le moniteur est indisponible. L’ABI est vérifiée contre les en-têtes du SDK local. Ce seuil surveillé ne borne pas le pic entre observations ni toute la mémoire du bureau. Pas de qualification Windows/Linux : ce parcours y est refusé.

Revue indépendante du helper et du superviseur acceptée. La revue principale a fait corriger deux courses avant validation finale : destruction du dossier temporaire avant récupération du child, et lecture des métadonnées de l’ancien fichier de progression avant ouverture de sa nouvelle version. La revue UI a conduit à déplacer également la préparation de l’aperçu dense dans un Web Worker. Les petits STL/ASCII et le STEP gardent leur parcours ; aucune provenance source guidée n’est inventée pour le STEP transféré.

### Contrôles exécutés

- `cargo test --workspace --offline --locked` : **470 PASS**, 12 tests opt-in ignorés par la suite ordinaire.
- Bureau : **49 PASS**, 1 test opt-in ignoré par défaut, puis **ce test réel exécuté séparément : PASS**. Il utilise le vrai helper, son rapport/STEP, la publication dans le cache et l’inspection native du cube de référence (1 mm³, six faces planes). Le premier passage a corrigé une attente de test erronée à 8 000 mm³ ; la fixture a bien des coordonnées 0..1. Aucun contournement de validation produit.
- Helper : **8 PASS**. Profil `large-stl` : **36 tests ciblés PASS** (lecteur, frontières, analyse/sélection, 3MF après placement, rôles CAD et audit asymétrique), construits dans le target séparé du helper.
- UI : **229 PASS**, dont consentement, annulation/réponse tardive et vrai worker d’affichage à 100 001 triangles ; Svelte **0 erreur / 0 avertissement**.
- Formatage Rust, Clippy strict workspace/bureau/helper, contrôle bureau, build UI, **12 tests Python et 4 tests SDK PASS**. Avertissement Vite connu sur la taille de bundle, sans erreur.
- `scripts/build-macos-dev.sh` : PASS. Les deux binaires dans `artifacts/FabSTEP.app/Contents/MacOS/` correspondent par SHA-256 aux sorties compilées. Le script `check.sh` inclut désormais les contrôles isolés et le transfert réel sur macOS. Les commandes de cette validation ont été exécutées en lots séquentiels, pas par une invocation unique du script complet.

### Essais de processus réels

Build de développement non optimisé, machine 16 Gio ; durées indicatives sous charge de validation concurrente, pas un benchmark comparatif avant/après. RSS échantillonnée par `ps`, pas un pic instrumenté exhaustif. Source privée jamais ajoutée au dépôt.

| Source / essai | Issue constatée | Durée | RSS maximale observée |
|---|---|---:|---:|
| FlanschTurboLuftfilter_REV02.STL, 540 170 triangles | Admission réussie, puis `planar.too_complex` : budget de cinq millions de comparaisons de contours dépassé ; aucun STEP | 108,125 s | 363,59 Mio |
| Même fichier, annulation en analyse | Processus tué/récupéré, aucune sortie exportable | 2,080 s ; kill/wait 3,4 ms | 172,89 Mio |
| Cube maillé synthétique, 101 568 triangles, annulation en phase `source_validation` | Processus tué/récupéré, aucune sortie exportable | 23,516 s ; kill/wait 2,2 ms | 125,41 Mio |
| Relance du même cube dense | Pipeline complet validé, sortie réussie ; rapport 11 208 277 octets et STEP 1 300 141 octets | 174,356 s | 2 600,33 Mio |

Ces mesures ont motivé le garde RSS de 4 Gio. Elles n’établissent pas qu’un modèle quelconque de 600 000 triangles sera convertible. La limite de comparaisons rencontrée par le fichier utilisateur reste entière ; piste d’optimisation séparée R-068, sans relèvement du budget dans G1.

**Limites graphiques :** le pilotage natif a ouvert l’application et le dialogue de fichier, mais « Open » est resté désactivé malgré sélection et chemin complet ; une tentative par coordonnées a aussi rencontré une erreur du pilote. Dialogue annulé, application laissée sans fichier. Comme lors des validations précédentes, cela ne suffit pas à conclure à un défaut d’import utilisateur. Le parcours visuel complet avertissement → essai → Annuler, les thèmes/langues montés et Fusion restent à confirmer via les actions R-067. Aucun succès graphique de ce parcours n’est revendiqué.

**Traces locales :** `artifacts/g1/` contient les journaux workspace/bureau/helper/UI, le test de transfert réel, les quatre rapports JSON de processus, l’outil de mesure local, les empreintes du bundle et le cube synthétique. Ces traces reconstruisibles ne sont pas versionnées. La qualification du paquet autonome v1 reste distincte et inchangée.


## V-1192 — G2 : filtrage spatial borné des contours denses (17 novembre 2026)

R-068 remplace les comparaisons exhaustives au-delà de 64 segments par un arbre de boîtes 2D fermées, déterministe et local au core. Le tri, les copies, la construction et le parcours partagent les cinq millions d’unités de travail ; nouvelles réservations faillibles, stockage linéaire, pile logarithmique, aucune liste quadratique de paires. Les prédicats de croisement/contact et l’inclusion des trous restent inchangés. Le filtre n’est éligible que pour des coordonnées projetées nulles ou comprises en valeur absolue entre 2^-200 et 2^200 : les opérations du filtre d’orientation restent normales et finies dans ce domaine. Hors de cette fenêtre, le parcours exhaustif conserve ses comportements numériques historiques. Aucun sommet, seuil, audit CAO ou plafond G1 n’est modifié. Le compteur de l’index inclut son coût de préparation : pour des boîtes très chevauchantes proches du plafond, un refus de budget peut survenir plus tôt que dans l’exhaustif ; aucune équivalence générale des seuils de refus n’est revendiquée.

Revue indépendante acceptée après correction des réservations initialement infallibles et comptabilisation des copies. L’orchestrateur a relu le diff et exécuté les tests. Corpus différentiel : contacts, colinéarités, zéro signé, écart d’une ULP, bornes dyadiques et extrêmes, puis 36 formes radiales transformées et leurs 36 mutations croisées ; résultats comparés à l’exhaustif et compteurs déterministes. Les trous séparés, extérieurs et imbriqués passent par le validateur complet après indexation. Un cas de boîtes chevauchantes épuise explicitement le budget. Le test d’intégration construit un prisme de 16 384 triangles et conserve ses deux contours de 4 096 sommets, les coordonnées et le volume polygonal attendu.

### Mesures

Build debug sur ce Mac ; exécutions avant/après sans autre compilation ni suite de tests concurrente. Un seul passage par cas : mesures indicatives, pas un benchmark statistique ni une garantie de latence. Le benchmark opt-in compare uniquement les couples d’arêtes, hors topologie et CAO : `cargo test -p fabstep-core --offline --locked --lib benchmark_dense_contour_exhaustive_against_bvh -- --ignored --nocapture`.

| Contour synthétique | Exhaustif historique | Index G2 | Résultat |
| --- | --- | --- | --- |
| 512 segments | 130 304 comparaisons, 6,881 ms | 13 986 unités, 0,617 ms | Les deux acceptent |
| 4 096 segments | Refus à 5 000 001 comparaisons, 217,720 ms | 124 822 unités, 4,869 ms | G2 accepte ; l’ancien parcours n’achève pas la preuve |

Fichier privé FlanschTurboLuftfilter_REV02.STL, 540 170 triangles, mêmes options (mm, Auto, soudage/planéité 1e-6 mm, ajustement 1e-5 mm, écart maximal 0,05 mm), helper G1 de `2d513ba` conservé pour la référence :

- Avant : **94,747 s**, RSS maximale échantillonnée **440,48 Mio**, refus `planar.too_complex`, phase analyse uniquement.
- Après : **108,812 s**, RSS maximale échantillonnée **514,42 Mio**, entrée en reconnaissance à **95,112 s**, puis refus `recognition.too_complex`. **Aucun rapport de succès ni STEP produit.** Le blocage des contours est franchi ; le coût de reconnaissance relève de R-069, non traité en relevant son budget.

Ces durées ne mesurent pas le même travail terminé ; aucun gain global de conversion n’est revendiqué. Un échantillonnage CPU avant modification situait du travail dans la séparation des composantes et la validation topologique. Les 95 premières secondes ne sont donc pas attribuées aux seuls contours. Le lanceur de qualification appelle le vrai helper, observe sa progression et impose ses propres gardes à 300 s et 4 Gio (180 s avant) ; ce n’est pas un essai graphique du superviseur Tauri. Les plafonds produit restent à 30 minutes et 4 Gio.

Journaux locaux non versionnés : `artifacts/g2/{flansch-before.json,flansch-after.json,contour-benchmark.log,planar-tests.log,dense-prism.log,check.log}`. Le fichier privé n’entre pas dans le dépôt. Les contrôles globaux et le paquet sont détaillés ci-dessous.

### Contrôles finaux

`scripts/check.sh` termine avec code 0 : **476 tests workspace réussis**, 13 opt-in ignorés par défaut (le nouveau benchmark exécuté séparément réussit) ; **49 bureau**, 1 transfert réel opt-in exécuté après le passage ordinaire ; **8 helper** ; **46 ciblés avec `large-stl`** (dont les neuf tests de contours et le prisme) ; **229 UI**, **12 Python**, **4 SDK**. Formatage, Clippy strict workspace/bureau/helper, contrôle bureau hors workspace, Svelte (0 erreur/0 avertissement) et build UI réussis. Les plafonds source et résultat sont revérifiés dans les deux profils. Aucun échec dans la suite globale finale ; une compilation intermédiaire pendant écriture a signalé deux littéraux Rust incomplets, corrigés avant cette suite.

`scripts/build-macos-dev.sh` réussit. `artifacts/FabSTEP.app` contient les exécutables identiques aux builds (SHA-256 bureau `6bfedb1969f6d45707ee44101930e514fe2a795b4096a9ddb032a58e664cf322`, helper `8462b4d5c43c6cc843842299c7353982fbd8d2080e041323d866b2e2e98fabf8`). Le paquet autonome 1.0.0 reste inchangé. Aucun nouvel essai graphique ni Fusion exécuté ; l’interface n’est pas modifiée. Le parcours manuel dense et l’annulation restent à qualifier dans l’action utilisateur R-068.
