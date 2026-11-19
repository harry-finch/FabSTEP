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


## V-1193 — G3 : préparation réutilisée et recherches d’arêtes locales (17 novembre 2026)

R-069 comporte deux optimisations locales du core, sans nouveau contrat public ni changement de tolérance. `pipeline.rs` conserve le soudage, ses statistiques et l’adjacence globale originale uniquement pour un mono-corps dont les indices source sont exactement 0..N. La finition volume/inversion et la segmentation restent partagées avec le parcours historique. Les corps multiples gardent leur préparation séparée, après libération du maillage et de l’adjacence globaux.

Dans `rounded::classify`, l’union des arêtes des triangles du patch remplace le scan global lorsqu’une borne 6T est inférieure à E. Trois insertions par triangle puis une unité par arête locale sont débitées avant les opérations correspondantes. Le scan global historique reste préféré pour les grandes couvertures ; aucun repli n’est tenté après épuisement. Les incidences du maillage entier, l’ordre BTree et le prédicat de hauteur sont inchangés. Le budget reste à dix millions.

**Preuves et revue :** le premier dépassement a été localisé par LLDB dans le binaire G2, `rounded.rs:850`, avec un débit de 810 255 unités. La revue indépendante du diff G3 est favorable. Cinq tests de préparation comparent bits des coordonnées/normales/volume/dimensions, triangles, contours, diagnostics et provenance ; ils couvrent unités, soudage déplacé et choix du représentant, inversion, composantes entrelacées, premiers refus et annulation coopérative. Trois tests du collecteur comparent l’oracle global avec appartenance intérieure/frontière, projections partielles, seuils à une ULP, ordre et débits exacts (6, 11 et 18 unités).

### Mesures et refus restant

Build debug, même Mac et mêmes options que V-1192 ; mesures chronométrées sans débogueur ni autre calcul lourd concurrent. Un passage par version, donc pas de garantie de latence. Fichier privé FlanschTurboLuftfilter_REV02.STL, 540 170 triangles :

| Version | Entrée en reconnaissance | Fin de l’essai | RSS maximale échantillonnée | Résultat |
| --- | --- | --- | --- | --- |
| G2 (`ffcb5d4`) | 97,329 s | 111,285 s | 489,94 Mio | `recognition.too_complex`, scan global des arêtes |
| G3 | 50,910 s | 66,177 s | 541,33 Mio | `recognition.too_complex`, recherche des arcs ouverts |

La phase d’analyse passe à environ 52 % de sa durée antérieure. La durée totale baisse, mais le moteur exécute davantage de reconnaissance : les deux refus ne terminent pas le même travail. Aucun rapport de succès ni STEP n’a été produit. Un second lancement sous LLDB, séparé des mesures, localise le refus restant dans `partial::candidates` (`partial.rs:221`, débit de 216 unités). Ce lancement de diagnostic est volontairement tué au point d’arrêt ; ce n’est pas une qualification de conversion. Le compteur est partagé avec les preuves précédentes : cette trace ne mesure pas le coût isolé des arcs, à profiler avant leur évolution. Les ajustements contraints par les extrémités n’étant pas monotones, supprimer des longueurs candidates demande une preuve distincte.

Benchmark différentiel de préparation sur le même fichier privé, dans le profil `large-stl` : **99,049 s historique contre 51,167 s réutilisé**, résultats d’analyse réussis et comparés bit à bit (géométrie, rapport, diagnostics, provenance). Ordre historique puis réutilisé, un seul passage ; il s’agit de l’analyse, pas d’une conversion CAO. Commande : `FABSTEP_PREPARATION_BENCH_STL=chemin cargo test --target-dir crates/fabstep-worker/target -p fabstep-core --features large-stl --offline --locked --lib benchmark_single_component_reuse_against_historical_preparation -- --ignored --nocapture`.

Benchmark du seul collecteur, corpus synthétique clairsemé : 73 728 unités / 10,182 ms en global contre 11 unités / 14,666 µs en local, avec même résultat. Ce corpus de copies déconnectées ne prétend pas être un document convertible et ce ratio ne mesure pas la conversion complète. Le lanceur réel conserve ses gardes de qualification (300 s après, 180 s avant, 4 Gio), distinctes des 30 minutes du produit. Les tentatives d’attachement du débogueur à ce lanceur ont perturbé son suivi de processus et sont exclues des mesures ; seul le lancement LLDB autonome fournit la seconde trace.

Traces locales : `artifacts/g3/{flansch-before.json,flansch-after.json,budget-stack.log,second-budget-launch.log,edge-benchmark.log,preparation-tests.log,patch-tests.log,core-integration.log}`. Le fichier privé n’est pas versionné.

### Contrôles finaux

`scripts/check.sh` termine avec code 0 : **484 tests workspace**, **49 bureau**, **8 helper**, **54 ciblés `large-stl`**, **1 transfert réel opt-in**, **229 UI**, **12 Python** et **4 SDK** réussis. Formatage, Clippy strict des trois builds, contrôle bureau hors workspace, Svelte (0 erreur/0 avertissement) et build UI passent. Quinze tests workspace opt-in sont ignorés par défaut ; les deux nouveaux benchmarks G3 ont été exécutés séparément avec succès, les autres opt-in ne sont pas rejoués dans ce lot. Aucun échec dans la suite globale finale.

Le build de développement macOS réussit et ses deux exécutables correspondent à ceux de `artifacts/FabSTEP.app` : SHA-256 bureau `0264065da6c131980b4367db03f3173ae0053b1514b23e869fd745037f662103`, helper `fdc26fcf28c7b93d52f64e8f2cf37a21ca1f35c53d9d487885df4164e3a2be91`. Journaux complémentaires : `artifacts/g3/{preparation-benchmark.log,check.log,build-app.log,bundle-hashes.json}`. Aucun essai graphique ni Fusion exécuté ; le paquet autonome 1.0.0 est inchangé. Le parcours manuel et son annulation restent à qualifier dans l’action liée à R-069 ; la recherche d’arcs relève de R-070 (G4), encore au stade idée.


## V-1194 — G4 : cercles denses et réutilisation des preuves Auto (17 novembre 2026)

R-070 corrige une classification numérique dans `partial::candidates` et supprime des preuves Auto répétées dans `rounded.rs`. Aucun contrat public, plafond, tolérance ou dépendance ne change.

Le profilage G3 montre 5 701 356 unités déjà consommées à l’entrée de la recherche d’arcs. Le budget est ensuite dépassé à 10 000 162, sur la face d’indice 1, contour 2, départ 42, longueur 216. Le contour complet compte 362 sommets. Son ajustement contraint par les deux extrémités adjacentes échoue, tandis que la preuve circulaire existante `recognition::fit_circle` réussit à la même tolérance (1e-5 mm), avec un rayon de 41,93550933193075 mm et un écart radial maximal de 5,637604694186393e-6 mm.

Le secours s’exécute seulement quand l’ajustement contraint échoue. Les décisions angulaires historiques restent intactes, notamment les profils D et les cercles clairsemés. La preuve de secours examine tous les points, leur ordre, les pas angulaires et le tour complet ; elle est débitée avant calcul. Les fenêtres d’arcs ouverts, leur recherche non monotone et la maximalité sont inchangées. Le secours ajoute un parcours linéaire aux contours dont le premier ajustement échoue : sur un cas proche du plafond qui ne bénéficie pas de l’exclusion, un refus peut survenir plus tôt. L’équivalence des seuils de budget n’est pas revendiquée. Il s’agit d’une correction de classification : les anciens candidats erronés du cercle complet ne sont pas conservés et aucune équivalence stricte de tous les candidats historiques n’est revendiquée.

Les preuves Auto réussies possèdent désormais leur patch sphérique ou leurs sections exactes. Le résultat (écarts et orientation) est réutilisé à l’assemblage, avec une unité de consultation ; support plan, preuve initiale, couverture et attribution finale restent exécutés. Le parcours explicite conserve sa preuve différée du cône unique. La revue indépendante vérifie notamment que la contrainte de chaîne torique concave est encore évaluée avant le transfert des sections. Aucun cache global n’est ajouté.

### Régressions ciblées

Les six tests `partial::tests` passent, dont quatre nouveaux : 48 combinaisons de cercles synthétiques quantifiés (128/360 points, quatre origines cycliques, deux sens, trois repères), débit exact 2n pour le secours, épuisements avant calcul, profils D, cercles clairsemés, bruit hors tolérance, croisement et retour angulaire. Le test historique de fenêtres non monotones reste passant. Les données synthétiques sont indépendantes du STL privé.

Les trois tests `auto_proof_reuse_tests` passent : modèle/rapport/provenance comparés à un oracle de recalcul, cas rejetés et guidage restreint, parcours explicite et déviation trop stricte. Pour la section cylindrique synthétique, deux preuves consommaient 768 unités ; la preuve unique plus consultation en consomme 385 avec les mêmes bits d’écarts et la même orientation. Une preuve invalidée n’est pas réutilisée et les budgets épuisés restent bloquants. Cette réduction de travail isolée ne mesure pas le temps de conversion complet.

### Mesures et limite restante

Build debug sur le même Mac, un passage chronométré par version sans débogueur ni compilation concurrente ; mesures indicatives, sans garantie de latence.

| Version | Entrée en reconnaissance | Fin de l’essai | RSS maximale échantillonnée | Résultat |
| --- | --- | --- | --- | --- |
| G3 (V-1193) | 50,910 s | 66,177 s | 541,33 Mio | Budget épuisé, face 1 / contour 2 |
| G4 | 57,753 s | 72,797 s | 551,75 Mio | Budget épuisé plus loin, face 7 278 / contour 0 |

**FlanschTurboLuftfilter_REV02.STL ne produit toujours aucun STEP ni rapport de succès.** La durée d’analyse varie également alors que G4 ne la modifie pas ; aucun gain global de durée n’est revendiqué. Un lancement de diagnostic séparé localise le nouveau dépassement à 10 000 008 unités, sur une fenêtre de 11 points au départ 73 d’un contour de 300 points, dans `partial::candidates`. Ce contour n’est pas le cercle complet initial. Les plafonds restent conservateurs et le refus explicite reste export-inéligible. L’optimisation des fenêtres générales demeure ouverte ; aucun saut fondé sur une monotonie non prouvée n’est intégré.

Les mesures utilisent le helper réel, avec gardes de qualification à 300 s et 4 Gio ; le produit conserve 30 minutes et 4 Gio. Les lancements LLDB autonomes sont volontairement tués au point d’arrêt et ne constituent pas des conversions qualifiées. Les tentatives de lancement sous LLDB bloquées avant `main` sont exclues des mesures. Traces locales : `artifacts/g4/{arc-budget-attach.log,circle-diagnosis.log,flansch-after.json,profile-after.log}`. Les contours extraits et le STL privé ne sont pas versionnés.

### Contrôles finaux

`sh scripts/check.sh` termine avec code 0 : **491 tests workspace, 49 bureau, 8 helper, 63 ciblés avec `large-stl`, 1 transfert réel opt-in, 229 UI, 12 Python et 4 SDK réussis**. Formatage, Clippy strict des trois builds, contrôle bureau hors workspace, Svelte (0 erreur/0 avertissement) et build UI passent. Les quinze tests workspace opt-in restent ignorés par la suite ordinaire ; aucun nouveau test opt-in dans G4. Les benchmarks G2/G3 ne sont pas rejoués dans ce lot. Une compilation intermédiaire du nouveau test de budget exigeait `Debug` sur `Errors`, corrigé avant les tests ciblés et la suite globale ; aucun échec dans la suite finale. L’avertissement Vite connu sur la taille du bundle reste présent.

`sh scripts/build-macos-dev.sh` réussit. Les deux exécutables de `artifacts/FabSTEP.app` correspondent aux builds : SHA-256 bureau `3d0deef7440a2b2b538dfb7b04d28177e3306132381834301141af1111125f8a`, helper `d8332ff8372c31c9f6f705ee644f5b86a055ca7b98cd5dbe60de164808567154`. Journaux complémentaires : `artifacts/g4/{partial-tests.log,proof-tests.log,check.log,build-app.log,bundle-hashes.json}`. Aucun nouvel essai graphique ni Fusion ; paquet autonome v1 inchangé. R-070 reste à tester via A-060 ; la recherche générale résiduelle est suivie dans l’idée R-071.


## V-1195 — R-071 : maximalité des arcs par intervalles cycliques (17 novembre 2026)

Le profilage local de Flansch sous G4 attribue 6 658 unités aux gardes de contours, 1 398 664 aux fenêtres ajustées et 2 919 238 à leur maximalité, avant le refus `recognition.too_complex`. Ces compteurs instrumentent uniquement les appels correspondants du core ; ils n’incluent pas toute la reconnaissance précédente. Les petits arrêts radiaux anticipés ne permettraient pas de réduire honnêtement le débit des fenêtres, puisque chacune visite déjà tous ses points pour l’ajustement. Cette piste n’a pas été intégrée.

`partial.rs` conserve les fenêtres, les sommes flottantes, les seuils angulaires et la preuve des unions. Pour les contours à sommets uniques, les ensembles d’arêtes sont remplacés uniquement au test de maximalité par des intervalles cycliques. Une chaîne de longueur L représente L−1 arêtes, moins qu’un tour complet ; le décalage du départ et les longueurs déterminent exactement l’inclusion. L’ordre des boucles et la priorité du candidat antérieur en cas d’égalité restent identiques. Une copie faillible, un tri par tas et le contrôle des doublons sont débités avant leurs opérations. Avec zéro ou un candidat, ou des indices répétés, la voie historique est conservée ; un épuisement du garde reste bloquant et ne déclenche pas de repli.

La revue indépendante est favorable. Elle signale qu’un grand contour avec très peu de candidats peut payer un garde plus coûteux que l’ancienne maximalité ; aucun gain uniforme ni équivalence des seuils de refus de ressources n’est revendiqué. Le plafond partagé reste à dix millions, sans nouveau contrat ni nouvelle preuve numérique.

### Équivalence et mesures locales

Les quatre nouveaux tests couvrent tous les couples d’intervalles des contours de 3 à 12 sommets, les rebouclages, égalités, ordre inversé et valeurs d’indices proches de usize::MAX. Le résultat conservé est comparé aux ensembles historiques ; les champs de Fit restent bit à bit identiques. Des indices répétés imposent le repli. Des budgets exacts vérifient copie, tri, comparaisons, épuisement sans repli et absence de garde sur un candidat unique. Les candidats réels de 18 profils D (trois densités, deux sens, trois repères) conservent ordre, chaînes, centre, normale et rayon bit à bit. Les six régressions partial existantes passent également, dont les fenêtres non monotones et les cercles complets quantifiés.

Benchmark opt-in reproductible, build debug, un passage historique puis optimisé sur ce Mac, sans autre calcul lourd concurrent. Commande : `cargo test -p fabstep-core --offline --locked --lib benchmark_cyclic_maximality_against_historical_sets -- --ignored --nocapture`. La comparaison couvre la recherche complète des candidats synthétiques, pas une conversion CAO ; mêmes candidats et mêmes bits vérifiés.

| Profil D | Ensembles historiques | Intervalles cycliques |
| --- | --- | --- |
| 97 sommets | 293 429 unités ; 40,495 ms | 162 633 unités ; 19,420 ms |
| 257 sommets | 5 310 879 unités ; 214,995 ms | 2 905 912 unités ; 199,512 ms |

Ces durées isolées ne sont pas un benchmark statistique ni une garantie de latence. Les économies de compteur correspondent à la suppression effective des constructions et comparaisons d’ensembles, pas à une hausse de budget.

### Flansch et prochain refus

Même fichier privé de 540 170 triangles et mêmes tolérances que V-1194. Le profilage après modification termine la reconnaissance : **4 cylindres, 8 cônes, 501 900 plans**. Les compteurs agrégés de cette exécution complète sont gardes 41 064, fenêtres 7 271 132, maximalité 388 596 ; ils couvrent davantage de travail et les passages successifs du parcours automatique. Ils ne représentent pas un unique compteur Work et ne se comparent pas directement à l’exécution interrompue.

Essais du helper réel sans débogueur ni compilation concurrente, un passage par version :

| Version | Entrée en reconnaissance | Fin | RSS maximale échantillonnée | Résultat |
| --- | --- | --- | --- | --- |
| G4 avant R-071 | 48,286 s | 62,914 s | 579,20 Mio | `recognition.too_complex` |
| R-071 | 49,747 s | 65,841 s | 593,56 Mio | `step.invalid_source_provenance` |

**Aucun STEP produit.** Le second refus est localisé dans `rounded_source_faces` : le résultat possède 501 912 faces et dépasse le plafond de suivi de 100 000, avant la construction du STEP. Ce contrôle utilisait le même message générique que les incohérences d’attribution. Le correctif de diagnostic conserve le code stable, le refus et le plafond, mais affiche maintenant le compte et la limite. Un test distingue la capacité dépassée d’une attribution réellement incohérente et vérifie le parcours normal. La frontière CAO applique indépendamment le même plafond (`fabstep-cad/src/lib.rs`, `MAX_RESULT_FACES`, constructeur mixte arrondi) : omettre le suivi source ne suffirait pas à rendre ce résultat admissible. La suite produit est suivie dans l’idée R-072 ; ni décimation ni relèvement de plafond ne sont intégrés.

Le lanceur de qualification conserve ses gardes à 300 s et 4 Gio, distinctes des 30 minutes du produit. Aucun gain global de durée n’est revendiqué, car le travail terminé diffère. Profilage temporaire retiré du code livré ; traces locales non versionnées `artifacts/r071/{stage-profile.log,stage-profile-after.log,flansch-before.json,flansch-after.json,partial-tests.log,benchmark.log,face-limit-test.log}`. Aucun contour ni STL privé n’est versionné.

### Contrôles finaux

`sh scripts/check.sh` termine avec code 0 : **496 tests workspace, 49 bureau, 8 helper, 68 ciblés avec `large-stl`, 1 transfert réel opt-in, 229 UI, 12 Python et 4 SDK réussis**. Formatage, Clippy strict des trois builds, contrôle bureau hors workspace, Svelte (0 erreur/0 avertissement) et build UI passent. Seize tests workspace opt-in restent ignorés par la suite ordinaire ; le nouveau benchmark de maximalité a été exécuté séparément avec succès, les anciens benchmarks ne sont pas rejoués dans ce lot. Aucun échec dans la suite finale. L’avertissement Vite connu sur la taille du bundle reste présent.

`sh scripts/build-macos-dev.sh` réussit. Les exécutables du paquet `artifacts/FabSTEP.app` sont identiques aux builds : SHA-256 bureau `15e88a1eb8e123a5bce566b887e7989daa7e7360e3a9638715b4c9c14482ca0f`, helper `208e2e4ea8f8a1d45e7400d03c4d9fcd72c5af854ed0b875a659b373b3695eb7`. Traces complémentaires : `artifacts/r071/{check.log,build-app.log,bundle-hashes.json}`. Aucun nouvel essai graphique ni Fusion ; paquet autonome v1 inchangé.


Rejeu final avec le helper réellement embarqué dans `artifacts/FabSTEP.app` après le correctif de message : **67,343 s**, reconnaissance à 51,025 s, RSS maximale échantillonnée 681,53 Mio ; refus `step.invalid_source_provenance` avec « Le modèle reconstruit contient 501912 faces ; le suivi des faces du résultat est limité à 100000. Aucun STEP n’est produit. ». Aucun rapport de succès ni STEP. Ce passage confirme le diagnostic final ; la variabilité entre passages et l’échantillonnage RSS ne permettent pas de conclure à un gain uniforme de temps ou de mémoire. Trace `artifacts/r071/flansch-final.json`. R-071 reste à tester via A-061.

## V-1196 — R-072 : qualification des résultats très facettisés (17 novembre 2026)

Qualification sur le fichier privé Flansch de 540 170 triangles, après R-071 (e29f1ad), sans modification de production. Deux explorations indépendantes en lecture seule ont examiné les réductions planes et les limites de la chaîne CAO ; l’orchestrateur a vérifié le code et effectué les mesures. Les instrumentations de tests ont été retirées et les trois fichiers Rust restaurés à l’identique. Aucun STL ni contour privé versionné.

### Origine des faces

Le profilage du parcours Auto mesure **480 024 plans avant `automatic::safe_planes`**. Ce garde remplace 21 854 régions par leurs 43 730 triangles, soit seulement **21 876 faces supplémentaires** ; les régions concernées ont de deux à quatre triangles. Le résultat final conserve 501 900 plans, quatre cylindres et huit cônes, soit 501 912 faces. L’hypothèse d’une grande région plane inutilement éclatée par ce garde est écartée : même supprimer tout son surcoût ne ramènerait pas le résultat sous 100 000 faces.

Deux segmentations diagnostiques du maillage entier, aux tolérances 2,5e-8 et 5e-8 mm, donnent respectivement **514 306 et 509 011 faces**, avec zéro contour dépassant le garde conservatrice `safe_planes` de 5e-8 mm, reproduite dans la sonde Rust (contrôle OCCT distinct via `Precision::Confusion()`). Durées isolées : environ 2,19 et 2,18 s. Elles incluent aussi les triangles reconstruits analytiquement par Auto : elles ne sont pas des alternatives équivalentes au résultat mixte. Ces essais gloutons ne prouvent pas un minimum théorique du nombre de faces. Ils ne justifient pas une nouvelle stratégie de regroupement en production.

Le classificateur de révolutions refuse quatre grandes zones de **64 080, 67 680, 73 442 et 73 440 triangles**, soit **278 642 triangles**, avec le motif « méridienne non affine sans raccord cylindre-plan démontré ». Chaque zone a deux contours circulaires ; les nombres d’arêtes de frontière correspondent aux contours attendus (720 ou 722). Cela ne prouve ni que leur méridienne est circulaire ni qu’un tore exact convient. Quatre autres zones de 2 160 triangles passent ce classement. La famille exacte des quatre zones refusées et le traitement des triangles restants demandent une qualification géométrique distincte.

### Limites indépendantes

| Étape | Contrôle actuel | Conséquence |
| --- | --- | --- |
| Suivi source/résultat (`fabstep-app`, `rounded_source_faces`) | 100 000 faces | Premier refus observé ; précède la validation native source. |
| Construction CAO (`fabstep-cad`, `MAX_RESULT_FACES`) | 100 000 faces | Retirer le suivi ne permet pas la construction mixte. |
| Audit du résultat et comparaison | 100 000 triangles, 300 000 points ; travail borné | Un résultat de 501 912 faces, chacune à trianguler, ne peut pas passer le plafond actuel. |
| Inspection du résultat | 100 000 faces/triangles, 300 000 points, 600 000 incidences | Capacité distincte ; un refus d’inspection laisse le STEP validé courant exportable. |
| Protocole helper | Rapport et STEP limités chacun à 128 Mio | Leur taille réelle n’est pas mesurée : aucun STEP n’a été construit. |
| Supervision produit | RSS 4 Gio, 30 minutes | Ces plafonds restent inchangés. |

La validation de la source (`fabstep-app::triangle_model`, `fabstep-cad::validate_source`) construit indépendamment un BRep par corps avec une face pour **chaque triangle du maillage analysé et soudé**, avec couture globale, contrôle du solide et des auto-interférences, volume et dimensions. Réduire les faces du résultat ne réduit pas automatiquement cette charge. La source du profil étendu reste plafonnée à 600 000 triangles et 1 800 000 sommets. Le budget d’audit de 50 000 000 unités est partagé entre les deux sens. Les limites de séparation multi-corps et de comparaison restent elles aussi en vigueur.

### Sonde native et mémoire

Compilation du test isolé réussie. Entrée en validation native à **49,748 s**, sur 540 170 faces ; arrêt manuel à **241,622 s** (code -9), sans résultat de validation ni STEP. Le superviseur expérimental échantillonnait le RSS par `ps` environ toutes les 100 ms et imposait 300 s depuis le lancement, analyse incluse, ou 4 Gio RSS. Pic RSS observé : **2 523 103 232 octets (2,35 Gio)**. Ses deux seuils n’ont pas déclenché : `stop_reason: null` dans le JSON doit être lu avec `manual-stop.txt`, qui consigne l’arrêt par l’orchestrateur.

Un échantillonnage `sample` de trois secondes, vers 205 s, rapporte une **physical footprint de 6.9G** (notation de l’outil), bien supérieure au RSS. Ce sont des métriques différentes : le RSS ne suffit pas à borner cette empreinte sur ce Mac. Le produit surveille également `resident_size` via `PROC_PIDTASKINFO` (`fabstep-desktop/src/supervisor.rs`), mais la sonde n’exécutait pas le superviseur produit. Il faut qualifier la mesure d’empreinte physique et son coût avant d’en déduire une correction produit. Aucun dépassement du plafond RSS produit ni défaut d’annulation graphique n’est établi.

La pile échantillonnée se trouve dans `measure_solid` → `BRepCheck_Analyzer` → contrôle des arêtes (`BRepLib_ValidateEdge`). Cela localise le travail pendant ces trois secondes, pas toute la durée de validation. La validité finale de la source, sa durée complète et son pic mémoire final restent inconnus. Cette exécution instrumentée et interrompue ne permet aucune comparaison de performance avant/après. Traces complémentaires : `source-sample.txt`, `manual-stop.txt`.

### Décision proportionnée

Aucun regroupement local utile n’est démontré pour franchir le plafond ; aucune hausse de limite, décimation ou nouvelle primitive intégrée. La prochaine investigation recommandée est de qualifier la surveillance de l’empreinte mémoire, mesurer les méridiennes des quatre zones refusées et profiler les étapes de validation native source. Il faudra ensuite proposer séparément toute extension de familles ou changement de validation, avec ses preuves, budgets et arbitrage utilisateur. Augmenter seulement le plafond de suivi déplacerait le refus vers d’autres contrôles.

### Reproductibilité et portée

Traces locales : `artifacts/r072/planes-profile.log`, `patch-profile.log`, `source-build.jsonl`, `source-native.log`, `source-capacity.json`. Les copies `automatic-instrumented.rs`, `rounded-instrumented.rs`, `cad-instrumented.rs` conservent l’instrumentation ; `guard_source.py` lance le test isolé avec une limite externe de 300 s et 4 Gio. Les profils core sont des tests ignorés activés explicitement, avec `FABSTEP_PROFILE_STL` ; ils terminent en environ 72 et 71 s. Le test natif utilise le profil `large-stl` et le même modèle triangulé que l’application, sans produire de STEP. Ce n’est pas un benchmark statistique ni une mesure de la conversion complète.

Le code de production est identique à e29f1ad ; la suite globale V-1195 n’est pas relancée pour cette qualification documentaire. Aucun nouveau build d’application, contrôle graphique ou réimport Fusion. `artifacts/FabSTEP.app` reste le build R-071 ; Flansch reste refusé sans STEP. La qualification ne revendique pas une conversion réussie.

## V-1197 — R-073 : RSS et empreinte physique du helper macOS (17 novembre 2026)

Le superviseur utilise désormais `proc_pid_rusage(RUSAGE_INFO_V0)` et refuse si **la mémoire résidente ou l’empreinte physique** atteint 4 Gio. Le contrôle du RSS est conservé. Le code `worker.memory_limit`, les 30 minutes, l’échantillonnage chaque seconde, le repli sur `try_wait` si l’enfant vient de sortir et le nettoyage avant suppression du dossier privé restent inchangés. Aucun plafond géométrique relevé, aucune nouvelle dépendance. `worker.monitor_failed` reste bloquant si la lecture échoue ; aucun repli silencieux sur le RSS seul.

### Contrat natif et mesures

Le SDK local (`sys/resource.h`, `libproc.h`) définit `rusage_info_v0` : UUID de 16 octets et dix `u64`, taille 96, alignement 8, offsets RSS 64 et empreinte 72. Assertions C compilées avec Clang contre les headers, assertions Rust constantes sur les mêmes valeurs. Le tampon direct est transmis selon le typedef opaque de l’API, sans indirection supplémentaire. L’API est disponible depuis macOS 10.9 ; cible FabSTEP macOS 26. Aucun portage Windows/Linux revendiqué.

Sonde indépendante sur un enfant Python, allocation touchée de 64 Mio : empreinte **5 833 136 → 73 056 736 octets**, RSS **14 041 088 → 81 231 872 octets**. L’empreinte augmente de plus de 60 Mio ; l’enfant est tué et récolté à la fin. Dix mille lectures successives prennent en moyenne **0,764 µs par lecture**, boucle Python comprise, sur un passage local ; ce n’est ni un benchmark statistique ni une garantie de coût sous pression. Cette sonde ne cherche pas à provoquer de compression ou à atteindre 4 Gio. Traces locales : `artifacts/r073/memory_probe.c`, `qualify_memory.py`, `memory-measures.json`.

### Non-régression

Les tests couvrent la limite moins un, la limite exacte, le dépassement avec RSS faible et empreinte haute, et RSS haut avec empreinte faible. La lecture native est exercée sur soi et sur un vrai enfant ; les PID hors domaine et inexistant sont refusés. Le chemin réel d’attente reçoit ensuite des mesures contrôlées ou une panne de lecture : chaque cas tue et récolte un vrai enfant, libère la réservation, conserve le dossier jusqu’à la récolte et ne publie aucun résultat. Aucun test n’alloue plusieurs Gio.

Les **12 tests ciblés du superviseur réussissent** (un transfert réel opt-in ignoré lors de cette exécution ciblée). Revue indépendante de l’ABI, du diff et des chemins d’erreur : aucun défaut relevé. Les essais d’annulation, d’invalidation, d’expiration et de nettoyage pendant unwind restent couverts.

### Portée

Le seuil est une observation périodique, pas une limite matérielle : le pic peut dépasser 4 Gio entre deux lectures ou pendant l’arrêt. Le RSS et l’empreinte physique sont des comptabilités différentes ; aucune relation d’ordre entre elles n’est supposée. L’essai source privé R-072 n’est pas relancé pour provoquer une allocation massive. Il n’y a donc pas de mesure d’arrêt à 4 Gio sur Flansch, ni de preuve nouvelle de sa convertibilité. Son refus de 501 912 faces demeure en amont de la validation source. Aucun essai graphique/Fusion nouveau ; le comportement d’annulation dans le build mis à jour reste proposé à l’utilisateur.

### Contrôles globaux et paquet

