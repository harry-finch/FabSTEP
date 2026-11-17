# AUDIT-01 — Rapport d'audit technique complet : FabSTEP (`v1.0.0` + développements post-v1)

**Date de l'audit initial (`v1.0.0`)** : 7 novembre 2026  
**Date de la mise à jour et complétion post-v1** : 10 novembre 2026  
**Auditeur** : Auditeur logiciel senior indépendant (Architecture, Géométrie/CAO, Sécurité, Performance, Qualité/Tests, UX/UI)  
**Périmètre audité** : Dépôt complet `/Users/mugen/Documents/01_Projets/FabSTEP` (`HEAD` sur `1d8c26c` / `tag: v1.0.0`, branche `codex/v1-mechanical`, **ainsi que l'ensemble des développements post-v1 présents dans l'arbre de travail** : `32` fichiers suivis modifiés pour `+4 598 / -190` lignes et `42` nouveaux fichiers de code, tests, ADR `0013`–`0023`, fixtures et scripts)  
**Mode d'intervention** : Audit strictement non destructif du code source (aucune modification du code, des tests, des dépendances ou de l'état Git ; seul ce rapport `docs/AUDIT_TECHNIQUE.md` est mis à jour).

---

## Phase A — Cartographie initiale du projet (état actuel post-v1)

### A.1. Langages, frameworks et bibliothèques
- **Cœur géométrique, import, vérification et orchestration** : **Rust** (édition 2021, `Cargo.lock` verrouillé, compilation et tests compatibles `--offline`).
  - Dépendances externes volontairement minimalistes : `serde` / `serde_json` (DTO et rapports CLI/IPC), `zip` (`default-features = false, features = ["deflate"]`) et `roxmltree` pour l'import 3MF Core (`fabstep-import`), `tempfile` pour l'écriture atomique de fichiers STEP à l'export (`fabstep-cli`, `fabstep-desktop`).
- **Pont CAO natif (`fabstep-cad`)** : **Rust + C++17** compilé via `cc` (`crates/fabstep-cad/build.rs`) et lié à **Open CASCADE Technology (OCCT 7.9.3)** (`TKDESTEP`, `TKXSBase`, `TKSTEP`, `TKSTEP209`, `TKSTEPAttr`, `TKSTEPBase`, `TKShHealing`, `TKTopAlgo`, `TKGeomAlgo`, `TKMesh`, `TKPrim`, `TKBO`, `TKBRep`, `TKGeomBase`, `TKG3d`, `TKG2d`, `TKMath`, `TKernel`).
- **Coque bureau (`crates/fabstep-desktop`)** : **Tauri v2** (`tauri 2.10.3`, `tauri-plugin-dialog 2`) — hors workspace Cargo racine par choix d'isolation des dépendances système WebView/GUI.
- **Interface utilisateur (`ui/`)** : **Svelte 5** (`^5.55.3`, runes `$state`, `$derived`, `$effect`, `$props`), **TypeScript** (`~6.0.2`), **Vite 8** (`^8.0.7`), **Three.js** (`^0.183.2` : `WebGLRenderer`, `OrbitControls`, `STLLoader`, `Raycaster`), **2 Web Workers dédiés** (`selection.worker.ts` et `step-selection.worker.ts`), icônes `@lucide/svelte`, police locale `@fontsource-variable/inter`.
- **Scripts d'ingénierie (`scripts/`)** : Python 3 (`generate-3mf-fixtures.py`, `generate-m4-fixtures.py`, `generate-m5-fixtures.py`, `generate-m6-fixtures.py`, `validate-freecad.py`, `validate-m5-freecad.py`, `validate-m6-freecad.py`, `validate-post-v1-combinations-freecad.py`, `package-source.py`).

### A.2. Architecture générale et modules
Le projet est structuré en **6 crates Rust**, **1 application frontale Svelte 5**, **23 ADR** (`docs/adr/0001` à `0023`) et **1 skill locale d'orchestration** (`skills/chef-orchestre/SKILL.md`) :

| Composant | Lignes (code + tests internes) | Responsabilité principale |
|---|---|---|
| `crates/fabstep-core` | ~7 350 lignes (`lib.rs`, `stl.rs`, `topology.rs`, `planar.rs`, `recognition.rs`, `mixed.rs`, `partial.rs`, `partial/short.rs`, `revolved.rs`, `rounded.rs`, `rounded/zone.rs`, `rounded_box.rs`, `pipeline.rs`) + 11 suites de tests | Noyau géométrique pur et déterministe (`f64`, millimètres, sans IO ni OCCT) : lecture STL, soudage, topologie manifold, fusion coplanaire, reconnaissance analytique (`Extrusion`, `Mixed`, `Partial`, `ShortArc`, `Revolved`, `ConicalChain`, `Rounded`, `SphericalZone`, `RoundedBox`), reconnaissance guidée de cylindres complets (`recognize_guided_cylinders`) et topologie d'inspection source (`selection_mesh_topology`). |
| `crates/fabstep-import` | ~1 040 lignes (`src/lib.rs`) | Lecture 3MF Core 1.0 bornée : archive ZIP, relations OPC, XML `.model`, transformations affines, séparation des objets/composants sans fusionner les corps. |
| `crates/fabstep-cad` | 3 805 lignes Rust (`src/lib.rs`) + 2 403 lignes C++ (`src/occt.cpp`) + 148 lignes (`src/occt.h`) | Frontière FFI unique vers OCCT : validation BRep du maillage source, construction de solides analytiques ou facettisés, suivi de provenance exacte face STEP ↔ face source (`FABSTEP_SOURCE_FACE_V1_<token>`), écriture/relecture STEP en mémoire (`StepInputBuffer`/`std::stringstream`), vérification de séparation multi-corps, tessellation d'audit et inspection native des faces STEP (`result_faces`). |
| `crates/fabstep-verify` | ~470 lignes (`src/lib.rs`) | Audit géométrique symétrique par échantillonnage déterministe et grille spatiale 3D entre le maillage source et la tessellation OCCT du STEP. |
| `crates/fabstep-app` | 1 570 lignes (`lib.rs`, `document.rs`, `parameter_search.rs`) + 22 fichiers de tests d'intégration (`tests/*.rs`) | Orchestration commune CLI/bureau : aperçu `DocumentPreview`, topologie de sélection `selection_topology`, conversion `convert_document` / `convert_shared_document_with_provenance`, inspection `result_faces`, session de reconstruction guidée jusqu'à 16 cylindres (`DocumentStep`), recherche assistée de tolérance radiale (`search_parameters`), budgets d'écart/volume/boîte englobante. |
| `crates/fabstep-cli` | 319 lignes (`src/main.rs`) + `tests/cli.rs` | Binaire en ligne de commande `fabstep check` et `fabstep convert` (texte et `--json`, écriture atomique via `NamedTempFile`). |
| `crates/fabstep-desktop` | 1 229 lignes (`src/main.rs`) | Binaire Tauri : machine à états `EngineState`/`CacheState` (`revision`, `source_id`, `epoch`, `guidance_session`), transport IPC binaire `upload_source`, commandes `preview_document`, `selection_topology`, `result_faces`, `convert_document`, `search_parameters`, `guided_convert`, `remove_guidance`, `recompute_guidance`, `invalidate_result`, `export_step`. |
| `ui/` | ~2 050 lignes (`src/`) + 10 fichiers de tests (`tests/*.test.ts`, 36 tests) | Interface atelier Svelte 5 / Three.js : panneaux Source, Conversion (avec recherche de tolérance radiale `ParameterSearch.svelte`), Inspecteur (Source `SourceInspection.svelte`, Résultat STEP `StepInspection.svelte`, correspondance `StepSourceInspection.svelte`, indications guidées), `Viewport.svelte` (coloration 8 teintes `step-display.ts`, picking `source-picking.ts`) et 2 Web Workers (`selection.worker.ts`, `step-selection.worker.ts`). |

### A.3. Synthèse des ajouts réalisés depuis le précédent audit (`v1.0.0`)
Depuis l'audit initial sur `v1.0.0` (`1d8c26c`), quatre grands chantiers ont été développés :
1. **Extensions géométriques ciblées (`fabstep-core` + `fabstep-cad`)** :
   - **Lot A (`ADR-0013`)** : combinaison d'un cône droit complet et de cylindres ouverts prouvés en mode `Surfaces` (`rounded.rs:270-285, 340-349, 448-465`).
   - **Lot B (`ADR-0014`)** : zones sphériques sans pôle entre deux anneaux plans positifs (`crates/fabstep-core/src/rounded/zone.rs`, `rounded.rs:726-736, 1060-1062, 1594-1625`).
   - **Lot C1a (`ADR-0015`)** : chaînes coniques coaxiales alternées de 3 à 8 cônes séparés par des portées cylindriques résolues, avec témoins source sur chaque anneau intérieur (`revolved.rs:69-113, 745-809`, `rounded.rs:984-1033`).
   - **Consolidation M2c (`ADR-0021`)** : récupération d'union d'arcs par sous-chaînes non monotones (`merge_candidates`, `partial.rs:302-407`), preuve UV des parois cylindriques ouvertes à triangulation quinconcée (`curved_wall`, `partial.rs:952-1059`), et génératrices droites partagées entre deux cylindres partiels (`shared_rails`, `partial.rs:412-463`, `IndexedWires::bind_line` et courbes PCurve 2D `Geom2d_Line` dans `occt.cpp:259-277, 649-740`).
   - **Arcs courts contextuels (`ADR-0022`)** : intervalles de 4 à 7 sommets entre deux parois déjà prouvées, avec intervalle borné des centres et contrôle de tangence (`crates/fabstep-core/src/partial/short.rs`, `partial.rs:518-548`).
