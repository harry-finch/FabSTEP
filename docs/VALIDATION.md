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