`sh scripts/check.sh` termine avec code 0 : **496 tests workspace, 50 bureau, 8 helper, 68 ciblés large-stl, 1 transfert réel opt-in, 229 UI, 12 Python et 4 SDK réussis**. Formatage, Clippy strict, contrôle bureau, Svelte (zéro erreur/avertissement) et build UI passent. Les 16 tests workspace opt-in restent ignorés par la suite ordinaire ; les benchmarks et fichiers privés ne sont pas rejoués pour ce correctif. L’avertissement Vite connu sur la taille du bundle demeure. Journaux : `artifacts/r073/{targeted.log,check.log,build.log}`.

`sh scripts/build-macos-dev.sh` réussit ; les exécutables embarqués correspondent octet pour octet aux builds contrôlés. SHA-256 bureau `055918cb118617a159ae81e6465b585aa48a1abf45ac8c020a33647ca581f585`, helper `1bab9ac76c82f4e43f3a6576a8fd83d0fb7c295548b1d141876f25851d86205f`. Paquet de développement : `artifacts/FabSTEP.app`. Release v1 inchangée. R-073 attend l’essai manuel d’annulation A-063.

## V-1198 — R-074 : profils des quatre grandes zones courbes de Flansch

Qualification du 17 novembre 2026 sur le même STL privé de 540 170 triangles. Aucun changement de production ; un test natif synthétique est conservé dans le module de tests CAO. Instrumentation temporaire du classificateur arrondi et du résultat Auto, retirée ensuite du core ; aucun sommet ni contour privé versionné. Les données complètes restent dans `artifacts/r074/`. Le but est de choisir une extension prouvable, sans déduire un solide valide d’un ajustement de courbe.

### Mesures

Une sonde core `large-stl` analyse la source et termine Auto avec les tolérances par défaut : ajustement 1e-5 mm et écart maximal 0,05 mm. **Un test opt-in réussi**, 63,37 s selon le harness, 63,95 s avec supervision ; RSS maximal échantillonné **673 480 704 octets**, empreinte physique **549 127 056 octets**. Garde externe à 300 s et 4 Gio sur le maximum des deux métriques, lecture toutes les 200 ms. Aucun appel de validation native du maillage Flansch. Résultat inchangé : 501 900 plans, quatre cylindres, huit cônes, aucun STEP.

Les quatre zones sont constituées de 90, 95, 103 et 103 anneaux. Tous leurs sommets appartiennent aux niveaux recensés et chaque triangle relie deux niveaux consécutifs. Les découpes ci-dessous sont choisies pour le diagnostic ; aucun algorithme de segmentation automatique nouveau n’est intégré. Les arcs sont ajustés sur tous leurs sommets dans le plan axial/radial, avec cercle contraint à passer par les deux anneaux moyens extrêmes de la portion ; les parties droites sont contrôlées sur tous leurs sommets. L’ajustement ne force pas les rayons à une valeur nominale.

| Zone (triangles) | Profil candidat mesuré | Rayon de l’arc | Amplitude de l’arc | Résidu maximal de l’arc |
| --- | --- | --- | --- | --- |
| 64 080 | Tore, cône de pente −1/15, cylindre | 0,50000049 mm | 86,18597° | 5,8613e-6 mm |
| 67 680 | Tore, cylindre, cône de pente +1, cylindre, cône de pente −0,75 | 2,00000058 mm | 90,00000° | 5,9438e-6 mm |
| 73 442 | Cône à +10°, tore | 0,29999767 mm | 100,00052° | 6,2784e-6 mm |
| 73 440 | Cône à −10°, tore | 0,29999791 mm | 100,00035° | 6,1835e-6 mm |

Chaque portion affine n’a que **deux anneaux**, sans niveau axial intérieur. Son résidu radial maximal est inférieur à 5,638e-6 mm. Les arcs contiennent respectivement 62 640, 64 800, 72 720 et 72 720 triangles. Le cône de la zone 73 442 a 722 triangles et raccorde 362 à 360 sommets ; les autres bandes affines ont 720 triangles. Les écarts radiaux sur les anneaux complets sont également mesurés, mais ne doivent pas être confondus avec une borne vectorielle cumulée sur toutes les frontières.

Un unique cercle ajusté sur chaque zone entière donne des résidus maximaux de 0,283219, 2,891604, 0,053548 et 0,053544 mm : cet ajustement global ne satisfait pas la tolérance. Les profils composés expliquent les refus ; cela n’établit pas une optimalité mathématique de la segmentation ni un minimum théorique de faces.

### Gain possible et limite indépendante

Les zones possèdent respectivement **62 177, 65 394, 71 313 et 71 432 plans** dans le résultat Auto, soit **270 316 plans distincts**, sans propriétaire partagé avec l’extérieur. L’orchestrateur a revérifié les identifiants : 278 642 triangles distincts, sommes des sections exactes, attribution et intersection des ensembles de plans cohérentes.

Il reste **231 584 plans et 12 faces analytiques hors des quatre zones**. Même leur remplacement idéal par les douze surfaces candidates laisserait **231 608 faces**. Ce calcul conditionnel n’est pas une conversion ; il montre que cette extension seule ne peut pas franchir le plafond de 100 000 faces. Les coûts de validation native source restent aussi indépendants (V-1196).

### Pourquoi le moteur refuse

`rounded::classify` découpe principalement aux portées cylindriques et admet les tores dans les raccords quart de cercle cylindre–plan déjà prouvés. Les contrats de R-061/ADR-0033 n’autorisent pas ces enchaînements mixtes. Les témoins axiaux intérieurs exigés pour certaines chaînes ne sont pas présents dans les parties droites de ce fichier ; ils ne peuvent pas être inventés.

Le DTO `MixedTorus` et `fabstep-cad::toroidal_face` savent représenter un arc méridien général borné avec ses deux anneaux. La preuve core reste plus étroite : `prove_section` associe les niveaux axiaux bas/haut aux angles croissants. Sur la branche de tore de rayon radial inférieur au rayon majeur, ces ordres sont opposés. Le miroir d’un arc de 100° traverse aussi la coupure numérique de `atan2` à ±π. Une correction par simple ajout de familles ou suppression du refus serait donc insuffisante.

La prochaine tranche proposée, R-075, traite uniquement le profil tore–cône–cylindre de 64 080 triangles et sa symétrie : segmentation unique, tangence calculée conjointement, anneaux réellement partagés, orientation et couverture UV adaptées, preuve des bandes à deux anneaux, borne continue puis validations natives et audit. Aucun nouveau champ ou ABI n’est nécessaire d’après la revue, mais une nouvelle règle de preuve est nécessaire. Les deux cônes et les arcs de 100° restent des extensions distinctes. Le guidage Tore, qui refuse actuellement un voisin conique, reste hors de cette première tranche.

### Prototype natif indépendant

Le test synthétique construit directement un `RoundedModel` avec deux plans, un tore R=10 mm / r=0,5 mm sur 86,1859°, un cône tangent de pente −1/15 et un cylindre, sans reprendre les coordonnées de Flansch. Le tore utilise des anneaux en ordre méridien inverse de l’ordre axial. Le cas nominal et son reflet axial sont exportés puis réimportés par OCCT 7.9.3 : chacun donne **un solide, cinq faces (deux planes, une cylindrique, une conique et une torique), cinq jetons de provenance**, et des dimensions 20 × 20 × 5 mm.

Volumes mesurés : **1393,237968865069 et 1393,237968865016 mm³**, contre **1393,237968865100 mm³** par intégration indépendante. Le test natif ciblé initial passe (0,10 s). Ce test qualifie la représentation et la construction, pas la détection STL, la validation de la source privée ou sa conversion complète. Journal initial `artifacts/r074/native-run.log`. Le test est ensuite conservé dans `fabstep-cad` pour protéger ce contrat lors de la prochaine extension.

### Reproductibilité et limites

Traces locales : `probe.log`, `usage.json`, `instrument.py`, `run.py`, `whole-fits.json`, `pieces.json`, `analyze.py`, `pieces.py`, fichiers `.nodes/.vertices/.triangles` et `owners`. Analyse dans l’environnement existant `/Users/mugen/anaconda3/bin/python3`, NumPy 2.3.5 et SciPy 1.16.3 ; aucune dépendance du projet ajoutée. Le graphe `profiles.png` représente les anneaux mesurés, avec des échelles différentes par panneau et des segments de lecture, sans prétendre montrer des surfaces reconstruites.

Les résidus de sommets ne prouvent ni l’écart continu sur les triangles, ni la tangence exacte, ni la validité de l’orientation ou du BRep. La canonicalisation conjointe des raccords doit encore compter tous les déplacements. Aucun relâchement de tolérance, simplification, nouvelle surface libre, hausse de plafond ou nouveau build produit pour cette qualification. La suite globale V-1197 n’est pas relancée pour un code de production inchangé ; aucun test graphique ou Fusion.

### Validation du test conservé

`cargo test -p fabstep-cad --offline --locked` : **85 tests réussis, zéro échec**. Formatage workspace et Clippy de tous les targets de `fabstep-cad` passent. Le test `inner_meridian_torus_cone_cylinder_reimports_with_shared_rings` reste dans `crates/fabstep-cad/src/lib.rs`. L’orchestrateur a relu le diff et rejoué ce test avec succès (0,11 s). Traces `cad-tests.log` et `root-native-check.log`. Aucun code de production modifié ; seuls le test, la documentation et la roadmap sont intégrés.

## V-1199 — R-075 : reconnaître le raccord tore–cône–cylindre

Portée approuvée par « c’est parti pour R-075 » après V-1198. Nouvelle reconnaissance en Auto et Surfaces, sans changement de format, d’ABI, de dépendance, de tolérance ou de plafond. Décision et limites : ADR-0035.

### Preuves et non-régression ciblée

Le chemin historique reste prioritaire. Après son refus géométrique, le nouveau chemin restaure les mesures des anneaux source et cherche dans les deux sens une seule chaîne plan–tore–cône–cylindre–plan. Plan, cône et portée déterminent conjointement le cercle tangent ; l’anneau de jonction est contrôlé en déplacement vectoriel et les paramètres finaux sont partagés. La branche radiale du tore détermine l’ordre méridien de ses anneaux ; les portions droites gardent leur ordre axial. Chaque triangle doit relier deux anneaux consécutifs et appartient à une seule portion. Les preuves UV, de résidu vectoriel et d’écart continu restent obligatoires. Un second découpage plausible ou un budget épuisé refuse le candidat.

Le corpus indépendant `fixtures/torus_cone_cylinder/` est reproductible par `scripts/generate-torus-cone-cylinder-fixtures.py`. Il couvre le nominal, son reflet axial, la branche radiale opposée, une permutation d’axes exacte, une faible perturbation du cercle terminal, une rotation oblique en 3MF et deux occurrences avec placement/reflet. Les volumes et dimensions attendus viennent du générateur analytique indépendant ; le nominal mesure 20 × 20 × 5 mm, pour 1393,2379688651004 mm³. Les solides attendus comptent deux plans, un tore, un cône et un cylindre. Provenance exacte, adjacences et audit échantillonné bidirectionnel sont vérifiés ; les résultats conservent les sommets source dans leur modèle et comptent les déplacements analytiques dans le rapport.

Les cas non tangent, sans anneau de jonction, avec triangle traversant la frontière et avec déformation hors tolérance restent refusés en Surfaces et ne reçoivent pas d’attribution torique en Auto. Leur topologie est fermée ; la CLI historique les a réimportés en solides facettisés avec une tolérance plane de 1e-9 mm. Un budget d’écart de 0,001 mm refuse également le nominal, malgré son ajustement aux sommets. Les tests internes distinguent cette limite géométrique de l’épuisement de Work après un premier candidat admissible et refusent deux débuts de portée plausibles.

La rotation arbitraire du STL en f32 a d’abord échoué au contrôle natif de coplanarité. Le cas original est conservé dans `rotated_f32_nonplanar.stl`, avec un test de refus `cad.invalid_model` en Surfaces. Aucun garde de planéité n’est assoupli. Les rotations positives sont testées séparément avec une permutation STL exacte et une transformation oblique 3MF f64 qui conserve les plans. Cette distinction ne promet pas la reconstruction d’un contour déformé par quantification.

Un perçage traversant dans un cylindre de rayon 12 mm et hauteur 5 mm ajoute une preuve native pour les normales de cavité : 2 176 triangles, un solide réimporté, six faces (deux plans, deux cylindres, un cône, un tore), volume attendu **868,7087417195507 mm³**, dimensions 24 × 24 × 5 mm. Auto et Surfaces passent, avec attribution exhaustive des six faces en Auto ; `through_bore_reimports_with_the_cavity_orientation_and_complete_face_partition` a été rejoué par l’orchestrateur avec succès. La topologie source possède deux incidences opposées par arête et une caractéristique d’Euler nulle. Les quatorze fichiers du corpus sont identiques après régénération. Traces `bore-test.log` et `corpus-checks.log`.

Le guidage Tore avec voisin conique reste refusé ; un test applicatif le vérifie. Les chaînes à deux cônes et les arcs de 100° restent hors de R-075.

### Budget de travail et essai privé

Le premier helper, avant optimisation du parcours, refusait Flansch avec `recognition.too_complex` en 64,507 s. La trace localise l’épuisement dans la recherche d’arcs historique (`partial.rs`), après classement des 64 080 triangles en trois sections et entrée de la preuve torique à 5 927 472 unités du budget commun. Le budget n’a pas été relevé et les prédicats de recherche d’arcs ne sont pas modifiés.

Le parcours des composantes remettait le triangle courant dans la pile par chacune de ses arêtes non coupées. Après validation de la topologie fermée, seule l’autre incidence est désormais ajoutée. Un test compare les membres triés et les dépilements de l’ancien et du nouveau parcours pour tous les départs d’un cube et plusieurs ensembles de coupes. Le compte vérifie **visites anciennes = 2 × visites nouvelles − 1** (73 → 37 sur le cube sans coupe) ; chaque visite d’un vrai voisin reste facturée. C’est du travail effectivement supprimé, pas une baisse arbitraire de son coût. Aucune amélioration de latence globale n’est revendiquée : les essais avant/après ne terminent pas le même travail.

Le helper propre atteint ensuite **439 738 faces**, contre 501 912 auparavant, et refuse par `step.invalid_source_provenance` au plafond de 100 000. Mesure isolée : **66,561 s**, RSS maximal échantillonné **648 151 040 octets**, empreinte physique **472 712 248 octets**. Garde externe de qualification à 300 s et 4 Gio sur le maximum des deux mesures. **Aucun STEP, aucun rapport de succès et aucune validation native de la source Flansch** : le contrôle de capacité du résultat intervient avant. Trace `artifacts/r075/flansch-optimized.json`.

Une sonde distincte vérifie l’attribution des 64 080 identifiants source de V-1198 : **62 640 triangles vers un tore, 720 vers un cône, 720 vers un cylindre**, sans propriétaire partagé avec l’extérieur de la zone. Le résultat global compte **439 723 plans, cinq cylindres, neuf cônes et un tore**. Les 62 177 anciens plans de cette zone sont remplacés par trois faces, soit **62 174 faces de moins**. Rayon majeur du tore 37,4677767947 mm ; rayon mineur 0,5000001697 mm ; amplitude 86,185924481°. Le paramètre `inner=false` correspond à son orientation paramétrique prouvée, pas à une déduction du nom de la pièce.

Le rapport Auto de cette sonde donne un résidu radial maximal de **0,000006199570111 mm**, une borne de corde maximale de **0,00150221237936 mm** et une borne composée de **0,00151849420402 mm** pour un budget de 0,05 mm. Ces bornes core ne remplacent pas la validation native et l’audit de l’export complet, non exécutés sur cette source privée. La sonde passe en 68,47 s (69,54 s surveillés), RSS 668 909 568 octets, empreinte physique 508 199 848 octets. Traces `owner-probe.log` et `owner-usage.json`. Instrumentation retirée ; aucun fichier privé versionné.

### Validation finale et paquet

La revue indépendante du détecteur, de ses preuves, du budget et de l’orientation conclut PASS sans défaut bloquant. L’orchestrateur a relu les changements et rejoué les suites ciblées : 14 tests internes rounded (un benchmark ignoré), trois tests core et sept tests applicatifs, dont le perçage traversant ajouté en complément.

`scripts/check.sh` termine avec code 0 : **510 tests workspace, 50 bureau, 8 helper, 74 ciblés large-stl, un transfert réel, 229 UI, 12 Python et quatre SDK**. Les seize tests workspace opt-in restent ignorés dans cette suite ; le transfert réel du helper est exécuté séparément. Formatage, Clippy strict, Svelte (zéro erreur/avertissement), builds UI et contrôle bureau passent. L’avertissement Vite déjà connu sur la taille du bundle demeure. Trace : `artifacts/r075/check.log`.

`scripts/build-macos-dev.sh` termine avec code 0. Le paquet `artifacts/FabSTEP.app` contient les deux exécutables reconstruits, identiques octet pour octet aux sorties vérifiées : SHA-256 bureau `246284674a12c0b2ece55fdb1f4aa57c216ad56acb7081560479743c18e2f15f`, helper `cf837b535a105b067b6a1f2a42b7d79bad9c8cf793d457f645d656a1a7f05c3f`. Traces `build-app.log` et `bundle-hashes.json` dans `artifacts/r075/`. La release 1.0.0 reste inchangée.

Aucun nouvel essai graphique ni Fusion exécuté : R-075 reste à tester, actions A-065 et A-066. Les autres chaînes et le coût de validation native de Flansch restent ouverts ; cette livraison ne produit pas de STEP complet pour ce fichier.


## V-1200 — R-063 : certificat complet du laboratoire de perçages sécants

Poursuite globale autorisée après R-075. Le laboratoire de V-1187 passe désormais sur la même référence fixe : cube de côté 20 mm, perçages centrés orthogonaux de rayons 3 et 6 mm. Aucun code de conversion ni contrat de production activé. Les seuls changements de calcul sont dans `scripts/cross_drill_certificate.hxx` et `scripts/qualify-cross-drill.cpp`.

La borne entre courbe 3D et courbe paramétrique relevée utilise l’interpolation d’Hermite aux extrémités et un reste d’ordre quatre majoré par intervalles. Les splines périodiques sont déroulées depuis leurs nœuds et pôles originaux ; aucun réajustement flottant non borné. La couverture des cycles orientés inclut les arcs de jonction dans une somme d’intervalles isolant le degré ±1. Les 56 opérations supplémentaires de cette couverture sont comptées dans le budget partagé de dix millions, qui reste inchangé.

Réimport : un solide, six plans, trois faces cylindriques et 36 sondes matière/vide conformes. À 1e-5 mm, les trois arêtes sont certifiées en **208 feuilles et 2 005 451 opérations** ; borne pcurves ≤ 9,65528e-6 mm, branches ≤ 7,45230e-8 mm. À 1e-7 mm, **605 feuilles et 5 489 909 opérations**, pcurves ≤ 9,97982e-8 mm et branches ≤ 7,06643e-8 mm. Ces certificats portent sur les courbes réellement réimportées et leurs paramètres, pas sur la reconstruction d’un STL.

Le volume idéal possède désormais un encadrement par série binomiale à 20 termes, avec reste majoré et opérations par intervalles : **[5500,8952492145254 ; 5500,895249214539] mm³**. Le volume natif adaptatif mesuré vaut **5500,895249460223 mm³** ; sa comparaison reste un contrôle numérique à la tolérance du laboratoire, pas un certificat du calcul OCCT. Le volume non adaptatif **5500,898895086297 mm³** reste distinct et ne convient pas à la garde actuelle de production. R-036 devra donc résoudre explicitement ce contrat de volume.

Les contrôles indépendants passent en 11 256 opérations : arithmétique, splines exactes, corruption intérieure d’Hermite, périodique cubique non uniforme comparée à OCCT D0 aux nœuds et milieux, couverture et tour doublé. Les adversaires natifs passent également : pôle 3D déplacé, pcurve décalée, couture périodique modifiée, poids invalide et budget épuisé. `--inspect-only` ne revendique aucun certificat.

Revue mathématique indépendante PASS après les corrections de comptage et les témoins périodiques. Rejeu natif de l’orchestrateur : sortie 0, rapport identique octet pour octet au passage de l’auteur (`artifacts/r076-f0/root/report.json`). Les traces de compilation, exécution, replay et inspection restent dans `artifacts/r076-f0/`. Pas de nouvelle suite Cargo pour ce laboratoire isolé, pas d’essai Fusion ni de modification de FabSTEP.app pour R-063 seul. L’intégration source/core/CAD R-036 reste à développer.


## V-1201 — R-076, R-077, R-078 : chaînes composées et bac percé

Poursuite globale C1b2/C2/D/E/F/G autorisée après R-075. Les extensions de cette entrée conservent les DTO, l’ABI, les tolérances, les plafonds et les contrôles natifs existants. Les décisions de reconnaissance sont dans ADR-0037. Les laboratoires F et D/G restent distingués dans V-1200 et V-1202.

### C2 : raccords composés et arcs dépassant 90°

`rounded/torus_cone/terminal.rs` admet une chaîne plan–quart de tore–cylindre–cône–cylindre–cône–plan avec deux cônes opposés, et une chaîne plan–tore–cône–plan dont l’arc est obtus, inférieur à 180° et axialement monotone. Les paramètres de tangence sont calculés conjointement ; les frontières sont des anneaux source réellement présents. L’unicité du découpage, le témoin du retournement radial, l’orientation UV, les déplacements vectoriels et la borne continue restent exigés. Les historiques et R-075 gardent la priorité.

Le corpus `fixtures/revolution_chains/`, produit indépendamment par `scripts/generate-revolution-chain-fixtures.py`, couvre les deux branches radiales, les reflets, une rotation oblique ASCII conservant la précision, et un raccord entre anneaux de 360 et 362 sommets avec couture proche de π. Huit cas positifs et cinq cas négatifs fermés/orientés : frontière absente, triangle traversant la jonction, déformation, défaut de tangence, bande terminale non résolue. Un budget de déviation insuffisant reste refusé. Trois tests core et trois tests applicatifs passent ; le réimport contrôle les types de surfaces, volume et dimensions analytiques, audit bidirectionnel, adjacences et partition exacte des indices source. Revue indépendante du calcul et du raccord PASS ; les régressions R-075 sont conservées.

### C1b2 : chaînes adjacentes et mixtes

`revolved/adjacent_chain.rs` cherche deux à huit cônes avec portions cylindriques éventuelles, après les succès historiques. Chaque portion nouvelle possède deux témoins intérieurs résolus et ses ruptures utilisent des anneaux complets. Toutes les décompositions concurrentes admissibles comptent dans l’unicité ; un dépassement du domaine de huit cônes rend toute cette nouvelle recherche non concluante, même après un premier candidat. Un épuisement du budget de travail reste une erreur bloquante. Primitives et Surfaces/Auto partagent cette règle, puis leurs preuves de paroi existantes.

Le corpus indépendant `fixtures/adjacent_chains/` couvre trois et huit cônes, les signes variés, les portées terminales et internes, reflet et rotation, puis les refus de courbure, témoins manquants, frontière traversée, pente non résolue, dépassement du nombre de cônes et faible budget. Les trois tests natifs vérifient notamment le solide réimporté et la provenance exacte de la chaîne mixte. Le cas historique à trois cônes suffisamment échantillonnés est déplacé sans changement de source de la liste négative vers les positifs ; ses autres refus restent inchangés.

La revue a démontré qu’un raccourci de préfiltrage par intervalles de pente en f64 pouvait écarter une décomposition à rayon 1e8 mm et tolérance 1e-12 mm, alors que les évaluations finales donnaient un résidu nul. Ce raccourci est supprimé, avec régression dédiée ; aucune monotonie non prouvée n’est utilisée. Les rayons mesurés sont conservés avant changement d’ordre des sommets et canonicalisation. Revue finale PASS. Après correction : trois tests internes, deux nouveaux tests core, six tests adjacents historiques core, huit tests de chaînes historiques core et trois nouveaux tests applicatifs passent (22 contrôles), plus les suites natives historiques adjacentes et chaînes précédemment réussies (2 + 3).

La première suite globale a trouvé un changement involontaire sur `same_sign_chains/curved_meridian.stl` : malgré son nom, le générateur contient neuf cordes affines avec témoins intérieurs. Le nouveau plafond de huit cônes remontait une erreur bloquante, supprimant le repli Auto historique. La correction distingue le domaine géométrique non pris en charge du véritable épuisement de travail ; aucune chaîne de neuf cônes n’est acceptée. Rejeu ciblé réussi : test historique avec conservation effective du STEP Auto (un solide, zéro cône), trois tests du solveur, deux tests core et trois tests natifs du nouveau corpus, soit neuf contrôles. Les fichiers source historiques et leurs erreurs attendues restent inchangés ; les tests du nouveau cas à neuf cônes contrôlent le refus géométrique sans confusion avec un budget épuisé. Journaux `c1-cap-regression.log` et `c1-cap-new-corpus.log`.

### E2 : perçage du fond d’un bac ouvert

Le moteur existant suffit, sans modification de production propre à ce lot. `scripts/generate-pierced-tray-fixtures.py` construit un bac 40 × 30 × 12 mm, fond de 3 mm et trou de rayon 2 mm, avec quatre coins extérieurs et quatre intérieurs arrondis. Le nominal possède 3 456 triangles. Les sources nominale, tournée et au trou déformé sont fermées à deux incidences opposées par arête ; le négatif à frontière manquante possède trois arêtes ouvertes. Cinq fichiers sont identiques après régénération.

Deux tests applicatifs vérifient onze plans et neuf cylindres, un solide réimporté, le volume analytique du bac diminué de 12π mm³, les dimensions, le rayon/les deux centres du perçage et l’attribution native exhaustive. Les sondes vérifient le vide traversant, le fond restant, la cavité ouverte et une paroi, sur la source et sur la tessellation STEP. La frontière absente est refusée `mesh.open`, le trou déformé ne reçoit pas de faux cylindre complet. Une première triangulation de laboratoire avait des connexions annulaires croisées ; elle a été corrigée dans le générateur avant qualification. Les contours finaux suivent des rayons cohérents et subdivisent les longs côtés pour résoudre la tessellation du trou. Aucun contrôle du moteur n’a été relâché.

### Budget et fichier privé Flansch

Les premières nouvelles reconnaissances sont valides sur le corpus mais épuisent le budget de dix millions dans la recherche historique d’arcs (`partial.rs`). Le parcours Auto reconstruit désormais une fois les ensembles d’arêtes des anneaux et compare leur égalité exacte via un index. Les doublons et l’ordre des supports sont conservés ; un test oracle couvre rotations, inversions, sommets identiques avec connexions différentes et épuisement. Les mesures des rayons sont retenues plutôt que recalculées. La dernière entrée de preuve torique passe de 6 721 591 à 6 241 358 unités dans la trace correspondante ; ce premier allègement reste insuffisant.

Le préfiltre sphérique réservait 563 048 unités pour les quatre patchs, alors qu’il n’examinait respectivement que 361/32 400, 1/34 200, 363/37 082 et 721/37 080 sommets avant rejet. Il débite maintenant une unité de préparation et quatre par sommet réellement visité, avant son examen : 5 788 unités pour ces passages, soit 557 260 unités non réservées pour du travail non exécuté. Le plafond ne change pas ; un candidat complet coûte 4N+1. Trois tests ciblés couvrent paramètres, candidat positif, rejet premier/intermédiaire/dernier sommet, épuisement avant le point décisif et non-réutilisation d’un budget épuisé. Cette correction de décompte n’est pas annoncée comme un gain de vitesse.

Le helper propre du fichier privé à 540 170 triangles atteint **231 608 faces**, puis refuse `step.invalid_source_provenance` au plafond de 100 000. **Aucun STEP, aucun rapport de succès et aucune validation native source Flansch**, le contrôle de capacité intervenant avant. Mesure isolée : **70,862 s**, RSS maximal échantillonné **503 447 552 octets**, empreinte physique **500 646 944 octets**, garde de qualification 300 s/4 Gio. Pas de gain global de latence revendiqué : les passages avant/après ne terminent pas le même travail. Trace `artifacts/r076/flansch-final.json`.

La sonde indépendante d’attribution passe dans le budget ordinaire de reconnaissance et contrôle l’absence de propriétaires communs avec l’extérieur de chaque zone :

| Triangles source | Surfaces et attribution |
| --- | --- |
| 64 080 | Tore 62 640, cône 720, cylindre 720 |
| 67 680 | Tore 64 800, deux cônes de 720 et deux cylindres de 720 |
| 73 440 | Tore 72 720, cône 720 |
| 73 442 | Tore 72 720, cône 722 |

Le résultat global contient **231 584 plans, sept cylindres, treize cônes et quatre tores**. Les quatre zones forment douze surfaces, sans frontière partagée avec un propriétaire extérieur. Borne composée core **0,001854212187174463 mm**, résidu radial maximal **0,000006802234830081498 mm**, borne de corde **0,0018373276977969562 mm**, budget 0,05 mm. Ces preuves ne remplacent pas l’audit ni la validation native du document entier, non atteints. Sonde : 70,91 s (71,289 s surveillés), RSS 530 874 368 octets, empreinte 468 124 512 octets. Traces `owner-probe.log`, `owner-usage.json` ; instrumentation retirée et aucun fichier privé versionné. La qualification des plans restants est suivie dans R-080.

Rejeu après le correctif C1 et reconstruction du paquet : même refus à **231 608 faces**, aucune sortie STEP/rapport, **69,359 s**, RSS 532 987 904 octets, empreinte physique 457 475 080 octets. Aucune garde temps/mémoire déclenchée ; trace `artifacts/geometries-next/flansch-confirmed.json`.

### Validation globale et paquet

Après correction du repli Auto, `sh scripts/check.sh` termine avec le code 0 : **530 tests workspace, 50 bureau, huit worker, 88 ciblés large-stl, un transfert réel vers le worker, 229 UI, douze Python et quatre SDK**. Formatage, Clippy avec `-D warnings`, contrôle Svelte (zéro erreur/avertissement), build UI et contrôle bureau passent. Seize tests opt-in du workspace sont ignorés par la suite ordinaire ; le transfert réel bureau est exécuté explicitement en fin de script. Le sous-ensemble large-stl conserve quatre tests opt-in ignorés. Avertissement Vite connu sur la taille du bundle, sans erreur. Journaux `artifacts/geometries-next/check.log` (premier échec documenté) et `check-final.log` (rejeu complet réussi).