2. **Inspection géométrique, provenance exacte et reconstruction guidée (`I1` à `I2b`, `ADR-0016` à `0020`)** :
   - **Lot I1 (`ADR-0016`)** : graphe de voisinage source par composante (`selection_mesh_topology` dans `pipeline.rs:188-234`), picking par rayon 3D (`source-picking.ts`), croissance réversible par anneaux topologiques dans un Web Worker (`selection.ts`, `selection.worker.ts`, `SourceInspection.svelte`).
   - **Lot I1b (`ADR-0017`)** : extraction des faces natives du STEP réimporté et de leur adjacence par arête au sein de chaque solide (`fabstep_result_faces` dans `occt.cpp:2009-2112`, `result_faces` dans `fabstep-cad/src/lib.rs:1542-1666`), sélection multi-faces et croissance dans un second Web Worker (`step-selection.ts`, `step-selection.worker.ts`, `StepInspection.svelte`).
   - **Lot I1c (`ADR-0018`)** : correspondance exacte face STEP ↔ triangles source placés via étiquettes d'entités STEP `FABSTEP_SOURCE_FACE_V1_<token>` vérifiées à la relecture via `XSControl_WorkSession` / `TransferReader` (`occt.cpp:350-420, 1401-1479, 1634-1671`) et associées au sidecar privé de `DocumentStep` (`document.rs:187-200, 375-404`).
   - **Lots I2a & I2b (`ADR-0019`, `ADR-0020`)** : reconstruction guidée de 1 à 16 parois cylindriques complètes à partir de faces planes sélectionnées sur le résultat STEP (`recognize_guided_cylinders` dans `mixed.rs:98-152`, `DocumentStep::guided_cylinder` / `remove_guidance` / `recompute_guidance` dans `document.rs:233-373`, conservation de la session native après refus dans `main.rs:185-248`).
3. **Ergonomie et recherche assistée (`UX1` à `UX3`, `ADR-0023`)** :
   - **Lot UX1** : réorganisation du sélecteur de mode de reconstruction et repli des limites détaillées (`App.svelte:369`).
   - **Lot UX2 (`ADR-0023`)** : recherche assistée de tolérance radiale testant jusqu'à 7 facteurs (`[0.5, 1.0, 1.2, 1.5, 2.0, 5.0, 10.0]`) sans modifier le budget d'écart ni publier de STEP exportable (`parameter_search.rs`, `ParameterSearch.svelte`).
   - **Lot UX3 (`V-1160`)** : coloration déterministe à 8 teintes et filtrage des diagonales internes de tessellation en vue `Résultat STEP` (`step-display.ts`, `Viewport.svelte:56-70, 114-117`, `app.css:58-60, 144-145`).
4. **Flux derialisation STEP en mémoire dans `occt.cpp`** :
   - Remplacement des fichiers temporaires disque (`StepTempFile` dans `/tmp`) par des flux mémoire `StepInputBuffer` (`std::streambuf`) et `std::stringstream` via `STEPControl_Reader::ReadStream` et `STEPControl_Writer::WriteStream` (`occt.cpp:1485-1555, 2135-2150`).

### A.4. État Git observé
- **Branche** : `codex/v1-mechanical...origin/codex/v1-mechanical`
- **HEAD** : `1d8c26c` (`tag: v1.0.0`, `Finalize FabSTEP 1.0.0 mechanical reconstruction release`)
- **Arbre de travail** : `32` fichiers suivis modifiés (`+4 598 / -190` lignes) et `42` fichiers non suivis correspondant aux lots post-v1 (`A`, `B`, `C1a`, `M2c` consolidé, arcs courts, `I1`–`I2b`, `UX1`–`UX3`, `skills/chef-orchestre/`, `docs/AUDIT_TECHNIQUE.md`).

---

## 1. Synthèse exécutive

### 1.1. Appréciation globale et évolution depuis l'audit `v1.0.0`
**FabSTEP** conserve une **rigueur d'ingénierie remarquable** après la vague intensive de développements post-v1 (+4 600 lignes modifiées et 42 nouveaux fichiers). Les ajouts fonctionnels majeurs (inspection topologique source et STEP dans des Web Workers dédiés, provenance exacte par jetons d'entités STEP `FABSTEP_SOURCE_FACE_V1_`, reconstruction guidée multi-cylindres avec conservation de session après refus, chaînes coniques alternées, zones sphériques sans pôle, cylindres partiels quinconcés/rails partagés/arcs courts, recherche assistée de tolérance radiale) ont été conçus **en respectant scrupuleusement les invariants fondateurs** du projet :
- Aucun repli silencieux vers un résultat facettisé ou partiel ;
- Aucune confiance accordée aux identifiants de triangles ou d'octets provenant de l'interface (le sidecar `DocumentStep` vérifie `Arc::ptr_eq(&source.bytes, bytes)` et résout les triangles source exclusivement côté Rust à partir des jetons STEP relus après transfert OCCT) ;
- Déport exemplaire des calculs de voisinage et de préparation d'affichage STEP dans deux Web Workers (`selection.worker.ts` et `step-selection.worker.ts`) avec transfert zero-copy d'`ArrayBuffer`.

Deux améliorations notables ont en outre corrigé ou atténué des points soulevés lors du premier audit :
1. **`[AUD-002]` est corrigé** : `Viewport.svelte` a été découpé en 5 effets réactifs distincts (`rebuild`, `rebuildEdges`, `highlight`, `recolor`, `fit`) et ne réinitialise plus la caméra lors de l'arrivée du résultat, du basculement de `Contours des régions` (`showEdges`) ou du passage entre les vues `Source` et `Résultat STEP`.
2. **Le volet I/O disque de `[AUD-005]` est résolu** : `occt.cpp` n'écrit plus aucun fichier temporaire sur le disque système (`/tmp`) lors des conversions et inspections ; toutes les passes `STEPControl_Writer` / `STEPControl_Reader` opèrent désormais en mémoire via `StepInputBuffer` et `std::stringstream`.

En revanche, **les autres recommandations de l'audit initial n'ayant pas encore été implémentées**, la croissance rapide du périmètre fonctionnel a **amplifié certains goulets d'étranglement architecturaux** (`[AUD-001]`, `[AUD-004]`, `[AUD-005]`, `[AUD-013]`, `[AUD-021]`) et introduit **quelques défauts numériques ou algorithmiques localisés dans les nouveaux modules** (`[AUD-016]` à `[AUD-020]`).

### 1.2. Points forts majeurs (incluant les ajouts post-v1)
1. **Conception de sécurité et d'intégrité de la provenance STEP ↔ Source (`ADR-0018`, `I1c`, `I2a/I2b`)** : Le mécanisme `FABSTEP_SOURCE_FACE_V1_<token>` ne repose ni sur l'ordre des faces dans le fichier STEP ni sur une proximité géométrique approximative. Il inspecte la table de transfert OCCT (`XSControl_WorkSession::TransferReader()->EntityFromShapeResult`), refuse tout doublon, jeton manquant ou face divisée, et lie la session guidée à l'allocation mémoire exacte (`Arc::ptr_eq`) de la source courante.
2. **Architecture Web Worker double et gardes de révision UI (`I1`, `I1b`, `UX3`)** : Toute la validation des DTO de topologie, la croissance par anneaux BFS (source et STEP) et le coloriage glouton 8 couleurs + extraction des bords de faces STEP (`prepareStepDisplay`) s'exécutent hors du fil principal avec invalidation stricte par `(revision, epoch, job, requestId)`.
3. **Élimination des fichiers temporaires disque dans `fabstep-cad` (`occt.cpp`)** : L'utilisation de `StepInputBuffer` et `std::stringstream` supprime toute dépendance à `/tmp` pendant le calcul CAO et élimine les appels système `open`/`unlink` intermédiaires.
4. **Discipline de test toujours aussi élevée** : La suite automatisée est passée de ~130 tests en `v1.0.0` à **340 tests automatisés** (**304 tests Rust** dont 55 dans `fabstep-cad`, 121 dans `fabstep-core`, 82 dans `fabstep-app` et 16 dans `fabstep-desktop`, plus **36 tests TypeScript** incluant des tests sur de vrais Web Workers Node), tous passés sans aucun échec.

### 1.3. Principaux risques et défauts identifiés (état cumulé)
- **Aucun constat critique (bloquant ou vulnérabilité de sécurité exploitable à distance)** n'a été relevé.
- **1 constat de gravité élevée (`[AUD-001]`, amplifié post-v1)** : Le verrou global `occt_mutex()` et l'absence d'interruption coopérative interne à une conversion sont désormais sollicités par **4 familles d'opérations** (`convert_document`, `result_faces`, `guided_convert`/`recompute_guidance`, et surtout `search_parameters` qui enchaîne jusqu'à **7 conversions complètes**). Si `search_parameters` vérifie l'annulation *entre* deux essais, chaque essai individuel reste non interruptible de bout en bout.
- **11 constats de gravité moyenne** (`[AUD-003]` à `[AUD-008]` issus de `v1.0.0`, et 5 nouveaux constats post-v1 `[AUD-016]` à `[AUD-019]` + `[AUD-021]`/`[AUD-020]`) :
  - **`[AUD-016]` (Nouveau — Confirmé)** : Incohérence à la frontière $10^{-7}\text{ mm}$ entre `partial.rs:698` (garde composante par composante XY et Z $\le 10^{-7}\text{ mm}$) et `occt.cpp:659` (distance euclidienne 3D $\le 10^{-7}\text{ mm}$ pour ancrer les 4 coins d'une face cylindrique partielle), pouvant provoquer un échec FFI `cad.invalid_model` sur un cylindre partiel accepté par `fabstep-core`.
  - **`[AUD-017]` (Nouveau — Confirmé)** : Dans `partial.rs:162-189`, `closed_circle` ajuste une boucle circulaire fermée via `constrained_circle(points, tol)`, qui contraint le cercle à passer par `points[0]` et `points[N-1]` — deux sommets **adjacents** séparés par une unique petite arête du polygone — au lieu d'utiliser `recognition::fit_circle` (moindres carrés sur tout l'anneau).
  - **`[AUD-018]` (Nouveau — Confirmé)** : `search_parameters` (`parameter_search.rs:127-148`) recalcule jusqu'à **7 fois** le décodage STL/3MF, la segmentation planaire (`analyze_document_with_provenance`), la validation BRep OCCT du maillage source (`validate_source`) et la séparation des corps (`validate_sources_separated`), alors que ces étapes sont **strictement invariantes** lorsque seule `cylinder_fit_tolerance_mm` varie.
  - **`[AUD-019]` (Nouveau — Confirmé)** : `DocumentStep::guided_cylinder` (`document.rs:263`) rappelle `self.result_faces()`, ce qui relit le STEP et relance une **tessellation OCCT complète à 0,01 mm** (`BRepMesh_IncrementalMesh`) uniquement pour lire la table `source_correspondence.faces`, en jetant aussitôt tout le maillage tessellé.
  - **`[AUD-004]` (Amplifié post-v1)** : En plus de la duplication de `validate_topology` + `planar::build` dans chaque module de reconnaissance, `analyze_bodies_with_provenance` (`pipeline.rs:161-234`) exécute désormais `validate_topology_impl` deux fois par composante (dans `source_components` puis dans `analyze_raw_with_provenance`), et le mode `Surfaces` exécute deux fois `partial::augment` lorsqu'un candidat cône unique échoue.