`sh scripts/build-macos-dev.sh` réussit. Les deux exécutables de `artifacts/FabSTEP.app` sont identiques à leurs builds vérifiés : SHA-256 bureau `e581cccf7ba074c5a300d8585369bba520932709f8e88aa0ac574234389efed2`, worker `8782f288acf789376a04dfc4ad45df6b686b4e1697a148f5694d30e194a98485`. Empreintes dans `bundle-hashes.json`. Aucun nouvel essai graphique natif ni Fusion, aucune mise à jour de la release 1.0.0. R-076/R-077/R-078 restent à tester via A-069 à A-074. Les brouillons D restent séparés et ne font pas partie de cette qualification.


## V-1202 — R-079 : références natives des raccords locaux D/G

Laboratoire borné dans `scripts/qualify-local-blends.cpp` et `scripts/qualify-local-blends.sh`, sous OCCT 7.9.3. Il prépare la proposition ADR-0036 ; aucune reconnaissance STL ni route de conversion de production activée. La construction d’un solide de référence ne prouve pas sa récupération depuis un maillage.

Le coin triédrique uniforme se réimporte en un solide avec six plans, trois quarts de cylindre et un octant sphérique. Volume natif **3161,8524963483842 mm³**, formule indépendante évaluée en binary64 **3161,8524963483783 mm³**. Trois arcs de grands cercles partagés avec la sphère, trois arcs distants sur les plans terminaux et six génératrices de tangence sont contrôlés avec centres, rayons, extrémités, axes et incidences opposées. Huit sondes matière/vide passent.

Les deux variantes de rayon linéaire se réimportent chacune en un solide avec six plans et un cône incliné. Les extrémités sont deux arcs elliptiques, leurs centres/demi-axes/domaines et contacts sont contrôlés, ainsi que les deux génératrices tangentes. Les volumes natifs valent **3165,4015953133612** et **3165,4015953135358 mm³**, pour une référence analytique évaluée **3165,4015953235107 mm³**. Neuf sondes par solide passent, dont trois distinguent l’enveloppe réelle d’un empilement incorrect de sections circulaires. Les trois solides conservent les dimensions 20 × 16 × 10 mm ; sept exclusions de domaine passent.

Pour G, l’élimination de la famille de sphères donne `(a+bz−x−y)²=2(1+b²)xy` sur la branche physique. La portion est un cône circulaire droit incliné, avec arrêts elliptiques pour `|b|<1` et rayons positifs. Les volumes D/G sont des **formules analytiques évaluées en double précision**, pas des intervalles certifiés. Les lois générales, sections paraboliques/hyperboliques et extrémités singulières restent hors de ce laboratoire.

Revue mathématique indépendante PASS sur les trois références ; rejeu natif de l’orchestrateur sortie 0, rapport identique octet pour octet (`artifacts/local-blends-lab/root/report.json`). Aucun essai Fusion. L’utilisateur approuve explicitement les deux domaines additifs core/CAO via A-068 ; la preuve source et l’intégration restent à effectuer. Aucun besoin de redemander A-045 pour F.


## V-1203 — R-081 : coin local à trois arrondis

Domaine additif approuvé par ADR-0036/A-068. Le détecteur `rounded/octant.rs` retrouve six supports plans réellement présents et un trièdre arrondi convexe, trois cylindres partiels et un octant sphérique de rayon commun. L’hypothèse est bornée à 32 000 triangles et 8 192 régions planes ; les six supports doivent dominer strictement la septième aire par un facteur quatre. Ces plafonds limitent cette reconnaissance supplémentaire, sans changer les limites d’import, de résultat ou du helper. Les cas plus généraux restent R-034.

### Preuves source et contrat

Les dix patches sont des disques connexes à une frontière simple ; les incidences réelles imposent les vingt et une interfaces du modèle. Les trois arcs centraux et trois arcs distants proviennent des chaînes source, avec paramètres, sens, extrémités et séparation contrôlés. Les triangles cylindriques sont comparés à leur interpolation affine en angle/hauteur, avec borne continue `r Δu²/8`. L’octant utilise un domaine projectif triangulaire positif ; orientation, contours monotones et degré de couverture, puis norme minimale de la facette idéale donnent la borne sphérique. Les disques plans projetés et leurs contours analytiques sont reliés par une homotopie de frontière bornée ; le changement du nombre d’enroulement borne les bandes entre cordes et arcs. Soudage, projection, corde et déplacements natifs des extrémités sont additionnés, puis conservés par Auto.

`MixedSphericalOctant` contient centre/rayon, trois directions orthonormales directes et trois usages orientés d’arcs. `RoundedModel.octants` est optionnel à la lecture et omis vide à l’écriture. La variante de propriétaire est additive, avec provenance après les sept familles existantes. L’ancien JSON et le nouvel identifiant sérialisé ont une régression dédiée. Le flux de volume est `πr²/12 · (C−reference)·Σd + πr³/6` pour cet octant convexe. Une indication locale qui découplerait ses voisins est refusée ; l’Auto validé conserve la partition complète.

La nouvelle entrée CAO partage les vrais objets arête/sommet des cylindres avec l’octant. Les trois pcurves bornent un quart de domaine UV sphérique, avec seulement l’arête dégénérée paramétrique au pôle. Anciennes entrées conservées ; modèles incomplets, sens/indices/rayons/repères ou jetons incohérents refusés. Après chaque réimport, le solide à dix faces est distingué du boîtier arrondi historique à vingt-six faces, puis supports, trois arcs centraux, trois arcs terminaux, six génératrices, volume et incidences sont revérifiés.

### Corpus et contrôles ciblés

Le générateur indépendant `scripts/generate-local-blend-fixtures.py` produit `fixtures/local_blends` : quinze STL, dont les références D et les futures références G non encore reconnues. Régénération identique à l’octet, SHA, frontières opposées, liens de sommets, Euler, partitions et quatre-vingts sondes matière/vide contrôlés. D utilise quatre positifs : nominal, miroir, permutation d’axes et rotation oblique ASCII avec tessellations différentes.

Trois tests core passent : partitions et paramètres, cas ouverts/déformés/tolérance/guidage, puis repli intérieur sur maillage fermé. Ce dernier permute deux sommets adjacents éloignés des trims, tous deux sur la sphère : analyse topologique réussie mais carte repliée refusée, sans octant Auto. Les seuils précis 32 000/8 192 n’ont pas de test de charge D dédié ; leurs gardes sont relues.

Deux tests natifs CAO passent, incluant dix jetons et neuf corruptions. Deux tests applicatifs passent sur les quatre positifs dans les deux modes : un solide, six plans, trois cylindres, une sphère, volume **3161,8524963483783 mm³**, dimensions 20 × 16 × 10 mm avant placement, arcs terminaux et adjacences, sondes source/STEP et attribution exhaustive. Provenance native exacte en Auto ; Surfaces conserve son contrat historique d’attribution core sans ajout implicite de suivi natif. Le mode Auto trop strict conserve un STEP facettisé validé, sans sphère, avec audit dans les deux sens sous le budget demandé ; Surfaces refuse `recognition.deviation_exceeded`. Le premier test applicatif supposait à tort un refus Auto : attente corrigée après examen du repli existant, sans changement du moteur.

Deux tests de document passent : deux corps avec triangles entrelacés produisent vingt faces et conservent placements/provenance séparés ; deux corps en contact sont refusés `cad.bodies_not_separated`. Le test de compatibilité JSON passe. Les revues indépendantes core et CAO sont PASS, suivies d’une vérification par l’orchestrateur.

L’interface reconnaît `spherical_octant` et vérifie son indice dans la liste additive. Elle le présente avec le libellé sphérique FR/EN existant ; aucune nouvelle commande ni panneau. La régression de sélection couvre interprétation complète, champ absent, mauvais indice et anciens modèles. Dix tests ciblés de sélection passent, Svelte indique zéro erreur/avertissement. Les diagnostics réutilisent couverture, intention géométrique et portée de borne existantes, avec traduction et divulgation progressive.

### Validation finale

`sh scripts/check.sh` termine avec le code 0 : **540 tests workspace, 50 bureau, huit worker, 93 ciblés large-stl, un transfert réel, 230 UI, douze Python et quatre SDK**. Formatage, Clippy strict, Svelte (zéro erreur/avertissement), build UI et contrôle bureau passent. Seize tests opt-in workspace et quatre opt-in du sous-ensemble large-stl restent ignorés ; le transfert réel bureau est rejoué explicitement. Le précontrôle Clippy a signalé une expression unité inutile et les huit arguments de la preuve : expression retirée, exception locale documentée pour garder les contextes de preuve explicites. Aucun contrôle géométrique assoupli.

Le paquet `artifacts/FabSTEP.app` est reconstruit et ses exécutables correspondent aux builds : SHA-256 bureau `1f6ea78fee7fb2d7caf5ce553db36e35718356fd6bcbc99e4a8ea96b76ed3eea`, worker `4643c2ba88e1d2092fe98ce34411d5ad88ef888ceb5ee14a85a11b935158e7c0`. Journaux et empreintes dans `artifacts/dg-next/`. Aucun nouvel essai graphique natif ou Fusion, release 1.0.0 inchangée. R-081 à tester via A-075/A-076. La qualification G reste séparée et n’est pas activée dans ce build.


## V-1204 — G1 : raccord conique à rayon linéaire (18 novembre 2026)

### Domaine et preuves

R-082 implémente le second contrat approuvé d’ADR-0036/A-068. La reconnaissance porte sur un bloc à six supports plans dominants et une arête convexe à rayon linéaire positif ; les limites locales de 32 000 triangles et 8 192 régions restent celles de D2. `fixtures/local_blends/` fournit cinq sources positives indépendantes (croissance, décroissance, miroir, permutation et placement oblique retessellisé) et les faux empilements circulaires, loi non linéaire, coupe parabolique et capot absent.

La preuve core exige sept disques, les chaînes de frontière, la couverture orientée en coordonnées du cône, des témoins intérieurs et une borne de Taylor par triangle. Les quatre extrémités peuvent être projetées sur leurs raccords exacts : le nombre est signalé par `recognition.linear_blend_endpoints_projected`, le déplacement entre dans la borne et les sommets de l’analyse source restent inchangés. Le test perturbé vérifie une projection supérieure à 0,5 µm. Le volume utilise le flux des six plans avec correction d’aire elliptique ; le cône a une contribution nulle depuis son sommet.

La représentation native conserve les deux ellipses 3D exactes. Une première construction échouait au contrôle d’orientation de la face conique après STEP pour certaines pentes : un écart paramétrique de 2,838e−7 mm dépassait de peu la tolérance d’arête de 2,837e−7 mm. La représentation sur le cône est désormais interpolée par Hermite, avec 8 à 128 intervalles et estimation OCCT inférieure à `Precision::Confusion()/8`. Il s’agit d’un contrôle numérique de cette représentation, pas d’un certificat continu. Aucune tolérance d’export n’a été relâchée. Le laboratoire natif couvre seize variantes de pente/placement (±0,12, ±0,2, ±0,4, ±0,67 et placements obliques), leurs réimports et sept refus de contrat.

### Contrôles ciblés exécutés

- Six tests core : priorité des arcs cylindriques constants, sources positives dans Auto/Surfaces, volume indépendant, propriétaire unique, refus des familles hors domaine, repli Auto, carte repliée, permutation des triangles et projection comptée.
- Deux tests CAO : cône, deux trims elliptiques et sept étiquettes réimportés ; valeurs scalaires, références, usages exclusifs et orientations invalides refusés.
- Trois tests application : cinq conversions positives dans les deux modes, types/volume/dimensions, provenance et sondes de matière, export DXF ELLIPSE des deux faces terminales ; cas négatifs et compatibilité JSON.
- Deux tests document : deux corps aux triangles source entrelacés gardent 14 faces natives distinctes, placements et quatre arcs elliptiques ; contact plan refusé par `cad.bodies_not_separated`.
- Quinze tests UI ciblés sélection/diagnostics : propriétaire `linear_blend` reconnu comme Cône, indices invalides/champ absent refusés, anciens modèles conservés ; déplacement géométrique visible. Svelte : zéro erreur et avertissement.

Les revues croisées core/CAO ont été distinctes des auteurs. La revue core a fait ajouter la charge des copies finales au budget et déplacer le calcul des bornes dans la branche qui en a besoin. Un contrôle numérique indépendant sur 9 600 interpolations donne un rapport erreur/borne maximal de 0,929 ; il complète la revue de la formule sans remplacer la preuve. Le suivi natif conserve la politique tout-ou-rien d’ADR-0018 ; l’attribution source est obligatoire et exhaustive, les étiquettes natives sont exactes sur les cas testés mais leur disponibilité n’est pas garantie pour tout transfert.

Le précontrôle Clippy a fait simplifier une expression booléenne et allouer séparément le champ optionnel G, sans changer sa représentation JSON. Aucun ancien modèle ni plafond n’est modifié. Journaux ciblés et suite complète : `artifacts/g-next/`.


La première suite globale a détecté une régression sur `m2c_open_arc.stl` : la signature de six plans d’une extrusion constante était interceptée par G avant le parcours cylindrique historique. La recherche exige désormais une variation mesurable dans le même sens des deux retraits de tangence, avant de donner priorité à sa preuve. Quatre variantes d’arc constant (source, subdivision, rotation et coin à rayon constant) sont couvertes par une régression core ; les erreurs de preuve et de budget après identification du candidat restent bloquantes. Le journal du premier échec est conservé dans `artifacts/g-next/check.log`.


### Validation finale

Après correction, `sh scripts/check.sh` termine avec le code 0 : **553 tests workspace, 50 bureau, huit worker, 101 ciblés large-stl, un transfert réel, 231 UI, douze Python et quatre SDK**. Formatage, Clippy strict, Svelte sans erreur/avertissement, build UI et contrôles bureau/helper passent. Seize opt-in workspace et quatre du sous-ensemble large-stl restent ignorés ; le test de transfert réel initialement ignoré au bureau est exécuté explicitement. Le build UI conserve son avertissement de taille de chunk, sans échec. Journaux : `artifacts/g-next/check-final.log` et `build-app.log`.

`artifacts/FabSTEP.app` est reconstruit ; les deux exécutables correspondent à leurs builds par SHA-256 : bureau `f03743f468fb87a2b7213f8c5b8230b122c8a8d63be01d320f578c5b32bd6036`, worker `1e76a88a5f96dfa1529ff77ca9967ed9980ff02814410f8ae23f67831e380a60`. Le paquet de développement dépend toujours du SDK local, la release 1.0.0 reste inchangée. Aucun nouvel essai graphique/Fusion ni mesure de performance avant/après n’est revendiqué. R-082 à tester via A-077/A-078. Flansch n’a pas été rejoué pour G : son dernier refus V-1201 reste la référence.

## V-1205 — F1 : reconnaissance source des perçages sécants (18 novembre 2026)

### Périmètre et références

R-083 met en œuvre le contrat autorisé A-045, décrit dans ADR-0038 : un bloc, deux trous traversants centrés et orthogonaux, quatre anneaux d’entrée et deux branches communes. Le grand rayon est au moins double du petit avec marge résolue. La recherche locale est bornée à 32 000 triangles et 8 192 régions ; chaque cycle comporte 12 à 4 096 témoins avec pas angulaire au plus π/4. Les axes décentrés/obliques, rapports indéterminés, tangences et triples intersections restent hors domaine.

Le générateur indépendant `scripts/generate-cross-drill-fixtures.py` fournit cinq positifs et quatre négatifs, avec incidence orientée, connexité, liens de sommets et sondes matière/vide vérifiés après relecture STL. Le manifeste donne les plages source, les limites et le volume numérique indépendant. La grande paroi a quatre contours et une caractéristique d’Euler de −2 ; le solide percé complet a χ=−4. Régénération par l’orchestrateur : neuf STL et manifeste identiques à l’octet près, copie dans `artifacts/f-next/corpus-replay/`.

### Preuve et construction

Le core conserve les sommets source, les quatre anneaux et les deux cycles de branche. Il démontre les supports, la partition en neuf domaines, les incidences opposées, les degrés des contours et l’orientation stricte des cartes cylindriques. Sur le grand cylindre, les deux limites idéales forment des courbes convexes disjointes dans le repère développé ; sur le petit, les limites sont des graphes axiaux. Les bornes de remplacement des contours s’ajoutent à l’interpolation des parois, aux projections mesurées et à une réserve native de 4e−7 mm. Cette réserve ne relève pas le budget demandé.

La CAO découpe les supports selon les domaines prouvés et conserve l’historique des neuf faces. Les deux branches peuvent se décomposer en plusieurs arêtes de couture : leur identité ne vient pas du nombre d’arêtes OCCT. Leur représentation spatiale et leurs deux courbes paramétriques sont construites conjointement, puis certifiées sur les données réellement réimportées. Le contrôle partage dix millions d’opérations et 16 384 feuilles par solide et vérification. Les supports, les cycles, les sondes de matière et le volume sont contrôlés avant et après STEP, ainsi que lors d’une combinaison de corps.

Le certificat F0 a été déplacé vers la CAO avec un en-tête de compatibilité pour le laboratoire. Son unité de compilation conserve les règles d’arithmétique d’intervalles (`-fno-fast-math -ffp-contract=off`, équivalent MSVC non essayé ici). Le volume natif utilise l’intégration adaptative et reste une mesure comparée à l’intervalle analytique indépendant. Les splines arbitraires restent refusées ; la tessellation n’admet que les limites certifiées du domaine et conserve le plafond résultat.

### Vérifications ciblées

Premiers tests core : trois réussis, dont branches inversées ou parcourues deux fois avec points pourtant situés sur les deux supports ; Clippy strict réussi. Deux tests CAO réussis : domaines, références et paramètres corrompus refusés, groupes de provenance complets et distincts. Cinq tests application réussis : dix conversions des cinq positifs dans Auto/Surfaces, types/volume/dimensions/fidélité, attribution exacte et voisinages, sondes matière/vide, refus des négatifs et du guidage couplé, ancien JSON, deux corps séparés à 18 faces et contact refusé. Journaux ciblés dans `artifacts/f-next/`.

Le rejeu natif F0 par l’orchestrateur conserve les certificats à 1e−5 et 1e−7 mm, les adversaires et l’intervalle de volume V-1200 (`lab-report.json`). Ce laboratoire n’est pas présenté comme une preuve source. Les tests UI ciblés de sélection passent et Svelte indique zéro erreur/avertissement ; les trois propriétaires supplémentaires sont affichés avec le libellé Cylindre existant.

### Régression identifiée en revue

Quatre trous borgnes opposés à pointes coniques peuvent présenter les mêmes quatre anneaux d’entrée qu’un candidat F. Un reproducteur indépendant fermé de 1 044 triangles démontre le défaut : avant F, le parcours Surfaces laisse le reconnaisseur historique trouver six plans, quatre cylindres et quatre cônes ; sans garde de jonction, F intercepte les pointes et bloque le repli. La reconnaissance exige désormais une arête source commune entre des parois petite/grande uniquement attribuables à leurs cylindres avant de commencer sa preuve bloquante. Les autres erreurs de preuve et de budget ne sont pas avalées. La validation finale et le rejeu du cas corrigé sont consignés ci-dessous une fois exécutés.


Après ajout du garde de jonction, les **quatre tests core** passent, dont la régression des trous borgnes. Les **deux tests CAO** passent après alignement des limites de témoins avec le core (12 à 4 096 points, pas au plus π/4) et ajout des adversaires de cycles clairsemés. Le test applicatif supplémentaire reconvertit effectivement les quatre trous borgnes en Primitives et Surfaces : un solide, six plans, quatre cylindres et quatre cônes. Clippy strict du workspace et formatage passent. Le premier essai de compilation de cette régression signalait seulement le fichier de référence pas encore installé ; générateur et STL sont désormais versionnés ensemble.


La revue indépendante CAO confirme les certificats des branches/paramètres après transfert et après combinaison multisolide, avec réserve cumulative de 4e−7 mm. Elle a relevé un dernier garde public : la tolérance des témoins, déjà plafonnée par les marges de stock et de séparation des rayons, doit aussi rester inférieure à r/2. Le wrapper et un modèle corrompu de petit rayon ont été renforcés ; le test vérifie un refus scalaire avant l’appel au noyau. Ce changement local de refus est appliqué pendant la suite globale, après compilation initiale ; le test CAO est donc rejoué explicitement sur le code final, en plus du parcours large-stl recompilé.


### Validation finale automatisée

`sh scripts/check.sh` termine avec le code 0 : **565 tests workspace, 50 bureau, huit worker, 107 ciblés large-stl, un transfert réel, 232 UI, douze Python et quatre SDK réussis**. Formatage, Clippy strict du workspace/bureau/worker, Svelte (zéro erreur/avertissement), builds UI/bureau et helper passent. Seize tests opt-in workspace et quatre ciblés large-stl restent ignorés ; le transfert réel initialement ignoré au bureau est rejoué explicitement avec succès. L’avertissement connu de taille de chunk UI reste présent. Journal : `artifacts/f-next/check.log`.

La qualification est automatisée sur macOS avec le SDK local. Les essais graphiques et les réimports manuels Fusion restent ouverts ; Windows/Linux et l’option de compilation MSVC ne sont pas validés ici. Aucun benchmark avant/après ni nouvel essai de Flansch n’est revendiqué.

Le rejeu CAO final (`cargo test -p fabstep-cad --test cross_drill --offline --locked`) réussit ses deux tests, y compris le refus scalaire r/2, puis le formatage est contrôlé à nouveau. `scripts/build-macos-dev.sh` termine avec succès et actualise `artifacts/FabSTEP.app`. Correspondance SHA-256 des exécutables copiés avec leurs builds : bureau `d534b15f8f9a1fc173d57207eb4f4412ba72079ef043dba8ee5129190cafa8a0`, worker `3beec03d1a31e0f621bba05bb1adc3d0e84a5d7e91a3fcd8f14c58e15f8f8d7c`. Journaux : `cad-final-replay.log`, `build-app.log`, `bundle-hashes.json` dans `artifacts/f-next/`. Paquet de développement dépendant du SDK local ; release 1.0.0 inchangée.


## V-1206 — C2b : révolution à méridienne cubique (18 novembre 2026)

### Qualification native préalable

`sh scripts/qualify-smooth-revolution.sh` termine avec code 0 sur OCCT 7.9.3. Cinq références synthétiques indépendantes du moteur : profil bombé, resserré, à inflexion, placement oblique et mise à l’échelle. Avant/après STEP : un solide valide, deux plans, une surface de révolution, axe et tour complet, génératrice cubique non rationnelle mono-travée et quatre pôles physiques conservés. Trente sondes matière/vide par forme et par passage, soit 300 contrôles ; intégration native adaptative comparée à l’intégrale polynomiale indépendante. Écart absolu de volume maximal au réimport : 3.0177e-09 mm³. Ce volume mesuré et les sondes ne sont pas une preuve continue de fidélité au STL.

Deux adversaires passent : le contrôle rejette la méridienne physiquement renversée malgré un volume théorique identique, et un rayon de contrôle nul est hors du domaine positif. Les cinq STEP, journaux et `report.json` sont dans `artifacts/smooth-revolution-lab/`. Les scripts sont versionnés ; ce laboratoire n’est pas appelé par l’application et ne reconnaît pas de source STL.

### Autorisation et intégration

Le contrat ADR-0039 a été approuvé explicitement après cette qualification. R-084 développe la reconnaissance source bornée, l’adaptateur natif et le parcours complet. La suite UI préliminaire passe 234 tests ; Svelte ne signale aucune erreur ni avertissement. Les résultats de production et les limites restantes seront consignés à la fin de l’intégration, sans déduire leur succès du laboratoire.


### Preuve source et revue

Les cinq tests core ciblés passent sur sept positifs et six négatifs : transformation, miroir, échelle, retessellation, attribution complète, volume, bornes, budget, guidage refusé et priorité historique. Douze tests ciblés de chaînes existantes et Clippy strict core passent. Le générateur indépendant est rejoué par l’orchestrateur dans `artifacts/c2-next/corpus-replay` : treize STL et manifeste identiques à l’octet près.

La revue indépendante a fait séparer les tolérances de hauteur des disques et d’ajustement radial : un centre de disque légèrement déplacé doit respecter la tolérance de planéité, même si la tolérance d’ajustement est plus large. Un autre test refuse de rebaptiser cubique une méridienne circulaire indiscernable. Le négatif historique `adjacent_chains/smooth` est exactement r(z)=10+0,04z² : il entre désormais dans C2 en Surfaces, avec zéro cône, mais demeure refusé en Primitives. Le corpus peu échantillonné peut conserver une reconstruction historique prouvée ; le test interdit une révolution ajustée, pas toute ancienne famille.

### Défaut de tessellation identifié pendant l’intégration

La première conversion `mirrored/Auto` est correctement bloquée par `validation.distance_exceeded`. Le diagnostic direct de l’orchestrateur confirme les pôles natifs et des sommets de tessellation proches de la surface à environ 1e−12 mm, mais des barycentres de triangles s’écartent radialement de **0,05086968 mm**, malgré une déflexion demandée de 0,005 mm. L’audit mesure **0,06024749 mm source→résultat et 0,05971518 mm résultat→source**, au-delà du budget de 0,055 mm. La variante non réfléchie mesure 0,01426108/0,01463523 mm. Journaux et maillages : `artifacts/c2-next/probe.log`, `*-probe.step`, `*-mesh.json`.

Changer arbitrairement la déflexion n’est pas un correctif fiable : des essais indépendants constatent une qualité non monotone du mailleur OCCT sur ce profil à inflexion. La correction choisie est une tessellation déterministe du domaine natif certifié, avec grille angle–hauteur bornée par les dérivées du profil et le plafond existant. Aucun relèvement de la tolérance d’audit. Son résultat final est à consigner après rejeu.


### Correction et contrôles ciblés de production

Le mailleur paramétrique évalue les faces natives réimportées sur une grille angle–hauteur calculée avec les quatre pôles physiques et leurs dérivées, y compris le terme mixte. Les disques reprennent les mêmes points de bord. Les corps ordinaires conservent le mailleur historique. Le test CAO compare sommets et barycentres des variantes à inflexion directe/réfléchie au profil nominal et exige un écart radial inférieur à 0,005001 mm pour une déflexion de 0,005 mm ; il passe. Le banc natif séparé mesure environ 0,001394 mm aux barycentres pour 34 440 triangles. Ce correctif vise la fidélité, sans promesse d’accélération.

Trois tests CAO passent : modèles scalaires incohérents refusés avant construction, provenance exacte et compatibilité des wrappers, bornes de tessellation, mutation d’un pôle intérieur du STEP réimporté refusée par le certificat, contrôle du maillage après le correctif. Quatre tests application réussissent (quatorze conversions Auto/Surfaces), ainsi que les trois tests de chaînes adjacentes et deux tests multisolides. Les nouveaux champs absents restent lisibles dans les anciens modèles et rapports. La première compilation du test JSON dans le crate CAO a signalé sa dépendance absente : le test a été déplacé dans l’application qui dispose déjà de serde_json, sans ajouter de dépendance. Les constructeurs de tests historiques ont été actualisés avec le champ optionnel absent.

Le contact des deux maillages source du test comporte 4 224 faces par corps : les 17 842 176 paires dépassent le budget existant de dix millions, donc `cad.bodies_separation_budget` refuse correctement l’export. Le second cas a des sources séparées mais des solides reconstruits qui se recouvrent ; les trois faces par corps permettent le contrôle et `cad.bodies_not_separated` le refuse. Les assertions distinguent ces deux motifs, sans modifier le comportement de séparation.

La revue indépendante finale valide la réserve native débitée de **1e−6 mm**, soit vingt fois le garde géométrique de 5e−8 mm. Par transfert, les centres, rayons et axes imposent une dérive canonique inférieure à quatre gardes ; deux transferts en consomment moins de huit. Les supports et limites paramétriques finaux, ainsi que les marges numériques, restent dans les douze gardes restants. Les courbes paramétriques des disques sont des cercles physiques contrôlés ; les quatre coarêtes de la paroi bornent son rectangle et les usages opposés sont vérifiés. Ce contrôle analytique borné ne prétend pas être un certificat d’arithmétique d’intervalles. Les seuils d’audit et d’export n’ont pas été augmentés.

Formatage et Clippy strict workspace passent avant la suite complète. Les journaux ciblés sont sous `artifacts/c2-next/` ; la suite complète doit encore être consignée ci-dessous à son terme.


### Clôture V-1206

`sh scripts/check.sh` termine avec code 0 : **579 tests workspace, 50 bureau, huit worker, 115 ciblés large-stl, un transfert réel, 234 UI, douze Python et quatre SDK réussis**. Formatage, Clippy strict workspace/bureau/worker, Svelte (zéro erreur, zéro avertissement), compilation UI et bureau réussissent. Seize tests opt-in workspace et quatre tests ciblés large-stl restent ignorés ; le test de transfert bureau ignoré dans la passe ordinaire est ensuite exécuté explicitement et réussit. Aucun échec final.

`sh scripts/build-macos-dev.sh` termine avec code 0 et reconstruit `artifacts/FabSTEP.app`. Les exécutables inclus sont identiques aux sorties Cargo :

- FabSTEP SHA-256 : `cbe5b5efa4ad0840a20c1f459aa502c7c3891871c0c9fe765032a437b6c374da`.
- Worker SHA-256 : `ac74427f748083ceaafa705f4b9b136cac4d5b72d3bd048daf58069aa910759f`.

Journaux locaux : `artifacts/c2-next/check.log`, `build-app.log`, `app-sha256.json`. Tests, corpus et générateurs sont versionnés et permettent de reproduire les contrôles sans ces journaux. L’avertissement Vite de taille de chunk reste présent ; aucun benchmark de vitesse ni gain global sur Flansch n’est revendiqué. Pas de nouvel essai graphique/Fusion, Windows/Linux ou d’installation autonome : A-081/A-082 portent les vérifications manuelles. La release 1.0.0 reste inchangée.

## V-1207 — R-080, attribution des faces planes restantes de Flansch

Le helper de développement V-1206 a relu le STL de 540 170 triangles en mm, en Auto avec les options par défaut. Il retrouve **231 608 faces** et refuse le résultat avant la validation native de la source (`step.invalid_source_provenance`). Aucune sortie STEP. Sur ce passage de 67,746 s, le RSS maximal échantillonné est de 611 139 584 octets et l’empreinte physique de 448 398 368 octets. Cette mesure décrit un seul passage jusqu’au refus.

La sonde core attribue 252 888 triangles aux 231 584 faces planes. Parmi ces faces, 223 126 ne contiennent qu’un triangle. Les 287 282 autres triangles appartiennent aux 24 faces courbes déjà reconnues. Une fois celles-ci retirées, la plus grande des quatre composantes connexes par arêtes rassemble 231 581 faces planes. En coupant les arêtes dont le dièdre excède 3°, on obtient 182 groupes mais le principal contient encore 225 692 propriétaires plans. Les normales changent assez progressivement pour que cette croissance traverse plusieurs arrondis tangents.

### Supports candidats sur la bride

La sonde Python explore trois intervalles de X mesurés sur la pièce. À une section intérieure, elle ajuste des cercles aux sommets voisins, affine leurs centres sur les arcs observés et teste les deux rayons majeurs possibles pour le rayon de congé choisi. Elle exige un échantillonnage sur au moins 80 % de la largeur axiale de la bande et une distance au support ≤ 0,00001 mm aux trois sommets de chaque triangle retenu. Elle exclut les triangles d’attribution ambiguë. Une face plane est comptée seulement si **tous** ses triangles correspondent au même support. Les composantes et distances aux barycentres ci-dessous portent sur ces faces entières. Axe, stations et rayons mineurs sont des hypothèses spécifiques à Flansch.

| X (mm) | Rayon mineur (mm) | Supports / composantes connexes | Faces planes entièrement attribuées | Écart maximal au barycentre (mm) |
| --- | ---: | ---: | ---: | ---: |
| 0 à 0,5 | 0,5 | 7 / 7 | 87 065 | 0,001522 |
| 6,5 à 7 | 0,5 | 4 / 4 | 63 376 | 0,000326 |
| 7 à 9 | 2 | 1 / 12 | 56 592 | 0,377758 |

Au total, 207 033 propriétaires plans ont un support torique candidat, 24 551 restent sans attribution complète par cette expérience. Dans la dernière bande, **1 230 barycentres** excèdent 0,05 mm. L’accord des sommets ne suffit donc pas à remplacer ces faces. Dans les deux premières bandes, tous les barycentres vérifiés respectent le budget. L’écart continu et réciproque, la découpe réelle de chaque secteur et ses arêtes communes avec les voisins restent à démontrer.

**Conséquence pour R-085.** Si chacune des onze composantes des deux premières bandes donnait une face valide, le comptage passerait de 231 608 à **81 178** (231 608 − 150 441 + 11). C’est un scénario à géométrie voisine inchangée. Le résultat réel dépendra notamment du nombre de faces nécessaires pour les limites et raccords. Le coût de validation native des 540 170 triangles source et le plafond de tessellation du résultat demandent toujours une mesure en situation.

### Reproduction

Avec le STL privé d’origine et Python équipé de NumPy/SciPy, depuis la racine du dépôt :

```sh
export FABSTEP_PROFILE_STL="$HOME/Downloads/FlanschTurboLuftfilter_REV02.STL"
export FABSTEP_PROFILE_DIR="$PWD/artifacts/r080"
FABSTEP_DUMP_PLANES=1 cargo test -p fabstep-core --features large-stl --target-dir artifacts/r080/target --release --offline --locked --test residual_planar_profile -- --ignored --exact residual_planar_profile
python3 scripts/qualify-residual-tori.py "$FABSTEP_PROFILE_DIR/planes-3deg.bin"
```

Deux exécutions donnent le même relevé binaire des triangles plans et le même JSON des supports, vérifiés octet par octet. Les coordonnées détaillées et le STL restent locaux. Les fichiers versionnés permettent le rejeu sur un poste disposant de la source. Formatage et Clippy ciblés contrôlés. Cette qualification ajoute exclusivement des outils de mesure hors application. Le build testé reste V-1206 et sa suite applicative complète est consignée dans cette entrée précédente.

## V-1208 — R-085, secteurs toriques et raccords de la bride

### Conversion par l’application

Le corpus public contient quatre variantes positives et trois contre-exemples. La régénération des sept STL et du manifeste donne huit fichiers identiques à l’octet près. Le secteur annulaire de référence possède une ouverture de 80°, une hauteur de 6 mm et un volume idéal de 264,312610251146 mm³. Son congé est borné par quatre arcs partagés. Le laboratoire OCCT 7.9.3 contrôle deux placements avant/après STEP et mesure un écart maximal de volume de 3,314e−11 mm³. Deux secteurs décalés en azimut sont refusés même si leur volume et leurs rayons correspondent.

Auto et Surfaces passent huit conversions avec volume, fidélité et comptage de sept faces sur la pièce nominale et ses variantes oblique, réfléchie et retessellée. En Auto, chaque sélection STEP retrouve exactement son groupe de triangles source. Le document intercalé à deux corps garde quatorze propriétaires distincts. Le test du STL ouvert, du pli angulaire et de la paroi déformée exclut une fausse face torique. Le repli Auto historique de la paroi déformée aboutit à un refus natif de planéité. Les tests core couvrent aussi le budget insuffisant, la permutation des triangles et le refus de correction locale. Trois tests CAO couvrent le réimport, les mutations de modèle et le changement du petit rayon dans un STEP déjà écrit.

La première écriture du prototype de production perdait les sept identifiants malgré un BRep correct. Le prétraitement géométrique OCCT remplaçait certaines faces déjà validées. Pour cette construction, le transfert conserve désormais les identités. Les contrôles géométriques restent actifs à chaque passage. La revue indépendante a fait ajouter la comparaison du repère réinféré avec pondération des axes par la taille physique, les centres et rayons des courbes 2D sur les plans et une réserve de transfert de 1e−5 mm débitée du budget. Le certificat analytique porte sur la configuration décrite dans ADR-0040.

### Couverture des bandes réelles de Flansch

Sur les onze supports de R-080, neuf cartes couvrent directement un rectangle d’azimut et de méridienne. Les deux autres ont un bord irrégulier avec la partition initiale. La recherche bornée examine les propriétaires entiers encore libres à une arête du contour. Elle teste toutes les combinaisons de cinq candidats au maximum et exige une seule complétion admissible dans cette recherche, puis l’absence de collision entre complétions. Trois propriétaires d’un triangle comblent les deux lacunes. Les onze secteurs regroupent **154 001 triangles et 150 444 faces planes**. Les tests contrôlent la compensation des arêtes intérieures, l’orientation des triangles en coordonnées angulaires, le parcours complet des quatre limites et la borne continue dans les deux sens. La borne maximale des onze secteurs ainsi complétés est inférieure à 0,001752 mm.

Deux jonctions relient directement trois de ces tores. Chaque interface suit 90 arêtes source partagées. L’ajustement simultané contraint les cercles guides à être tangents et impose le même plan méridien sur les deux côtés de chaque jonction. La preuve UV repasse pour les trois secteurs (11 729, 6 030 et 6 033 propriétaires). Déplacement maximal mesuré aux sommets de ces trois zones : 0,000006360 mm. Le laboratoire réimporte trois faces toriques reliées par deux arcs partagés, avec huit bords libres et zéro solide. Certaines courbes paramétriques STEP des limites sont des splines linéaires à deux pôles. Le laboratoire en contrôle le degré, les pôles, les nœuds et le domaine affine. Un déplacement de 0,05 mm d’un voisin est détecté. Ces conclusions concernent les trois parois et leurs deux interfaces.

Pour les neuf rectangles initiaux, le recalage angulaire des méridiens tient compte de la longueur des rails voisins. Sur un tube long, une erreur angulaire de quelques dix-millièmes de degré suffit à manquer la face adjacente. Dix-sept des dix-huit incidences méridiennes ont une bande cylindrique compatible après recalage, avec contrôle du bord commun et de toute la carte cylindrique. Treize passent directement. Quatre nécessitent de s’arrêter à une station complète de la source et de conserver un triangle extérieur avec son propriétaire. L’écart continu maximal des dix-sept incidences est inférieur à 0,000027586 mm. Ce compte porte sur les incidences et peut désigner deux fois le même tube.

Le secteur à 15 845 propriétaires partage deux véritables quarts de cercle avec deux bandes cylindriques. Sur les paramètres mesurés de cette zone, le laboratoire construit et relit trois faces, deux arcs communs d’incidences opposées, huit arêtes libres et zéro solide. Aire analytique 71,76780497593708 mm². L’écart maximal par face est inférieur à 2,253e−12 mm². Une bande écourtée à son extrémité distale est refusée. Une autre zone haute touche un propriétaire dont les trois sommets suivent un autre tore. Sa composante candidate de 2 294 triangles garde un bord irrégulier.

### Rejeu du document et reproduction

Le helper recompilé, en Auto et millimètres, refuse le document à **231 608 faces** en amont de la construction native du corps. Aucune sortie STEP. Mesure isolée de 72,208 s, RSS maximal échantillonné de 655 556 608 octets et empreinte physique de 400 852 000 octets. Les nouvelles parois privées sont des résultats de qualification. Le parcours de production exige la preuve du solide entier. Le scénario arithmétique de remplacement des onze zones par onze faces donne 81 175 faces avant toute modification des voisins. Les limites du résultat final et la validation native du maillage dense restent à mesurer après intégration des raccords.

Depuis la racine, reprendre le dump privé décrit dans V-1207 et utiliser un Python équipé de NumPy et SciPy. Le STL, les coordonnées et les STEP locaux de Flansch sont ignorés par Git.

```sh
python3 scripts/qualify-sector-boundaries.py --align-cylinders --native-parameters artifacts/r085-source/sector-15845-parameters.json artifacts/r080/planes-3deg.bin
python3 scripts/qualify-sector-partition.py --align-cylinders --native-parameters artifacts/r085-source/shared-tori.json artifacts/r080/planes-3deg.bin
sh scripts/qualify-torus-sectors.sh
sh scripts/qualify-torus-sectors.sh --open-chain artifacts/r085-source/sector-15845-parameters.json
sh scripts/qualify-torus-sectors.sh --shared-tori artifacts/r085-source/shared-tori.json
```

### Ceinture inférieure de Flansch

Le graphe des arêtes source identifie un cycle parcourant sept secteurs toriques et cinq bandes cylindriques distinctes. La sonde compare les deux extrémités des mêmes bandes et ajuste les tangentes communes aux cercles guides. Les cartes des douze faces sont ensuite revérifiées avec les mêmes méridiens aux interfaces. La bande couvre 90 011 triangles et 87 518 propriétaires plans entiers. La borne maximale mesurée sur ses cartes est de 0,001752 mm.

Sur ces paramètres, la CAO relit une coque ouverte connexe de douze faces. Douze méridiens circulaires ont deux incidences opposées. Les 24 arêtes libres constituent exactement deux cycles de douze arêtes. L’écart maximal d’aire par face est inférieur à 1,083e−11 mm². Un voisin déplacé de 0,05 mm est refusé. La première boucle correspond à 504 arêtes source adjacentes à un propriétaire plan. La seconde correspond à 507 arêtes source et 500 propriétaires extérieurs.

La recherche des flancs coaxiaux dans le dump des plans associe quatre secteurs bas à quatre secteurs hauts, par des parois de 180 ou 184 triangles et 90 arêtes source à l’interface haute de chacune. Les trois autres parois candidates dépassent la station X=6,5 mm, atteignent environ X=9,99 à 10,03 mm et ne rejoignent aucun des quatre secteurs hauts identifiés. Ce comptage établit la connexité des faces voisines. Une preuve complète des domaines des flancs et des contours du dessus est encore nécessaire à l’assemblage du solide.

```sh
python3 scripts/qualify-flange-rim.py --native-parameters artifacts/r085-source/belt.json artifacts/r080/planes-3deg.bin
sh scripts/qualify-torus-sectors.sh --belt artifacts/r085-source/belt.json
```

Le propriétaire plan à X=0 comporte 2 314 triangles. Son contour extérieur est exactement le cycle des 504 arêtes basses de la ceinture. Ses cinq contours intérieurs sont disjoints et parcourus une fois. Quatre ont 360 arêtes, le cinquième en a 362. Le relevé optionnel des interfaces plan/courbe attribue chaque contour entièrement à un seul cône existant. L’écart radial maximal de l’ajustement circulaire reste inférieur à 0,0000058 mm, la borne corde–cercle à 0,001603 mm. Le dump binaire historique des plans reste identique après activation du relevé supplémentaire. Pour reproduire cette attribution, ajouter `FABSTEP_DUMP_ANALYTIC_SEAMS=1` à la commande de la sonde R-080 (V-1207), puis passer son fichier `analytic-seams.tsv` à `qualify-flange-rim.py --analytic-seams`. Le prochain assemblage du plan doit réutiliser les anneaux des cinq cônes sur son contour intérieur.


### Contrôles globaux

`sh scripts/check.sh` termine avec code 0. Les 588 tests workspace, 50 bureau, huit worker, 121 ciblés large-stl, un transfert réel, 235 UI, douze Python et quatre SDK passent. Formatage, Clippy strict workspace/bureau/worker et Svelte réussissent. Seize tests opt-in workspace et quatre large-stl restent ignorés dans cette suite. Le transfert bureau est rejoué explicitement. La sonde privée R-080 actualisée a été exécutée séparément sur Flansch.

Le build macOS de développement a été reconstruit avec `sh scripts/build-macos-dev.sh`. Les fichiers livrés sont identiques aux sorties Cargo (SHA-256 bureau `9f313a182398074677a98f1a4c6f3990cbd7f5be8d12e3a34ee729317f0b8fa6`, helper `e6cce2638258af49eed68227a94fac984901ce4a965387d43c6ccc94e2448bda`). L’avertissement existant de taille de chunk Vite demeure. Les essais manuels du bureau et de Fusion restent ouverts dans A-084/A-085. Windows et Linux n’ont pas été qualifiés dans ce lot.

## V-1209 — R-085, assemblage du pourtour de Flansch

### Assemblage source et réimport natif

Le plan inférieur rejoint les douze faces de la ceinture V-1208 sur son contour extérieur. Ses cinq trous reprennent les anneaux du modèle Auto. La somme orientée des aires des sept arcs et cinq segments, diminuée des cinq disques, donne une aire nette de **2 876,916048013662 mm²**. Les limites sont parcourues une fois. La distance bornée entre régions planes atteint au plus **0,001676 mm**. Les cinq parois coniques occupent 3 602 triangles. La preuve vérifie les deux anneaux de chaque cône, leurs voisins orientés dans la source et une borne continue de **0,002494 mm** sur les triangles.

| Faces mesurées et assemblées | Triangles source | Faces après STEP |
| --- | ---: | ---: |
| Ceinture inférieure | 90 011 | 12 |
| Plan de base et cinq cônes | 5 916 | 6 |
| Quatre cylindres de flanc et quatre secteurs toriques supérieurs | 65 628 | 8 |
| Cinq bandes cylindriques supérieures | 1 727 | 5 |
| Deux flancs plans simples et flanc marqué | 1 110 | 3 |
| **Total** | **164 392** | **34** |

Les quatre flancs courts ont chacun deux interfaces de 90 arêtes source. Leurs cartes cylindriques restent à moins de 0,000349 mm du maillage. Les quatre secteurs toriques hauts sont revérifiés avec les mêmes centres et angles. Les cinq bandes hautes utilisent les tangentes ajustées du bas. Les deux flancs plans simples possèdent chacun deux triangles, quatre coins et les quatre voisins attendus. La borne mesurée sur les bandes hautes reste inférieure à 0,000026 mm.

Le troisième flanc plan comprend **dix boucles intérieures**. Les 1 106 triangles couvrent le rectangle extérieur privé de ces dix découpes. Le contrôle compare l’orientation, la séparation des contours, les quatre chaînes de bord et l’aire nette de **448,078923812396 mm²**. Le déplacement maximal par projection est inférieur à 0,000002900 mm. Après STEP, le contrôle retrouve les 1 004 segments des boucles, leurs sommets ordonnés et les quatre arêtes partagées avec le pourtour.

Sur les 34 faces, OCCT 7.9.3 relit **une coque connexe, zéro solide et 16 contours libres**. L’écart maximal d’aire par face est inférieur à 8,64×10⁻¹⁰ mm². Les courbes paramétriques rectangulaires sont contrôlées dans l’espace aux deux extrémités et à mi-parcours. Les témoins natifs refusent un trou déplacé à aire constante, un congé décalé, une bande écourtée, une face posée sur la mauvaise fenêtre et une découpe comblée après transfert. Un méridien placé du mauvais côté en coordonnées paramétriques est refusé dans le contrôle avant transfert.

### Document entier et interfaces restantes

La sonde lit les propriétaires réellement attribués par Auto aux **540 170 triangles**. Les 34 groupes ci-dessus prennent l’intégralité de **151 763 propriétaires**. Les **375 778 autres triangles** conservent chacun leur attribution parmi 79 845 propriétaires. En recomptant chaque triangle, la partition résultante comporte **79 879 faces**, soit 20 121 sous le plafond de suivi. Les contre-exemples couvrent une répétition, un propriétaire amputé ou scindé, un singleton oublié et deux cônes intervertis. Ce décompte est une qualification de la partition avant construction du solide.

Les 16 cycles sont retrouvés dans les arêtes orientées de la source. Quatre sorties de cônes rejoignent chacune un cylindre existant et la cinquième un tore. Les dix découpes latérales rejoignent des faces planes le long de leurs segments. Le cycle principal relie **692 arêtes source à 320 propriétaires plans**. Dans le STEP relu, ces arêtes suivent une fois **11 segments et neuf arcs**, avec une borne maximale de **0,001693 mm** entre les polylignes et les courbes. Un arc déplacé, un bord dupliqué et un retour en arrière sur la source sont refusés. La table locale `interface-map.json` associe à chaque intervalle sa position en millimètres sur la courbe réimportée et les deux incidences orientées dans le maillage analysé. Le partage natif de ces bords avec les voisins restants est le prochain travail d’assemblage.

Trois des voisins sont des flancs cylindriques longs de 409 triangles au total. Leurs domaines en angle et hauteur ont des contours supérieurs variables, avec deux ressauts axiaux par flanc. La preuve source couvre les 274 arêtes des limites supérieures et borne la distance à 0,001694 mm. Ces trois cylindres sont extérieurs à la coque native des 34 faces. Leur raccordement aux surfaces du dessus demande de conserver ces limites mesurées. Les sept flancs refusent les 28 mutations de rayon, d’azimut, de station et d’orientation de triangle.

Le helper du paquet de développement refuse de nouveau Flansch à **231 608 faces** avant construction native de la pièce. Temps de ce passage **67,843 s**, RSS maximale échantillonnée **573 014 016 octets**, empreinte **480 789 536 octets**. Aucun STEP du document. Les mesures concernent le laboratoire hors application. L’intégration dans Auto, la validation du solide entier, son volume et le coût de sa tessellation restent ouverts.

### Reproduction depuis le STL d’origine

Depuis la racine, utiliser le SDK OCCT 7.9.3 et un Python avec NumPy/SciPy. Le STL reste sur le poste de qualification. Les listes de sommets, les paramètres mesurés et les STEP de laboratoire sont écrits sous `artifacts/`, ignoré par Git. Les stations de recherche et les familles de faces attendues sont spécifiques à cette qualification.

```sh
export FABSTEP_PROFILE_STL="$HOME/Downloads/FlanschTurboLuftfilter_REV02.STL"
export FABSTEP_PROFILE_DIR="$PWD/artifacts/r085-replay/source"
assembly="$PWD/artifacts/r085-replay/assembly"
FABSTEP_DUMP_PLANES=1 FABSTEP_DUMP_CONES=1 FABSTEP_DUMP_ANALYTIC_SEAMS=1 FABSTEP_DUMP_OWNERS=1 cargo test -p fabstep-core --features large-stl --target-dir artifacts/r080/target --release --offline --locked --test residual_planar_profile -- --ignored --exact residual_planar_profile
python3 scripts/qualify-flange-assembly.py --check-negatives "$FABSTEP_PROFILE_DIR" "$assembly"
python3 scripts/qualify-flange-ownership.py --check-negatives "$FABSTEP_PROFILE_DIR" "$assembly/patches.tsv" "$assembly/counts.json"
sh scripts/qualify-torus-sectors.sh --pierced-side "$assembly/base.json" "$assembly/flanks.json" "$assembly/upper.json" "$assembly/pierced.json"
python3 scripts/qualify-flange-interface.py --check-negatives --map "$assembly/interface-map.json" "$assembly/source-contour.json" artifacts/r085-native/pierced-side/main-contour.json
```

Le rejeu depuis le STL reproduit à l’octet les paramètres, les groupes de triangles et les rapports de partition et d’interface. Les essais graphiques FabSTEP/Fusion de la configuration publique de V-1208 sont toujours suivis dans A-084 et A-085.

### Non-régression

`sh scripts/check.sh` termine avec le code 0. Les suites Rust totalisent 772 réussites, dont 588 dans le workspace et un transfert réel bureau/helper. Vingt et une occurrences de tests sont ignorées lors des passages par défaut, dont le transfert rejoué explicitement. Les 235 tests UI réussissent. Svelte signale zéro erreur et zéro avertissement. Formatage, Clippy strict et builds passent. Les douze tests Python historiques passent dans la suite globale. Après ajout des deux cas publics du contour, un rejeu avec NumPy réussit les quatorze tests Python. Avec le Python sans NumPy, seuls ces deux nouveaux tests sont ignorés. La sonde Rust privée `large-stl` et son contrôle Clippy ciblé passent aussi séparément. Le paquet de développement utilisé pour le rejeu Flansch est celui de V-1208.

## V-1210 — R-085, perçages, épaulements et marquages de Flansch

### Coque mesurée sur la pièce

Le rejeu part du STL de 540 170 triangles et retrouve le témoin de 34 faces de V-1209. Chaque ligne décrit une étape cumulative de la construction OCCT 7.9.3, contrôlée à nouveau après transfert STEP.

| Ajout | Triangles STL de l’ajout | Faces dans la coque | Contrôle après transfert |
| --- | ---: | ---: | --- |
| Trois flancs cylindriques à bord irrégulier | 409 | 37 | Trois arcs du bas, deux génératrices communes et 274 portions du contour supérieur. Les six ressauts sont conservés. |
| Cinq chaînes de perçage, dont le passage central, et son col plan | 286 678 | 60 | 22 faces de révolution ou plans annulaires, le plan du col et quatre sorties de petits perçages à X = 7 mm. |
| Deux épaulements plans percés | 2 310 | 62 | Quatre anneaux, quatre arcs et six segments droits partagés. Un rail se divise en trois segments contigus. |
| Dix marquages en creux | 5 984 | 1 814 | 1 752 faces planes, 1 004 raccords au flanc et 4 240 arêtes intérieures à deux usages opposés. |
| Extrémités des rails | 4 | 1 816 | Deux plans, huit arêtes partagées et deux paires de sommets identifiées sur la source. |
| Facettes bordant les flancs longs | 267 | 2 083 | 267 bases et 162 raccords latéraux partagés. Les 210 côtés libres de ces facettes rejoignent le prochain pourtour. |

Les 22 faces des chaînes restent à moins de 0,012861 mm du maillage selon leurs cartes continues. Les quatre sorties ont été revérifiées à la station commune de leurs épaulements. Les contacts sur les trois flancs longs conservent les deux plateaux et leurs relevés de hauteur pour une station basse commune à X = 6,5 mm. L’écart de leur bord transféré à la courbe attendue reste inférieur à 1,08 × 10⁻⁹ mm. La profondeur mesurée du marquage est de 0,300001 mm. Une variation de 0,01 mm est détectée après réimport même lorsque le nombre de faces et la validité de la coque sont conservés. Le témoin refuse aussi une paroi écourtée, un perçage rétréci, un changement de normale, un intervalle de rail oublié et un triangle intérieur en double.

### Attribution et raccords restants

Les 1 814 faces reprennent **459 773 triangles** et **153 943 anciens propriétaires complets**. Les **80 397 triangles** restants possèdent **77 665 propriétaires**, tous plans dans l’attribution Auto. Chaque triangle a une attribution unique. Si ces propriétaires sont conservés, le total est de **79 479 faces**. La fermeture du corps, son volume, sa tessellation et sa reconnaissance automatique restent à qualifier.

Le STEP à 1 814 faces contient une coque ouverte, zéro solide et **deux boucles**. Les 781 arêtes de la première suivent 959 segments source en sens opposé. Deux quarts de cercle rassemblent chacun 90 segments. Les 478 arêtes de la seconde suivent chacune un segment. Les extrémités concordent à moins de 0,000005574 mm. La borne continue cercle–corde reste sous 0,001671 mm. Le contrôle des lignes et splines du relevé global donne un écart **échantillonné** inférieur à 0,000422 mm. Le laboratoire vérifie séparément les courbes des flancs longs.

Le reliquat comporte une grande nappe de 80 393 triangles, avec deux contours de 951 et 478 segments. Deux rectangles de deux triangles occupent les extrémités des rails. Chacun touche le contour principal en un sommet qui a deux identités topologiques dans la coque partielle. Cette distinction explique pourquoi la seule comparaison du nombre de boucles ne suffirait pas à orienter les faces à raccorder.

Les deux plans d’extrémité se raccordent par huit arêtes existantes. À chaque extrémité, le parcours du contour et les triangles voisins confirment que deux sommets STEP coïncidents reprennent le même sommet source. Après cette identification locale, OCCT relit **1 816 faces**, une coque et zéro solide. Les normales, les huit incidences, les positions, les aires et les **1 251 autres arêtes libres** sont revérifiées. La borne de déplacement des deux rectangles est inférieure à 0,00000434 mm. Les deux cycles natifs comptent désormais **773 et 478 arêtes**. La mise en correspondance retrouve les **951 et 478 segments** des deux limites de la nappe restante, avec orientation opposée, sans branchement ni sommet répété. Les **459 777 triangles** déjà assemblés laissent **80 393 triangles pour 77 663 faces planes**. L’ajout des deux plans conserve le total projeté de 79 479 faces.

### Facettes bordant les trois cylindres longs

Une limite courbe du cylindre peut s’écarter du plan de la facette voisine de 0,001664 mm à mi-parcours. La sonde reprend la courbe et le sommet opposé du triangle mesuré pour construire une face réglée. Sur chaque triangle retenu, elle borne le déplacement du sommet, l’écart continu entre la courbe et la corde et le signe du jacobien projeté sur le plan du triangle. Ce dernier contrôle refuse un repli, même à faible distance du maillage.

Sur les 274 arêtes supérieures, **267** correspondent chacune à un propriétaire d’un seul triangle. Après réimport séparé de ces 267 faces, OCCT retrouve **trois plans, 184 faces réglées sur un arc et 80 sur un bord cubique**. Les génératrices du sommet restent rectilignes. L’écart continu au maillage, transfert inclus, est inférieur à **0,001671 mm**. Un sommet déplacé de 0,01 mm est détecté. Les sept autres arêtes appartiennent à des propriétaires de plusieurs triangles.

Les **267 faces** sont ensuite cousues à la coque de 1 816 faces. Le manifeste associe l’arête de base, les deux côtés et le sommet opposé aux trois points source de chaque triangle. Les 801 incidences orientées donnent 267 raccords au cylindre, 162 coutures entre nouvelles faces et 210 segments libres. Pour chaque base et chaque génératrice partagée, le contrôle retrouve le voisin source attendu, l’orientation opposée et la courbe sur la face. Le noyau conserve les 1 816 faces initiales. Un voisin interverti est refusé. Les 264 limites paramétriques dégénérées au sommet des faces réglées ont chacune un seul sommet et sont exclues du comptage des arêtes matérielles.

Le dernier STEP relu contient **2 083 faces**, **une coque ouverte**, **zéro solide** et **1 194 arêtes libres**. Les deux boucles physiques comprennent 716 et 478 arêtes. Elles reprennent, en sens opposé, les 1 372 arêtes qui séparent la coque des facettes restantes du STL. Les 460 044 triangles qualifiés laissent trois nappes de 52 956, 13 600 et 13 567 triangles, ainsi que trois triangles isolés. Les deux cycles du STEP ont chacun des sommets distincts en topologie. Dans le STL, ils se rencontrent en six sommets communs aux bords des composantes restantes. Les **80 126 triangles** de ce reliquat conservent **77 396 propriétaires plans**, soit un total projeté de **79 479 faces**. Les sept bords de flanc encore portés par des propriétaires à plusieurs triangles et les deux quarts de cercle à partager entre 90 voisins chacun font partie du prochain raccordement.

### Reproduction

Depuis la racine du dépôt, avec le SDK OCCT 7.9.3 et un interpréteur Python équipé de NumPy et SciPy, lancer la commande suivante. La sonde Rust compile en mode gros STL dans sa cible de laboratoire. Les coordonnées privées et les STEP intermédiaires restent sous le dossier de sortie.

```sh
PYTHON=python sh scripts/qualify-flange-replay.sh "$HOME/Downloads/FlanschTurboLuftfilter_REV02.STL" artifacts/r085-replay-v1210
```

Le rejeu produit notamment `ownership.json` (partition intermédiaire à 37 faces), `final/lettering.step` (coque marquée), `frontier/frontier.json` (boucles STEP), `source-frontier.json` (arêtes orientées et triangles voisins) et `remainder.json` (partition entière à cette étape). La coque à 1 816 faces se trouve dans `rail-shell/rail-ends.step`. Les fichiers `rail-shell/frontier.json`, `final-source-frontier.json` et `rail-partition.json` décrivent ses deux contours. Le témoin final est `trim-shell/trim-shell.step`. Le rapport de transfert `trim-shell/report.json`, le relevé OCCT `trim-frontier/frontier.json`, les incidences orientées `trim-source-frontier.json` et le recomptage `trim-partition.json` permettent de reprendre sur les six composantes restantes.

Le helper du paquet de développement V-1208 refuse de nouveau la conversion intégrale de Flansch à **231 608 faces** pour une limite résultat de 100 000. Sur ce passage, 74,848 s, RSS maximale échantillonnée de 440 401 920 octets et empreinte physique de 506 446 880 octets. Le dossier de sortie du helper ne contient aucun STEP de la pièce.

### Non-régression

Le rejeu complet depuis le STL et la suite `sh scripts/check.sh` se terminent avec le code 0. Les suites Rust cumulent **772 réussites** et 21 occurrences de tests ignorés dans les passages ordinaires. Le transfert bureau/helper a été exécuté explicitement. Les **235 tests UI** passent et Svelte rapporte zéro erreur et zéro avertissement. Formatage, Clippy strict et builds passent. Les **20 tests Python** passent avec NumPy/SciPy. Avec le Python système du poste, 12 passent et huit sont ignorés en l’absence des bibliothèques scientifiques. La sonde Rust opt-in de Flansch passe aussi sous Clippy avec la feature `large-stl`.