- **9 constats de gravité faible** (`[AUD-009]` à `[AUD-015]` + nouveaux `[AUD-020]`, `[AUD-021]`, `[AUD-022]`) et **1 constat résolu (`[AUD-002]`)**.

---

## 2. Tableau de bord

| Domaine | Note `v1.0.0` (/10) | Note actuelle (/10) | Niveau de risque | Commentaire synthétique |
|---|:---:|:---:|:---:|---|
| **Architecture** | **9,0 / 10** | **8,8 / 10** | Faible | Frontières de crates et d'autorité (`DocumentStep`, Web Workers) irréprochables ; érosion légère due au grossissement de 4 fichiers au-delà de 1 350–3 800 lignes (`[AUD-021]`). |
| **Qualité du code** | **8,5 / 10** | **8,2 / 10** | Faible à Moyen | 0 warning `clippy`, 0 erreur `svelte-check`, typage fort ; mais `App.svelte` atteint 65 Ko en lignes compactées (`[AUD-013]`) et deux subtilités numériques/algorithmiques dans `partial.rs` (`[AUD-016]`, `[AUD-017]`). |
| **Sécurité** | **9,2 / 10** | **9,3 / 10** | Faible | Provenance STEP/source exemplaire (zéro confiance accordée aux IDs envoyés par l'UI, liaison `Arc::ptr_eq`, validation stricte dans les workers, budgets partout). |
| **Performance** | **7,2 / 10** | **7,0 / 10** | Moyen | Suppression bienvenue des fichiers temporaires disque dans `occt.cpp`, mais redondances accrues dans `search_parameters` ($7\times$ `validate_source`, `[AUD-018]`), `guided_cylinder` (`[AUD-019]`) et `pipeline.rs` (`[AUD-004]`), toujours sous `occt_mutex()` global (`[AUD-001]`). |
| **Tests & CI** | **8,4 / 10** | **8,6 / 10** | Faible à Moyen | Couverture remarquable des nouveaux lots (36 tests TS dont workers réels, 22 suites d'intégration Rust) ; absence persistante de workflow CI versionné (`[AUD-008]`) et de tests de composants `.svelte` (`[AUD-007]`). |
| **Documentation** | **8,8 / 10** | **8,7 / 10** | Faible | 11 nouveaux ADR (`0013`–`0023`) et `VALIDATION.md` (`V-1111`–`V-1160`) très précis ; léger retard de mise à jour dans `ARCHITECTURE.md`, `README.md` et `fabstep --help` (`[AUD-022]`). |
| **UX / UI & A11y** | **8,5 / 10** | **8,8 / 10** | Faible | Correction du reset caméra (`[AUD-002]`), sélecteur de modes clarifié (`UX1`), coloration STEP 8 teintes sans diagonales internes (`UX3`) ; friction résiduelle lors de l'édition d'une tolérance pendant une session guidée (`[AUD-020]`). |

---

## 3. Constats détaillés

### 3.1. Problèmes critiques
*Aucun problème critique n'a été identifié.*

---

### 3.2. Problèmes élevés

#### `[AUD-001]` — Sérialisation globale sur `occt_mutex()` et absence d'interruption coopérative interne pendant un calcul en cours *(Confirmé — Amplifié post-v1)*
- **Sévérité** : **Élevée** (Performance / Réactivité / Expérience utilisateur sur fichiers moyens à lourds)
- **Catégorie** : Performance & Architecture concurrente
- **Statut** : **Confirmé et amplifié** par les nouvelles commandes post-v1 (`result_faces`, `guided_convert`, `recompute_guidance`, `search_parameters`).
- **Localisation** :
  - `crates/fabstep-cad/src/lib.rs:344-348` (`occt_mutex()`) et `crates/fabstep-cad/src/lib.rs:1572` (`result_faces`)
  - `crates/fabstep-desktop/src/main.rs:388-580` (`result_faces`, `convert_document`, `search_parameters`, `guided_convert`)
  - `crates/fabstep-app/src/lib.rs:206-384` (`convert_analysis_with_recognized`)
  - `crates/fabstep-app/src/document.rs:638-798` (`convert_analyzed_document`)
  - `crates/fabstep-app/src/parameter_search.rs:127-148` (`search_parameters`)
- **Description précise** :
  1. Toutes les opérations OCCT acquièrent un unique `Mutex<()>` global (`occt_mutex()` dans `crates/fabstep-cad/src/lib.rs:344`).
  2. Depuis les lots post-v1, ce verrou est partagé entre la conversion principale (`convert_document`), l'inspection des faces natives (`result_faces`, qui relit et tesselle le STEP à `0,01 mm`), la reconstruction guidée (`guided_convert`, `remove_guidance`, `recompute_guidance`) et la recherche de paramètres (`search_parameters`, qui exécute jusqu'à **7 conversions complètes**).
  3. Dans `search_parameters` (`parameter_search.rs:135-142`), le callback `check_current()` est bien appelé **avant et après** chacun des 7 essais, ce qui permet d'arrêter la recherche entre deux essais si l'utilisateur modifie un réglage. En revanche, **à l'intérieur d'un essai** (ou d'un `convert_document` / `guided_convert`), aucune vérification de fraîcheur (`CacheState::current`) n'est effectuée entre :
     - la validation BRep source (`validate_sources_separated`, `validate_source`),
     - la boucle sur les `1..=32` corps (`document.rs:665-713`),
     - la construction BRep du solide reconnu (`build_*_step`),
     - la tessellation d'audit (`tessellate_step`) et la comparaison de maillages (`compare_meshes`),
     - et l'assemblage multi-racines (`combine_steps` / `combine_steps_with_provenance`).
- **Impact** :
  Si l'utilisateur modifie une tolérance ou change de fichier pendant un essai coûteux (ex. maillage de 50 000 triangles ou document 3MF de 10 à 32 corps), la tâche obsolète continue d'exécuter toutes les étapes OCCT de tous les corps restants et monopolise `occt_mutex()`, bloquant le démarrage effectif de la nouvelle requête pendant plusieurs secondes à plusieurs dizaines de secondes.
- **Solution proposée** :
  Passer un prédicat d'annulation coopérative `is_cancelled: &dyn Fn() -> bool` à `convert_analyzed_document` et `convert_analysis_with_recognized` (vérifié avant chaque corps, avant `validate_source`, avant `build_*_step`, avant `tessellate_step`, avant `compare_meshes` et avant `combine_steps`), retournant immédiatement `Err(FabError::new("engine.stale", ...))` dès que `(request_id, source_id, epoch)` n'est plus courant dans `EngineState`.
- **Risque de régression** :
  Faible : l'interruption se fait uniquement aux frontières entre appels Rust/FFI, sans interrompre un appel C++ OCCT au milieu de son exécution.

---

### 3.3. Problèmes moyens

#### `[AUD-016]` — Incohérence entre la tolérance par composante (`partial.rs`) et la distance euclidienne 3D (`occt.cpp`) lors de l'ancrage des sommets des cylindres partiels *(Nouveau post-v1 — Confirmé)*
- **Sévérité** : **Moyenne** (Fiabilité géométrique aux limites numériques / Contrat d'erreur)
- **Catégorie** : Qualité du code & Cohérence `fabstep-core` ↔ `fabstep-cad`
- **Statut** : **Défaut confirmé** par analyse du code (`partial.rs` vs `occt.cpp`).
- **Localisation** :
  - `crates/fabstep-core/src/partial.rs:695-701` et `crates/fabstep-core/src/partial.rs:905-913` (`NATIVE_TOL = 1e-7`)
  - `crates/fabstep-cad/src/occt.cpp:644-674` (`partial_cylindrical_face`, `Precision::Confusion() = 1e-7`)
- **Description précise** :
  Dans `crates/fabstep-core/src/partial.rs:697-700`, la preuve d'une paroi cylindrique partielle vérifie que les extrémités des arcs inférieur (`bottom[0]`, `bottom.last()`) et supérieur (`top[0]`, `top.last()`) coïncident avec le cylindre idéal à `NATIVE_TOL = 1e-7 mm` **composante par composante** :
  ```rust
  if (i == 0 || i + 1 == bottom.len())
      && (distance > NATIVE_TOL || bz.abs() > NATIVE_TOL || (tz - height).abs() > NATIVE_TOL)
  {
      return Ok(None);
  }
  ```
  De même, `partial.rs:907` vérifie `final_fit.center[0].hypot(final_fit.center[1]) > NATIVE_TOL`.
  Or, côté C++ dans `crates/fabstep-cad/src/occt.cpp:644-674`, `partial_cylindrical_face` construit la surface `gp_Cylinder` sur le repère `lower.center` de rayon `lower.radius` et de hauteur `height`, puis associe chacun des 4 sommets topologiques de `BRepBuilderAPI_MakeFace` aux sommets source `points[id]` en testant la **distance euclidienne 3D** contre `tol = Precision::Confusion() = 1e-7` :
  ```cpp
  for (std::size_t ring : cylinder.rings) {
    for (std::size_t id : arcs[ring].vertices) {
      if (BRep_Tool::Pnt(vertex).Distance(point_at(points, id)) <= tol) {
        matched = id;
        break;
      }
    }
  }
  if (matched == invalid) {
    throw CadFailure{invalid_model, "Un sommet de la paroi n'a pas d'extrémité source."};
  }
  ```
  Si une extrémité supérieure présente par exemple un écart dans le plan XY de $0{,}8 \times 10^{-7}\text{ mm}$ et un écart axial Z (`|tz - height|`) de $0{,}8 \times 10^{-7}\text{ mm}$, toutes les conditions de `partial.rs` sont satisfaites ($0{,}8 \times 10^{-7} \le 10^{-7}$), mais la distance euclidienne 3D entre le coin analytique du cylindre OCCT et `point_at(points, id)` vaut $\sqrt{(0{,}8)^2 + (0{,}8)^2} \times 10^{-7} \approx 1{,}13 \times 10^{-7}\text{ mm} > 10^{-7}\text{ mm}$.
- **Impact** :
  1. Un candidat accepté par `fabstep-core` échoue dans `fabstep-cad` avec l'erreur générique `cad.invalid_model` (*« Un sommet de la paroi n'a pas d'extrémité source »*) au lieu d'être filtré proprement dans `partial.rs`.
  2. Plus critique encore : dans `recognize_cylinders` (`crates/fabstep-app/src/lib.rs:160-166`), lorsqu'une pièce est déjà reconnue comme extrusion valide (`Ok(r)`), `recognize_mixed` est tenté pour enrichir d'éventuels arcs ouverts. Si `recognize_mixed` accepte un arc à cette frontière $[10^{-7}, \sqrt{2}\times 10^{-7}]\text{ mm}$, il remplace l'extrusion valide (`Ok(Recognized::Mixed(mixed))`) et fait échouer la conversion à l'étape OCCT !
- **Solution proposée** :
  Dans `crates/fabstep-core/src/partial.rs`, vérifier directement la distance euclidienne 3D entre les 4 coins analytiques exacts du cylindre (`lower.center + R u_0`, `lower.center + R u_1`, `upper.center + R u_0`, `upper.center + R u_1`) et les 4 sommets `model.vertices[id]` avec une borne stricte `<= 0.9 * NATIVE_TOL` (`9e-8 mm`).
- **Risque de régression** :
  Très faible : n'affecte que la bande numérique $[0{,}9 \times 10^{-7}, 1{,}42 \times 10^{-7}]\text{ mm}$ où OCCT aurait de toute façon rejeté le modèle.

---

#### `[AUD-017]` — Ajustement d'anneau fermé dans `partial::closed_circle` contraint sur deux sommets adjacents du polygone (`points[0]` et `points.last()`) *(Nouveau post-v1 — Confirmé)*
- **Sévérité** : **Moyenne** (Fiabilité géométrique / Stabilité numérique)
- **Catégorie** : Qualité algorithmique (`fabstep-core`)
- **Statut** : **Défaut algorithmique confirmé** par inspection de `crates/fabstep-core/src/partial.rs:85-189`.
- **Localisation** : `crates/fabstep-core/src/partial.rs:162-189` (`closed_circle`) et `crates/fabstep-core/src/partial.rs:85-130` (`constrained_circle`).
- **Description précise** :
  Lorsqu'une pièce contient à la fois un cylindre partiel et une boucle circulaire fermée (trou ou bossage complet), `partial::extract_loops` appelle `closed_circle(&pts, tol)` (`partial.rs:230`).
  Or `closed_circle` délègue l'estimation du cercle à `constrained_circle(points, tol)` (`partial.rs:163`).
  La fonction `constrained_circle` a été écrite pour un **arc ouvert** : elle impose que le cercle passe exactement par `a = points[0]` et `b = *points.last()?`, place le centre sur la médiatrice du segment $[a, b]$ de longueur `chord = hypot(b - a)` et estime le décalage `offset` perpendiculairement à $[a, b]$.
  Mais pour une **boucle fermée** de $N$ sommets (`pts`), `points[0]` et `points[N - 1]` ne sont pas deux extrémités opposées : ce sont **deux sommets adjacents** séparés par une unique arête de facettisation de longueur $c = 2 R \sin(\pi / N)$ !
  Cela entraîne deux défauts :
  1. **Mauvais conditionnement numérique** : contraindre le centre sur la médiatrice d'une seule petite arête $[p_0, p_{N-1}]$ amplifie toute erreur d'arrondi (notamment issue d'un STL binaire `f32`) sur la normale à cette arête d'un facteur $\approx 2 R / c = 1 / \sin(\pi / N)$ (soit un facteur $\times 20$ pour $N = 64$ ou $\times 40$ pour $N = 128$).
  2. **Rejet erroné sur maillages denses ou tolérances élargies** : la garde `if chord <= tolerance * 4. { return None; }` (`partial.rs:94`) exige que la longueur d'une seule facette dépasse $4 \times \text{tolérance}$, alors que la corde réelle du cercle est son diamètre $2 R$.
- **Impact** :
  Une pièce combinant un trou cylindrique complet (par ex. en STL binaire `f32` ou finement tessellé) et une encoche cylindrique partielle peut voir son trou complet rejeté ou décentré dans `partial.rs`, alors que le même trou est parfaitement reconnu par `recognition::fit_circle` (ajustement de Kåsa sur l'ensemble des sommets) dans `mixed.rs:333`.
- **Solution proposée** :
  Dans `crates/fabstep-core/src/partial.rs:162-189`, remplacer l'appel `let fit = constrained_circle(points, tol)?;` dans `closed_circle` par `let fit = crate::recognition::fit_circle(points, tol)?;` (en conservant `constrained_circle` uniquement pour `open_arc` où les extrémités doivent rester exactes sur les arêtes planes adjacentes).
- **Risque de régression** :
  Faible : aligne l'ajustement des anneaux fermés de `partial.rs` sur celui de `mixed.rs` et `recognition.rs`.

---

#### `[AUD-018]` — Recalcul complet de l'import, de la segmentation planaire et de la validation BRep source OCCT à chaque essai de `search_parameters` (jusqu'à $7\times$) *(Nouveau post-v1 — Confirmé)*
- **Sévérité** : **Moyenne** (Performance / Latence de la recherche de réglages)
- **Catégorie** : Performance & Algorithmes (`fabstep-app`)
- **Statut** : **Problème de performance prouvé par analyse algorithmique**.
- **Localisation** :
  - `crates/fabstep-app/src/parameter_search.rs:127-148` (`search_parameters`)
  - `crates/fabstep-app/src/document.rs:580-660` (`convert_document`, `analyze_document_with_provenance`, `validate_sources_separated`)
  - `crates/fabstep-app/src/lib.rs:217` (`validate_source(&triangle_model(&analysis.mesh))`)
- **Description précise** :
  La recherche assistée (`ADR-0023`, `search_parameters`) évalue jusqu'à 7 valeurs de `cylinder_fit_tolerance_mm` (`[0.5, 1.0, 1.2, 1.5, 2.0, 5.0, 10.0] * current`), tout en maintenant `input_unit`, `weld_tolerance_mm`, `plane_tolerance_mm` et `max_deviation_mm` strictement constants.
  Or, `search_parameters` appelle `convert_document(bytes, format, &trial_options)` dans une boucle (`parameter_search.rs:135-142`). À chacune des 7 itérations, le moteur ré-exécute intégralement :
  1. Le décodage binaire/ASCII STL ou le décompactage ZIP + parsing XML 3MF (`preview_document`).
  2. La séparation des composantes, le soudage, la validation topologique et la fusion coplanaire (`analyze_document_with_provenance`), qui ne dépendent pourtant **que** de `input_unit`, `weld_tolerance_mm` et `plane_tolerance_mm`.
  3. La validation BRep OCCT du maillage source triangulé (`validate_source(&triangle_model(&analysis.mesh))` dans `fabstep-app/src/lib.rs:217`), qui coud et vérifie tous les triangles source dans OCCT et est **100 % indépendante** de `cylinder_fit_tolerance_mm`.
  4. Pour les documents multi-corps, la preuve OCCT de séparation spatiale des maillages source (`validate_sources_separated(&models)` dans `document.rs:658`), elle aussi **100 % indépendante** de `cylinder_fit_tolerance_mm`.
- **Impact** :
  Sur un maillage de 20 000 à 100 000 triangles ou un document 3MF multi-corps, `validate_source` et `validate_sources_separated` représentent souvent 40 % à 70 % du temps total d'une conversion. Les répéter 7 fois sur des données strictement identiques alourdit considérablement la durée de `search_parameters` (d'où le message UI *« cela peut prendre plusieurs minutes »*).
- **Solution proposée** :
  Dans `search_parameters` :
  1. Exécuter **une seule fois** avant la boucle : `analyze_document_with_provenance(bytes, format, options)`, `validate_sources_separated` (si `bodies.len() > 1`) et `validate_source(&triangle_model(&body.analysis.mesh))` pour chaque corps, avec des points de contrôle `check_current()?`.
  2. Dans la boucle des 7 tolérances radiales, n'exécuter pour chaque corps que la reconnaissance analytique (`recognize_surfaces`), la construction STEP du solide candidat, la vérification du solide/fidélité et `combine_steps`.
- **Risque de régression** :
  Nul sur le résultat : les étapes factorisées ont des entrées bit-à-bit identiques sur les 7 essais.

---

#### `[AUD-019]` — Relecture et tessellation OCCT redondantes du STEP (`self.result_faces()`) lors de chaque reconstruction guidée (`guided_cylinder`) *(Nouveau post-v1 — Confirmé)*
- **Sévérité** : **Moyenne** (Performance / Latence de la reconstruction guidée)
- **Catégorie** : Performance (`fabstep-app` / `fabstep-cad`)
- **Statut** : **Problème de performance confirmé** par lecture de `crates/fabstep-app/src/document.rs:233-373`.
- **Localisation** :
  - `crates/fabstep-app/src/document.rs:263-270` (`DocumentStep::guided_cylinder`)
  - `crates/fabstep-app/src/document.rs:375-404` (`DocumentStep::result_faces`)
  - `crates/fabstep-cad/src/occt.cpp:2009-2112` (`fabstep_result_faces`)
- **Description précise** :
  Pour pouvoir cliquer sur **Essayer un cylindre**, l'utilisateur a déjà ouvert la vue **Résultat STEP**, ce qui a déjà appelé `DocumentStep::result_faces()` via la commande Tauri `result_faces`.
  Pourtant, lorsque `guided_convert` appelle `DocumentStep::guided_cylinder(&self, face_ids)` (`document.rs:233`), la ligne 263 exécute à nouveau :
  ```rust
  let inspected = self.result_faces()?;
  if inspected.source_correspondence.status != "exact" { ... }
  let faces = inspected.source_correspondence.faces;
  ```
  Or `self.result_faces()` appelle `fabstep_cad::result_faces(&self.bytes)`, qui acquiert `occt_mutex()`, relit le fichier STEP via `STEPControl_Reader`, parcourt toutes les faces et **exécute `BRepMesh_IncrementalMesh(shape, 0.01, false, 0.1, true)`** (`occt.cpp:2030`), alloue et copie tous les sommets et triangles de la tessellation vers Rust, avant que `guided_cylinder` ne jette immédiatement `inspected.result.mesh` et `inspected.result.faces` pour ne lire que `inspected.source_correspondence.faces` !
- **Impact** :
  Sur un modèle complexe comportant des milliers de facettes ou plusieurs cylindres déjà guidés, chaque ajout d'indication cylindrique paie une tessellation OCCT complète à `0,01 mm` totalement inutile avant même de lancer la reconstruction guidée.
- **Solution proposée** :
  Extraire dans `fabstep-cad` une fonction légère de lecture des jetons de provenance des faces STEP (sans `BRepMesh_IncrementalMesh`), ou mettre en cache `OnceLock<Result< Vec<ResultFaceSource>, FabError >>` (ou le vecteur `source_correspondence.faces` résolu lors de `result_faces`) directement dans `DocumentStep` (qui est immuable par construction).
- **Risque de régression** :
  Très faible : `DocumentStep` étant immuable (`bytes` et `source_faces` privés et jamais modifiés après création), la correspondance `face_id -> ResultFaceSource` est une fonction pure de `DocumentStep`.

---

#### `[AUD-002]` — Réinitialisation forcée de la caméra 3D (`fit()`) lors de la fin de conversion ou du basculement de `showEdges` *(Corrigé depuis l'audit `v1.0.0`)*
- **Sévérité initiale** : Moyenne (UX / Ergonomie 3D)
- **Statut actuel** : **Corrigé dans les développements post-v1**.
- **Preuve observée** :
  Dans `ui/src/components/Viewport.svelte:89-112, 201-205`, le composant sépare désormais la reconstruction des maillages (`rebuild`), la reconstruction des arêtes (`rebuildEdges`), la surbrillance de sélection (`highlight`), le thème/couleurs (`recolor`) et le cadrage (`fit`) dans 5 blocs `$effect` indépendants. Dans `rebuild()`, `const shouldFit = renderedRevision !== sourceRevision || !meshes.size;` (`Viewport.svelte:92`) garantit que la caméra n'est cadrée automatiquement qu'au chargement d'une nouvelle géométrie source ou sur action explicite (`fitSignal++` via `F` ou le bouton Cadrer).

---

#### `[AUD-003]` — `scripts/package-source.py` utilise `os.walk` au lieu de `git ls-files`, exposant l'archive source à l'inclusion de fichiers locaux non suivis *(Confirmé — Aggravé post-v1)*
- **Sévérité** : **Moyenne** (Chaîne d'approvisionnement / Intégrité de distribution)
- **Catégorie** : Sécurité & Distribution
- **Statut** : **Confirmé et aggravé** par la présence actuelle de nombreux fichiers non suivis dans le dépôt (`docs/AUDIT_TECHNIQUE.md`, `fixtures/m6a_rounded_box_rotated.step`, etc.).
- **Localisation** : `scripts/package-source.py:13-46`.
- **Description précise** :
  `iter_source_files()` parcourt le répertoire de travail avec `os.walk(ROOT)` en excluant uniquement une liste noire (`EXCLUDED_DIRS`, `EXCLUDED_FILES`, suffixes `.DS_Store`, `.pyc`, `.log`). Tout fichier local non suivi situé hors de `target/`, `node_modules/` ou `dist/` (par exemple `docs/AUDIT_TECHNIQUE.md`, `fixtures/m6a_rounded_box_rotated.step`, ou un fichier de travail temporaire à la racine) est embarqué silencieusement dans l'archive de release `FabSTEP-<version>-source.tar.gz`.
- **Solution proposée** :
  Utiliser `git archive` ou `git ls-files --cached` (et refuser de produire l'archive de release si `git status --porcelain` n'est pas vide, sauf option explicite `--allow-dirty`).
- **Risque de régression** : Faible.

---

#### `[AUD-004]` — Recalcul redondant de `validate_topology` et `planar::build` dans la cascade de reconnaissance analytique et dans `analyze_bodies_with_provenance` *(Confirmé — Amplifié post-v1)*
- **Sévérité** : **Moyenne** (Performance / Complexité algorithmique)
- **Catégorie** : Performance & Architecture de `fabstep-core`
- **Statut** : **Confirmé et amplifié** par les ajouts post-v1 dans `pipeline.rs` et `rounded.rs`.
- **Localisation** :
  - `crates/fabstep-core/src/pipeline.rs:161-234` (`analyze_bodies_with_provenance`, `source_components`, `analyze_raw_with_provenance`)
  - `crates/fabstep-core/src/mixed.rs:88-89` (`recognize_mixed`)
  - `crates/fabstep-core/src/partial.rs:512-513` (`partial::augment`)
  - `crates/fabstep-core/src/revolved.rs:141-142` (`recognize_revolved`)
  - `crates/fabstep-core/src/rounded.rs:163-164` (`recognize_rounded`) et `rounded.rs:278` (`single_cone_open_cylinders` -> `crate::partial::augment`)
- **Description précise** :
  Outre la ré-exécution de `topology::validate_topology(&analysis.mesh, options)` et `planar::build_with_provenance(&analysis.mesh, &topology, options)` dans chaque module de reconnaissance (`mixed`, `partial`, `revolved`, `rounded`), les développements post-v1 ont ajouté deux nouvelles duplications :
  1. **Doublon systématique dès l'analyse source (`pipeline.rs:161-234`)** : `analyze_bodies_with_provenance` appelle `source_components(mesh, options)` qui exécute `validate_topology_impl(mesh, options, false)` (ligne 181), puis appelle immédiatement `analyze_raw_with_provenance(component.mesh, options)` qui ré-exécute `validate_topology_impl(&mesh, options, true)` (ligne 239) sur chaque composante déjà extraite et soudée !
  2. **Double appel à `partial::augment` en mode `Surfaces` (`rounded.rs:278`)** : lorsqu'une pièce possède un candidat cône unique qui finit par retourner `recognition.unsupported` dans `recognize_rounded`, `single_cone_open_cylinders` a déjà exécuté `crate::partial::augment(analysis, options)`, puis le repli vers `Primitives` -> `Cylinders` -> `recognize_mixed` ré-exécute une seconde fois `partial::augment(analysis, options)`.
- **Solution proposée** :
  1. Dans `pipeline.rs`, conserver le `ValidatedTopology` par composante issu de `source_components` au lieu de rappeler `validate_topology_impl` dans `analyze_raw_with_provenance`.
  2. Stocker la table de provenance `planar_triangle_faces: Vec<usize>` (déjà calculée à la ligne 240 de `pipeline.rs` !) et l'adjacence topologique dans `Analysis` afin que `mixed`, `partial`, `revolved` et `rounded` la réutilisent directement en $O(1)$ sans ré-exécuter `validate_topology` ni `planar::build_with_provenance`.
- **Risque de régression** :
  Très faible (`fabstep-core` est pur et déterministe ; tous les tests existants vérifient l'identité exacte des sorties).

---

#### `[AUD-005]` — Cycles multiples de sérialisation/relecture STEP en mémoire et de tessellations OCCT par conversion *(Partiellement amélioré sur l'I/O disque — Confirmé sur le CPU OCCT)*
- **Sévérité** : **Moyenne** (Performance CPU)
- **Catégorie** : Performance (`fabstep-cad` / `fabstep-app`)
- **Statut** : **I/O disque temporaire corrigée post-v1** (`StepInputBuffer` / `std::stringstream` dans `occt.cpp:1485-1555`), mais **multiples parsings STEP et doubles tessellations OCCT toujours présents**.
- **Localisation** :
  - `crates/fabstep-cad/src/occt.cpp:1485-1565` (`write_and_verify`)
  - `crates/fabstep-cad/src/occt.cpp:1590-1695` (`fabstep_combine_steps`, `fabstep_combine_steps_with_provenance`)
  - `crates/fabstep-cad/src/occt.cpp:1975-2007` (`fabstep_tessellate_step`)
  - `crates/fabstep-cad/src/occt.cpp:2009-2112` (`fabstep_result_faces`)
- **Description précise** :
  Grâce au passage à `StepInputBuffer` et `std::stringstream` post-v1, aucune opération n'accède plus au disque `/tmp`. En revanche, en termes de CPU OCCT, la conversion analytique d'un document multi-corps suivie de l'inspection `Résultat STEP` effectue toujours :
  1. Par corps : écriture STEP en mémoire (`WriteStream`) + relecture STEP (`ReadStream`) + `BRepCheck_Analyzer` dans `write_and_verify`.
  2. Par corps : relecture STEP (`ReadStream`) + `BRepMesh_IncrementalMesh` (déflexion `min(0.1 * max_deviation, 0.01)`) dans `tessellate_step` pour l'audit `fabstep-verify`.
  3. Pour le document : relecture STEP de chaque corps (`ReadStream`) + test de séparation `shapes_separated` + écriture STEP combiné (`WriteStream`) + relecture STEP combiné (`ReadStream`) + `BRepCheck_Analyzer`.
  4. À l'ouverture de `Résultat STEP` (`result_faces`) : nouvelle relecture STEP (`ReadStream`) + nouvelle passe `BRepMesh_IncrementalMesh` à `0,01 mm`.
- **Solution proposée** :
  Lors de `write_and_verify`, si l'audit de fidélité est requis (modes analytiques), produire la tessellation d'audit directement sur le `TopoDS_Shape` réimporté avant la fin de l'appel FFI, ce qui économise une relecture `STEPControl_Reader::ReadStream` complète par corps.
- **Risque de régression** : Faible.

---

#### `[AUD-006]` — Reconstruction et validation OCCT répétées pour des géométries identiques dans les documents 3MF multi-instances *(Confirmé)*
- **Sévérité** : **Moyenne** (Performance sur assemblages 3MF)
- **Catégorie** : Performance (`fabstep-import` / `fabstep-app`)
- **Statut** : **Inchangé depuis `v1.0.0`**.
- **Localisation** : `crates/fabstep-import/src/lib.rs:285-355`, `crates/fabstep-app/src/document.rs:500-713`.
- **Description précise** :
  Lorsqu'un fichier 3MF instancie $N$ fois le même `object_id` avec des transformations rigides (ou miroirs), chaque instance est dupliquée en un `PreviewBody` indépendant et traverse séparément `analyze_mesh_bodies_with_provenance`, `validate_source`, `recognize_surfaces`, `build_*_step`, `tessellate_step` et `compare_meshes`.
- **Solution proposée** :
  Conserver la validation de séparation spatiale sur toutes les instances placées, mais factoriser ou mettre en cache la validation BRep source pour les instances partageant le même `(object_id, instance_path_local, déterminant_signe, échelle)` à une transformation euclidienne rigide près.
- **Risque de régression** : Modéré (doit préserver la provenance exacte par instance `FABSTEP_SOURCE_FACE_V1_` introduite en I1c).

---

#### `[AUD-007]` — Couverture des tests automatisés UI limitée aux modules TypeScript purs ; aucun test de composant `.svelte` *(Confirmé — Étendu post-v1)*
- **Sévérité** : **Moyenne** (Qualité / Tests frontaux)
- **Catégorie** : Tests & Intégration continue
- **Statut** : **Confirmé**. La suite TS est passée de 9 à **36 tests** (incluant désormais les vrais Web Workers `selection.worker.ts` et `step-selection.worker.ts`), mais aucun composant `.svelte` n'est testé automatiquement.
- **Localisation** : `ui/tests/*.test.ts` (10 fichiers) vs `ui/src/App.svelte` et `ui/src/components/*.svelte` (9 composants : `Viewport.svelte`, `ProcessingIndicator.svelte`, `ParameterSearch.svelte`, `SourceInspection.svelte`, `StepInspection.svelte`, `StepSourceInspection.svelte`, `Dialog.svelte`, `Disclosure.svelte`, `Section.svelte`).
- **Description précise** :
  Toute la logique d'orchestration UI dans `App.svelte` (machine à états à 376 lignes gérant simultanément la conversion, le debounce 250 ms, l'undo/redo, les 2 Web Workers, le basculement `Source` / `Résultat STEP` / `Zone source`, la session guidée `mutateGuidance` et la recherche de paramètres `searchParameters`) repose uniquement sur `svelte-check` et sur la revue visuelle manuelle documentée dans `docs/VALIDATION.md`.
- **Solution proposée** :
  Extraire le contrôleur d'état réactif de `App.svelte` dans un module testable ou ajouter quelques tests de composants (`@testing-library/svelte` ou Playwright) avec un mock de `window.__TAURI_INTERNALS__`.
- **Risque de régression** : Nul.

---

#### `[AUD-008]` — Dépendance `fabstep-cad` liée à `/opt/homebrew/opt/opencascade` sans verrouillage de version OCCT ni CI déclarée dans le dépôt *(Confirmé)*
- **Sévérité** : **Moyenne** (Reproductibilité des builds / Chaîne d'approvisionnement)
- **Catégorie** : Builds, CI/CD & Portabilité
- **Statut** : **Inchangé depuis `v1.0.0`** (avec l'ajout de `TKG2d` dans `crates/fabstep-cad/build.rs:48`).
- **Localisation** : `crates/fabstep-cad/build.rs:1-54`, absence de `.github/workflows/`.
- **Description précise** :
  1. `build.rs` replie `OCCT_ROOT` sur `/opt/homebrew/opt/opencascade` sans vérifier `Standard_Version.hxx` et n'inclut pas de détection par défaut pour Linux (`/usr/include/opencascade`) ou Windows.
  2. Aucun workflow d'intégration continue (`.github/workflows/`) n'est présent dans le dépôt pour exécuter automatiquement les vérifications Rust/UI à chaque commit.
- **Solution proposée** :
  Vérifier `OCC_VERSION_COMPLETE` dans `build.rs` / `occt.cpp` et ajouter un workflow CI exécutant `cargo fmt --check`, `cargo clippy`, `cargo test` et `npm --prefix ui run test && npm --prefix ui run check && npm --prefix ui run build`.
- **Risque de régression** : Très faible.

---

### 3.4. Problèmes faibles et améliorations

#### `[AUD-020]` — Réinitialisation de la session d'indications cylindriques guidées lors de l'ajustement d'une tolérance dans le panneau Avancé *(Nouveau post-v1 — Confirmé)*
- **Sévérité** : **Faible** (Cohérence fonctionnelle / Ergonomie du mode guidé)
- **Catégorie** : UX/UI & Machine à états (`App.svelte` / `document.rs`)
- **Statut** : **Confirmé** par lecture de `ui/src/App.svelte:143-173, 239-252, 369` et `crates/fabstep-app/src/document.rs:193-200`.
- **Localisation** : `ui/src/App.svelte:239-252, 369`, `crates/fabstep-desktop/src/main.rs:50-67`, `crates/fabstep-app/src/document.rs:193-200`.
- **Description précise** :
  Dans `ui/src/App.svelte:369`, les champs `cylinder_fit_tolerance_mm` et `max_deviation_mm` du panneau *Avancé* sont affichés en mode `Planar` dès que `stepView || !!guidedAttempt` est vrai, ce qui suggère à l'utilisateur qu'il peut ajuster ces seuils pour faire aboutir une indication cylindrique refusée (`guidedAttempt.status === 'refused'`).
  Or, toute modification d'un champ numérique appelle `editNumber` -> `invalidate(true)` (`App.svelte:239-252`), ce qui exécute immédiatement `guidedAttempt = null; guidanceHints = [];` côté UI et `invalidate_result` côté Rust (`self.guidance_session = None`), effaçant toutes les indications cylindriques précédemment acceptées (jusqu'à 16 indications, lot I2b) et relançant une conversion `Planar` vierge.
- **Solution proposée** :
  Soit conserver les `guidanceHints` lors d'un changement de tolérance radiale / budget d'écart en rejouant la session guidée avec les nouvelles options (tout en revalidant intégralement chaque indication), soit afficher un avertissement explicite indiquant que modifier les tolérances réinitialise les indications manuelles en cours.
- **Risque de régression** : Modéré si l'on rejoue les indications (doit conserver l'invariant qu'aucune indication ancienne n'est acceptée sans re-preuve complète avec les nouvelles options).

---

#### `[AUD-021]` — Dépassement du seuil de 1 200 lignes sur 5 fichiers Rust/C++ et duplication des primitives vectorielles 3D dans 7 modules *(Nouveau post-v1 — Confirmé)*
- **Sévérité** : **Faible** (Maintenabilité / Lisibilité)
- **Catégorie** : Architecture & Qualité du code
- **Statut** : **Confirmé**.
- **Localisation** :
  - `crates/fabstep-cad/src/lib.rs` (**3 805 lignes**, dont 1 666 lignes de code et 2 139 lignes de tests)
  - `crates/fabstep-cad/src/occt.cpp` (**2 403 lignes**)
  - `crates/fabstep-core/src/rounded.rs` (**1 743 lignes**)
  - `crates/fabstep-core/src/partial.rs` (**1 355 lignes**)
  - `crates/fabstep-desktop/src/main.rs` (**1 229 lignes**)
  - Fonctions `dot`, `cross`, `sub`, `add`, `scale`, `len`, `norm` redéfinies dans `planar.rs`, `recognition.rs`, `mixed.rs`, `partial.rs`, `revolved.rs`, `rounded.rs`, `rounded_box.rs`.
- **Description précise** :
  La skill locale `skills/chef-orchestre/SKILL.md` (ligne 59) prescrit : *« Éviter les fichiers fourre-tout (> 800–1200 lignes) »*. Si l'extraction de `rounded/zone.rs` (318 lignes) et `partial/short.rs` (226 lignes) va dans le bon sens, 5 fichiers dépassent désormais 1 200 lignes (et jusqu'à 3 805 lignes pour `fabstep-cad/src/lib.rs`).
- **Solution proposée** :
  1. Mutualiser les opérations vectorielles 3D élémentaires dans un module interne `crates/fabstep-core/src/vec3.rs`.
  2. Déplacer le module `#[cfg(test)] mod tests` de `crates/fabstep-cad/src/lib.rs` (lignes 1668–3805) dans un fichier dédié `crates/fabstep-cad/src/tests.rs` ou `crates/fabstep-cad/tests/`.
  3. Découper `occt.cpp` en unités logiques (`occt_provenance.cpp`, `occt_builders.cpp`, `occt_inspect.cpp`).
- **Risque de régression** : Nul.

---

#### `[AUD-022]` — Désynchronisation partielle de `ARCHITECTURE.md`, `README.md` et `fabstep-cli` (`HELP`) vis-à-vis des lots post-v1 *(Nouveau post-v1 — Confirmé)*
- **Sévérité** : **Faible** (Documentation)
- **Catégorie** : Documentation technique
- **Statut** : **Confirmé**.
- **Localisation** :
  - `ARCHITECTURE.md:49-83`
  - `README.md:18-45`
  - `crates/fabstep-cli/src/main.rs:24-26` (`HELP`)
- **Description précise** :
  Alors que `STATUS.md`, `ROADMAP.md`, `docs/VALIDATION.md` et `docs/adr/0013..0023` sont parfaitement à jour :
  1. `ARCHITECTURE.md` ne mentionne ni `parameter_search.rs` dans `fabstep-app`, ni `partial/short.rs` et `rounded/zone.rs` dans `fabstep-core`, ni les 2 Web Workers (`selection.worker.ts`, `step-selection.worker.ts`) dans `ui`, ni les nouvelles commandes IPC (`selection_topology`, `result_faces`, `search_parameters`, `guided_convert`, `remove_guidance`, `recompute_guidance`) dans `fabstep-desktop`.
  2. `README.md` ne mentionne pas la recherche assistée de tolérance radiale (`ADR-0023`), les zones sphériques sans pôle (`ADR-0014`), les chaînes coniques alternées (`ADR-0015`) ni les arcs courts contextuels (`ADR-0022`).
  3. Le texte `HELP` de `crates/fabstep-cli/src/main.rs:24-26` ne mentionne pas les nouvelles familles supportées en modes `cylinders`, `primitives` et `surfaces`.
- **Solution proposée** :
  Mettre à jour la cartographie des composants dans `ARCHITECTURE.md`, le résumé des fonctionnalités dans `README.md` et la constante `HELP` dans `crates/fabstep-cli/src/main.rs`.
- **Risque de régression** : Nul.

---

#### `[AUD-009]` — Poids du bundle JS frontal (`1 519 kB` minifié) dû à l'import statique de `three` dans le chunk initial *(Confirmé)*
- **Sévérité** : **Faible** (Performance de démarrage WebView)
- **Localisation** : `ui/src/components/Viewport.svelte:4-6`, sortie `vite build` (`dist/assets/index-BnrMQjSb.js` : `1 519,41 kB` minifié / `236,98 kB` gzip ; Web Workers compilés séparément à `3,61 kB` et `5,68 kB`).
- **Solution proposée** : Isoler `three` dans un chunk dédié via `build.rolldownOptions.output.manualChunks` ou `codeSplitting` dans `ui/vite.config.ts`.
- **Risque de régression** : Nul.

---

#### `[AUD-010]` — Validation `fs::canonicalize` dans `fabstep-cli` silencieusement inopérante si le chemin de sortie n'existe pas encore mais pointe vers l'entrée via un lien symbolique parent *(Confirmé)*
- **Sévérité** : **Faible** (Robustesse CLI)
- **Localisation** : `crates/fabstep-cli/src/main.rs:148-150`.
- **Solution proposée** : Canonicaliser le dossier parent de `output` combiné avec `output.file_name()` afin de détecter toute collision même à travers un lien symbolique de répertoire.
- **Risque de régression** : Très faible.

---

#### `[AUD-011]` — Troncature de `product_name` caractère par caractère sans normalisation NFC préalable *(Confirmé)*
- **Sévérité** : **Faible** (Qualité du code)
- **Localisation** : `crates/fabstep-app/src/document.rs:901-916`.
- **Solution proposée** : Éviter de couper entre une lettre de base et un accent combinant Unicode (`NFD` macOS) lors de la troncature à 60 octets.
- **Risque de régression** : Très faible.

---

#### `[AUD-012]` — Clés de traduction mortes dans `ui/src/lib/locales/fr.ts` et `en.ts` *(Confirmé)*
- **Sévérité** : **Faible** (Maintenabilité UI)
- **Localisation** : `ui/src/lib/locales/fr.ts:14-60`, `ui/src/lib/locales/en.ts:16-62`, ainsi que la clé `inspection.pendingOptions` (`fr.ts:11`) et l'entrée `guidance.complete_cylinder_verified` (`App.svelte:184`, alors que le backend émet uniquement `guidance.cylinder_applied`).
- **Solution proposée** : Nettoyer les sections inutilisées (`file`, `export`, `status`, `alerts`, `help`) ou documenter explicitement leur conservation au titre du design system commun FabApps, et supprimer le doublon `guidance.complete_cylinder_verified`.
- **Risque de régression** : Nul (vérifié par `svelte-check`).

---

#### `[AUD-013]` — Densité extrême et formatage mono-ligne de `ui/src/App.svelte` (376 lignes / 65,1 Ko) *(Confirmé — Amplifié post-v1)*
- **Sévérité** : **Faible** (Maintenabilité / Revue de code)
- **Localisation** : `ui/src/App.svelte:1-376` (passé de 38 Ko à **65,1 Ko**, avec des lignes uniques dépassant 7 500 caractères aux lignes 369–371 et 375).
- **Description précise** :
  Si les nouveaux panneaux d'inspection et de recherche ont bien été extraits dans des composants dédiés (`ParameterSearch.svelte`, `SourceInspection.svelte`, `StepInspection.svelte`, `StepSourceInspection.svelte`), `App.svelte` concentre désormais toute la coordination de 4 fonctionnalités majeures et son template HTML/CSS est intégralement compacté sur 9 lignes géantes (lignes 367–375).
- **Solution proposée** :
  Formater `ui/src/App.svelte` multi-lignes et extraire la logique d'état (conversion + workers + session guidée + recherche) dans un module `workspace.svelte.ts`.
- **Risque de régression** : Nul.

---

#### `[AUD-014]` — Permissions explicites `capabilities` absentes dans `crates/fabstep-desktop` *(Confirmé)*
- **Sévérité** : **Faible** (Durcissement de sécurité Tauri v2)
- **Localisation** : `crates/fabstep-desktop/tauri.conf.json:1-34`.
- **Solution proposée** : Ajouter un fichier `crates/fabstep-desktop/capabilities/main.json` déclarant explicitement les seules commandes internes autorisées pour la fenêtre `main`.
- **Risque de régression** : Faible.

---

#### `[AUD-015]` — Codes internes de jalons et de lots (`M2b`..`M6b`, `C1a`, `I1`..`I2b`, `UX1`..`UX3`) sans table de correspondance dans `ARCHITECTURE.md` *(Confirmé)*
- **Sévérité** : **Faible** (Documentation / Onboarding)
- **Localisation** : `ARCHITECTURE.md`, `ui/src/lib/locales/fr.ts`, `fixtures/`.
- **Solution proposée** : Ajouter un tableau récapitulatif des jalons (`M0` à `M6b`) et des lots post-v1 (`Lot A`, `Lot B`, `C1a`, `I1`–`I2b`, `UX1`–`UX3`) avec renvoi vers les ADR `0001` à `0023` dans `ARCHITECTURE.md`.
- **Risque de régression** : Nul.

---

## 4. Optimisations recommandées (Analyse de performance)

Conformément à la méthodologie d'audit, les opportunités de performance sont classées en trois catégories distinctes :

### 4.1. Problèmes de performance mesurés (Lors de l'exécution de la suite de tests)
1. **Coût cumulé des conversions et validations OCCT sur les suites multi-essais (`guided.rs` : `58,5 s` pour 6 tests ; `conical_chains.rs` : `52,7 s` pour 3 tests ; `complete_tori.rs` : `50,5 s` pour 3 tests)** :
   - **Mesure observée** : L'exécution des tests d'intégration `fabstep-app` montre que les opérations impliquant des maillages curvilignes denses ou des re-conversions successives (`guided.rs` exécute plusieurs cycles `convert_document_with_provenance` + `guided_cylinder` + `result_faces`) prennent entre **8 et 18 secondes par test** en profil `dev`.
   - **Cause directe** : Combinaison de `validate_source` (couture OCCT de tous les triangles source), de `result_faces` appelé une seconde fois dans `guided_cylinder` (`[AUD-019]`), et des passes multiples de lecture/tessellation STEP (`[AUD-005]`).

### 4.2. Problèmes de performance appuyés par une analyse algorithmique
1. **Factorisation des étapes invariantes dans `search_parameters` (`[AUD-018]`)** :
   - **Analyse** : `search_parameters` exécute jusqu'à 7 essais où seule `cylinder_fit_tolerance_mm` varie. Factoriser `analyze_document_with_provenance`, `validate_sources_separated` et `validate_source(&triangle_model(&analysis.mesh))` hors de la boucle des 7 essais réduit de **$7\times$ à $1\times$** le coût de lecture/soudage/segmentation planaire et surtout de couture BRep OCCT du maillage source.
2. **Suppression de la tessellation OCCT redondante dans `DocumentStep::guided_cylinder` (`[AUD-019]`)** :
   - **Analyse** : `guided_cylinder` appelle `self.result_faces()` qui exécute `BRepMesh_IncrementalMesh(shape, 0.01)` sur tout le solide STEP uniquement pour obtenir la table de correspondance `face_id -> ResultFaceSource`, alors que le maillage tessellé est immédiatement jeté.
3. **Suppression de la double validation topologique dans `analyze_bodies_with_provenance` et des recalculs dans `recognize_surfaces` (`[AUD-004]`)** :
   - **Analyse** : `source_components` (`pipeline.rs:181`) et `analyze_raw_with_provenance` (`pipeline.rs:239`) exécutent chacun `validate_topology_impl` sur chaque composante, puis `mixed`, `partial`, `revolved` et `rounded` recalculent `validate_topology` et `planar::build_with_provenance`.
4. **Annulation coopérative inter-étapes et inter-corps sous `EngineState` (`[AUD-001]`)** :
   - **Analyse** : Permet d'interrompre un essai de `search_parameters` ou un `convert_document` obsolète entre deux étapes OCCT au lieu d'attendre la fin complète du traitement de tous les corps.

### 4.3. Optimisations seulement envisageables (À évaluer selon les besoins futurs)
1. **Déduplication de la validation source pour les instances 3MF congruentes (`[AUD-006]`)** : Utile uniquement sur les plateaux 3MF comportant de nombreuses copies d'un même objet.
2. **Séparation du chunk `three` dans Vite (`[AUD-009]`)** : Améliore le temps de parsing initial de la WebView sans impacter les calculs géométriques.

### 4.4. Protocole de benchmark reproductible proposé
```bash
# Compiler en mode release hors-ligne
cargo build --release --workspace --offline

# Benchmark 1 : Pièce mixte / arrondie (cascade complète Surfaces)
hyperfine --warmup 2 \
  "./target/release/fabstep convert fixtures/m5_toroidal_fillet.stl --mode surfaces -o /tmp/bench_m5.step --force"

# Benchmark 2 : Boîtier arrondi convexe (M6a, 12 cylindres + 8 sphères + 6 plans)
hyperfine --warmup 2 \
  "./target/release/fabstep convert fixtures/m6a_rounded_box.stl --mode surfaces -o /tmp/bench_m6a.step --force"

# Benchmark 3 : Chaîne conique alternée post-v1 (C1a)
hyperfine --warmup 2 \
  "./target/release/fabstep convert fixtures/c1a_stepped_conical_shaft.stl --mode primitives -o /tmp/bench_c1a.step --force"
```

---

## 5. Suggestions d'amélioration (Proportionnées à la maturité du projet)

1. **Corriger les deux points numériques/algorithmiques dans `crates/fabstep-core/src/partial.rs` (`[AUD-016]` et `[AUD-017]`)** :
   - Utiliser `recognition::fit_circle` dans `closed_circle` (`partial.rs:163`) au lieu de `constrained_circle`.
   - Vérifier la distance 3D euclidienne des 4 coins analytiques dans `partial::prove` (`partial.rs:697-701`) pour garantir qu'aucun cylindre partiel accepté par `fabstep-core` ne soit rejeté par `occt.cpp:659`.
2. **Optimiser `search_parameters` et `guided_cylinder` (`[AUD-018]` et `[AUD-019]`)** :
   - Factoriser `analyze_document_with_provenance` et `validate_source` une seule fois avant les 7 essais de `search_parameters`.
   - Éviter de relancer `BRepMesh_IncrementalMesh` dans `DocumentStep::guided_cylinder`.
3. **Découper les fichiers dépassant 1 200 lignes (`[AUD-021]` et `[AUD-013]`)** :
   - Déplacer les 2 139 lignes de tests unitaires de `crates/fabstep-cad/src/lib.rs` dans un fichier de test dédié, extraire `vec3.rs` dans `fabstep-core`, et formater `ui/src/App.svelte`.

---

## 6. Plan d'action priorisé

### Phase A — Corrections rapides et sûres (Quick Wins, sans risque architectural)
1. **`[AUD-016]`** : Aligner la vérification d'extrémités de `partial.rs:697-701` sur la distance euclidienne 3D utilisée dans `occt.cpp:659`.
2. **`[AUD-017]`** : Remplacer `constrained_circle` par `recognition::fit_circle` dans `partial::closed_circle` (`partial.rs:163`).
3. **`[AUD-003]`** : Sécuriser `scripts/package-source.py` via `git ls-files` / `git archive`.
4. **`[AUD-022]` & `[AUD-015]`** : Mettre à jour `ARCHITECTURE.md`, `README.md` et `HELP` dans `fabstep-cli` avec les nouveaux modules et lots post-v1.

### Phase B — Fiabilisation et performance à fort impact
1. **`[AUD-018]`** : Factoriser l'analyse source et `validate_source` / `validate_sources_separated` hors de la boucle des 7 essais de `search_parameters`.
2. **`[AUD-019]`** : Mettre en cache ou extraire sans tessellation la correspondance `source_correspondence` dans `DocumentStep` pour accélérer `guided_cylinder`.
3. **`[AUD-004]`** : Supprimer le double appel à `validate_topology_impl` dans `analyze_bodies_with_provenance` (`pipeline.rs:181, 239`) et partager `ValidatedTopology` + `planar_triangle_faces` dans `Analysis`.
4. **`[AUD-001]`** : Propager un jeton d'annulation coopérative dans `convert_analyzed_document` et `convert_analysis_with_recognized`.

### Phase C — Qualité, CI et maintenabilité
1. **`[AUD-008]`** : Ajouter un workflow CI et vérifier la version d'OCCT dans `build.rs`.
2. **`[AUD-021]` & `[AUD-013]`** : Scinder `crates/fabstep-cad/src/lib.rs` (code vs tests), extraire `vec3.rs` dans `fabstep-core` et formater `ui/src/App.svelte`.
3. **`[AUD-007]` & `[AUD-020]`** : Clarifier le comportement des tolérances pendant une session guidée et ajouter des tests automatisés de l'orchestration UI.

### Phase D — Améliorations futures (Optionnelles)
1. **`[AUD-005]` & `[AUD-006]`** : Mutualiser la passe de tessellation d'audit dans `write_and_verify` et dédupliquer la validation des instances 3MF identiques.
2. **`[AUD-009]` & `[AUD-014]`** : Code-splitting de `three` dans Vite et déclaration explicite des `capabilities` Tauri v2.

---

## 7. Limites de l'audit et contrôles effectivement exécutés

### 7.1. Contrôles automatisés réellement exécutés pendant cet audit (10 novembre 2026)
- `cargo fmt --all -- --check` : **SUCCÈS** (code 0).
- `cargo clippy --workspace --all-targets --offline -- -D warnings` : **SUCCÈS** (code 0, 0 avertissement).
- `cargo clippy --manifest-path crates/fabstep-desktop/Cargo.toml --all-targets --offline -- -D warnings` : **SUCCÈS** (code 0, 0 avertissement).
- `npm --prefix ui run test` : **SUCCÈS** (**36/36 tests passés** en 208 ms, incluant l'exécution réelle des deux Web Workers `selection.worker.ts` et `step-selection.worker.ts`).
- `npm --prefix ui run check` (`svelte-check`) : **SUCCÈS** (`0 errors and 0 warnings`).
- `npm --prefix ui run build` (`vite build`) : **SUCCÈS** (build de production en 1,00 s ; workers émis séparément à `3,61 kB` et `5,68 kB` ; avertissement de taille de chunk sur `index.js` à `1 519,41 kB`).
- `cargo test --manifest-path crates/fabstep-desktop/Cargo.toml --offline` : **SUCCÈS** (**16/16 tests passés** en 4,41 s).
- `cargo test --workspace --offline` : **SUCCÈS** (**288/288 tests passés** sans échec : `fabstep-app` 82 tests, `fabstep-cad` 55 tests, `fabstep-cli` 12 tests, `fabstep-core` 121 tests, `fabstep-import` 11 tests, `fabstep-verify` 7 tests).
- **Total général exécuté** : **340 tests automatisés passés (304 Rust + 36 TypeScript/Web Workers), 0 échec, 0 warning `clippy`, 0 erreur `svelte-check`**.

### 7.2. Ce qui n'a pas pu être vérifié
- **Interaction manuelle avec la fenêtre native Tauri macOS** : Conformément aux règles de l'audit, seules les suites automatisées ont été lancées ; les essais visuels interactifs dans la fenêtre native restent documentés dans `docs/VALIDATION.md`.
- **Compilation et exécution sur Windows et Linux** : Audit réalisé exclusivement sur macOS arm64 avec OCCT 7.9.3.

---

## Top 10 des actions à plus fort impact

1. **`[AUD-001]`** — Ajouter une annulation coopérative inter-étapes et inter-corps dans `convert_analyzed_document` et `convert_analysis_with_recognized` pour libérer rapidement `occt_mutex()` lorsqu'une requête ou un essai `search_parameters` devient obsolète.
2. **`[AUD-018]`** — Factoriser `analyze_document_with_provenance`, `validate_sources_separated` et `validate_source` hors de la boucle des 7 essais de `search_parameters` (`crates/fabstep-app/src/parameter_search.rs`).
3. **`[AUD-016]`** — Vérifier la distance euclidienne 3D des 4 coins analytiques dans `crates/fabstep-core/src/partial.rs:697-701` pour éviter tout rejet FFI `cad.invalid_model` dans `occt.cpp:659`.
4. **`[AUD-017]`** — Utiliser `recognition::fit_circle` au lieu de `constrained_circle` dans `partial::closed_circle` (`crates/fabstep-core/src/partial.rs:163`).
5. **`[AUD-019]`** — Éviter de relancer `self.result_faces()` (et sa tessellation OCCT à `0,01 mm`) à chaque appel à `DocumentStep::guided_cylinder` (`crates/fabstep-app/src/document.rs:263`).
6. **`[AUD-004]`** — Éliminer la double exécution de `validate_topology_impl` dans `analyze_bodies_with_provenance` (`pipeline.rs:181, 239`) et partager `ValidatedTopology` + `planar_triangle_faces` dans `Analysis`.
7. **`[AUD-003]`** — Remplacer `os.walk` par `git ls-files` / `git archive` dans `scripts/package-source.py` pour empêcher l'inclusion de fichiers locaux non suivis dans l'archive source.
8. **`[AUD-008]`** — Ajouter un workflow CI versionné (`.github/workflows/ci.yml`) et vérifier la version d'OCCT dans `crates/fabstep-cad/build.rs`.
9. **`[AUD-021]` & `[AUD-013]`** — Scinder les fichiers dépassant 1 200 lignes (`crates/fabstep-cad/src/lib.rs`, `occt.cpp`, `rounded.rs`, `partial.rs`), mutualiser `vec3.rs` et formater `ui/src/App.svelte`.
10. **`[AUD-020]` & `[AUD-022]`** — Protéger ou rejouer la session d'indications guidées lors d'un ajustement de tolérance et synchroniser `ARCHITECTURE.md`, `README.md` et `fabstep --help` avec les lots post-v1.