Le rapport final détecte douze altérations d’attribution ou de frontière source. Les expériences OCCT vérifient séparément les contre-exemples géométriques à chaque assemblage. Le produit et le paquet macOS conservent la version de V-1208. Les essais manuels FabSTEP et Fusion restent suivis dans A-084 et A-085.

## V-1211 — R-086, congé intérieur de poche à deux parois d’arrêt

**Cas qui change.** Sur la poche publique `fixtures/pocket_fillet/staggered.stl`, les deux extrémités du même quart de cylindre sont découpées en 16 et 23 intervalles. Avant R-086, Auto conservait 50 plans. Le helper du paquet de développement produit un STEP dont FreeCAD 1.1.1 relit un solide fermé valide, 11 plans et un cylindre. Rayon relu 1,999999933 mm, longueur 16 mm, volume 5 773,734516628 mm³ pour 5 773,734517543 mm³ attendus. La découpe différente des deux arcs suffit à expliquer le changement. La poche nominale était déjà reconnue.

| Épreuve | Résultat |
| --- | --- |
| Cinq STL positifs, dont une rotation oblique en ASCII, un remaillage et un rayon de 3 mm du côté opposé | 11 plans et un quart de cylindre concave dans chaque solide réimporté. Vérification des deux tangences, des deux parois d’arrêt, des points de matière et de vide, de l’attribution exhaustive des triangles et de la fidélité dans les deux sens. |
| Même rotation en STL binaire f32 | Solide fidèle à 142 plans. L’arrondi des coordonnées rend les deux génératrices incompatibles avec le seuil natif de 10⁻⁷ mm aux coins. Écarts source/résultat vérifiés dans les deux sens sous 10⁻⁵ mm. |
| Fond absent ou paroi déformée | Le fond ouvert entraîne `mesh.open`. Avec un sommet décalé de 0,16 mm à l’intérieur du congé ou sur une butée, Auto conserve le solide facettisé, respectivement à 33 et 50 plans. |
| Arête extérieure de bloc | Le rejeu avec un seul segment supplémentaire passe de 24 plans à 6 plans et un cylindre. Le test core subdivise ensuite l’un ou l’autre des deux arcs de 16 à 32 intervalles. Il vérifie la flèche du côté le plus grossier (0,004818 mm) et le refus sous 90 % de cette borne. |
| Autre poche, construite directement dans FreeCAD | Deux maillages de 80 et 156 triangles. Les STEP réimportés dans FreeCAD ont chacun un solide à 12 faces, un congé de rayon 2,5 mm entre les plans Y = 6 et 31 mm, et un volume de 19 583,531530 et 19 583,531532 mm³ pour 19 583,531537 mm³ attendus. |
| Pièce privée `body 26mm.stl`, conversion Auto | 1 solide, 10 848 plans, 14 cylindres et 7 cônes. Le volume de 17 053,652458875804 mm³ et les nombres de faces reproduisent le contrôle du raccord cylindre–chanfrein. Écarts échantillonnés de 0,002619 et 0,003052 mm dans les deux sens. |
| Flansch privé, 540 170 triangles, helper du paquet | Refus reproduit à 231 608 faces pour un plafond de 100 000, code `step.invalid_source_provenance`. Aucun document STEP produit. |

### Reproduction

Le corpus public et son manifeste sont régénérables avec la bibliothèque standard Python. Le contrôle indépendant de FreeCAD crée la pièce avec ses propres opérations CAO, écrit deux STL, appelle la CLI puis relit ses deux STEP. Depuis la racine du projet sur le poste macOS équipé de FreeCAD :

```sh
python3 scripts/generate-pocket-fillet-fixtures.py artifacts/r086/regenerated
cargo test -p fabstep-app --offline --locked --test pocket_fillet
cargo test -p fabstep-core --offline --locked independently_sampled_convex_fillet
cargo build -p fabstep-cli --offline --locked
PYTHONPATH=/Applications/FreeCAD.app/Contents/Resources/lib \
  /Applications/FreeCAD.app/Contents/Resources/bin/python \
  scripts/validate-pocket-fillet-freecad.py target/debug/fabstep artifacts/r086/freecad
```

L’essai manuel FabSTEP et Fusion est suivi dans A-086. Le cas qualifié est un congé isolé entre deux butées perpendiculaires à son axe. Une butée oblique demande une autre preuve du contour.

### Non-régression et paquet

Les contrôles de `scripts/check.sh` ont été exécutés en deux passages. Après les **592 tests workspace** réussis, rustdoc ne retrouvait plus la bibliothèque locale `fabstep_import` dans la cible Cargo partagée avec une autre compilation. Le rejeu séquentiel de `cargo test --workspace --doc --offline --locked`, puis des commandes restantes du script, réussit. Au total, 777 passages de tests Rust réussis (SDK, bureau, worker, gros STL et transfert réel compris) et 21 occurrences ignorées dans les suites par défaut. Les trois tests de poche repassent séparément sous les deux profils de taille. Les 235 tests UI passent, Svelte compte zéro erreur et zéro avertissement, Clippy strict, formatage et builds passent. Vite signale la taille du bundle. Les 20 tests Python passent avec l’interpréteur scientifique, dont huit sont ignorés avec le Python système. Les deux exécutables du paquet macOS de développement reconstruit ont été comparés octet par octet aux binaires compilés.

## V-1212 — R-087, cylindre de poche avec une butée oblique

### Situation initiale mesurée

Le corpus public `fixtures/oblique_pocket/` reprend une poche de rayon 2 mm dans un bloc de 30 × 20 × 12 mm. La paroi droite et la paroi inclinée arrêtent le même congé. Sur le cas nominal, le contour théorique de la butée inclinée a pour demi-axes √5 et 2 mm. Le volume de matière attendu vaut 5 771,780239856 mm³.

| STL | Faces planes réimportées par Auto (build R-086) | Différence examinée |
| --- | ---: | --- |
| nominal | 46 | Butée de pente +0,5 |
| reverse | 54 | Pente −0,5 |
| staggered | 76 | 16 et 23 intervalles sur les arcs opposés |
| rotated | 27 | Rotation de la pièce, coordonnées ASCII de haute précision |
| compound | 60 | Inclinaison de la butée suivant deux directions |
| warped_stop | 76 | Sommet déplacé radialement sur la butée |

Chaque résultat est un solide fermé facettisé. Sur le cas nominal, 20 faces planes se partagent la butée oblique. À triangles et placement identiques, un STL ASCII de haute précision donne 27 plans dont un seul pour cette butée. Sur le cas déformé, le test applicatif contrôle le volume, l’attribution exhaustive et la fidélité dans les deux sens.

### Lecture du STL sans manifeste

La sonde de laboratoire `scripts/qualify-oblique-pocket.py` déduit les deux plans tangents, le cercle et l’ellipse à partir du seul STL. Elle exige onze régions planes dominantes et une paroi résiduelle connexe dont les quatre chaînes de bord sont partagées. Elle vérifie l’orientation et la couverture des triangles en coordonnées angle–hauteur. L’interpolant linéaire des hauteurs de la butée définit une correspondance avec le domaine exact. La borne cumule la flèche radiale, l’erreur axiale d’interpolation de la butée et le déplacement maximal des sommets. Elle concerne la paroi du congé. L’intégration exige également la preuve des boucles planes, des jonctions natives et du solide entier.

Sur les cinq STL publics, la borne dans les deux sens va de 0,00134 à 0,00270 mm (valeurs arrondies). Sur la seconde poche FreeCAD, elle vaut 0,00831 et 0,00098 mm. Le sommet déformé entraîne un refus, avec un résidu supérieur à 0,17 mm pour un seuil de 0,00001 mm. La sonde est réservée aux petits maillages (4 096 triangles, 2 Mio) et emploie NumPy. Le moteur Rust de l’application ne l’appelle pas.

### Deuxième pièce indépendante

FreeCAD 1.1.1 construit une poche de rayon 2,5 mm de dimensions différentes, avec une pente de 0,75 et un cylindre d’axe Y. Dans le STEP de référence relu, le solide fermé comprend onze plans et un cylindre concave. Les deux génératrices mesurent 23 et 24,875 mm. La paroi d’arrêt inclinée partage avec le cylindre une ellipse de demi-axes 3,125 et 2,5 mm. Volume théorique 20 109,552139012 mm³, volume réimporté 20 109,552138972 mm³. Le script vérifie également huit positions dans la matière ou dans le vide. La CLI R-086 restitue respectivement 53 et 116 faces planes sur les deux STL indépendants de 82 et 164 triangles.

Sur le STEP de référence, le noyau de FabSTEP extrait le quart d’ellipse et le quart de cercle sur deux faces adjacentes au même cylindre. L’export DXF écrit une entité ELLIPSE et une entité ARC sur leurs profils respectifs. Le STEP de référence de FreeCAD ne sérialise pas les courbes dans les coordonnées des surfaces (PCURVE). À la relecture, sur 257 valeurs du même paramètre, l’écart maximal entre l’arête 3D et sa représentation sur la surface est de 1,95 × 10⁻⁶ mm sur le cylindre et de 4,1 × 10⁻¹⁵ mm sur le plan. La tolérance d’arête déclarée est de 10⁻⁷ mm. Ces mesures motivent un contrôle explicite du futur transfert natif.

### Reproduction

```sh
python3 scripts/generate-oblique-pocket-fixtures.py artifacts/r087/regenerated
python scripts/qualify-oblique-pocket.py fixtures/oblique_pocket/nominal.stl
python -m unittest discover -s scripts/tests
cargo test -p fabstep-app --locked --offline --test oblique_pocket --test pocket_fillet
cargo test -p fabstep-cad --locked --offline --test oblique_pocket_profiles
cargo build -p fabstep-cli --locked --offline
PYTHONPATH=/Applications/FreeCAD.app/Contents/Resources/lib \
  /Applications/FreeCAD.app/Contents/Resources/bin/python \
  scripts/validate-oblique-pocket-freecad.py artifacts/r087/freecad --cli target/debug/fabstep --baseline
```

L’option `--baseline` mesure le résultat initial. Sans cette option, le script exige de chaque conversion une poche analytique équivalente à la référence. Les essais natifs et DXF ci-dessus portent sur le STEP généré par FreeCAD. L’intégration du nouveau contour dans Auto relève d’ADR-0041.

Contrôles du lot préparatoire : les cinq tests Rust ciblés passent (profil natif, repli sur déformation et non-régression R-086), ainsi que les 23 tests Python avec NumPy. Le manifeste et les six STL sont reproductibles octet par octet. Formatage Rust et Python, Ruff, Clippy strict sur les cibles app et CAD et validation FabBoard passent. La suite globale et les essais manuels dans Fusion n’ont pas été rejoués pour ce lot de qualification. Le paquet de développement reste celui de R-086.

## V-1213 — R-087 intégré, STEP de la poche à butée oblique

Auto et Surfaces reconstruisent les cinq STL publics de V-1212 en **un solide à 11 plans et un cylindre concave**. Le contrôle applicatif compare le volume et les dimensions de chaque solide réimporté au manifeste indépendant. En Auto, chaque groupe de triangles est attribué à sa face native. Le cylindre a quatre voisins : le fond, la paroi tangente, la butée droite et la butée oblique. Le profil de la première butée contient un arc circulaire et celui de la seconde une ellipse. Les tests comparent son centre, ses demi-axes et son point milieu aux valeurs attendues, puis vérifient l’entité ELLIPSE dans le DXF de la face.

| Essai | Résultat après réimport |
| --- | --- |
| Poche nominale R2, pente 0,5 | 46 plans avant R-087. FreeCAD relit le STEP du helper de développement avec 11 plans et un cylindre, pour un volume de 5 771,780239857 mm³. La borne composée du core vaut 0,002706 mm. Les écarts échantillonnés sont de 0,001908 mm du STL vers le résultat et 0,002320 mm en sens inverse. |
| Seconde poche dessinée dans FreeCAD, R2,5, pente 0,75 | Avant R-087, 53 plans et 116 plans sur les maillages de 82 et 164 triangles. Après R-087, chaque STEP restitue les demi-axes 3,125 et 2,5 mm et le volume 20 109,552139011 mm³. Les quatre arêtes et les huit points de matière/vide sont contrôlés en mode strict. L’écart entre l’arête elliptique 3D et sa courbe sur le cylindre, échantillonné à 257 paramètres, atteint 1,78 × 10⁻⁹ mm. |
| Sommet déformé sur la butée | Auto restitue un solide à 76 plans. La source reste intégralement attribuée et les écarts dans les deux sens restent sous 10⁻⁵ mm. |
| Deux poches, triangles entrelacés et décalage de 16 mm en Z | 2 solides, 22 plans, 2 cylindres. Chacun conserve ses quatre voisins et l’attribution à ses propres triangles du fichier. Le contrôle natif refuse deux corps superposés. |
| Contrat natif | Une troisième poche construite directement en Rust vérifie le volume et les identités des quatre voisins. Treize mutations invalides des courbes, des coins et de la topologie sont refusées. Les anciens modèles à deux arcs circulaires conservent leur représentation JSON. |
| Pièces privées | `body 26mm.stl` retrouve 14 cylindres, 7 cônes et 10 869 faces au total, avec un volume de 17 053,652458875804 mm³. Le helper refuse Flansch à 231 608 faces pour un plafond de 100 000, sans produire de STEP. |

**Limites observées.** La preuve porte sur un corps comportant onze régions planes dominantes, un congé de fond concave à 90° et une butée de chaque type. Une translation peut modifier les coordonnées effectivement encodées dans un STL binaire. Sur le témoin nominal translaté de 40 mm selon X puis réencodé en f32, l’écart minimal de la butée à un plan dépasse la tolérance de planéité de 10⁻⁶ mm. Auto conserve les 46 plans. Un budget de déviation de 0,0001 mm sur le témoin nominal entraîne `recognition.deviation_exceeded`.

### Reproduction

```sh
cargo test --offline --locked -p fabstep-core --test oblique_pocket
cargo test --offline --locked -p fabstep-cad --test oblique_pocket_contract
cargo test --offline --locked -p fabstep-app --test oblique_pocket
cargo build -p fabstep-cli --offline --locked
PYTHONPATH=/Applications/FreeCAD.app/Contents/Resources/lib \
  /Applications/FreeCAD.app/Contents/Resources/bin/python \
  scripts/validate-oblique-pocket-freecad.py artifacts/r087-integrated/freecad --cli target/debug/fabstep
```

Le contrôle indépendant doit être lancé sans `--baseline` pour exiger les surfaces analytiques. Le helper embarqué dans `artifacts/FabSTEP.app` a aussi converti le témoin nominal, le maillage FreeCAD fin et le témoin déformé. Les deux exécutables de l’application locale ont été comparés octet par octet aux binaires de développement.

Le 18 novembre 2026, `sh scripts/check.sh` a terminé avec succès. Sur l’ensemble des profils, 795 exécutions de tests Rust ont réussi et 21 ont été ignorées. Le passage du bureau au helper réel sur macOS a réussi. Les 235 tests d’interface passent, Svelte signale zéro erreur et zéro avertissement, et les builds aboutissent. Les 23 tests Python passent avec NumPy. L’interpréteur sans NumPy en ignore 11. Le test ciblé de sélection du congé passe aussi dans le profil ordinaire. L’essai manuel dans FabSTEP et Fusion reste suivi par A-088.

## V-1214 — Deux congés de fond opposés (R-088)

Le binaire du commit `789fdf5` reconnaît déjà deux bandes cylindriques concaves indépendantes qui partagent le même fond et les mêmes butées. Le jalon enregistre les cas dans la suite de tests. Les vérifications portent sur les chemins Auto et Surfaces et sur le helper de `artifacts/FabSTEP.app`.

| Source | Résultat réimporté |
| --- | --- |
| Poche nominale, 266 triangles | 1 solide, 11 plans et 2 cylindres de rayons 2 et 3 mm. Volume 5 804,637181484 mm³ pour 5 804,637182013 mm³ théoriques. |
| Même poche, 350 triangles, trois rangées à subdivisions différentes par cylindre | Mêmes treize faces, mêmes rayons, même volume à 10⁻⁶ mm³ près. |
| Poche tournée et translatée, 410 triangles | Deux rayons de 1,5 et 3,5 mm et 11 plans. Volume 5 312,011075014 mm³ pour 5 312,011079353 mm³ attendus. |
| Maillage fermé avec un sommet du raccord droit déformé de 0,16 mm | Un seul cylindre sur le groupe gauche. Les 58 autres faces sont planes. Tous les triangles restent attribués et les écarts échantillonnés dans les deux sens respectent la tolérance. |
| Autre pièce construite dans FreeCAD 1.1.1, 132 puis 284 triangles | Une poche à deux cylindres de rayons 1,75 et 3,25 mm, de longueur 23 mm. Le volume théorique est de 21 020,250850545 mm³. L’erreur absolue maximale au réimport est inférieure à 0,000006 mm³. |

Sur les trois sources positives, le test d’intégration associe chaque cylindre à ses triangles source et à ses quatre voisins. Les profils des butées contiennent deux quarts de cercle aux centres et rayons prévus dans le repère monde. Chaque DXF encode deux entités `ARC`. Le script FreeCAD contrôle indépendamment les quatre arêtes par cylindre, le partage des deux butées et du fond, les dimensions, la concavité et dix sondes dans la matière ou dans le vide. Les quatre fichiers publics ont aussi été convertis avec le helper embarqué. Les congés de ce domaine sont séparés par un fond plan.

```sh
cargo test -p fabstep-app --test opposed_pocket --offline --locked
python3 scripts/generate-opposed-pocket-fixtures.py artifacts/r088/reproduced
diff -r -x README.md fixtures/opposed_pocket artifacts/r088/reproduced
PYTHONPATH=/Applications/FreeCAD.app/Contents/Resources/lib \
  /Applications/FreeCAD.app/Contents/Resources/bin/python \
  scripts/validate-opposed-pocket-freecad.py artifacts/r088/freecad --cli target/debug/fabstep
```

## V-1215 — Mesure initiale de la jonction de deux congés adjacents (R-089)

La pièce indépendante comporte une poche borgne dont deux arêtes de fond perpendiculaires portent un congé de rayon 2,5 mm. Dans le STEP de référence FreeCAD 1.1.1, les onze plans et les deux cylindres forment un seul solide fermé, d’un volume théorique de 21 005,152485090 mm³. Chaque cylindre rejoint un plan de fond, une paroi latérale, une butée droite et l’autre cylindre. Une même arête en quart d’ellipse de demi-axes 3,535534 et 2,5 mm appartient aux deux cylindres. Sa longueur est de 4,775247 mm.

Après réimport de la référence, les deux tracés paramétriques ont été inspectés sur 257 points chacun. L’écart échantillonné atteint 0,000002593 mm sur chaque face. Cette mesure est supérieure à la tolérance déclarée de l’arête (0,0000001 mm). L’implémentation devra borner et contrôler le tracé sur chaque support pendant et après le transfert STEP. Le cas R-087 vérifiait une arête sur un cylindre et un plan.

| Source | Référence | Auto à la qualification |
| --- | --- | --- |
| STL public nominal, 320 triangles | Deux congés de rayon 2 mm, volume 5 781,551628288 mm³ | 43 plans |
| STL public tourné, 648 triangles | Deux congés de rayon 3 mm, volume 5 311,353996706 mm³ | 57 plans |
| Témoin fermé avec un sommet de la jonction déplacé | Cette jonction ne suit plus les deux cylindres nominaux | 51 plans |
| STL maillé indépendamment par FreeCAD, 114 triangles | Deux congés de rayon 2,5 mm | 71 plans, écart de volume +0,303779 mm³ |
| Même pièce FreeCAD, 240 triangles | Même géométrie analytique | 159 plans, écart de volume +0,049864 mm³ |

Les deux conversions du maillage FreeCAD passent les contrôles de fermeture, de dimensions et les 16 sondes matière/vide. Le mode strict du script exige l’ellipse commune dans les deux résultats et échoue sur le premier résultat facettisé. Cette mesure précède l’intégration. Les résultats analytiques se trouvent dans V-1216. Voir [ADR-0042](adr/0042-adjacent-pocket-fillets.md).

```sh
python3 scripts/generate-adjacent-pocket-fixtures.py artifacts/r089/corpus
diff -r -x README.md fixtures/adjacent_pocket artifacts/r089/corpus
PYTHONPATH=/Applications/FreeCAD.app/Contents/Resources/lib \
  /Applications/FreeCAD.app/Contents/Resources/bin/python \
  scripts/qualify-adjacent-pocket-freecad.py artifacts/r089/freecad --cli target/debug/fabstep
```

Ajouter `--require-analytic` à la dernière commande pour vérifier une future implémentation.

## V-1216 — R-089 intégré, STEP de la poche à congés adjacents

Les STL de V-1215 et le témoin court sont convertis par les parcours Auto et Surfaces. Sur chaque positif, le STEP réimporté a onze plans et deux cylindres concaves. Les deux faces se partagent la même ellipse. Chacune conserve ses triangles source en Auto, une butée circulaire et deux rails rectilignes.

| Source ou contrôle | Mesure obtenue |
| --- | --- |
| Nominal public, rayon 2 mm | 13 faces contre 43 plans auparavant, volume 5 781,551628296 mm³. Borne source de 0,003419 mm. Les écarts mesurés entre les maillages source et résultat sont de 0,002053 et 0,003289 mm pour une tolérance de 0,05 mm. |
| Poche tournée, rayon 3 mm | 13 faces contre 57 plans auparavant, volume 5 311,353996726 mm³. |
| Poche courte de 3,75 mm, rayon 2 mm | 13 faces, volume 6 873,536138299 mm³. La revue a aussi reproduit à 3,99 mm un refus lié au prolongement de l’ellipse au-delà des deux rails. Après correction, Auto réimporte deux cylindres sur ce cas et le test `short.stl` couvre le quart réellement utilisé. |
| Deux STL indépendants FreeCAD | 114 et 240 triangles. Les anciens résultats à 71 et 159 plans deviennent chacun un solide à 11 plans et 2 cylindres. Volume relu 21 005,152484857 mm³ pour 21 005,152485090 mm³ théoriques. Les 16 sondes matière/vide passent. |
| Arête partagée du STEP de FreeCAD | Demi-axes relus de 3,535534 et 2,5 mm. Écart maximal mesuré de 2,363 × 10⁻⁹ mm sur 257 points par tracé paramétrique. Le noyau contrôle aussi les courbes sur les deux supports après transfert. |
| Reconstruction multi-corps | Une poche adjacente et une poche à butée oblique de R-087, placées séparément et entrelacées dans le STL, donnent 2 solides, 22 plans et 3 cylindres. Leurs identités et leurs voisins restent dans leur corps. |
| Cas limites | Le sommet de jonction déformé donne 51 plans. Un étirement sur X seul empêche la jonction. La réflexion suivie d’une rotation en précision ASCII conserve les cylindres et le volume. Une tolérance de déviation de 0,0001 mm refuse la reconstruction analytique. |
| Contrat CAO | Une troisième poche construite algébriquement vérifie le volume, les voisins, les labels, le sens de l’ellipse et l’ordre des parois. Les mutations de courbe, de rayon, de rails et d’incidence sont refusées. Pour un rayon de 2,5 mm, une longueur de poche de 2,51 mm passe le contrôle natif, le pincement à 2,50 et le croisement à 2,49 sont refusés. |

Le paquet de développement a converti les trois positifs du corpus public, le témoin déformé et le maillage fin FreeCAD. Le STEP du helper a été relu indépendamment avec le contrôle strict. Le contrôle manuel dans Fusion est suivi dans FabBoard.

```sh
cargo test --locked --offline -p fabstep-core --test adjacent_pocket
cargo test --locked --offline -p fabstep-cad --test adjacent_pocket_contract --test oblique_pocket_contract
cargo test --locked --offline -p fabstep-app --test adjacent_pocket --test opposed_pocket --test oblique_pocket
cargo build --locked --offline -p fabstep-cli
PYTHONPATH=/Applications/FreeCAD.app/Contents/Resources/lib \
  /Applications/FreeCAD.app/Contents/Resources/bin/python \
  scripts/qualify-adjacent-pocket-freecad.py artifacts/r089/freecad --cli target/debug/fabstep --require-analytic
```

Le rejeu des pièces privées avec le helper du jalon donne toujours 10 869 faces pour `body 26mm.stl`, dont 14 cylindres et 7 cônes, et un volume de 17 053,652458876 mm³. Sur Flansch, le helper indique 231 608 faces pour un plafond de 100 000 et aucun STEP exportable.

Le 18 novembre 2026, la suite `sh scripts/check.sh` a terminé avec succès. Elle compte 815 exécutions de tests Rust réussies sur les différents profils et 21 ignorées. Les 235 tests d’interface, les contrôles Svelte et Clippy, les builds et le passage du bureau au helper réel ont réussi. Les 23 tests Python passent avec NumPy, l’interpréteur sans NumPy en ignore 11. Après la correction des poches courtes, le contrôle de hauteur et les tests core, CAO et application du jalon ont aussi été rejoués séparément. Les essais manuels R-088 et R-089 sont suivis par A-090 et A-091.

## V-1217 — Qualification du coin à deux rayons distincts (R-090)

Le corpus public et la pièce construite indépendamment dans FreeCAD 1.1.1 portent sur les deux congés adjacents d’une poche. Les deux rayons sont différents. La branche commune est non plane. Elle se termine à la hauteur du petit rayon, où le grand cylindre rejoint la paroi verticale du petit le long d’un autre arc de cercle. Le solide de référence a onze plans et deux cylindres.

| Témoin | Géométrie de référence | Auto avant intégration |
| --- | --- | --- |
| `nominal.stl`, 416 triangles | Rayons 2 et 3 mm, volume 5 791,673203086 mm³ | 51 plans |
| `reflected.stl`, 912 triangles | Rayons échangés, 3,5 et 1,5 mm, pièce réfléchie, volume 5 308,068388490 mm³ | 69 plans |
| `warped_seam.stl`, jonction déformée | Maillage fermé. Un sommet quitte les deux supports cylindriques attendus. | 55 plans |
| Autre poche, FreeCAD, 116 et 246 triangles | Rayons 1,75 et 3,25 mm, volume de référence 21 015,076001588 mm³ | 69 et 155 plans, volumes 21 015,481149277 et 21 015,139973474 mm³ |

Les 18 sondes de matière et de vide passent sur la référence FreeCAD et ses deux conversions. Sur sa jonction, la distance maximale échantillonnée au plan passant par trois points est de 0,031214 mm. Le grand cylindre a cinq arêtes et le petit en a quatre. Le troisième arc circulaire mesure 1,559118 mm. Un test de régression du core vérifie que ces trois STL publics ne se voient pas attribuer l’ellipse commune du cas de même rayon.

### Courbe spatiale et tracés sur les cylindres

Avec `q = 1 − r/R`, le dénominateur commun est `D = (1 − qt²)² + 4t²`. Les identités des deux cylindres sont vérifiées en substituant les coefficients donnés dans le script de courbe. Le dénominateur reste positif et les cinq poids de Bézier sont positifs. Le STEP de l’arête seule, relu sur trois couples de rayons (3/2, 7/1 et 4/3,99 mm), conserve le degré quatre et cinq pôles. L’écart maximal aux points de référence échantillonnés est inférieur à 2,85 × 10⁻¹³ mm.

Dans le paramètre de la courbe 3D, les deux tracés cylindriques utilisent des cubiques d’Hermite. Les dérivées angulaires et les positions axiales ont le même dénominateur `D ≥ 1`. La formule du reste d’Hermite et la majoration des dérivées donnent, en arithmétique réelle, des bornes d’interpolation de 9,18 × 10⁻⁹, 1,34 × 10⁻⁹ et 7,65 × 10⁻¹⁰ mm, respectivement, avec 256, 512 et 512 intervalles. Les tracés 2D construits dans FreeCAD donnent un écart maximal échantillonné inférieur à 3,05 × 10⁻¹¹ mm sur ces trois exemples. Les arrondis du noyau et les transformations dues au STEP sont contrôlés séparément.

### Transfert sur le solide de référence

La sonde OCCT 7.9.3 remplace l’arête de la pièce construite en CAO par la quartique exacte, en conservant les deux sommets et les faces voisines. Poser uniquement l’arête 3D laisse les deux cylindres sans tracé paramétrique. La projection générique rétablit le solide, mais le contrôle natif retrouve un écart allant jusqu’à 1,71 × 10⁻⁶ mm après transfert.

En attachant les deux tracés d’Hermite au même paramètre, le solide reste valide avant et après réimport, avec 256 intervalles par support. OCCT mesure au plus 2,26 × 10⁻¹¹ mm après transfert pour une tolérance d’arête de 10⁻⁷ mm. FreeCAD retrouve la quartique à cinq pôles, les 18 sondes passent et le volume est de 21 015,076001598 mm³. Ce parcours part des faces et sommets d’une référence CAO. Il restera à reconstruire le solide à partir des triangles du STL.

Pour reprendre la reconnaissance, le sondage source a utilisé les cinq plans qui touchent la zone résiduelle. Le fond est orthogonal aux quatre autres. La hauteur des interfaces au-dessus du fond distingue les deux butées et les deux plans tangents. Les rayons mesurés permettent de classer les triangles de chaque bande par leurs sommets. Sur les STL FreeCAD, les chaînes communes comptent 14 et 35 segments et les arcs supplémentaires 4 et 10 segments. Sur le témoin déformé, certains triangles ne satisfont aucun des deux cylindres. Il reste à démontrer la couverture continue, l’orientation et la fidélité de chaque bande avant toute conversion.

### Reproduction

```sh
python3 scripts/generate-unequal-pocket-fixtures.py artifacts/r090/public-corpus
diff -r -x README.md fixtures/unequal_pocket artifacts/r090/public-corpus
cargo test --locked --offline -p fabstep-core --test adjacent_pocket
cargo build --locked --offline -p fabstep-cli
PYTHONPATH=/Applications/FreeCAD.app/Contents/Resources/lib \
  /Applications/FreeCAD.app/Contents/Resources/bin/python \
  scripts/qualify-unequal-pocket-freecad.py artifacts/r090/reference --cli target/debug/fabstep
sh scripts/qualify-unequal-pocket-transfer.sh artifacts/r090/transfer
```

L’option `--require-analytic` du contrôle FreeCAD demande la quartique et les deux supports sur le résultat de la conversion STL. Avant intégration, elle échoue à la première pièce facettisée. Le script de transfert contrôle un solide synthétique indépendant de la CLI. Les scripts `qualify-unequal-pocket-curve.py` et `qualify-unequal-pocket-traces.py` permettent de rejouer les vérifications des courbes seules avec l’interpréteur de FreeCAD.

## V-1218 — Conversion STL des deux congés adjacents de rayons différents (R-090)

La CLI de FabSTEP a converti les deux maillages indépendants de V-1217 en mode Auto. Le contrôle strict dans FreeCAD 1.1.1 a réimporté les fichiers STEP produits par la CLI. Sur chaque fichier il a retrouvé un solide, onze plans, deux cylindres concaves de rayons 1,75 et 3,25 mm et une seule arête commune non plane. Le petit cylindre a quatre bords, le grand en a cinq. Les dimensions sont 46 × 38 × 14 mm. Les 18 points témoins de matière et de vide sont classés correctement sur chaque solide.

| STL de référence | Ancien STEP | Nouveau STEP | Borne continue source/forme | Écart échantillonné source → tessellation / retour |
| --- | --- | --- | --- | --- |
| Grossier, 116 triangles | 69 plans | 13 faces | 0,016128 mm | 0,004756 / 0,005916 mm |
| Fin, 246 triangles | 155 plans | 13 faces | 0,002725 mm | 0,002476 / 0,002326 mm |

FreeCAD mesure 21 015,076001589 mm³ sur les deux STEP. L’intégration indépendante des sections donne 21 015,076001588 mm³. La courbe commune transférée est une quartique rationnelle à cinq pôles. Les deux tracés sur les cylindres s’en écartent de moins de 1,8 × 10⁻¹² mm sur 257 échantillons par face. La tolérance de l’arête est de 10⁻⁷ mm. Les mesures échantillonnées du tableau utilisent la tessellation du solide avec une déflexion de 0,005 mm.

Les tests `unequal_pocket` de l’application contrôlent les pièces publiques de rayons 2 et 3 mm, puis 1,5 et 3,5 mm en miroir, en modes Auto et Surfaces. En Auto, les groupes de triangles source correspondent exactement aux treize faces. Le DXF de la paroi adjacente contient l’arc supplémentaire au rayon du grand congé. Le témoin à jonction déformée garde son solide facettisé. Le test CAO `unequal_pocket_contract` vérifie les jetons dans un STEP à deux racines et le refus d’une quartique aux poids altérés.

Un STL tourné de 0,31 rad autour de Z puis 0,52 rad autour de X et réécrit en coordonnées f32 produisait 416 plans lors du premier essai. La recherche des onze régions peut désormais reprendre avec la tolérance cylindrique, plafonnée à 0,00001 mm. Les supports sont ensuite mesurés sur les triangles et vérifiés à la tolérance de planéité demandée (0,000001 mm par défaut). Le même fichier donne 13 faces et une borne de 0,006913 mm. Un sommet du fond déplacé de 0,000005 mm empêche la reconnaissance analytique. Le moteur livré dans le paquet macOS a reconverti les deux témoins publics et le STL incliné en 13 faces. Le témoin déformé y donne un solide de 55 faces planes.

Deux couples de rayons supplémentaires, 1,25/4,25 mm et 2,25/2,5 mm, donnent également 13 faces en Auto. Le test de document avec les triangles de deux pièces entrelacés vérifie la provenance d'une poche à deux rayons et d'une poche à un congé à butée oblique dans le même STEP.

Sur Flansch (540 170 triangles), le moteur isolé de ce build retrouve 231 608 faces. Le plafond de suivi du résultat est de 100 000 faces. Il renvoie `step.invalid_source_provenance` et ne produit aucun STEP.

Pour rejouer le contrôle indépendant depuis la racine du dépôt :

```sh
cargo build --locked --offline -p fabstep-cli
PYTHONPATH=/Applications/FreeCAD.app/Contents/Resources/lib \
  /Applications/FreeCAD.app/Contents/Resources/bin/python \
  scripts/qualify-unequal-pocket-freecad.py artifacts/r090/product-reference \
  --cli target/debug/fabstep --require-analytic
```

**Contrôles de développement.** Le passage général des exécutables de test du workspace compte 626 tests réussis et 16 ignorés. La phase rustdoc de ce passage a rencontré des dépendances de compilation périmées après reconstruction des binaires pendant les tests. Le rejeu isolé de `cargo test --workspace --doc --offline --locked` passe pour tous les crates. Les suites ciblées ont été rejouées après les ajustements finaux, y compris les sept tests R-090 du noyau et les profils ordinaires et `large-stl`. Format Rust, Clippy strict, 236 tests d’interface, contrôle Svelte, compilation bureau/helper et essai du helper embarqué passent. Le contrôle interactif dans Fusion est ouvert dans A-093.

## V-1219 — R-091, perçages sécants de rayons plus proches

Le STL historique `f_ratio.stl` (rayons 3 et 5,5 mm) donnait 306 plans avec le build R-090. Auto reconstruit maintenant un solide fermé à six plans et trois parois cylindriques. Le grand trou communique avec les deux parties du petit trou. Les deux coutures communes sont fermées sur les sept STEP contrôlés par le script FreeCAD. Les 84 sondes matière/vide passent, ainsi que les mesures de rayons, d’axes et de dimensions. L’écart de volume mesuré dans FreeCAD par rapport à l’intégrale des sections reste inférieur à 0,0000061 mm³.

| Source STL | Rayons en mm | Faces STEP | Borne de fidélité en mm, arrondie par excès |
| --- | --- | ---: | ---: |
| Historique | 3 et 5,5 | 9 | 0,013752 |
| Incliné, ASCII | 3 et 5 | 9 | 0,008933 |
| Miroir, proche du seuil | 3 et 4,51 | 9 | 0,006597 |
| FreeCAD, 884 et 1 766 triangles | 3 et 5 | 9 chacun | 0,021398 et 0,003347 |
| FreeCAD avec subdivisions axiales, 1 388 et 2 774 triangles | 3 et 5 | 9 chacun | 0,021439 et 0,003366 |

Sur les deux STL FreeCAD à 884 et 1 766 triangles, le rejeu du commit R-090 produit respectivement 586 et 1 178 faces planes. La CAO certifie les trois STEP de la première partie du tableau avec une borne continue inférieure à 6,7 × 10⁻⁸ mm sur les courbes communes et sur leurs tracés paramétriques. La plus grande exécution nécessite 5 197 118 opérations pour ces arêtes, dans le budget de 10 millions. Le laboratoire F0 conserve ses refus sur les courbes perturbées.

Sur les deux STL de rayons 3 et 4,5 mm puis 3 et 4,49 mm, Auto produit 298 faces planes. Les tests du noyau vérifient aussi le refus lorsque les seules arêtes de bande qui confirment le rayon sont concentrées dans un secteur trop étroit. Les tests de document mélangent les facettes de deux pièces et vérifient les dix-huit faces, leurs appartenances et les deux solides après réimport.

Un essai supplémentaire incline la pièce avant son maillage binaire. Il se reproduit en maillant le fichier `f_ratio_mid-reference.step` avec les mêmes déflexions (0,025 mm et 0,20 rad). Aux réglages par défaut, R-090 et R-091 signalent tous deux `planar.invalid_contour` pendant le regroupement des facettes planes. Avec `--plane 0.00003 --fit 0.00003`, Auto obtient neuf faces et une borne de 0,021427 mm. Le volume FreeCAD s’écarte de la référence de 0,01468 mm³, au-delà du critère de comparaison de ce corpus. Cet essai reste ouvert dans R-036. Il est distinct des sept STEP du tableau.

Reproduction depuis la racine du dépôt sur macOS avec FreeCAD installé :

```sh
cargo build --locked --offline -p fabstep-cli
PYTHONPATH=/Applications/FreeCAD.app/Contents/Resources/lib \
  /Applications/FreeCAD.app/Contents/Resources/bin/python \
  scripts/validate-cross-drill-ratios-freecad.py artifacts/r091/reference --mesh --cli target/debug/fabstep
sh scripts/qualify-cross-drill.sh
```

**Vérifications finales.** Le passage intégral sur les sources finales dans une cible isolée (`CARGO_PROFILE_TEST_OPT_LEVEL=2`) compte 634 tests Rust réussis et 16 ignorés, avec succès des doctests. Formatage et Clippy strict passent. Les suites ciblées sur le profil ordinaire et le profil `large-stl` couvrent le noyau, la CAO, la conversion et les documents à deux corps. Les 236 tests d’interface, le contrôle Svelte et la compilation passent. Le build macOS local contient le helper testé sur trois fichiers, dont le STL FreeCAD à 884 triangles et le témoin de repli au seuil. L’essai manuel dans Fusion figure dans A-094.

## V-1220 — R-092, bloc percé incliné avant maillage en STL binaire

Le bloc de 24 × 20 × 18 mm de V-1219 est percé à Ø6 et Ø10 mm. Sur le build R-091, ses deux maillages inclinés échouaient en Auto avec `planar.invalid_contour`. En élargissant manuellement les tolérances, le maillage de 884 triangles donnait neuf faces, mais une erreur de volume de 0,01468 mm³. R-092 produit les neuf faces aux réglages par défaut. Le script de qualification dessine le solide dans FreeCAD, le place avant maillage et vérifie les STEP exportés par la CLI dans leur repère d’origine.

| Pose du bloc | Triangles | Borne de fidélité maximale en mm¹ | Écart absolu de volume FreeCAD en mm³¹ |
| --- | ---: | ---: | ---: |
| Pose de V-1219 | 884 | 0,021399 | 0,000089 |
| Pose de V-1219 | 1 766 | 0,003349 | 0,000094 |
| Rotation de 24° autour de (1, 2, 3) | 884 | 0,021398 | 0,000028 |
| Même rotation de 24° | 1 766 | 0,003348 | 0,000053 |

¹ Arrondi par excès. Le volume indépendant attendu est de 6 515,476725493 mm³. La borne de fidélité mesure la distance entre le maillage source et le modèle reconstruit. L’écart de volume compare la relecture FreeCAD à l’intégrale des sections.

Les onze STEP du corpus (quatre maillages inclinés et sept témoins de V-1219) contiennent chacun un solide fermé à six plans et trois faces cylindriques. Les deux courbes communes sont fermées, les axes, les rayons et les dimensions sont contrôlés. Les 132 sondes de matière et de vide passent. Seize permutations de l’ordre des facettes des quatre sources inclinées donnent également les deux perçages dans le contrôle du noyau.

**Protection du mode Plans.** Une plaque percée portant un pli de 0,0000016 mm constitue le contre-exemple. Son regroupement initial en deux pans donne un solide de onze faces et de 272,00011904 mm³. Une tentative systématique de fusion avait rendu son contour inconstructible en CAO. Le test d’intégration sur `fixtures/precision_perforated_crease.stl` exige désormais le résultat historique. Deux trous qui se touchent, un sommet intérieur hors plan, une couture déformée et un budget trop serré sont couverts par les tests du noyau.

**Reproduction sur macOS.** Le fichier de 884 triangles à ouvrir dans FabSTEP est `fixtures/cross_drill/freecad_mid_rotated.stl`. Les deux poses, leurs deux résolutions et la relecture se reproduisent depuis la racine du dépôt.

```sh
cargo build --locked --offline -p fabstep-cli
PYTHONPATH=/Applications/FreeCAD.app/Contents/Resources/lib \
  /Applications/FreeCAD.app/Contents/Resources/bin/python \
  scripts/validate-cross-drill-ratios-freecad.py artifacts/r092/reference --mesh --cli target/debug/fabstep
cmp fixtures/cross_drill/freecad_mid_rotated.stl artifacts/r092/reference/freecad_rotated_coarse.stl
```

**Autres pièces.** Sur une plaque de 24 × 20 × 6 mm portant un seul trou de Ø6 mm, maillée dans la pose de V-1219, le refus initial de contour disparaît. Auto réimporte un solide de 252 faces planes. Le mode explicite Cylindres donne un solide à sept faces, dont le cylindre du trou. Cette différence de choix entre les modes est suivie dans R-093. Le helper embarqué a également rejoué Flansch (540 170 triangles). Il retrouve 231 608 faces et retourne `step.invalid_source_provenance` au plafond existant de 100 000 faces, sans produire de STEP.

**Tests de développement.** Le passage complet de `cargo test --workspace --offline --locked` en profil de test optimisé compte 638 réussites et 16 tests optionnels ignorés. Les doctests, le formatage, Clippy strict et les 236 tests d’interface passent. Les tests ciblés de segmentation, perçages et précision passent aussi en profil ordinaire. Le helper compilé avec `large-stl` passe ses tests et le STEP du témoin incliné produit par le helper de FabSTEP.app passe la même relecture FreeCAD. Le build du bureau et le test de dialogue avec son helper passent. L’essai interactif Fusion reste ouvert dans A-095.

Sur le boîtier arrondi de 28 700 triangles, six passages alternés de la commande `check --mode planar` donnent des médianes de temps CPU de 2,99 s avant et 2,89 s après, sous charge concurrente. Le nombre de faces reste de 27 989. Pour le bloc droit à 884 triangles, les médianes sont de 0,093 et 0,107 s. Ces mesures caractérisent le coût local du contrôle.

## V-1221 — R-093, perçage traversant d’une plaque inclinée en Auto

La référence FreeCAD est un bloc de 24 × 20 × 6 mm percé perpendiculairement à ses grandes faces. Le trou mesure Ø6 mm et le volume théorique vaut 2 710,353997 mm³. Chaque pose est appliquée **avant** l’export STL binaire. Pour chaque STEP Auto, FreeCAD 1.1.1 vérifie un solide valide et fermé, ses dimensions dans le repère du dessin, l’axe et le rayon du trou, les deux contours circulaires partagés avec les faces planes et 15 positions dans la matière ou le vide.

| Pose et triangles source | Faces planes avant R-093 | Faces réimportées en Auto | Volume relu (mm³) | Borne locale annoncée (mm, arrondi supérieur) |
|---|---:|---|---:|---:|
| Pose de V-1220, 268 | 252 | 6 plans et 1 cylindre | 2 710,354031 | 0,003732 |
| Pose de V-1220, 520 | 492 | 6 plans et 1 cylindre | 2 710,354025 | 0,000935 |
| Seconde pose, 268 | 238 | 6 plans et 1 cylindre | 2 710,354005 | 0,003731 |
| Seconde pose, 520 | 458 | 6 plans et 1 cylindre | 2 710,354005 | 0,000935 |

Les 60 sondes passent. Sur la première pose, un budget d’écart de 0,001 mm conserve les 252 plans du maillage grossier et le cylindre du maillage fin. Pour le témoin de 268 triangles, le mode explicite Cylindres garde ses sept faces. Le test applicatif vérifie la provenance de tous les triangles, retrouve les 126 triangles attendus en sélectionnant le cylindre et rejoue la correction guidée sur cette face. Le STEP produit par le helper de l’application passe aussi la relecture de la première pose dans FreeCAD. Le rapport du helper et celui de la CLI comptent chacun 134 sommets ajustés sur ce témoin.

**Autre profil extérieur.** Un prisme de 6 mm d’épaisseur a pour coins (−12, −8), (11, −8), (9, 10) et (−8, 10) mm. Il porte un trou de Ø4,5 mm centré en (2, 1) mm. Après rotation de 37° autour de la direction (2, 1, 3) et maillage en 332 triangles, Auto passe de 321 faces planes à six plans et un cylindre. FreeCAD relit un solide fermé, retrouve le centre du trou et mesure 2 064,574192 mm³ pour 2 064,574123 mm³ dans le dessin.

**Contre-exemples.** Si les deux ouvertures circulaires sont conservées mais que les génératrices de la paroi sont vrillées de huit pas de maillage, les triangles restent facettisés. Quatre permutations de l’ordre des facettes et un trou excentré de 1,5 mm donnent chacun sept faces. L’ellipse, l’heptagone, la paroi conique et les ouvertures décalées entre les deux faces conservent leur résultat plan antérieur. Une paroi à laquelle il manque un triangle est refusée avec `mesh.open`.

**Pièces réelles.** Les conversions Auto avant et après de `body 26mm.stl` donnent un solide de 10 869 faces, dont 14 cylindres et 7 cônes. Le volume réimporté de 17 053,652459 mm³, les dimensions et les deux mesures de fidélité sont identiques. Le test optionnel sur ce fichier retrouve les quatre raccords cylindre–chanfrein et leur correspondance source, puis rejoue un essai guidé. Le helper de FabSTEP.app retrouve 231 608 faces sur Flansch (540 170 triangles), puis retourne `step.invalid_source_provenance` au plafond de 100 000 faces. Le dossier de calcul contient le refus et aucun STEP.

**Temps du noyau.** Pour six appels alternés par version à `check --mode auto` en build de développement, le temps CPU médian sur la plaque passe de 0,068 à 0,061 s. Sur le contre-exemple elliptique, la médiane se situe à 0,066 s dans les deux versions. Ces mesures portent sur les fichiers de 268 triangles.

**Reproduction sur macOS.** Le fichier à ouvrir dans l’application est [inclined_plate_hole.stl](../fixtures/inclined_plate_hole.stl). Le script recrée les deux poses et leurs maillages, lance les conversions, relit les STEP et imprime les mesures.

```sh
cargo build --offline --locked -p fabstep-cli
PYTHONPATH=/Applications/FreeCAD.app/Contents/Resources/lib \
  /Applications/FreeCAD.app/Contents/Resources/bin/python \
  scripts/qualify-inclined-plate-hole-freecad.py artifacts/r093/reference --cli target/debug/fabstep
cmp fixtures/inclined_plate_hole.stl artifacts/r093/reference/main-coarse.stl
```

**Validation du jalon.** La suite Rust du workspace en profil de test optimisé compte 641 tests réussis et 16 tests optionnels ignorés. Le test optionnel de la correction guidée sur `body 26mm.stl` passe aussi en exécution séparée. Le bureau macOS passe ses 51 tests, dont le lancement du helper embarqué. Les huit tests du helper, les 236 tests de l’interface, le formatage, Clippy strict, le contrôle Svelte et le build passent. Le contrôle interactif de la sélection et la mesure dans Fusion sont suivis dans [A-096](../roadmap/actions/A-096-verifier-le-trou-de-la-plaque-inclinee-dans.md).

## V-1222 — R-094, congé à rayon variable sur une bande entre deux arrêts

Sur `fixtures/local_blends/g_ruled_strip.stl`, R-093 renvoyait un solide à 176 faces planes. Le noyau peut à présent prendre deux témoins intérieurs sur une génératrice reliant les deux arrêts. Il vérifie sur cette arête source la position à un tiers et à deux tiers de sa hauteur, son faible écart angulaire, la distance au cône et la séparation du modèle à sections circulaires. Les contrôles de tangence, d’orientation, de couverture et de déviation sur les triangles entiers s’appliquent ensuite.

Les essais `cargo test -p fabstep-core -p fabstep-app --test linear_blend` couvrent deux nouvelles bandes de 256 et 288 triangles de paroi, en pente croissante puis décroissante avec permutation des axes. Auto et Surfaces retrouvent un solide, six plans et un cône, deux sections elliptiques et le volume de référence de 3 165,401595 mm³. Les tests contrôlent les propriétaires source, les sondes de matière, les faces réimportées et la fidélité dans les deux sens. Le témoin à sections circulaires conserve le refus analytique.

## V-1223 — Deux segments cubiques tangents sur un solide de révolution

Une méridienne cubique par morceaux est déterminée par deux groupes d’au moins sept anneaux chacun et un unique raccord source dont la tangente est commune. Les paramètres, la dérivée, les rayons positifs, la déviation par triangle et la couverture sont contrôlés. Les anciens modèles à un seul segment se relisent avec les paramètres initiaux.

Les témoins `two_span.stl` et `two_span_mirror.stl` de 6 272 et 5 904 triangles donnent deux plans et un mur de révolution. Les STEP contiennent une spline de degré trois avec un nœud intérieur de multiplicité deux. La relecture avec `scripts/validate-cubic-spans-freecad.py` mesure au plus 0,000000104 mm d’écart radial sur 492 positions. Les volumes obtenus en intégrant séparément les deux travées de la surface relue sont de 3 379,242890 et 3 379,242907 mm³, pour 3 379,242918 mm³ dans le dessin. La propriété générique Volume de FreeCAD 1.1.1 indique environ 3 378,74 mm³ sur les mêmes STEP. Le contrôle natif de FabSTEP utilise l’intégration adaptative sur les travées du STEP. Une retessellation FreeCAD de 3 832 triangles sans anneaux complets conserve son repli Auto à 3 610 plans.

## V-1224 — Perçages sécants à axes orthogonaux décentrés

Le modèle conserve la position du petit axe dans le repère du grand. Les anciens rapports sans décalage se relisent avec des axes centrés. Les deux courbes et leurs représentations paramétriques sont encadrées après réimport. Le contrôle de transversalité dépend des rayons et de l’excentration mesurés, avec une marge pour le déplacement des deux axes. Le volume de recouvrement est vérifié indépendamment par intégration des sections.

Les STL FreeCAD de 886 et 1 736 triangles donnent neuf faces, dont six plans et trois cylindres. Le volume de référence vaut 5 301,123749 mm³. L’écart au volume intégré est inférieur à 0,000004 mm³ sur ces deux modèles. La lecture indépendante contrôle 12 points dans la matière ou le vide par pièce. Dans une seconde pose inclinée, le maillage grossier produit neuf faces. Une tessellation fine de cette pose conserve 1 723 plans car ses quatre bouches ne sont pas toutes identifiées par la segmentation initiale. La garde de séparation rejette le témoin trop proche de la tangence.

## V-1225 — Congé de poche entre deux butées inclinées

Les trois dessins FreeCAD de `fixtures/double_oblique_pocket` donnent 108, 132 et 142 triangles. Sur ces essais, le nombre de faces passe de 81, 91 et 128 plans à onze plans et un cylindre. Le lecteur indépendant contrôle les deux arêtes elliptiques et huit points de matière ou de vide par pièce. Les tests couvrent les deux exports DXF, la provenance source et le budget de déviation. Une rotation de 37° autour de (1, 2, 3) conserve le repli à 142 plans.

## V-1226 — Bac à deux perçages dans le fond et un perçage latéral

Le générateur indépendant `scripts/qualify-multibore-tray-freecad.py` dessine et contrôle le bac à deux résolutions (1 308 et 2 004 triangles). Les deux STEP produits par Auto contiennent onze plans et onze cylindres. Les six bouches, les huit congés et dix-huit sondes de matière ou de vide sont vérifiés après relecture. L’écart au volume dessiné de 5 767,098637 mm³ est inférieur à 0,000025 mm³. Ce lot étend le corpus de validation du moteur existant.

## V-1227 — Révolution cubique à pointe axiale

Les vingt tests core, CAO, app et document des suites `smooth_revolution` et `smooth_revolution_document` passent. L’ancien modèle à 3 281 plans devient une révolution et un disque. La lecture indépendante avec FreeCAD retrouve les deux solides, leurs pointes et les volumes de 1 005,309634 et 522,401965 mm³. Sur 492 positions de profil, l’écart radial maximal reste inférieur à 0,0000001 mm. Un déplacement du sommet de 0,02 mm, une interruption de l’éventail ou un budget insuffisant empêchent l’acceptation de la pointe.

## V-1228 — Relief bicubique à rive horizontale, du STL au STEP

À partir de dessins FreeCAD d’une bosse et d’une selle, le CLI de production reconstruit quatre STL de 5 138 à 16 526 triangles. La relecture indépendante de chaque STEP vérifie un solide fermé à cinq plans et une surface bicubique, les volumes de 2 090 et 1 920 mm³ et huit points de matière ou de vide. Les bornes continues du moteur vont de 0,001016 à 0,001780 mm. La distance échantillonnée à l’aperçu tessellisé est une mesure distincte.

Reproduire avec `scripts/qualify-height-patch-freecad.py <sortie> --cli <chemin-vers-fabstep>` sous Python FreeCAD. Les suites Rust `height_patch` couvrent la provenance complète, deux corps séparés, les budgets et les entrées hors domaine. Le réglage est explicite dans l’interface avancée et le CLI (`--mode height-patch`). L’essai dans Fusion est suivi dans FabBoard.

## V-1229 — Rebord en saillie et deux cavités dans un bac

Le dessin indépendant possède un rebord de 2 mm, deux fonds à 2 et 4 mm et un perçage traversant le fond gauche. Les deux maillages de 1 256 et 2 052 triangles donnent vingt et un plans et dix-sept cylindres. FreeCAD retrouve les épaisseurs, la matière, le vide et les arêtes des dix-sept cylindres. Les volumes de 13 755,66055 et 13 755,66056 mm³ sont cohérents avec les 13 755,66058 mm³ du dessin. La suite `rimmed_tray` vérifie la provenance et le refus d’un maillage troué. Reproduire avec `scripts/qualify-rimmed-tray-freecad.py`.

## V-1230 — Indication sphérique depuis une face STEP

La suite `guided_sphere` part des faces du STEP réimporté. Elle vérifie une demi-sphère en mode Plans, une zone à deux bords et une sphère complète en Auto, l’égalité des propriétaires source avec la région reconstruite, le recalcul et le retrait. La calotte et une face du disque sélectionnées ensemble sont refusées, de même qu’une paroi cylindrique. Le raccordement à un voisin courbe reste hors du périmètre du bouton Sphère.

## V-1231 — Deux congés adjacents dans un angle aigu ou obtus

Les quatre STL FreeCAD à 65° et 115° (deux densités chacun) donnent onze plans et deux cylindres. La relecture indépendante retrouve un seul arc elliptique partagé par les deux cylindres, de demi-grand axe 4,65289744 ou 2,96422263 mm, et contrôle ses deux p-courbes. L’écart au volume du dessin reste inférieur à 0,00017 mm³. Le témoin à 40° conserve ses facettes. Un sommet déplacé ou une tolérance insuffisante empêchent l’acceptation analytique. Le test `angled_pocket` contrôle les propriétaires de triangles et la fidélité. Le [corpus](../fixtures/angled_pocket/README.md) contient les commandes de reproduction.

## V-1232 — Deux congés de lois affines sur les arêtes opposées d’un bloc

Deux pièces qui produisaient 476 et 774 plans donnent six plans et deux cônes dans Auto. FreeCAD retrouve dix-huit arêtes, dont quatre elliptiques. Deux dessins indépendants remesillés à 354/1 384 et 384/1 430 triangles donnent également huit faces. La plus grande borne est de 0,019501 mm. Les volumes reconstruits de 3 142,5011774 et 3 156,6370021 mm³ concordent avec le dessin à moins de 0,000002 mm³. Trente-six sondes matière/vide, les deux budgets de raccord et la partition source sont contrôlés. Une loi non linéaire sur le second coin ou un pli empêche l’acceptation de la paire.

## V-1233 — Rebord torique intérieur et extérieur sur un récipient ouvert

Les quatre STL FreeCAD de 10 922 à 30 018 triangles donnent trois plans, deux cylindres et deux tores. Avant l’extension, le petit récipient conservait 4 494 ou 7 123 faces selon la densité. FreeCAD relit les volumes à moins de 0,001 mm³ des dessins de 1 978,143279 et 3 512,827228 mm³. Les sondes vérifient la matière des deux côtés du rebord et le vide au-dessus du fond. La suite `rounded_lip` couvre la pièce retournée, la provenance des deux parois toriques et une méridienne endommagée. Commandes dans [le corpus](../fixtures/rounded_lip/README.md).

## V-1234 — Révolution cubique sur un maillage irrégulier

Trois STL issus de deux dessins FreeCAD produisent deux plans et une révolution. Deux de ces pièces produisaient auparavant 4 276 et 3 202 plans. FreeCAD contrôle les STEP réimportés à 495 positions, avec un écart radial maximal de 0,000000252 mm au profil dessiné. Le quatrième STL s’écarte réellement du profil de 0,08637 mm et conserve le refus analytique au budget de 0,05 mm. Les commandes, profils et densités figurent dans [le corpus](../fixtures/irregular_revolution/README.md).

## V-1235 — Deux axes de perçage obliques entre eux

Les maillages indépendants de 1 088, 1 562, 1 068 et 1 556 triangles à 60° et 75° produisent six plans et trois faces cylindriques. Le réimport et la lecture FreeCAD contrôlent quatre bouches et deux courbes d’intersection. Les p-courbes et les courbes 3D sont certifiées à 0,0000001 mm. Les volumes analytiques valent 23 856,17586745 et 21 089,93782459 mm³. La borne continue source atteint au plus 0,010555 mm. Le test à trois bouches conserve la matière, le document mixte conserve ses deux corps et les angles falsifiés sont refusés. Les commandes se trouvent dans [le corpus](../fixtures/oblique_cross_drill/README.md).

## V-1236 — Révolution à deux cubiques tangentes sur un maillage irrégulier

Quatre maillages indépendants de 4 442 à 7 916 triangles donnent deux plans et une seule face de révolution après réimport. La position du raccord est estimée entre les hauteurs mesurées. Le test contrôle la continuité des tangentes, la partition source et une borne maximale de 0,049746 mm pour un budget de 0,05 mm. Une rupture de tangente conserve le refus. Les commandes et les références FreeCAD figurent dans [le corpus](../fixtures/irregular_revolution/README.md).

## V-1237 — Fermeture et attribution des zones résiduelles de la bride

Le laboratoire raccorde les facettes restantes et remplace 68 426 triangles par 19 domaines toriques interrompus. Le fichier de 22 622 983 octets se réimporte avec un solide fermé de 13 802 faces et un volume de 54 334,693960 mm³ (source : 54 336,072562 mm³). Le contrôle du graphe retrouve les propriétaires des 540 170 triangles. OCCT produit 145 370 triangles à 0,01 mm et 215 486 à 0,005 mm. Le plafond applicatif vaut 100 000. Le candidat de laboratoire ne constitue donc pas encore un résultat FabSTEP exportable.

Depuis l’entrée de V-1210, lancer `PYTHON=/Applications/FreeCAD.app/Contents/Resources/bin/python sh scripts/qualify-flange-residual.sh --checkpoint CHEMIN_V1210 SORTIE`. Sur un autre poste, le mode `--public SORTIE` génère indépendamment une rampe torique interrompue de 8 232 triangles, reconstruite en 4 363 faces. Lancer ensuite `python3 scripts/qualify-flange-residual-controls.py SORTIE`. Les quatre variantes doivent refuser un propriétaire dupliqué, un rayon déplacé, un enroulement inversé et une borne de déviation sous-estimée.

## V-1238 — Trois congés cylindriques connectés au fond d’une poche

Sur six maillages issus de trois dessins, le STEP relu contient onze plans et trois cylindres. Deux arcs elliptiques possèdent chacun deux courbes paramétriques sur leurs faces incidentes. Les volumes concordent à 0,001 mm³ avec les références de 17 894,797218, 17 428,748259 et 20 844,678295 mm³. Les tests couvrent l’attribution des triangles, un document à deux corps et un déplacement de 0,06 mm sur une jonction. Une encoche empêche la reconnaissance du U entier. Commandes dans [le corpus des poches en U](../fixtures/u_pocket/README.md).

## V-1239 — Maillages à bord imposé d’un tore et d’un éventail rationnel

Le maillage torique vérifie la trace isoparamétrique, l’incidence orientée des arêtes et une borne continue de l’erreur d’interpolation sur chaque triangle. Le maillage des triangles rationnels contrôle le pôle, le domaine, le cône des normales et les nœuds communs avec les voisins. Les deux consomment des points d’arête en coordonnées monde. Les exécutables de qualification sont `qualify-trimmed-torus-mesh.cpp` et `qualify-flange-fan-mesh.cpp` dans `scripts/`, compilés avec le SDK OCCT utilisé par FabSTEP (TKMesh, TKPrim et TKDESTEP). Le témoin torique s’exécute avec `--self-test .005`, celui de l’éventail avec `--self-test`. Les tests négatifs couvrent le budget de triangles, une chaîne décalée, le désordre des paramètres, une déflexion trop fine, un sous-domaine et un faux pôle.

Sur le STEP de V-1237 relu, les 34 tores passent de 134 157 à 50 237 triangles, à déflexion de 0,005 mm et angle de 0,35 rad. La borne certifiée maximale atteint 0,004823 mm. La rampe publique passe de 6 920 à 1 868 triangles pour sa face torique. Les 5 854 éventails passent de 51 764 à 13 424 triangles en conservant les bords OCCT du témoin privé. Ces comparaisons par famille précèdent le contrôle du maillage du solide entier.

## V-1240 — Perçages à axes gauches

Quatre STL indépendants de 1 102, 1 554, 1 112 et 1 588 triangles produisent six plans et trois faces cylindriques. Les volumes de référence à 60° et 75° sont 23 854,860545 et 21 087,665160 mm³. Le STEP relu se trouve à moins de 0,00012 mm³ de ces volumes. Le test attribue chaque triangle à sa paroi physique et vérifie les deux sens de fidélité. Sur le quatrième STL, le regroupement à la tolérance de cylindre pinçait un contour. La segmentation reprend à la tolérance de plan plus stricte pour ensuite refaire toutes les preuves. Le dessin à trois bouches conserve sa matière. Rejeu dans [le corpus](../fixtures/oblique_cross_drill/README.md).

## V-1241 — Congés variables portés par deux arêtes gauches

Les quatre maillages des deux blocs produisent six plans, deux cônes et quatre ellipses. Les volumes concordent à 0,001 mm³ avec les dessins de 3 151,607063 et 3 152,958225 mm³. FreeCAD contrôle les normales des appuis, les rayons aux extrémités, les p-courbes et 24 sondes matière/vide. Une encoche ou une déformation de la seconde bande empêche la reconnaissance de la paire. Les maillages et commandes figurent dans [le corpus des arêtes gauches](../fixtures/skew_linear_blends/README.md).

## V-1242 — Maillage fermé du STEP de laboratoire de la bride

Le routage réserve un seul jeu de points à chaque arête. Les voisins ordinaires passent par OCCT après contrôle de leur complexité, tandis que les tores découpés et les éventails admissibles reprennent les points des mêmes bords. L’essai `sh scripts/qualify-trim-edge-cache.sh` couvre deux placements, un arc rationnel et les deux représentations UV d’une couture. Une paroi retournée, une extrémité manquante et un budget insuffisant sont refusés. `sh scripts/qualify-trim-face-mesh.sh --witness` reconstruit et réimporte un solide public à sept faces.

Sur le STEP privé de V-1237, `sh scripts/qualify-trim-face-mesh.sh CHEMIN/candidate.step` couvre les 13 802 faces en 82 602 triangles à 0,005 mm. Après soudure de contrôle, 41 293 sommets et 123 903 arêtes sont présents. Chaque arête a deux utilisations opposées. Le volume signé du maillage vaut 54 342,466472 mm³ et celui du solide STEP relu vaut 54 334,693960 mm³. Le parcours depuis le STL doit encore établir la fidélité et la provenance de son propre résultat.

## V-1243 — Quatre congés en circuit autour d’un fond réel

Trois poches à deux densités et une septième triangulation construite par sections alternées donnent onze plans, quatre cylindres, quatre ellipses et huit p-courbes. FreeCAD contrôle les volumes à 0,001 mm³ et les aires des trois fonds (221, 293,25 et 26 mm²). Une ouverture à la place du fond conserve les quatre jonctions mais reste traversante après les deux exports. Une encoche, un nœud déplacé et trois descriptions CAO falsifiées empêchent l’acceptation du circuit. Commandes et sources dans [le corpus des circuits fermés](../fixtures/closed_pocket/README.md).

## V-1244 — Domaines toriques découverts directement dans le STL

Le moteur pur retrouve une même rampe torique interrompue à deux densités, y compris après changement de repère. Les cas à facette absente ou dupliquée ne fournissent aucun domaine. Sur le STL privé passé par l’importeur ordinaire du helper, 43 domaines couvrent 493 542 des 540 170 triangles. Avec une face par triangle résiduel, la construction prévue compte 46 671 faces. Ce décompte concerne la reconnaissance.

Rejeu public par `cargo test --offline -p fabstep-core --test torus_atlas`. Pour le STL privé, définir `FABSTEP_LARGE_STL` au chemin du fichier puis lancer `cargo test --offline -p fabstep-core --features large-stl --test torus_atlas -- --include-ignored --nocapture`. Les trois tests passent sur le Mac de développement.

## V-1245 — Audit des distances du maillage complet

L’audit examine toujours chaque sommet, milieu d’arête distinct bit à bit et barycentre. Un triangle déjà rencontré permet de terminer une requête dès que sa distance effective à l’échantillon est inférieure au maximum déjà démontré. Les requêtes susceptibles d’augmenter ce maximum poursuivent la recherche dans l’arbre. Le plafond de 50 millions d’opérations est conservé et la réponse reste le même maximum d’échantillonnage.

Sur le STL privé et le maillage de V-1242 (82 602 triangles), l’ancienne recherche refusait après 1,52 s avec `validation.distance_budget`. La nouvelle termine en 1,00 s en profil release. Elle rend 0,011552224 mm de la source vers le résultat et 0,009855085 mm en retour, sur 2 430 757 et 419 948 échantillons comptabilisés. Le test `witnessed_upper_bounds_preserve_the_exhaustive_maximum` compare les bits du résultat au calcul exhaustif. Les tests des petits triangles, des milieux d’arête arrondis différemment et de l’épuisement du budget passent également.

## V-1246 — Validité de la source dans le budget mémoire du helper

La construction d’une BRep par triangle pour contrôler le STL privé a atteint 7 538 334 848 octets d’empreinte physique et a été interrompue. Après la validation de la coque par le core, la preuve spatiale bornée examine 24 466 574 paires de boîtes et 4 812 829 paires de triangles en 0,4 à 0,57 s. L’empreinte maximale mesurée sur l’analyse et le certificat vaut 344 998 608 octets. Les deux méthodes concordent sur trois STL publics, un octaèdre sain et un octaèdre auto-intersectant.

Dix-sept tests réussissent avec `FABSTEP_LARGE_SOURCE_STL=chemin cargo test --offline -p fabstep-verify --features large-stl -- --include-ignored --nocapture`. Ils couvrent les intersections coplanaires et transverses, le contact entre triangles non incidents, les voisinages à un sommet ou une arête et l’épuisement du budget. Contrat dans [ADR-0045](adr/0045-large-source-embedding.md).

## V-1247 — Construction CAO des domaines toriques et de leurs voisins

Le constructeur prend des domaines déduits du maillage soudé et une partition exhaustive. Les triangles voisins reprennent les mêmes arêtes circulaires. Chaque face doit conserver son identifiant à la couture et au transfert dans le STEP. Le réimport vérifie la géométrie, les raccords et le volume. L’écriture garde ses p-courbes et retire uniquement des blancs syntaxiques aux débuts d’entités avant le comptage des octets.

La rampe publique indépendante à 8 232 triangles se réimporte avec 4 363 faces, un volume de 93,454200 mm³, un fichier de 10 211 998 octets et une borne continue de 0,013721 mm. Les trois tests natifs de `cargo test --offline -p fabstep-cad --test torus_atlas` contrôlent aussi les jetons sur deux corps séparés et les refus de partitions ou contours falsifiés. Les quatre contrôles négatifs de V-1237 passent encore avec le code partagé par le laboratoire et le constructeur.

## V-1248 — Révolutions à retour axial

Quatre STL de FreeCAD couvrent deux orientations et deux résolutions. Chaque STEP relu présente deux disques et une révolution cubique. Les retours axiaux mesurent 2,008 et 1,431 mm. La comparaison au support de référence porte sur 980 points et 120 sondes de matière, avec un écart maximal de 0,0000000914 mm. Un essai CLI à 0,05 mm donne une borne continue de 0,048982 mm et des écarts échantillonnés de 0,010956 et 0,010199 mm. Les tests de refus couvrent un repli dégénéré, un point aberrant, les disques manquants et le budget insuffisant. Sources et commande de reproduction dans `fixtures/returning_meridian/README.md`.

## V-1249 — Tessellation bornée des secteurs intérieurs de tore

La borne de courbure dépend de l’intervalle angulaire réellement couvert par chaque triangle. Douze secteurs analytiques, dont une couture et la paroi intérieure d’un tore à petit rayon intérieur, passent le contrôle indépendant. Sur le même ensemble de secteurs à 0,005 mm, la tessellation passe de 134 157 à 42 339 triangles. La plus grande borne obtenue est de 0,004987 mm. Le décompte expérimental de 99 494 triangles pour le modèle privé entier précède sa certification finale.

## V-1250 — Conversion Auto d’une rampe torique dense dans le helper

Le témoin public contient 194 694 triangles. Avec les options ordinaires du helper, le STEP relu contient 659 faces, un solide de 93,536793 mm³ et une face torique. Le fichier pèse 1 784 117 octets. Les écarts source vers résultat et retour valent 0,004717 et 0,004703 mm, avec une tessellation à 0,005 mm. Le rejeu optimisé a duré 28,46 s et atteint une empreinte physique de 1 038 960 248 octets. Le contrôle du résultat, du volume et des deux distances est reproductible avec `python scripts/qualify-large-torus-worker.py chemin/vers/fabstep-worker` (NumPy pour générer le témoin).

## V-1251 — Budget mémoire de la recherche d’interférences CAO

Sur la rampe de 4 363 faces, le même contrôle OCCT a mesuré 1 114 734 592 octets de RSS et 19,17 s en une passe, puis 214 482 944 octets et 41,51 s par groupes spatiaux. Les boîtes conservatrices englobent la tolérance des sous-formes. Toute paire de faces susceptible de se rencontrer entre dans un contrôle OCCT avec leurs identités topologiques d’origine. Les plafonds de comparaisons et de groupes provoquent un refus explicite. Le banc `scripts/qualify-atlas-interference.sh` compare les décisions sur des voisinages valides et des intersections internes ou entre groupes.

## V-1252 — Reparamétrisation des bords circulaires dans le STEP

Une arête sur tore était refusée par le contrôle exact d’OCCT après transfert alors que ses deux extrémités étaient conservées. Le constructeur vérifie les pôles et poids de l’arc rationnel puis aligne la paramétrisation de sa courbe 2D avec celle de la courbe 3D. Le lieu géométrique, les extrémités et la tolérance des arêtes sont conservés. Le banc C++ couvre deux directions, l’orientation inverse et les refus. Sur le modèle privé direct, 29 courbes 2D sur 2 166 504 entités ont été alignées. Le STEP obtenu pèse 130 701 976 octets ; le contrôle exact du solide relu passe. Les interférences et le budget du helper font l’objet d’un contrôle distinct.

## V-1253 — Audit complet du maillage réimporté de 98 576 triangles

L’audit de distance spatialement ordonné conserve les positions de tous les sommets, milieux et barycentres. Il réordonne des blocs de 16 384 points, avec une mémoire temporaire d’environ 512 Kio. Le plafond demeure de 50 millions d’opérations. Sur le maillage de laboratoire à bords toriques reparamétrés, le parcours initial épuisait ce plafond en 2,44 s ; un comptage de référence complet demandait 64 691 835 opérations. Le parcours par blocs aboutit en 2,62 s avec les mêmes maxima que la référence, 0,0115522233945 mm et 0,0100176527580 mm. Les comptes sont de 2 430 757 et 569 719 échantillons. Un test contrôle l’identité bit à bit des échantillons entre les deux ordres, y compris aux frontières de blocs.

V-1252, complément des unités. Onze cas indépendants couvrent notamment un modèle en pouces, des angles en degrés, un plan et une direction oblique en UV. Seuls les arcs du modèle métrique en radians dont la représentation est reconnue sont éligibles.

**Rejouer la recherche source de V-1244.** Avec le Python NumPy de FreeCAD, exécuter `python scripts/qualify-flange-stock-domains.py chemin/vers/FlanschTurboLuftfilter_REV02.STL artifacts/torus-source`. Le laboratoire indépendant retrouve 26 hypothèses et 43 domaines couvrant 493 542 des 540 170 triangles. Le répertoire produit contient le repère mesuré, les boucles et l’affectation des triangles. Le témoin est exprimé dans ce repère. Pour reconstruire et valider un STEP dans le repère du fichier, utiliser le helper de l’application.

## V-1254 — Réserve de soudage pour la source brute du helper

Une occurrence de sommet de la rampe dense a été décalée de 0,000019997 mm avant écriture du STL. Avec un soudage de 0,000050000 mm, le helper retourne un solide de 659 faces et 93,536793075 mm³. La borne continue enregistrée inclut le soudage et atteint 0,013798369 mm. Le détecteur et la construction native se partagent le budget restant après soudage. Deux tests ciblés contrôlent également le seuil de refus, les identités source et les doubles attributions.

**Localiser le coût des interférences.** `sh scripts/qualify-atlas-interference.sh artifacts/atlas-interference --ruled-faces fichier.step` examine chaque face de raccord réglée avec le test d’auto-intersection individuel d’OCCT. Les 246 raccords de la rampe publique et les 14 882 du STEP expérimental de Flansch passent ce test. Le contrôle des seules faces de Flansch, lecture comprise, dure 174 s. Le contrôle obligatoire des intersections entre faces distinctes se poursuit dans le helper.

## V-1255 — Helper optimisé dans le paquet macOS de développement

Sur le même STL public de 194 694 triangles et la même version du noyau, le binaire debug du helper termine en 163,71 s et le binaire release en 85,37 s. Le calcul entre en construction après 74,67 s en debug et 4,20 s en release. Les deux rapports contiennent les mêmes 659 faces, le même volume et les mêmes distances. Le maximum du RSS et de l’empreinte physique échantillonnés vaut respectivement 457 752 576 et 460 144 640 octets. Le build du bureau conserve son profil de développement et embarque le helper release adjacent. Le worker reste compilé séparément et le superviseur applique les mêmes plafonds.

### Pistes mesurées pour les interférences du gros atlas

Le découpage spatial de Flansch donne 6 949 groupes à contrôler. Les bancs suivants ont servi à choisir la suite du travail.

| Variante | Mesure sur le même cas | Décision |
| --- | --- | --- |
| Grouper les triangles coplanaires indépendants des contours toriques | 46 671 → 38 763 faces, baisse de 2,8 % de l’empreinte à la relecture, durée de 48,55 → 57,23 s | Conserver comme piste de réduction de taille |
| Exclure les paires de triangles séparées par un axe avec marge | 351 131 paires exclues, 6 750 groupes restants. Sur les mêmes 32 groupes, 92,27 → 90,20 s | Gain trop faible pour ajouter un second contrôle en production |
| Couvrir les portions toriques dans le repère UV | 46 125 → 29 804 paires tore/autre, 433 groupes modifiés sur 6 949. Sur neuf groupes concernés, 67,83 → 47,13 s CPU, dont 16,62 s gagnées dans un seul groupe | Poursuivre uniquement avec une mesure du contrôle entier |

La priorité pour Flansch est de réduire le coût du contrôle exact des intersections arête/face et face/face sur les raccords. La reconnaissance des supports, l’écriture, la relecture géométrique, le maillage et la distance ont chacun un témoin reproductible.

**Contre-épreuve en mode ordinaire.** Le fichier privé `body 26mm.stl`, converti en mode Auto avec le CLI optimisé et l’unité millimètre, donne un solide relu de 10 869 faces, dont 14 cylindres et 7 cônes. Son volume vaut 17 053,652459 mm³. Les écarts échantillonnés source vers résultat et retour sont de 0,002619 et 0,003052 mm. Le fichier source et son STEP restent locaux.

## V-1256 — Rejeu de Flansch avec le superviseur du bureau et le helper du paquet

Le helper release livré dans `artifacts/FabSTEP.app/Contents/MacOS/fabstep-worker` (SHA-256 `8df654952277c02e2b5d16a2eb5b5d4b3b4cff7b4be27bdcfa9dd45e0be99447`) a traité le STL binaire original de 540 170 triangles, en Auto, millimètres et déviation maximale de 0,05 mm. Le banc appelle la fonction réelle de supervision du bureau avec les octets du STL. La surveillance mémoire, l’échéance de trente minutes et la récupération du processus sont actives.

Le calcul a atteint la construction. Après **1 380,137 s**, le parent a reçu **`cad.invalid_solid`** sur `stl/component/0` au contrôle d’auto-intersection du solide. Le test vérifie que le parent ne possède aucun résultat et que le processus fils est récupéré. Le contrôle exact des courbes, les écarts du témoin de laboratoire et le nombre de triangles admissible ne suffisent donc pas à autoriser cet export. Les essais graphiques et dans Fusion des nouveaux domaines restent proposés dans FabBoard.

Rejeu manuel : ouvrir le STL, choisir le millimètre et le mode Automatique, accepter l’essai de gros STL puis lancer la reconstruction. La rampe publique indépendante et son contrôle complet sont reproductibles avec le script de V-1250. Les limites appliquées sont celles de l’ADR-0034.

### Localisation du refus V-1256

Sur la coque avant transfert, les 739 premiers groupes spatiaux passent. Le groupe 740 signale deux faces. La face native 39 332 reproduit le défaut à elle seule. Les six triangles d’analyse concernés parmi 14 882 raccords réglés sont 397 534, 397 648, 397 670, 397 673, 397 901 et 397 904 (indices à partir de zéro).

Dans le témoin minimal, la ligne signalée par OCCT a une corde nulle et onze points confondus au sommet à environ 7 × 10⁻¹⁵ mm. Les six arcs balayent environ 1° et leurs sommets sont à environ 1,17 × 10⁻⁸ mm du plan de l’arc. La projection de chaque éventail conserve le signe de son déterminant jusqu’au sommet. En laboratoire, une surface rationnelle de degrés (2,1) réutilisant les arêtes d’origine passe le contrôle local et celui de la paire avec sa voisine torique sur les six faces et sur la rampe publique. Cette expérience locale porte sur les faces avant transfert.

## V-1257 — Raccord circulaire exact au sommet singulier

Le nouveau constructeur intervient lorsqu’un raccord à base circulaire a une projection injective et que le seul défaut individuel signalé se concentre au sommet. Il remplace son support par une surface rationnelle quadratique de même contour. Les arêtes et sommets restent partagés. La face candidate passe les contrôles géométriques et d’interférence, puis la coque complète suit la validation habituelle.

Sur Flansch, le constructeur change exactement six des 46 671 supports. Toutes les identités de faces sont conservées. Le groupe 740 et les 14 882 tests individuels passent après couture. Le volume calculé de la coque varie de moins de 0,000000002 mm³. Le témoin public à arc de 10 mm reproduit le défaut initial et vérifie le partage du bord et la fidélité du remplacement. La rampe indépendante à 8 232 triangles est convertie, relue et tessellée à 0,005 mm en 15 442 triangles avec une borne de 0,013721 mm.

## V-1258 — Verdict du paquet corrigé sur le STL original

Le même banc supervise le helper optimisé du paquet de développement (SHA-256 `82fa61333f5addd3b4ac978419e635b45b52b85dd82f8ecb91ccdff1e2305793`). En mode Auto, avec les paramètres de V-1256 et les 540 170 triangles de Flansch, le superviseur atteint son échéance à **1 800,039 s** et retourne **`worker.timeout`**. Le calcul se trouve en phase de construction. Aucune publication de résultat et aucun processus fils subsistant. Le contrôle complet de la pièce demeure ouvert.

Sur le même paquet, le test indépendant de la rampe de 194 694 triangles réussit avec 659 faces, un volume de 93,536793 mm³ et des distances de 0,004717 et 0,004703 mm. La suite Rust du workspace passe **707 tests**, aucun échec, 18 ignorés. Le bureau avec le vrai helper passe 51 tests, l’interface 238. Le formatage, Clippy, le build macOS, la compilation UI et le lint FabBoard passent. Les actions FabBoard conservent les vérifications manuelles dans FabSTEP et Fusion.

Pour reprendre, commencer par mesurer la répartition du coût entre interférences arête/face et face/face sur le banc natif. Comparer toute optimisation sur le contrôle complet de la rampe et sur les témoins négatifs de V-1251 avant le rejeu du helper sur Flansch. Les gains mesurés sur un sous-ensemble de groupes figurent après V-1255.

## V-1259 — Contre-expertise du contrôle d’interférence d’OCCT 7.9.3

Le banc public `sh scripts/qualify-atlas-interference.sh` compare la référence OCCT au parcours par groupes. Il vérifie le refus d’une face dont le fil est ouvert, le verdict sur l’éventail quasi plan, l’acceptation de son raccord réparé au tore dans une même feuille puis entre deux feuilles, ainsi que les quatre configurations des triangles séparés ou sécants parmi 329 faces.

Le test bi-forme de haut niveau examine les auto-interférences de chaque argument individuellement. Dans le chemin mono-forme, la routine de contrôle d’une face par elle-même réinitialise aussi la carte des paires brutes avant `PostTreat`. L’enveloppe publique vérifie le type d’argument et la validité BRep. Sauter simplement la routine individuelle omet des opérations nécessaires au verdict. [CheckerSI d’OCCT 7.9.3](https://github.com/Open-Cascade-SAS/OCCT/blob/V7_9_3/src/BOPAlgo/BOPAlgo_CheckerSI.cxx) · [ArgumentAnalyzer](https://github.com/Open-Cascade-SAS/OCCT/blob/V7_9_3/src/BOPAlgo/BOPAlgo_ArgumentAnalyzer.cxx) · [BRepAlgoAPI_Check](https://github.com/Open-Cascade-SAS/OCCT/blob/V7_9_3/src/BRepAlgoAPI/BRepAlgoAPI_Check.cxx)

| Expérience locale | Verdict |
|---|---|
| Deux triangles sécants dans deux arguments de BRepAlgoAPI_Check | Acceptation erronée. Le contrôle mono-forme de production refuse le témoin. |
| Sous-classe proposée, défaut de topologie et raccord légitime | Accepte la face ouverte et refuse le raccord valide entre deux feuilles. |
| Deux arguments avec purge et filtrage des paires | Faux refus au groupe 11 de la rampe publique sur une paire face/face d’origine. La présence des autres faces dans le groupe importe. La paire extraite et testée seule est valide. |
| Mono-forme adaptée, diagonales complètes, vérifications de type et BRep, repli sur avertissement | Rampe acceptée. Sur dix groupes de Flansch choisis à l’avance, 7,99 s CPU cumulées contre 7,67 s pour la référence. |
| API publique parallèle sur la rampe | Les 48 groupes passent. Un passage de 20,48 à 7,64 s murales, RSS de 173 031 424 à 633 044 992 octets. |
| Même API parallèle sur dix groupes du STEP de laboratoire Flansch | Dix verdicts identiques, dont cinq diagonales. 15,12 à 5,39 s murales cumulées, 7,67 à 10,51 s CPU. Le STEP comporte 46 671 faces. Les 6 949 groupes comprennent les diagonales. |

Le rejeu jusqu’au premier refus sur le STEP de laboratoire `direct7/generic.step` (46 671 faces, antérieur à la réparation des six raccords de V-1257) s’arrête à la diagonale 3 094 sur 6 949, après 447 818 paires comptées et 1 189,62 s. Le pic mesuré du maximum RSS/empreinte physique est de 3 699 200 176 octets. Sur cette même diagonale, l’API séquentielle de référence refuse en 1,35 s, la mono-forme adaptée en 1,24 s et l’API publique parallèle en 0,52 s. Le noyau signale six interférences arête/face, reproduites séparément. Les faces B-splines 28 457, 28 459, 28 461, 28 463, 28 465 et 28 467 (numérotation native à partir de 1) passent chacune seule. Les arêtes signalées appartiennent à la face 5 326 et respectivement à la face d’indice suivant chacune des six B-splines. Le groupe et une paire minimale sont conservés localement dans `artifacts/argon/occt/diagnose/`. Les groupes ultérieurs n’ont pas été examinés. Ce fichier de laboratoire et le solide en cours de construction à partir du STL sont deux états différents.

**Rejeu sur le STL original avec le helper expérimental.** Une copie isolée du worker a utilisé l’API publique `SetRunParallel(true)` sur chaque groupe. Le fichier de 540 170 triangles a été traité en Auto, en millimètres, avec un écart maximal de 0,05 mm. Les limites de trente minutes et de 4 Gio étaient actives. Le worker v2 a répondu `cad.invalid_solid` pendant la construction après 1 055,81 s, avec un maximum échantillonné de 3 351 089 400 octets (3,12 Gio). Il n’a publié ni `result.step` ni rapport de réussite. Le même prototype accepte la rampe publique avec le même rapport v2. Les deux mesures sur Flansch couvrent respectivement la relecture d’un ancien STEP de laboratoire et la construction actuelle depuis le STL. La réponse du worker n’indique aucun numéro de groupe. Le build utilisateur conserve le contrôle séquentiel.

**Correspondance sur la coque actuelle.** Une instrumentation locale indépendante a capturé la coque native de 46 671 faces et 92 334 arêtes avant transfert, puis a interrompu le calcul sans émettre de STEP. L’appariement géométrique des six paires avec le témoin précédent utilise les sommets et le point médian des arêtes. L’écart maximal est de 8,08 × 10⁻⁸ mm et le deuxième candidat est toujours à au moins 0,034 mm. Sur cette coque, le contrôle public **séquentiel** refuse séparément chacune des six paires. Ce sont les faces B-splines 28 457 à 28 467 d’indices impairs contre les arêtes 15 738 à 15 743 dans le même ordre. Les six petits BRep sont rejouables indépendamment (`artifacts/argon/occt/native/pair-1.brep` à `pair-6.brep`). Le rejeu séquentiel ciblé de la feuille diagonale **3 088** de cette coque (46 faces) reproduit les six interférences en 2,52 s. Le témoin local est `artifacts/argon/occt/native/failing-native-group.brep`. La capture BRep ne contient pas les identifiants des triangles STL. Ces faits établissent un problème arête/face avant transfert. Ils ne démontrent pas encore la correction à appliquer ni l’absence d’autres interférences.

## V-1260 — Performances du maillage, de la fidélité et du helper

Les deux corpus sont la rampe publique de 194 694 triangles et le STL privé Flansch de 540 170 triangles. Les signatures de l’atlas sérialisé, du modèle planaire et du rapport d’analyse sont identiques pour les deux versions sur chaque corpus. Les épreuves numériques sur la segmentation et les arcs courts ci-dessous concernent la nomination de candidats, avant certification.

| Opération mesurée | Rampe | Flansch |
|---|---:|---:|
| Segmentation du maillage déjà préparé, médiane CPU sur neuf mesures alternées | 50,39 → 31,30 ms | 202,51 → 119,51 ms |
| Mesure des cartes toriques, médiane CPU sur dix-huit passages en alternance | 25,69 → 21,23 ms | 391,12 → 207,22 ms |
| Fidélité bidirectionnelle, médiane murale sur huit mesures alternées | 165,28 → 136,25 ms | 996,97 → 824,15 ms |
| Pic RSS, fidélité et préparation en processus frais | 242,9 → 225,6 Mio | 495,7 → 470,2 Mio |

Pour le BVH, l’arbre conserve les mêmes décisions et le même ordre de visite. Les rapports finaux correspondent octet par octet aux rapports NEON. Les compteurs de travail atteignent respectivement 3 056 449 et 34 210 829 unités. Une deuxième série Flansch donne 995,09 → 841,35 ms. La pile des distances donne 866,33 ms ajoutée à la compaction lors de cette série.

Le helper de référence et le helper modifié ont traité la même rampe en protocole v2. La section DATA du STEP et les 2 261 octets du rapport sont identiques. Chacun publie un solide de 659 faces et de 93,536793 mm³. Les écarts échantillonnés sont de 0,004717 et 0,004702 mm. Sur un passage complet, les RSS respectifs valent 494 206 976 et 478 838 784 octets. Cette mesure inclut les changements du core et du BVH. Le gain propre à la suppression des listes de sélection n’a pas été isolé. Le helper conserve en mémoire l’analyse dense pendant la conversion.

Sur 256 panneaux de 72 triangles arrondis en f32, l’ordre décroissant d’aire ramène la médiane de 14 à 6 régions planes. Il améliore 248 panneaux et en fragmente davantage cinq. Sur les 256 témoins pliés, aucun des deux ordres ne franchit le pli de pente 0,01. Cette sonde reproduit la croissance avec les seuils du core sans reconstruire un solide.

Sur 1 536 arcs de rayon 2 mm à 15°, 30° et 45°, avec douze points et un bruit radial de 0,00001 mm, le passage de Kåsa à trois pas supplémentaires de Gauss–Newton conserve chaque décision de candidature. Au rayon de 0,06 mm, il perd quatorze candidatures. Pour une ellipse de rapport 0,7, les deux variantes nomment les mêmes 22 arcs de 15° sur 512. Le seuil d’étendue radiale et le conditionnement des arcs courts sont importants. Une amélioration du résidu quadratique ne garantit pas une meilleure décision.

### Témoins de reconstruction pour R-113

La recette `scripts/qualify-interrupted-surfaces-freecad.py` génère deux solides en millimètres, écrit leur STEP avec les courbes paramétriques, le réimporte, puis maille chaque référence à deux densités. En fournissant aussi le CLI, la recette reconstruit chaque STL en Auto et contrôle ses STEP avec FreeCAD. Les mesures suivantes proviennent de FreeCAD 1.1.1 / OCCT 7.8.1 pour les références et relectures, et de FabSTEP / OCCT 7.9.3 pour les reconstructions.

| Solide témoin | Référence réimportée | Auto actuel |
|---|---|---|
| Bouchon Ø24 mm, lèvre de rayon 2 mm, taillée par un plan incliné de 15° | Un tore, un cylindre, trois plans. Deux arêtes B-splines partagées entre tore et plan, avec deux coordonnées UV variables. Volume 1 273,706621 mm³. | 708 triangles donnent 631 faces planes. 2 496 triangles donnent 2 274 faces planes. |
| Axe Ø20 mm, perçage transversal Ø4,8 mm décalé de 3,2 mm | Deux cylindres et deux plans. Deux arêtes B-splines partagées par les cylindres. Volume 4 686,722892 mm³. | 428 triangles donnent 253 faces planes. 644 triangles donnent 370 faces planes. |

Les deux références sont fermées et valides après transfert. Chacune des arêtes communes a deux incidences de sens opposés. Les quatre STEP produits par Auto sont des solides valides. Les nombres de faces sont des mesures de départ pour les futurs reconnaisseurs.

### Témoin de composition pour R-114

Le bloc avec congé et perçages sécants décrit dans [R-114](../roadmap/items/R-114-composer-deux-reconnaissances-locales-sur-un.md) a été qualifié avec et sans chacune de ces opérations. Les trois STEP de référence passent les contrôles BRep et BOP après réimport. La distance minimale entre les surfaces courbes des deux opérations est de 5,5 mm.

En Auto aux deux densités, le congé isolé conserve six plans et un cylindre, et les perçages isolés six plans et trois faces cylindriques. La référence combinée comporte six plans et quatre faces cylindriques. Sur la pièce combinée, avec 982 puis 1 878 triangles, Auto conserve uniquement le cylindre du congé et produit respectivement 614 et 1 154 faces planes. Les six sorties Auto passent BRepCheck. Le contrôle BOP avancé de FreeCAD 1.1.1 signale `GeomAbs_C0` sur des arêtes des perçages isolés aux deux densités. Les résultats du congé et de la pièce combinée passent également ce contrôle avancé. Dans un second triplet à fraisure et congé, Auto retrouve déjà les deux familles ensemble aux deux densités.

### Contrôles d’intégration

Le workspace en profil test optimisé et verrouillé termine avec 711 tests réussis et 18 optionnels ignorés. Cette campagne a démarré avant la dernière variante du calcul UV. Le workspace final a été recompilé, les suites analyse/atlas ont été rejouées et les trois tests atlas en release avec `large-stl` ont passé, y compris le cas privé Flansch activé explicitement. Le contrôle de validité du groupe de triangles, la reconnaissance de l’atlas et la reconstruction d’un STEP complet sont trois épreuves distinctes.

Les onze tests du worker, les six tests ciblés du module document en profil gros STL, les 54 tests ordinaires du bureau et le test opt-in d’échange avec le helper réel passent. Les 241 tests UI, le contrôle Svelte, le build de l’interface, Clippy strict du workspace et du worker, le contrôle de compilation du bureau et le formatage passent. Le build Vite garde son avertissement de taille de module. La rampe en protocole v1 donne un rapport de 33 402 531 octets. Le helper du dernier paquet de développement la convertit également en v2 avec le même volume et les mêmes 659 faces. Le build local macOS a été recréé dans `artifacts/FabSTEP.app`.

## V-1261 — Raccord triangulaire entre deux bords toriques décalés

Le témoin indépendant est un secteur annulaire fermé. Deux tores de rayons de tube 2,4 et 1,8 mm portent des bords presque tangents dont les sommets sont décalés dans la direction angulaire. Le petit STL contient 7 104 triangles. Sur la coque entière, le constructeur précédent échouait au 19e groupe du contrôle OCCT séquentiel. Sur la bande examinée, 32 des 126 couples arête/face étaient refusés.

Pour un triangle presque coplanaire à un seul arc, le constructeur peut utiliser une surface bilinéaire rationnelle découpée sur un triangle. Les trois arêtes matérielles sont partagées. Les poids, le sens du jacobien projeté et la borne bidirectionnelle sur tout le triangle sont vérifiés avant la couture. À la relecture, le noyau contrôle les pôles effectifs, les poids et les trois courbes de bord dans l’espace paramétrique. Le banc natif couvre les deux orientations, un coin inversé par un déplacement de 60 nm, les génératrices droites représentées par une spline affine et des pénétrations réelles.

| Source publique | STEP réimporté | Vérification |
|---|---|---|
| Secteur à deux tores, 7 104 triangles | 1 solide, 962 faces, 304,172708 mm³ | Borne continue de 0,009580 mm. Les 962 identités source sont conservées. Le maillage de contrôle à 0,005 mm contient 2 624 triangles. |
| Même secteur, 166 848 triangles | 1 solide, 962 faces, 304,172709 mm³ | Helper v2 en Auto, 2 tores. Distances échantillonnées de 0,004546 mm de la source au résultat et 0,004540 mm en retour. STEP de 2 577 390 octets. |
| Rampe à encoches, 194 694 triangles | 1 solide, 659 faces, 93,536793 mm³ | Helper v2 en Auto, 1 tore. Distances de 0,004717 et 0,004702 mm. |

Une permutation cyclique des trois axes du petit STL conserve les 962 faces, les 2 624 triangles de contrôle et la borne de 0,009580 mm. Sur le même fichier dense, le helper de la candidate NEON précédente renvoie `cad.invalid_solid` et ne crée aucun STEP. FreeCAD relit les deux variantes du secteur et trouve dans chacune un solide valide avec 2 tores, 640 plans et 320 surfaces B-splines. Les tests sont reproductibles depuis la racine du dépôt. La rampe utilise le Python de FreeCAD ou un Python disposant de NumPy.

```sh
sh scripts/qualify-atlas-interference.sh
cargo build --release --manifest-path crates/fabstep-worker/Cargo.toml --offline --locked
python3 scripts/qualify-large-torus-worker.py crates/fabstep-worker/target/release/fabstep-worker --fixture staggered
python3 scripts/qualify-large-torus-worker.py crates/fabstep-worker/target/release/fabstep-worker --fixture ramp
```

**Flansch, contrôles ciblés.** Sur la coque construite depuis le STL original de 540 170 triangles, le même contrôle OCCT refuse 26 des 251 couples examinés sur le raccord entre les deux bords toriques avec l’ancien constructeur. Il les accepte tous après correction. Le contrôle étendu aux 1 907 nouvelles faces et aux 15 020 arêtes de contour des 43 tores accepte les 7 385 couples de boîtes englobantes qui se recouvrent. Les 1 907 supports et leurs bords sont retrouvés après un transfert STEP par lots. Le groupe historique de 46 faces passe avant et après transfert. Ces mesures portent sur une partie du contrôle nécessaire pour exporter le corps entier.

**Coût du contrôle.** Un regroupement spatial expérimental couvre les 1 391 063 paires de boîtes de la coque native. Les groupes larges et la variante qui isole les grandes faces dépassent tous deux 4 Gio sur un échantillon de Flansch. Le produit conserve le contrôle mono-forme séquentiel.

Un second prototype conserve le découpage actuel. Il teste la distance au tore complet depuis le centre de la boîte voisine. Cette distance varie au plus de la demi-diagonale sur la boîte. L’exclusion tient aussi compte des tolérances, d’une marge numérique et des boîtes réelles des arêtes et sommets des deux faces. Sur la coque de référence, 1 195 groupes contenant un tore deviennent 990, pour un surcoût médian de préparation de 84 ms. La rampe garde ses 48 groupes. Un échantillon stratifié de 40 groupes natifs, dont 30 choisis parmi les groupes allégés ou supprimés, passe de 198,83 à 99,59 s. Cette sélection ne permet pas d’estimer la durée du contrôle complet. Avant une intégration, éprouver l’épuisement du budget temporaire et comparer les deux contrôles entiers, avant et après STEP. Recette locale dans `artifacts/r085-reprise/cost/TORUS-FILTER.md`.

**Rejeu du STL Flansch entier.** Le helper v2 compilé en release a traité le fichier original en Auto, en millimètres, avec un écart demandé de 0,05 mm. La garde externe a arrêté le processus à 1 800,11 s. Le maximum de RSS ou d’empreinte échantillonné est de 3 250 179 168 octets (3,03 Gio). La dernière phase publiée est la construction du corps 0. Un échantillonnage de pile en cours d’exécution situe le calcul dans le contrôle mono-forme des auto-intersections, dont `BOPAlgo_PaveFiller::PerformEF`. Le répertoire privé du job ne contient ni STEP ni rapport de réussite à l’arrêt. Le verdict final sur les interférences du corps et la fidélité du STEP complet restent ouverts.

**Vérifications de cette tranche.** Sur macOS, 711 tests du workspace réussissent, 18 optionnels sont ignorés dans la suite standard. Les tests natifs explicitement rejoués ci-dessus s’y ajoutent. Les 11 tests worker, les 54 tests ordinaires du bureau, le test séparé de supervision avec le helper réel du bundle et les 241 tests UI réussissent. Le formatage Rust, Clippy du workspace, le contrôle Svelte, la compilation de l’interface et la création du paquet de développement passent. FreeCAD a servi au contrôle externe des nouveaux STEP publics.

## V-1262 — Contrôle des tores annulaires et raccords sur arcs courts

**Défaut de transfert.** Sur le corps Flansch de V-1261, le contrôle OCCT 7.9.3 accepte le raccord du triangle source 14 023 avant écriture et le refuse après relecture du STEP. Il s’agit de la face 8 393, retrouvée par ses sommets. Le contrôle du groupe de 47 faces échoue aussi, avec et sans filtre torique. Le contrôle géométrique BRep accepte la face dans les deux états. L’ancien raccord est une B-spline polynomiale de degrés (2,1) dont une rangée de pôles coïncide au sommet. Sa base est un arc de rayon 2 mm, de corde 0,003315 mm. En échangeant uniquement les pôles du support entre les deux états, on inverse les deux verdicts d’auto-intersection. Le déplacement maximal mesuré entre pôles homologues est de 6,05 × 10⁻¹³ mm.

Le constructeur essaie le domaine triangulaire rationnel existant sur un arc court quand le loft effectivement produit a cette rangée effondrée et que l’erreur d’interpolation quadratique du cercle est inférieure au quart de la précision de confusion du noyau. La certification porte sur les arêtes partagées, l’orientation du domaine entier et la borne au triangle source incluant ses déplacements. Sur le corps corrigé, le groupe de 47 faces et la face seule passent avant et après transfert. Un témoin public indépendant reproduit le refus historique et valide les deux orientations du raccord corrigé après relecture.

**Paires de faces disjointes.** Pour un tore annulaire de rayons finis R > r > 0, la distance au support au centre de la boîte voisine, diminuée de sa demi-diagonale, borne la distance au support de toute la boîte. Le filtre tient compte des tolérances des deux faces, des boîtes physiques de leurs arêtes et sommets et d’une marge d’arrondi. Un tore croisé constitue un contre-exemple à la formule utilisée hors de ce domaine. Les feuilles diagonales passent systématiquement dans OCCT. Une couverture incomplète ou un budget temporaire épuisé conserve la paire pour le noyau. Les plafonds de paires, groupes et comparaisons de boîtes restent débités avant exclusion.

Les témoins versionnés couvrent le trou d’un tore, une pénétration, une orientation inclinée, des bords physiques décalés et l’épuisement de chacun des quatre budgets temporaires. Le secteur public de 962 faces tourné comme solide de 148° autour de (1,3,2) passe le contrôle entier avec et sans filtre. Le filtre y écarte 102 paires. Sur Flansch corrigé, il écarte 14 506 des 1 057 549 paires de boîtes natives et 14 498 des 1 058 237 paires après transfert. Ces chiffres mesurent des exclusions géométriques, pas un gain de temps global.

Rejeu des témoins publics depuis la racine avec le SDK OCCT installé. Le générateur de la rampe nécessite NumPy.

```sh
sh scripts/qualify-atlas-interference.sh
cargo build --release --locked --manifest-path crates/fabstep-worker/Cargo.toml
python3 scripts/qualify-large-torus-worker.py crates/fabstep-worker/target/release/fabstep-worker --fixture ramp
python3 scripts/qualify-large-torus-worker.py crates/fabstep-worker/target/release/fabstep-worker --fixture staggered
```

**Tessellation du STEP de laboratoire.** Sur les 46 671 faces relues, deux petits raccords (31 002 et 31 220) portent une coarête dégénérée supplémentaire. La signature exige un unique contour et les trois côtés matériels. Elle admet aussi un coin dont une p-courbe droite entière sur le support rationnel se trouve dans une boule de rayon inférieur à la moitié de la précision de confusion autour du sommet partagé. Un segment qui dépasse la tolérance en 3D, une p-courbe qui revient en arrière et un trou distinct restent refusés. Sur le tore 3 139, les images UV d’un même sommet diffèrent de 1,356 × 10⁻⁷ rad, soit 6,78 × 10⁻⁸ mm. Le mailleur retrouve le coin à partir des coordonnées constantes des deux traces isoparamétriques, puis contrôle son écart au point 3D prescrit.

À 0,005 mm, le maillage de laboratoire comptait 107 464 triangles avant extraction. Traiter les 2 019 supports bilinéaires dont chaque côté possède une corde commune ramène le même BRep à 99 388 triangles. La preuve contrôle l’écart continu, le cône des normales, l’orientation et les extrémités. Un bord subdivisé ou une preuve insuffisante laisse le cache recalculer le maillage avec la face ordinaire. Le noyau relit indépendamment les octets STEP du même solide. Dans cette relecture, il compte 99 386 triangles avant extraction et retire 918 triangles effondrés aux pôles.

| API sur le STEP de laboratoire | Résultat | Vérification |
|---|---|---|
| Audit requis à 0,005 mm | 98 468 triangles | 0,011552221 mm source → STEP sur 2 430 757 échantillons et 0,010017653 mm STEP → source sur 569 233. |
| Inspection à 0,01 mm | 46 671 faces, 86 834 triangles | Plages de faces complètes. Le maillage ressoudé à 10⁻⁷ mm contient 130 251 arêtes avec deux usages opposés. |

Le helper normal et la capture de laboratoire ont des sorties distinctes. La capture de 132 109 885 octets conserve les paramètres et les étiquettes du writer de production, puis termine le job de laboratoire explicitement en erreur. Les mesures de fidélité issues de cette capture n’autorisent aucun export depuis le bureau. Les procédures privées de cette série sont dans `artifacts/r085-suite/{capture,full,repair,downstream,triangle-mesh}/`. Les trois bancs publics du raccord se rejouent avec `sh scripts/qualify-atlas-interference.sh`, le témoin torique avec `qualify-trimmed-torus-mesh --self-test .005`.

**Témoins de conversion.** Le helper du paquet de développement v2 convertit la rampe de 194 694 triangles en 659 faces (93,536793 mm³) et le secteur à bords décalés de 166 848 triangles en 962 faces (304,172709 mm³). Les maxima échantillonnés restent respectivement de 0,004717 / 0,004702 mm et 0,004546 / 0,004540 mm. Le secteur de laboratoire à intervalles de méridien inégaux (7 104 triangles) conserve 962 faces et une borne continue de 0,0127684 mm. Sur la pièce indépendante `body 26mm.stl`, le CLI Auto retrouve la référence de V-1256 avec 10 869 faces, dont 14 cylindres et 7 cônes, pour un solide de 17 053,652459 mm³.

**Validité du solide relu.** Un contrôle de laboratoire omet l’étape d’interférence pour atteindre le contrôle géométrique exact du solide par le chemin d’écriture et de relecture du helper. Avant écriture, le solide est valide. Après relecture, les 46 671 étiquettes sont présentes et distinctes, le volume passe de 54 337,961918051 à 54 337,961935345 mm³ et l’écart maximal des six bornes est de 1,82 × 10⁻¹² mm. Le helper de laboratoire renvoie néanmoins `cad.read_failed` au bout de 155 s, pour un pic de 3 450 507 416 octets.

Une seule face est invalide, y compris extraite avec ses deux voisines. La face 28 956 correspond à l’étiquette 28 772 et au triangle source 246 050 (indexation STL à partir de zéro). Ce raccord possède deux bords circulaires. Le statut de face `BRepCheck_UnorientableShape` provient de l’arête de rayon 10,006 mm. Sa courbe 3D et sa trace sur la surface s’écartent de 1,20921 × 10⁻⁷ mm après transfert. La tolérance de l’arête relue est de 1,20379 × 10⁻⁷ mm. Ce refus explique pourquoi la fermeture apparente du maillage, les étiquettes et la mesure du volume ne suffisent pas à certifier un STEP. Le contrôle se reproduit avec la face isolée de 2,4 ko dans `artifacts/r085-suite/transfer-proof/`.

**Loi de parcours du bord circulaire.** Après couture, l’arête 3D est un cercle et le bord de la surface est une Bézier quadratique rationnelle. Une trace UV linéaire parcourt les bons points à une vitesse différente. La correction ajuste cette loi sur le même bord. Elle vérifie les pôles et poids réels, borne l’erreur d’interpolation sur tout l’intervalle et conserve le support, le cercle, la tolérance et la trace sur la face voisine. Sur le témoin indépendant de rayons 0,8 et 11,25 mm, OCCT 7.9.3 reproduit le refus après transfert. Le témoin corrigé passe dans les deux sens de parcours. Un déplacement de pôle de 0,00001 mm écarte la correction.

Sur le corps privé, 29 traces sur 46 671 faces remplissent les conditions. Sur la face isolée 28 956, l’écart maximal après une nouvelle relecture passe de 1,20921 × 10⁻⁷ mm à 4,31 × 10⁻¹² mm avec le correctif intégré. La comparaison sur les 540 170 triangles du STL retrouve le même maillage de contrôle de 98 468 triangles et les deux mêmes distances échantillonnées.

Le helper de laboratoire corrigé repasse les contrôles géométriques du solide entier avant écriture et après relecture, puis retrouve une bijection entre les 46 671 étiquettes et les 46 671 faces réimportées. Les deux états comptent 31 746 plans, 43 tores et 14 882 B-splines. Le volume passe de 54 337,961918051 à 54 337,961935345 mm³. La borne géométrique du constructeur est de 0,027027217 mm et la taille du STEP est de 132 113 374 octets. Les contrôles de transfert isolés prennent 186,68 s pour un pic échantillonné de 3,28 Gio. Le laboratoire termine par une erreur volontaire. Les deux appels de contrôle des interférences ont leur propre campagne sur les formes sauvegardées.

**Contrôle des interférences sur les captures.** La liste ordonnée des faces de chaque composé a été comparée à celle du dispatcher du produit. Sur la capture précédant la correction de la loi circulaire, le contrôle mono-forme OCCT accepte directement les 6 793 groupes natifs et les 6 763 groupes relus. Sur la capture corrigée, les représentations des composés, les incidences, les localisations, les orientations, les tolérances, les p-courbes et les extrémités UV mémorisées ont été comparées. Des contre-exemples modifient notamment le partage, le sens et une extrémité de couture de 10⁻⁹. Un verdict hérité exige un enregistrement complet sur le composé de référence correspondant.

| État du solide corrigé | Verdict repris sur composé identique | Composés différents revérifiés par OCCT | Refus |
|---|---:|---:|---:|
| Natif | 6 439 | 354 | 0 |
| Relu depuis le STEP | 6 413 | 350 | 0 |

Le cumul des temps CPU des contrôles des groupes corrigés est de 13 533 s avant transfert et 13 598 s après, en incluant les mesures des composés identiques. Sur l’état de référence, environ 70 % du temps CPU provient de 442 groupes natifs et 439 groupes relus qui dépassent chacun 10 s. Le groupe (1880, 1881) réunit 80 petites B-splines et 11 plans. Il prend 65,9 s CPU avant transfert et 69,1 s après. Cette géométrie reste identique après correction de la loi de bord. Les mesures du banc réparti portent sur les contrôles OCCT, le temps mural du helper est mesuré séparément. Le résultat de chaque groupe et les empreintes des captures figurent dans `artifacts/r085-suite/full/arc/final.json` et les journaux associés.

**Rejeu du paquet de développement.** Sur le STL original, le helper inclus dans `artifacts/FabSTEP.app` termine à l’échéance de 1 800,19 s. Le pic échantillonné est de 3 259 632 808 octets (3,04 Gio). Un échantillonnage de pile à la 27ᵉ minute montre les opérations arête/face du contrôle mono-forme `BRepAlgoAPI_Check` dans le dispatcher de l’atlas. Le répertoire du job ne contient ni résultat STEP ni rapport final. Le helper de cette même application réussit la rampe et le secteur publics avec les métriques indiquées plus haut.

**Tests du dépôt.** `sh scripts/check.sh` passe sur macOS. Les 143 binaires de test du workspace, recompilés après la dernière correction de la courbe de bord, réussissent aussi en exécution directe : 715 tests, 18 ignorés dans la suite standard. Les 241 tests UI, les 54 tests du bureau, les 11 tests du helper, les contrôles Rust/TypeScript, le profil `large-stl` et le test supplémentaire du bureau avec le helper compilé passent. Le banc C++ public couvre le filtre d’interférences, la relecture des raccords, leur maillage et le bord circulaire dans les deux sens. L’essai manuel FabSTEP/Fusion des deux témoins R-115 reste suivi dans A-124.

## V-1263 — Plans fragmentés sur STL binaire tourné

Deux solides fermés de 96 triangles utilisent le même pavage irrégulier. La première pièce est un bloc, la seconde a une pente sur une partie de sa face supérieure. Les coordonnées sont écrites en f32 après transformation. Les deux fixtures du dépôt se régénèrent avec `scripts/generate-r115-planes.py`.

| Pièce | Avant | Après, modes Planar et Auto |
|---|---|---|
| Bloc tourné | 8 groupes dans l’analyse. Refus CAO en Planar et 89 faces en Auto. | 6 faces planes, 1 solide, 0,068359373 mm³. |
| Bloc avec pli | 9 groupes dans l’analyse. Refus CAO en Planar et 89 faces en Auto. | 7 faces planes, 1 solide, 0,075836181 mm³. Les deux faces du pli sont adjacentes. |

Le deuxième essai visite les graines par aire décroissante à indices source constants. La partition retenue doit réunir des régions entières. Chaque triangle satisfait la tolérance du document sur le plan ancré au premier sommet du contour. La vérification des contours applique la marge CAO de 5 × 10⁻⁸ mm. L’essai consomme un budget de parcours et de tri ainsi que le budget restant de vérification des segments. Les reconnaisseurs analytiques conservent leur segmentation habituelle.

Le test `cargo test --offline -p fabstep-app --test quantized_planes` vérifie le solide STEP relu, les contours, le plan des trois sommets de chaque triangle et l’appartenance exacte aux six ou sept groupes attendus. Les écarts échantillonnés sont inférieurs à 10⁻⁷ mm dans les deux directions. FreeCAD relit les deux exports Auto comme solides fermés valides de six et sept plans, avec des volumes de 0,068359373 et 0,075836181 mm³. Agrandir la pièce pliée d’un facteur 16 conserve le refus natif en mode Planar. Sur le secteur torique dense tourné en STL f32, le premier contour reste ramifié et le refus initial est conservé.

**Coût de la seconde segmentation.** Trois STL fermés de 97 340 triangles ont été comparés en release avec la version antérieure à R-115. Le pavé, sa variante à un pli et la pièce comportant 83 plans conservent leurs groupes de triangles. Sur les deux premiers, la garde sur les normales des plans source écarte l’essai avant de reconstruire l’adjacence Auto. Les médianes des neuf passages d’Auto sont de 735 → 714 ms et 720 → 654 ms, avec dispersion et charge concurrente. Sur la pièce à 83 plans, la seconde partition est essayée puis écartée. Les cinq passages donnent 949 → 993 ms (+4,6 %) en Auto. Le gain recherché est la récupération des faces sur les deux petits témoins. Les temps du corpus dense ne démontrent pas une accélération générale. Les entrées générées, mesures alternées et contrôles des appartenances sont dans `artifacts/r115-perf/`.


## V-1264 — Contours obliques et arbre cylindrique percé (R-113)

Le banc reconstruit les deux références de V-1260 dans FreeCAD 1.1.1 et maille leurs STEP relus. Chaque enregistrement STL est comparé dans son ordre réel à la facette du mailleur CAO et à la face dont elle provient. Le lecteur de laboratoire exige un maillage binaire fermé et connexe de 10 000 triangles au plus. La qualification de la lèvre utilise les dimensions du dessin. La proposition d’arbre percé utilise uniquement les coordonnées et l’incidence des triangles. Le STEP de référence fournit un contrôle indépendant.

| Pièce et entrée | Attribution vérifiée | Borne continue dans les deux sens | STEP réimporté |
|---|---:|---|---|
| Lèvre coupée à 15°, 708 puis 2 496 triangles | 708/708 et 2 496/2 496, cinq faces | Carreau torique nominal ≤ 0,049915 / 0,045125 mm. Chacun des deux bords plan–tore ≤ 0,008793 / 0,003614 mm. | Référence à cinq faces. Auto donne 631 / 2 274 plans. |
| Arbre Ø20 et perçage Ø4,8 mm, 428 puis 644 triangles | 428/428 et 644/644, quatre faces | Sur les quatre surfaces mesurées depuis chaque STL, ≤ 0,045666 / 0,012051 mm. | Prototype à deux cylindres et deux plans aux deux densités. Auto donne 253 / 370 plans. |
| Second arbre Ø19,4 et perçage Ø4,4 mm, 648 triangles | 648/648, quatre faces | Quatre surfaces mesurées ≤ 0,011378 mm. | Prototype à deux cylindres et deux plans. |

**Lèvre.** Les dimensions du dessin définissent le tore et son plan de coupe. La carte suit leurs deux vraies branches communes. Ses triangles ont une orientation constante et son périmètre parcourt une fois chaque côté. Une borne de dérivée seconde contrôle la distance aux triangles STL sur tout le carreau et sur chaque corde de bord. Une droite tracée en UV entre deux sommets corrects quitte le plan de 0,000871 / 0,000401 mm au milieu d’une corde. Un sommet déplacé de 0,04 mm est refusé. L’attribution couvre les cinq faces. La borne de surface annoncée concerne le carreau torique.

**Arbre percé.** Une proposition indépendante mesure le cylindre extérieur à partir des deux contours circulaires des bouchons, repère la paroi concave et ajuste l’axe et le rayon du perçage. Le certificat recontrôle les surfaces dans le repère obtenu. L’orientation des triangles dans les deux cartes cylindriques distingue leurs propriétaires, y compris lorsque leurs sommets appartiennent aux deux cylindres. Les deux bouches couvrent chacune un tour, avec des usages opposés sur les faces voisines. Les contrôles de fidélité portent sur l’intérieur des triangles et l’écart entre chaque contour polygonal et la branche analytique. Après rotation, translation et nouvelle quantification f32, les attributions restent identiques. Déplacer un sommet du perçage de 0,03 mm fait échouer le contrôle.

Le constructeur de laboratoire utilise les mesures du STL. Le STEP relu repasse la validité BRep, le contrôle d’interférences, les axes, rayons, bouchons, sondes de matière et dimensions. Les écarts des traces paramétriques sur chaque arête partagée sont mesurés à 129 positions et confrontés à la tolérance de cette arête. Le volume adaptatif de la première référence est de 4 686,771216 mm³. Les deux reconstructions donnent 4 686,771283 et 4 686,771192 mm³. La seconde référence et son résultat donnent 4 301,440831 et 4 301,440862 mm³. L’intégration de sections du dessin vérifie le volume des références. Le chiffre de 4 686,722892 mm³ en V-1260 provenait de la quadrature par défaut de FreeCAD.

Sur la seconde référence, la position initiale de la couture périodique du cylindre de coupe produisait une trace hors tolérance après transfert. Une rotation de 180° de ce cylindre autour de son propre axe fournit une référence valide pour les mêmes dimensions. Ce cas motive le contrôle des deux traces après réimport pour une future intégration native.

Depuis la racine du dépôt sur macOS, avec le SDK OCCT (dont `DRAWEXE` dans le PATH) et FreeCAD installé :

```sh
cargo build --release --offline --locked -j 2 -p fabstep-cli
PYTHONPATH=/Applications/FreeCAD.app/Contents/Resources/lib \
  /Applications/FreeCAD.app/Contents/Resources/bin/python \
  scripts/qualify-interrupted-surfaces-freecad.py artifacts/r113-qualification/results target/release/fabstep
```

Les répertoires de cas contiennent les rapports JSON, les STL, les références et les sorties. Les STEP dont le nom se termine par `-measured-lab.step` proviennent du prototype. Les fichiers `-auto.step` proviennent du CLI de production. Les contrôles de cette tranche utilisent exclusivement ces petits modèles synthétiques.

## V-1265 — R-116, plans fragmentés sur un STL quantifié

Le témoin public `fixtures/r116-quantized-roof.stl` contient 320 triangles. Avant ce changement, Auto réimportait 133 plans. Le recours qui classe les graines selon la stabilité de leur normale après une paire de passes Taubin ramène ce résultat à six plans, un solide et 0,006143998039 mm³. Les maxima de fidélité échantillonnés sont de 0,000000177 et 0,000000170 mm. Les coordonnées utilisées pour construire et contrôler le STEP proviennent du maillage soudé.

Les tests `cargo test -p fabstep-core --lib planar --offline` et `cargo test -p fabstep-app --test taubin_planes --offline` couvrent le classement déterministe, le bruit inférieur au seuil, le relief à conserver, les modes Plans et Auto et l’attribution des triangles.
