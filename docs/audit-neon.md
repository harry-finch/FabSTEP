## 0 — Cartographie initiale (Phase A)

  • État Git : branche fabboard/geometrie, HEAD = 2e51866 (« R-085 Livrer les jalons géométriques et consigner le délai restant sur
  Flansch »). Modifications locales préexistantes conservées intactes (M .agents/skills-fabboard.json, ?? .agents/skills/fab-cleanup/, ??
  .claude/skills/fab-cleanup).
  • Stack & tailles (~87 385 lignes de code hors dépendances et artefacts) :
      • Workspace Rust (edition = 2021, rust-version = 1.88, Cargo.toml) :
          • fabstep-core : 99 fichiers, 39 554 lignes — géométrie pure, déterministe, zéro IO/OCCT (serde uniquement).
          • fabstep-app : 78 fichiers, 17 082 lignes — pipeline documentaire, provenance, recherche de paramètres, guidage.
          • fabstep-cad : 20 fichiers, 16 233 lignes — ABI C/C++17 + noyau OpenCASCADE 7.9.x (crates/fabstep-cad/src/occt.cpp 4 677 l. +
          15 .hxx, crates/fabstep-cad/src/lib.rs 7 499 l.).
          • fabstep-import : 5 fichiers, 1 573 lignes — 3MF Core/OPC (zip =9.4.1, roxmltree =0.21.1, flate2).
          • fabstep-verify : 4 fichiers, 1 271 lignes — audit de distance bidirectionnel par BVH.
          • fabstep-cli : 6 fichiers, 927 lignes — commandes check / convert.
      • Crates hors workspace :
          • fabstep-desktop : 6 fichiers, 4 403 lignes — Tauri 2, état/révisions, sections DXF, superviseur gros STL
          (crates/fabstep-desktop/src/main.rs 1 809 l., crates/fabstep-desktop/src/supervisor.rs 1 137 l.).
          • fabstep-worker : 1 fichier, 610 lignes — helper isolé compilé seul avec large-stl (jusqu’à 600 000 triangles / 1 800 000
          sommets, ADR-0034).
      • Frontend (ui/package.json) : Svelte 5.57, TypeScript 6.0, Vite 8.3, Three.js 0.186, 4 Web Workers, 5 732 lignes (ui/src/App.svelte
      1 565 l., ui/src/components/Viewport.svelte 347 l., ui/src/lib/workflow.ts 398 l.).
  • Cible & CI : macOS 26 Apple Silicon (aarch64-apple-darwin), CI GitHub Actions (.github/workflows/check.yml), scripts scripts/check.sh,
  scripts/build-macos-dev.sh, scripts/package-macos.py, scripts/package-source.py.
  • Composants critiques : parsing 3MF/STL non fiable, pipeline topologique & reconnaisseurs fabstep-core, frontière FFI + contrôle
  d’auto-intersections OCCT (torus_atlas_interference.hxx), audit BVH fabstep-verify, superviseur de processus & IPC fabstep-
  desktop/fabstep-worker, rendu/raycasting WebGL Viewport.svelte.
  ──────
  ## 1 — Synthèse exécutive
  FabSTEP présente un niveau d’ingénierie et de rigueur géométrique remarquable pour un outil CAO/maillage de cette envergure. Les
  invariants fondamentaux sont respectés avec constance : déterminisme total dans fabstep-core (zéro HashMap/HashSet, comparaisons
  f64::total_cmp, budgets explicites), frontière FFI C++/OCCT proprement encapsulée (noexcept + with_occt + Drop systématique), parsing
  3MF/ZIP/XML durci contre les bombes de décompression et les traversées OPC, machine à états UI/bureau protégée par révisions monotones
  et gardes d’export atomiques, et une suite automatisée très riche (707 tests workspace, 50 bureau, 8 worker, 238 UI, 0 avertissement
  Clippy/Svelte).
  En revanche, l’expansion rapide des familles géométriques post-v1 (R-027 à R-111) et la montée en charge vers les gros maillages
  (jusqu’à 600 000 triangles, cas Flansch) ont révélé d’importants goulots d’étranglement de performance et d’empreinte mémoire sur toute
  la chaîne :
  1. Côté C++/OCCT (fabstep-cad) — cause confirmée du timeout de 30 min sur Flansch : la vérification d’interférence par groupes de
  feuilles (torus_atlas_interference.hxx) regroupe deux feuilles dans un unique TopoDS_Compound passé à BRepAlgoAPI_Check(group, false,
  true), ce qui multiplie par 10–50× le coût des auto-intersections face/face (déjà 174 s pour un seul passage) et retestera toutes les
  paires intra-feuilles ; de plus, ce contrôle complet est exécuté deux fois dans fabstep_build_torus_atlas (avant et après STEP),
  atlas_fan::repair lance IntTools_FaceFace sans filtrage sur 14 882 faces (174 s pour 6 faces à réparer), et BRepAdaptor_Surface(face)
  est appelé en O(E²) avec Restriction = true.
  2. Côté Rust pur (fabstep-core & fabstep-verify) : amplification mémoire et CPU massive sur les gros maillages due aux BTreeMap<Edge,
  Vec<...>> / Vec<Vec<usize>> (millions de petites allocations), à 5–6 passages redondants de validate_topology + segmentation planaire en
  mode Auto, au clonage inconditionnel de tout le MixedModel avant partial::augment_auto, aux 43,7 M de recherches BTreeMap dans
  weld_controlled, et à des calculs redondants dans fabstep-verify (sort_by O(N log² N) et deux hypot() par test de triangle).
  3. Côté IPC / Packaging / UI : fabstep-worker sérialise le maillage complet (Analysis.mesh + model) dans report.json (risque de dépasser
  128 Mio + expansion DOM serde_json::Value dans le bureau), package-macos.py omet encore fabstep-worker du bundle release, le mutex
  global EngineState.inner est maintenu pendant les écritures disque/fsync, et l’UI souffre d’un raycasting Three.js sans BVH (jusqu’à 64
  raycasts/frame en peinture de faces), de proxies profonds Svelte 5 $state sur des tableaux de 100k triangles, et d’un build Vite ralenti
  par l’import barrel de @lucide/svelte (1 920 icônes compilées).
  Appréciation de maturité : Très solide sur la fiabilité géométrique et la sûreté de la v1, mais en phase de croissance rapide post-v1 où
  la consolidation des performances (C++/Rust/UI), la déduplication de certains gros modules et la mise à jour du packaging release
  deviennent prioritaires avant d’élargir davantage le périmètre géométrique.
  ──────
  ## 2 — Tableau de bord

   Domaine               │ Évaluation  │ Risque │ Commentaire
  ───────────────────────┼─────────────┼────────┼─────────────────────────────────────────────────────────────────────────────────────────
   Documentation         │ Bon         │ Faible │ Très complète (44+ ADR, VALIDATION.md, ARCHITECTURE.md, INT_AUDIT_01.md), mais
                         │             │        │ README.md accumule les ajouts post-v1 de façon peu structurée, STATUS.md mentionne une
                         │             │        │ ancienne branche (fabboard/mise-d-equerre) et docs/FEATURES.md reste figé en v1.0.0.
   Architecture          │ Bon         │ Moyen  │ Frontières entre crates exemplaires (core pur, cad seul à toucher OCCT, worker isolé
                         │             │        │ pour large-stl). En revanche, plusieurs fichiers deviennent monolithiques (fabstep-
                         │             │        │ cad/src/lib.rs 7 499 l., occt.cpp + .hxx ~9 500 l. en 1 TU, rounded.rs 3 345 l.,
                         │             │        │ App.svelte 1 565 l.).
   Qualité du code       │ Bon         │ Moyen  │ Déterminisme irréprochable, Clippy strict -D warnings et svelte-check à 0 erreur.
                         │             │        │ Duplication notable (Frame dupliqué 7×, construction de carte d’arêtes 6×) et quelques
                         │             │        │ fonctions géantes (693 l. dans rounded.rs).
   Sécurité              │ Bon         │ Faible │ Parsing 3MF/ZIP/XML très bien durci, FFI C++ noexcept + bornes vérifiées, CSP Tauri
                         │             │        │ stricte sans sortie réseau. Petits durcissements de défense en profondeur possibles
                         │             │        │ dans supervisor.rs et tauri capabilities (déjà suivi en R-023).
   Intégrité des données │ Excellent   │ Faible │ Réimport STEP systématique, écritures atomiques (NamedTempFile + sync_all + persist),
                         │             │        │ révisions monotones et invalidation stricte. Deux failles mineures : troncature
                         │             │        │ silencieuse > 64 Mio dans fabstep-cli et écriture non atomique dans package-source.py.
   Performances          │ Perfectible │ Élevé  │ Excellent respect des budgets sur la v1, mais goulots critiques confirmés sur les
                         │             │        │ maillages denses (timeout 30 min sur Flansch, pic RAM élevé, surcoût IPC report.json,
                         │             │        │ raycasting O(N) × 64/frame et proxies $state côté UI).
   Tests                 │ Excellent   │ Faible │ 707 tests workspace, 50 bureau, 8 worker, 238 UI, 23 Python (12 exécutés) tous au vert.
                         │             │        │ Lacunes : pas de tests de composants Svelte montés, tests large-stl et
                         │             │        │ real_worker_handoff non exécutés dans cet audit, et 88 items roadmap encore en statut «
                         │             │        │ À tester » côté validation manuelle/Fusion.
   Builds / CI/CD        │ Perfectible │ Moyen  │ scripts/check.sh et check.yml sont rigoureux, mais package-macos.py omet fabstep-
                         │             │        │ worker, check.yml omet cargo fetch pour fabstep-worker, occt.cpp recompile tout en un
                         │             │        │ seul TU, et le build Vite compile 1 920 icônes Lucide.
   UX/UI                 │ Bon         │ Moyen  │ Ergonomie d’atelier cohérente, jetons CSS respectés à 100 %, parité FR/EN vérifiée à la
                         │             │        │ compilation. Bugs/frictions confirmés : Escape en peinture de faces efface la
                         │             │        │ sélection, champs de tolérance invalidant le STEP à chaque frappe, focus perdu à la
                         │             │        │ fermeture de Dialog.
   Maintenabilité        │ Perfectible │ Moyen  │ Excellente traçabilité ADR/roadmap, mais la dette de taille des fichiers clés (lib.rs,
                         │             │        │ occt.cpp, rounded.rs, App.svelte) et l’accumulation de composants UI inutilisés
                         │             │        │ freinent l’évolution sereine.
  ──────
  ## 3 — Constats détaillés
  ### Performance & Optimisation (C++/OCCT, Rust, IPC, Frontend)
  #### [AUD-001] Explosion quadratique et travail répété dans atlas_interference::Checker (cause principale du timeout 30 min sur Flansch)
  • Domaine : Performance / fabstep-cad
  • Priorité : P0
  • Statut : Confirmé
  • Localisation : crates/fabstep-cad/src/torus_atlas_interference.hxx:36-124
  • Preuves :
      • L. 88–121 : pour chaque paire de feuilles chevauchantes (x, y) (x != y, soit 6 949 groupes sur Flansch), les faces de x et de y
      sont ajoutées dans un unique TopoDS_Compound group puis passées à BRepAlgoAPI_Check check(group, false, true);.
      • Dans OCCT, BRepAlgoAPI_Check(group, false, true) (constructeur mono-forme) lance BOPAlgo_ArgumentAnalyzer en mode auto-
      intersection sur toutes les faces de group : chaque face f subit de nouveau IntTools_FaceFace(f, f) (coût mesuré en V-1254 : 174 s
      pour un seul passage sur les 14 882 faces réglées), et toutes les paires intra-feuilles (a_i, a_j) et (b_i, b_j) déjà testées dans
      inspect(x, x) et inspect(y, y) sont retestées dans chaque groupe croisé. Les 43 tores ayant de grandes boîtes englobantes 3D creuses
      apparaissent dans des centaines de groupes. De plus, RunParallel reste à false.
  • Impact concret : Sur Flansch (46 671 faces), le premier passage d’interférence atteint 23 minutes au groupe 740/6 949 et le helper
  expire à 30 minutes (worker.timeout, V-1256 à V-1258).
  • Recommandation :
      1. Exécuter l’auto-intersection individuelle (f contre f) une seule fois par face (en excluant les faces planes à contour valide qui
      ne peuvent s’auto-intersecter, et en évitant de répéter les faces déjà vérifiées).
      2. Pour x != y, utiliser la vérification bi-forme BRepAlgoAPI_Check(groupA, groupB, BOPAlgo_UNKNOWN, false, true) (ou
      BOPAlgo_ArgumentAnalyzer avec SelfInterMode = false et test croisé Shape1 vs Shape2 uniquement) afin de ne jamais retester (f, f),
      (a_i, a_j) ni (b_i, b_j).
      3. Ajouter un pré-filtre géométrique conservateur spécifique aux faces toriques (enveloppe annulaire [R - r - tol, R + r + tol] +
      tranche axiale [z_min, z_max] ou sous-boîtes UV) pour écarter les faces situées dans le vide central du tore.
      4. Évaluer SetRunParallel(true) sur BOPAlgo_ArgumentAnalyzer (ou parallélisation contrôlée des paires de feuilles avec
      IntTools_Context dédié par thread).
  • Effort estimé : Moyen à Élevé
  • Risque de régression : Moyen — ne doit manquer aucune auto-intersection arête/face ou face/face réelle.
  • Méthode de validation après correction : Exécuter tous les témoins négatifs de scripts/qualify-atlas-interference.sh (V-1251), puis
  chronométrer sur la rampe publique (194 694 triangles) et sur Flansch sous les plafonds ADR-0034.
  #### [AUD-002] Exécution en double du contrôle complet d’interférence dans fabstep_build_torus_atlas
  • Domaine : Performance / fabstep-cad
  • Priorité : P1
  • Statut : Confirmé
  • Localisation : crates/fabstep-cad/src/occt.cpp:4087, crates/fabstep-cad/src/occt.cpp:4124
  • Preuves : const auto before = measure_solid(solid, stage); (L. 4087) exécute BRepCheck_Analyzer + atlas_interference::Checker{}.
  run(faces) + BRepGProp + BRepBndLib avant l’écriture STEP ; puis const auto after = measure_solid(reread, stage); (L. 4124) réexécute
  l’intégralité de measure_solid après réimport STEP.
  • Impact concret : Double exactement le temps de validation complète (facteur 2× immédiat sur Flansch et sur la rampe publique).
  • Recommandation : Avant l’écriture STEP (before), conserver uniquement la topologie/incidences, BRepCheck_Analyzer et les métriques
  (volume, boîte, comptes de faces) nécessaires à same_measurement et atlas_transfer::verify, et exécuter atlas_interference::Checker
  uniquement sur le solide réimporté reread (qui est la référence de l’invariant d’export).
  • Effort estimé : Faible
  • Risque de régression : Faible — le solide exporté reste intégralement soumis au contrôle d’interférence après réimport ; seul le
  moment de détection d’une auto-intersection passe après l’écriture/relecture STEP.
  • Méthode de validation après correction : Tests fabstep-cad (y compris témoins d’auto-intersection refusés) + benchmark avant/après sur
  la rampe publique.
  #### [AUD-003] atlas_fan::repair exécute IntTools_FaceFace sans filtrage sur 14 882 faces (~174 s sur Flansch)
  • Domaine : Performance / fabstep-cad
  • Priorité : P1
  • Statut : Confirmé
  • Localisation : crates/fabstep-cad/src/torus_atlas_build.hxx:211-218, crates/fabstep-cad/src/torus_atlas_fan.hxx:161-179,
  crates/fabstep-cad/src/torus_atlas_fan.hxx:285-294
  • Preuves : atlas_fan::repair(original) est appelé sur toutes les faces ; pour chacune des 14 882 faces éventails réglées,
  reports_apex_only lance IntTools_FaceFace self; self.Perform(face, face, false); (mesuré à 174 s en V-1254) pour ne réparer au final que
  6 faces quasi-planaires (hauteur apex ~1,17e-8 mm).
  • Impact concret : ~3 minutes perdues à chaque exécution sur Flansch, indépendamment même d’AUD-001.
  • Recommandation : Soit (option prudente, risque quasi nul) pré-filtrer reports_apex_only par un critère géométrique rapide sur la
  quasi-planéité de l’apex (ex. distance apex au plan de l’arc < 1e-4 mm), soit (option optimale) construire directement le carreau
  rationnel quadratique (2, 1) exact dans patch_faces pour les éventails éligibles, éliminant l’approximation ThruSections et les 14 882
  tests IntTools_FaceFace.
  • Effort estimé : Faible (pré-filtre) à Moyen (construction directe)
  • Risque de régression : Très faible pour le pré-filtre (les cas non filtrés restent de toute façon contrôlés par
  atlas_interference::Checker après couture) ; Moyen pour la construction directe (vérifier réimport FreeCAD/Fusion).
  • Méthode de validation après correction : Tests unitaires torus_atlas + V-1256/V-1257 (les 6 faces singulières de Flansch doivent
  rester réparées).
  #### [AUD-004] DocumentReport sérialise les maillages complets (Analysis.mesh et Analysis.model) dans report.json (worker) et sur l’IPC
  Tauri

  • Domaine : Performance / Fiabilité / fabstep-worker & fabstep-desktop
  • Priorité : P1
  • Statut : Confirmé
  • Localisation : crates/fabstep-core/src/lib.rs:144-148, crates/fabstep-worker/src/main.rs:404-409,
  crates/fabstep-desktop/src/supervisor.rs:538-540, crates/fabstep-desktop/src/supervisor.rs:618
  • Preuves : Analysis dérive Serialize sans #[serde(skip)] sur mesh: Mesh et model: PlanarModel. fabstep-worker écrit &converted.report
  dans report.json (plafonné à REPORT_LIMIT = 128 MiB), puis supervisor.rs:539 le désérialise en DOM générique serde_json::Value
  (expansion mémoire 3–5×) avant de le re-sérialiser vers WebKit.
  • Vérification croisée (Phase H) : Contrairement au mode ordinaire où ui/src/components/Viewport.svelte:260 et
  ui/src/components/Viewport.svelte:326 lisent result.analysis.mesh / model pour tracer les arêtes de régions en vue Source (et
  ui/src/lib/selection.ts:49 lit body.analysis.mesh.triangles.length), le mode gros STL désactive la sélection source et les arêtes
  denseRaw (!denseRaw à la ligne 254 de Viewport.svelte). Le maillage et le modèle planaire sérialisés dans report.json du worker sont
  donc entièrement inutilisés tout en risquant de déclencher worker.output_too_large (> 128 Mio) sur un STL de 500k–600k triangles !
  • Impact concret : Risque de refus worker.output_too_large sur gros maillages réussis, pics RAM de plusieurs centaines de Mio dans le
  processus bureau et latence JSON/IPC importante.
  • Recommandation : Introduire un DTO de rapport allégé pour fabstep-worker (omettant analysis.mesh, analysis.model et les vecteurs
  lourds non utilisés en mode dense, comme le fait déjà fabstep-cli en ne sérialisant que analysis.report) et désérialiser dans un type
  Rust dédié dans supervisor.rs au lieu de serde_json::Value. En mode ordinaire, remplacer body.analysis.mesh.triangles.length dans
  selection.ts par body.analysis.report.triangles et ne renvoyer la géométrie d’arêtes source qu’au besoin.
  • Effort estimé : Moyen
  • Risque de régression : Moyen — vérifier les tests supervisor.rs et ui/tests/ pour préserver les champs attendus par l’inspecteur.
  • Méthode de validation après correction : Mesurer la taille de report.json sur la rampe publique (194k triangles) avant/après ;
  exécuter cargo test --manifest-path crates/fabstep-desktop/Cargo.toml et real_worker_handoff.
  #### [AUD-005] fabstep-core : amplification mémoire et CPU (topologie BTreeMap<Edge, Vec> + 5–6 passages redondants + clonage complet en
  Auto)
  • Domaine : Performance / Mémoire / fabstep-core
  • Priorité : P1
  • Statut : Confirmé
  • Localisation :
      • crates/fabstep-core/src/pipeline.rs:504-714 (weld_controlled, validate_topology_impl_controlled)
      • crates/fabstep-core/src/automatic.rs:37-121, crates/fabstep-core/src/rounded.rs:358-374, crates/fabstep-core/src/rounded.rs:882,
      crates/fabstep-core/src/partial.rs:655-665
      • crates/fabstep-core/src/planar.rs:120-185, crates/fabstep-core/src/complete_torus.rs:56-68,
      crates/fabstep-core/src/complete_sphere.rs:49-65
  • Preuves :
      1. validate_topology_impl_controlled (L. 618–712) alloue un BTreeMap<(usize, usize), Vec<(usize, bool)>> (~810k Vec sur 540k
      triangles), adjacency: Vec<Vec<usize>> (540k Vec pour un degré fixe de 3), puis, sans libérer edges, alloue un BTreeMap + ~6 Vec +
      un BTreeSet par sommet (~2,16 M d’allocations tas sur 270k sommets).
      2. weld_controlled (L. 517–555) effectue 27 recherches BTreeMap<[i64; 3], Vec<usize>> par coin de triangle (43,7 millions de
      recherches O(log V) sur 540k triangles).
      3. Un appel recognize_automatic relance validate_topology(&mesh) jusqu’à 5–6 fois et planar::build_with_provenance jusqu’à 4 fois
      sur le même a.mesh immuable (dont complete_torus et complete_sphere avant même de vérifier la caractéristique d’Euler V - E + F en
      O(1)).
      4. rounded.rs:882 exécute let partial_backup = automatic.then(|| (model.revolved.mixed.clone(), partial_owners.clone())); avant même
      de savoir si partial::candidates() trouve le moindre candidat, maintenant 4 copies simultanées des faces planes et 3 cartes d’arêtes
      complètes en RAM.
      5. planar::segment (L. 120–185) alloue 8–10 structures tas (BTreeSet, BTreeMap, VecDeque, Vec) pour chaque face d’un seul triangle
      (~4–5 millions d’allocations sur un maillage courbe de 500k triangles).
  • Impact concret : Temps CPU de 50–100+ s dans fabstep-core sur les maillages denses et pic mémoire de plusieurs centaines de Mio à > 1
  Gio (contribuant au plafond de 4 Gio / empreinte physique observée en R-072/R-073).
  • Recommandation :
      1. Introduire un MeshTopology interne compact (table de demi-arêtes triée à plat Vec<(u32, u32, u32)> comme déjà prouvé dans
      torus_atlas.rs:141-171, adjacence Vec<[u32; 3]>, contrôle d’anneau de sommet sans allocation) calculé une fois et propagé dans
      automatic → rounded → partial → partial/chain.
      2. Ajouter un chemin rapide 1-triangle + réutilisation des buffers pending/region dans planar::segment.
      3. Supprimer le clone inconditionnel partial_backup en rounded.rs:882 (vérifier les candidats d’abord ou muter après preuve) et
      libérer segmentation avant partial::augment.
      4. Vérifier V - E + F en O(1) avant validate_topology dans complete_torus / complete_sphere.
      5. Remplacer BTreeMap<[i64; 3], Vec<usize>> dans weld_controlled par une table de hachage ouverte déterministe à graine fixe + liste
      chaînée par indices (head/next en Vec<u32>).
  • Effort estimé : Moyen à Élevé (réalisable par lots indépendants)
  • Risque de régression : Faible à Moyen — l’ordre déterministe et les codes d’erreur/budgets doivent être vérifiés bit à bit sur le
  corpus existant.
  • Méthode de validation après correction : Suite complète cargo test --workspace + tests large-stl + comparaison temps/RSS sur la rampe
  publique et Flansch.

  #### [AUD-006] scripts/package-macos.py omet le binaire fabstep-worker du paquet release .app
  • Domaine : Packaging / Fiabilité / Distribution
  • Priorité : P1
  • Statut : Confirmé
  • Localisation : scripts/package-macos.py:302-331, scripts/package-macos.py:447-507 (à comparer avec scripts/build-macos-dev.sh:7-11 et
  crates/fabstep-desktop/src/supervisor.rs:249-262)
  • Preuves : build-macos-dev.sh compile et copie fabstep-worker dans FabSTEP.app/Contents/MacOS/fabstep-worker, mais package-macos.py ne
  copie, ne relocalise (@loader_path), ne signe et ne vérifie que Contents/MacOS/FabSTEP.
  • Impact concret : Toute future livraison packagée via package-macos.py (post-v1) échouera systématiquement sur convert_large_document
  avec worker.unavailable. (La v1.0.0 déjà livrée n’incluait pas R-067, mais la prochaine release en dépend.)
  • Recommandation : Étendre package-macos.py pour embarquer fabstep-worker, relocaliser ses dylibs dans Contents/Frameworks, le signer,
  l’enregistrer dans dependencies.json et le tester sous isolation Homebrew dans verify_app.
  • Effort estimé : Faible à Moyen
  • Risque de régression : Faible — conserver la séparation Cargo stricte (large-stl jamais activé sur fabstep-desktop, ADR-0034).
  • Méthode de validation après correction : python3 scripts/package-macos.py verify ... --require-no-homebrew incluant un test de fumée
  sur le worker relocalisé.
  #### [AUD-007] Raycasting Three.js sans BVH (jusqu’à 64 raycasts/frame en peinture de faces) et proxies profonds Svelte 5 $state

  • Domaine : Performance / UX / ui
  • Priorité : P1
  • Statut : Confirmé
  • Localisation : ui/src/lib/source-picking.ts:4-23, ui/src/lib/face-paint.ts:19-28, ui/src/components/Viewport.svelte:78-87,
  ui/src/components/Viewport.svelte:220-225, ui/src/App.svelte:46-102
  • Preuves :
      1. FacePaintStroke.sample() échantillonne tous les 4 px jusqu’à 64 fois par requestAnimationFrame ; chaque échantillon appelle
      getBoundingClientRect(), alloue un THREE.Vector2 et exécute raycaster.intersectObjects() en O(N) sans BVH sur une BufferGeometry non
      indexée (jusqu’à ~6,4 M de tests de triangles/frame sur un STEP de 100k triangles).
      2. App.svelte déclare bytes, preview, result, stepDocument, stepDisplay, stepSelection, sourceZone avec $state(...) au lieu de
      $state.raw(...), enveloppant des tableaux de 100k triangles / 300k sommets dans des Proxy récursifs Svelte 5.
      3. Viewport.svelte:220,225 construit les sommets sur le thread UI via .flatMap(t => t.flatMap(...)) (100k tableaux temporaires + un
      number[] de 900k éléments avant copie dans Float32Array), et rebuildEdges() (L. 262–273) appelle JSON.stringify(patchOwners[index])
      par triangle.
  • Impact concret : Saccades ou gel du thread UI lors de l’affichage et de la peinture de faces sur les résultats STEP denses (20k–100k
  triangles).
  • Recommandation :
      1. Passer les runes de données massives immuables en $state.raw(...) (en veillant à remplacer l’objet plutôt que de muter en place ;
      garder processing en $state ou immutabiliser workflow.ts:134,163).
      2. Remplacer .flatMap() par une boucle remplissant directement un Float32Array(triangles.length * 9) pré-alloué, et remplacer JSON.
      stringify + clés chaînes par des clés entières dans rebuildEdges().
      3. Mettre en cache getBoundingClientRect() une fois par flushPaint(), réutiliser un Vector2 partagé, et accélérer le raycasting par
      un BVH léger (construit dans step-selection.worker.ts ou via three-mesh-bvh si dépendance acceptée).
  • Effort estimé : Faible ($state.raw, boucle Float32Array, cache rect) à Moyen (BVH raycasting)
  • Risque de régression : Faible — vérifier que le hit retourné reste strictement la face visible la plus proche.
  • Méthode de validation après correction : Mesure frame-time / CPU dans DevTools WebKit sur un STEP de 50k–100k triangles + npm --prefix
  ui run test et check.
  #### [AUD-008] Appels BRepAdaptor_Surface(face) en O(E²) (UVBounds inutile) et copie de 256 Mio dans fabstep_tessellate_step
  • Domaine : Performance / Mémoire / fabstep-cad
  • Priorité : P1
  • Statut : Confirmé
  • Localisation : crates/fabstep-cad/src/trim_edge_cache.hxx:328, crates/fabstep-cad/src/occt.cpp:2104,
  crates/fabstep-cad/src/occt.cpp:2866, crates/fabstep-cad/src/occt.cpp:4615, crates/fabstep-cad/src/torus_atlas_domain.hxx:113,
  crates/fabstep-cad/src/torus_atlas_transfer.hxx:58
  • Preuves :
      1. BRepAdaptor_Surface(face) a par défaut Restriction = Standard_True, ce qui déclenche BRepTools::UVBounds(face) (parcours de
      toutes les arêtes de la face). Dans trim_edge_cache.hxx:328, il est construit à l’intérieur de la boucle sur les arêtes de trim →
      O(E²) sur chaque face torique à ~1 000 arêtes ! Même appel superflu dans complete_torus_seam_nodes (occt.cpp:2866, appelé sur les 46
      671 faces même non toriques) et torus_atlas_transfer.hxx:58 (93 342 appels).
      2. occt.cpp:4615 (fabstep_tessellate_step) utilise std::istringstream stream(std::string(reinterpret_cast<const char*>(bytes),
      size)); au lieu du StepInputBuffer zéro-copie déjà présent à la ligne 2432 — allouant jusqu’à 256 Mio (2 copies de 128 Mio) pour
      tesseller un STEP dense.
  • Impact concret : Surcoût CPU quadratique lors de la tessellation/vérification des tores découpés et pic RAM inutile de 256 Mio lors de
  l’audit de fidélité.
  • Recommandation : Passer false (BRepAdaptor_Surface(face, false)) ou utiliser BRep_Tool::Surface(face) partout où les bornes UV ne sont
  pas requises ; remplacer occt.cpp:4615 par StepInputBuffer.
  • Effort estimé : Faible (quelques lignes)
  • Risque de régression : Quasi nul — aucun changement géométrique.
  • Méthode de validation après correction : cargo test -p fabstep-cad --offline --locked + mesure RSS/temps de fabstep_tessellate_step
  sur la rampe publique et Flansch.
  #### [AUD-009] fabstep-verify : tri O(N log² N), double hypot() par test de triangle et BTreeSet sur 1,62 M de milieux
  • Domaine : Performance / fabstep-verify
  • Priorité : P2
  • Statut : Confirmé
  • Localisation : crates/fabstep-verify/src/lib.rs:223, crates/fabstep-verify/src/lib.rs:254-270,
  crates/fabstep-verify/src/lib.rs:309-333, crates/fabstep-verify/src/lib.rs:409-417
  • Preuves :
      1. L. 223 : triangles.sort_by(...) trie toute la tranche à chaque nœud du BVH (O(N log² N)), alors que source_embedding.rs:63 dans
      le même crate utilise déjà select_nth_unstable_by (O(N log N)).
      2. L. 310–315 : triangle_distance_squared recalcule ab, ac, cross(ab, ac) et deux appels libm hypot() à chaque test de triangle
      (dizaines de millions d’appels).
      3. L. 257 & 261–262 : maximum_squared calcule da/db avant d’empiler, puis recalcule node.bounds.distance_squared(point)
      immédiatement après dépilage.
      4. L. 409 : BTreeSet<[u64; 3]> effectue jusqu’à 1,62 M d’insertions sur Flansch (vs HashSet pré-dimensionné qui préserve exactement
      la déduplication bit à bit [u64; 3]).
  • Impact concret : Temps d’audit élevé sur les gros maillages et pression accrue sur le budget MAX_WORK = 50_000_000 (un split SAH à
  8–16 bacs réduirait aussi le nombre d’opérations Work).
  • Recommandation : Précalculer la normale unitaire unit dans Triangle à la construction du BVH ; utiliser select_nth_unstable_by (ou SAH
  à bacs) ; empiler (id, dist) sur la pile BVH ; remplacer BTreeSet<[u64; 3]> par HashSet::with_capacity(...) (ou table déterministe).
  • Effort estimé : Faible à Moyen
  • Risque de régression : Très faible — exactitude bit à bit vérifiable sur les tests existants
  (pruned_search_agrees_with_exhaustive_search_on_translated_thin_triangles).
  • Méthode de validation après correction : cargo test -p fabstep-verify --offline --locked + benchmark avant/après sur la rampe publique
  (194k triangles) et un maillage de 540k triangles.
  #### [AUD-010] Mutex global EngineState.inner maintenu pendant les écritures disque et sync_all()
  • Domaine : Concurrence / Réactivité / fabstep-desktop
  • Priorité : P2
  • Statut : Confirmé
  • Localisation : crates/fabstep-desktop/src/main.rs:997-1008, crates/fabstep-desktop/src/main.rs:1060-1076,
  crates/fabstep-desktop/src/section.rs:479-490
  • Preuves : export_step, export_face_dxf et export_section_dxf verrouillent state.inner.lock() puis effectuent NamedTempFile::new_in,
  write_all, sync_all() et persist sous ce verrou. Pendant ce temps, les commandes synchrones (invalidate_result, get_conversion_progress,
  upload_source, set_face_profile_context) bloquent sur ce mutex.
  • Impact concret : Gel temporaire de l’interface lors de l’export d’un gros STEP vers un volume lent (USB, SMB, iCloud).
  • Recommandation : Cloner Arc<DocumentStep> (ou les octets DXF) sous verrou, libérer le verrou pour exécuter write_all + sync_all(),
  puis reprendre le verrou uniquement pour revérifier la fraîcheur et exécuter file.persist(&path) (si la révision a changé entre-temps,
  NamedTempFile se nettoie automatiquement).
  • Effort estimé : Faible
  • Risque de régression : Très faible — le contrat d’atomicité et de non-export périmé reste intégralement préservé.
  • Méthode de validation après correction : cargo test --manifest-path crates/fabstep-desktop/Cargo.toml --offline --locked.
  #### [AUD-011] Travail redondant dans parameter_search (7× parsing/analyse + provenance jetée) et fabstep-worker (provenance jetée +
  fsync sur progress.json)
  • Domaine : Performance / fabstep-app & fabstep-worker
  • Priorité : P2
  • Statut : Confirmé
  • Localisation : crates/fabstep-app/src/parameter_search.rs:152-190, crates/fabstep-worker/src/main.rs:170-199,
  crates/fabstep-worker/src/main.rs:388-409
  • Preuves :
      1. parameter_search rappelle convert_document_with_source_proofs jusqu’à 7 fois en ne faisant varier que cylinder_fit_tolerance_mm,
      répétant 7× la décompression 3MF, le parsing XML, le soudage et la segmentation planaire, et construisant des jetons de provenance
      STEP ensuite jetés.
      2. fabstep-worker appelle convert_shared_document_with_provenance_controlled et conserve source_faces + GuidanceSource alors que
      supervisor.rs:616 enveloppe le résultat dans DocumentStep::untracked(step) ; il exécute aussi sync_all() à chaque mise à jour de
      progress.json.
  • Impact concret : Latence inutile sur la recherche assistée (surtout sur 3MF multi-corps) et surcoût RAM/I/O dans le worker gros STL.
  • Recommandation : Factoriser l’analyse documentaire une fois avant la boucle de parameter_search et désactiver la provenance pendant
  les essais ; dans fabstep-worker, éviter la construction de provenance non consommée (en conservant la voie torus_atlas) et supprimer
  sync_all() sur progress.json (tout en gardant l’écriture temporaire + rename atomique).
  • Effort estimé : Moyen
  • Risque de régression : Moyen sur parameter_search (vérifier les tests vérifiant la trace des ConversionPhase) ; Faible sur fabstep-
  worker.
  • Méthode de validation après correction : Tests parameter_search et fabstep-worker + mesure chronométrée sur guided_cylinder_instances.
  3mf.
  #### [AUD-012] Projection complète du maillage par paire candidate dans partial.rs / partial/chain.rs et allocation par triangle dans
  mesh_section.rs
  • Domaine : Performance / fabstep-core
  • Priorité : P2
  • Statut : Confirmé
  • Localisation : crates/fabstep-core/src/partial.rs:374-401, crates/fabstep-core/src/partial.rs:856-857,
  crates/fabstep-core/src/partial/chain.rs:315-316, crates/fabstep-core/src/mesh_section.rs:316-328
  • Preuves :
      1. partial::prove et partial::chain::prove projettent tous les sommets du maillage (a.mesh.vertices.len(), soit 270k sommets = 6,5
      Mio par paire candidate) et débitent work.spend(a.mesh.vertices.len()), alors que seuls les sommets locaux de bottom, top et
      selected sont lus.
      2. partial.rs:374-401 alloue un Vec<P2> dans la double boucle O(L²) des fenêtres d’arcs (évitable avec une tranche &points_2x[start.
      .start + length]).
      3. mesh_section.rs:316 alloue Vec::with_capacity(2) sur le tas pour chaque triangle du maillage, même les 99 % qui ne coupent pas le
      plan.
  • Impact concret : Épuisement prématuré du budget de travail (10 M) sur maillages denses dès ~35 paires candidates dans partial, et
  allocations inutiles lors des coupes source.
  • Recommandation : Projeter à la demande (ou sur le sous-ensemble local) dans partial::prove / chain::prove ; utiliser un tableau doublé
  pour les fenêtres d’arcs ; utiliser un tableau pile [Node; 2] + compteur dans mesh_section.rs:316.
  • Effort estimé : Faible à Moyen
  • Risque de régression : Faible (conserver un débit de budget cohérent sur les sommets visités).
  • Méthode de validation après correction : cargo test -p fabstep-core --offline --locked.
  ──────
  ### UX/UI, Fiabilité, Build & Documentation
  #### [AUD-013] Bugs UX confirmés : Escape en peinture de faces efface la sélection, champs de tolérance invalidant à chaque frappe,
  Viewport ne rafraîchit pas un nouveau preview à révision égale, focus perdu sur Dialog
  • Domaine : UX/UI / ui
  • Priorité : P2
  • Statut : Confirmé
  • Localisation :
      • ui/src/components/Viewport.svelte:111 + ui/src/App.svelte:465 (Escape)
      • ui/src/App.svelte:878-952 (<input type="number"> vs ui/src/components/NumField.svelte)
      • ui/src/components/Viewport.svelte:205 (renderedNative === !!preview)
      • ui/src/components/Dialog.svelte:29 (showModal() non gardé + focus non restauré)
  • Preuves :
      1. cancelPaintKey (Viewport.svelte:111) appelle e.preventDefault() sans e.stopImmediatePropagation() (contrairement à
      cancelTransformKey ligne 72) → le listener window de App.svelte:465 vide toute la sélection.
      2. Les 4 tolérances d’App.svelte utilisent oninput sur <input type="number"> : effacer un chiffre envoie NaN, invalide immédiatement
      le STEP et affiche une erreur rouge en cours de saisie, alors que NumField.svelte (déjà dans le dépôt !) gère la virgule décimale,
      Enter/blur et Escape.
      3. Viewport.svelte:205 teste renderedNative === !!preview (booléen) au lieu de la référence renderedPreview === preview : si preview
      change à sourceRevision constante (ex. changement de tolérance de soudage), le maillage 3D n’est pas reconstruit.
      4. Dialog.svelte:29 appelle dialog?.showModal() sans vérifier !dialog.open et ne restaure pas le focus sur le bouton déclencheur à
      la fermeture.
  • Impact concret : Perte accidentelle d’une sélection manuelle patiente, friction de saisie dans les réglages avancés, aperçu
  potentiellement désynchronisé et rupture de navigation clavier.
  • Recommandation : Ajouter e.stopImmediatePropagation() dans cancelPaintKey, migrer les 4 tolérances vers NumField.svelte, comparer
  renderedPreview === preview dans Viewport.svelte:205, garder !dialog.open et restaurer previousFocus dans Dialog.svelte.
  • Effort estimé : Faible
  • Risque de régression : Faible — vérifier l’historique undo/redo (commitOptions()) lors du passage à NumField.
  • Méthode de validation après correction : npm --prefix ui run test, check, et vérification manuelle clavier/peinture dans le bureau.

  #### [AUD-014] Build frontend : import barrel @lucide/svelte (1 920 icônes compilées), polices inutiles et bundle monolithique 877 Ko
  • Domaine : Performance build / Frontend / ui
  • Priorité : P2
  • Statut : Confirmé (mesuré lors de npm --prefix ui run build)
  • Localisation : ui/vite.config.ts, ui/src/app.css, imports @lucide/svelte dans ui/src/App.svelte et composants
  • Preuves : npm --prefix ui run build transforme 4 262 modules et exécute vite-plugin-svelte:compile 1 920 fois (3,8 s / 71 % du build)
  sur des icônes non utilisées (cupcake, indian-rupee, pill-bottle…), produit un chunk unique index-BW4F-gR-.js de 877 Ko (avertissement
  Vite > 500 Ko), et embarque 7 polices Inter dont cyrillique, grec et vietnamien (~85 Ko inutiles pour FR/EN).
  • Impact concret : Temps de build/dev ralenti et charge initiale WebView alourdie (déjà noté en R-022 / AUD-009).
  • Recommandation : Passer à des imports profonds @lucide/svelte/icons/<nom> ; n’importer que @fontsource-variable/inter/latin.css (et
  latin-ext.css) ; configurer le code-splitting (three dans un chunk dédié) dans vite.config.ts après mesure du temps de démarrage WebView.
  • Effort estimé : Faible
  • Risque de régression : Très faible.
  • Méthode de validation après correction : npm --prefix ui run build (vérifier la chute du nombre de modules transformés, du temps de
  build et du poids de dist/).
  #### [AUD-015] fabstep-cli tronque silencieusement les fichiers > 64 Mio et retarde le contrôle --force après conversion
  • Domaine : Intégrité des données / Fiabilité / fabstep-cli
  • Priorité : P2
  • Statut : Confirmé
  • Localisation : crates/fabstep-cli/src/main.rs:159-164, crates/fabstep-cli/src/main.rs:248-262
  • Preuves : .take(64 * 1024 * 1024 + 1).read_to_end(&mut bytes) ne rejette pas bytes.len() > 64 * 1024 * 1024 avec input.too_large
  (contrairement au bureau main.rs:386-391), renvoyant une erreur de parsing tronqué trompeuse. Par ailleurs, persist_noclobber n’est
  testé qu’après toute la conversion.
  • Impact concret : Diagnostic d’erreur trompeur sur gros fichiers en CLI et temps de calcul gaspillé quand le fichier de sortie existe
  déjà sans --force.
  • Recommandation : Retourner input.too_large immédiatement si bytes.len() > 64 * 1024 * 1024 ; ajouter un pré-contrôle if !force &&
  path.try_exists().unwrap_or(false) avant conversion (en conservant persist_noclobber contre le TOCTOU).
  • Effort estimé : Faible
  • Risque de régression : Nul.
  • Méthode de validation après correction : Tests unitaires/CLI dans crates/fabstep-cli/tests/cli.rs.
  #### [AUD-016] Dette structurelle : fichiers monolithiques, compilation C++ en 1 TU, CI cargo fetch manquant pour fabstep-worker et
  durcissements mineurs
  • Domaine : Architecture / Maintenabilité / CI / Sécurité
  • Priorité : P3
  • Statut : Confirmé (risque CI latent : Probable)
  • Localisation :
      • Monolithes : crates/fabstep-cad/src/lib.rs (7 499 l.), crates/fabstep-cad/src/occt.cpp (4 677 l. + 15 .hxx inclus dans 1 seul TU),
      crates/fabstep-core/src/rounded.rs (3 345 l.), ui/src/App.svelte (1 565 l.).
      • CI : .github/workflows/check.yml:30-31 omet cargo fetch --manifest-path crates/fabstep-worker/Cargo.toml --locked (fonctionne
      aujourd’hui par coïncidence car les crates du worker sont un sous-ensemble du Cargo.lock racine, mais cassera dès qu’une version
      divergera).
      • Défense en profondeur : crates/fabstep-cad/src/occt.cpp:4061 (*output = {} manquant en entrée de fabstep_build_torus_atlas),
      crates/fabstep-cad/src/occt.cpp:2307 (set_error sans garde if (!error || capacity == 0)),
      crates/fabstep-desktop/src/supervisor.rs:249-262 (helper_path suit les symlinks) et crates/fabstep-desktop/src/supervisor.rs:325-363
      (read_limited ne vérifie pas file.metadata()?.is_file() après ouverture), scripts/package-source.py:89-93 (écriture directe sans
      temp+rename), crates/fabstep-cad/build.rs:83-84 (-isystem sans garde MSVC et NDEBUG non défini en release).
      • Documentation : STATUS.md:3 mentionne encore fabboard/mise-d-equerre au lieu de fabboard/geometrie ; docs/FEATURES.md n’inclut pas
      les capacités post-v1 (Auto, guidage, DXF, gros STL) ; README.md présente les ajouts post-v1 en deux endroits séparés (L. 7–28 et L.
      169–183).
      • UI : 6 composants design-system inutilisés (Alerts, AppBar, StatusBar, Tag, Field, Popover) et ui/src/App.svelte:306 compare un
      message d’erreur français en dur au lieu d’un code d’erreur stable.
  • Recommandation : Traiter de façon incrémentale : ajouter cargo fetch pour le worker en CI, corriger les 4 gardes défensives C++/Rust
  en quelques lignes, scinder occt.cpp en 4–5 unités de traduction .cpp compilées en parallèle, extraire messages.ts d’App.svelte, et
  consolider README.md / STATUS.md.
  • Effort estimé : Faible (CI, gardes, docs) à Moyen (découpage progressif)
  • Risque de régression : Très faible.
  • Méthode de validation après correction : sh scripts/check.sh.
  ──────
  ## 4 — Optimisations recommandées
  │ Distinction explicite du niveau de preuve : Mesuré (chiffres observés dans l’environnement ou VALIDATION.md), Analyse algorithmique
  │ (complexité démontrée par lecture directe du code), Envisageable (gain plausible à confirmer par benchmark).

   Composant        │ Problème / Opportunité │ Gain attendu      │ Niveau de preuve  │ Coût              │ Méthode de mesure / validation
  ──────────────────┼────────────────────────┼───────────────────┼───────────────────┼───────────────────┼────────────────────────────────
   fabstep-cad      │ BRepAlgoAPI_Check      │ 5× à 20× sur le   │ Mesuré (V-        │ Moyen–Élevé       │ Chronométrage sur rampe 194k
   (torus_atlas_int │ mono-forme re-teste    │ contrôle          │ 1254/1256–1258) + │                   │ et Flansch + qualify-atlas-
   erference.hxx)   │ (f, f) et intra-       │ d’interférence    │ Analyse           │                   │ interference.sh
                    │ feuilles sur 6 949     │ Flansch (sortie   │ algorithmique     │                   │
                    │ groupes + boîtes       │ du timeout 30     │                   │                   │
                    │ toriques creuses       │ min)              │                   │                   │
   fabstep-cad      │ measure_solid          │ ~2× immédiat sur  │ Analyse           │ Faible            │ Temps total
   (occt.cpp:4087,4 │ (interférences         │ le temps          │ algorithmique     │                   │ fabstep_build_torus_atlas
   124)             │ comprises) exécuté 2×  │ d’interférence    │                   │                   │ avant/après sur rampe 194k
                    │ dans                   │                   │                   │                   │
                    │ fabstep_build_torus_at │                   │                   │                   │
                    │ las                    │                   │                   │                   │
   fabstep-cad      │ IntTools_FaceFace      │ ~174 s            │ Mesuré (V-1254)   │ Faible–Moyen      │ Temps de atlas_fan::repair sur
   (torus_atlas_fan │ inconditionnel sur 14  │ économisées sur   │                   │                   │ Flansch + témoins 6 faces V-
   .hxx:285)        │ 882 faces réglées      │ Flansch           │                   │                   │ 1256
   fabstep-cad      │ BRepAdaptor_Surface(fa │ Réduction majeure │ Analyse           │ Faible            │ Benchmark trim_edge_cache /
   (trim_edge_cache │ ce) en O(E²)           │ du temps de       │ algorithmique     │                   │ extract_face_meshes sur rampe
   .hxx:328,        │ (UVBounds)             │ tessellation/tran │                   │                   │ 194k et Flansch
   occt.cpp:2866)   │                        │ sfert des tores   │                   │                   │
                    │                        │ denses            │                   │                   │
   fabstep-cad      │ istringstream(string)  │ −256 Mio de pic   │ Analyse           │ Faible (3 l.)     │ RSS / empreinte physique
   (occt.cpp:4615)  │ au lieu de             │ RAM sur STEP 128  │ algorithmique     │                   │ (proc_pid_rusage) durant
                    │ StepInputBuffer        │ Mio               │                   │                   │ fabstep_tessellate_step
   fabstep-core     │ Topologie MeshTopology │ 3× à 6× en temps  │ Analyse           │ Moyen–Élevé       │ Benchmark analyze +
   (pipeline.rs,    │ plate partagée, 1-tri  │ CPU core et −50 % │ algorithmique     │                   │ recognize_automatic sur rampe
   automatic.rs,    │ fast-path              │ à −70 % sur le    │                   │                   │ 194k et Flansch + parité bit à
   rounded.rs,      │ planar::segment,       │ pic RAM core sur  │                   │                   │ bit
   planar.rs)       │ suppression clone      │ 540k triangles    │                   │                   │
                    │ partial_backup, grille │                   │                   │                   │
                    │ ouverte dans           │                   │                   │                   │
                    │ weld_controlled        │                   │                   │                   │
   fabstep-worker & │ Analysis.mesh + model  │ −95 %+ sur la     │ Analyse           │ Moyen             │ Taille de report.json + RSS
   supervisor.rs    │ sérialisés dans        │ taille de         │ algorithmique     │                   │ bureau sur real_worker_handoff
                    │ report.json + DOM      │ report.json et    │                   │                   │ / rampe 194k
                    │ serde_json::Value      │ centaines de Mio  │                   │                   │
                    │                        │ économisés côté   │                   │                   │
                    │                        │ bureau            │                   │                   │
   fabstep-verify   │ select_nth_unstable_by │ 2× à 4× sur       │ Analyse           │ Faible–Moyen      │ Benchmark fabstep-verify sur
   (lib.rs)         │ + normales             │ l’audit de        │ algorithmique     │                   │ rampe 194k et Flansch (temps +
                    │ précalculées (sans     │ fidélité +        │                   │                   │ Work)
                    │ hypot()) + (id, dist)  │ réduction du      │                   │                   │
                    │ pile BVH + HashSet     │ compteur Work     │                   │                   │
                    │ midpoints (+ SAH bacs) │                   │                   │                   │
   ui (App.svelte,  │ $state.raw sur données │ Passage de        │ Analyse           │ Faible (sans BVH) │ Profiling WebKit DevTools
   Viewport.svelte, │ de maillage + boucle   │ saccades/gel à 60 │ algorithmique     │ / Moyen (avec     │ (Scripting + Frame time) sur
   face-paint.ts)   │ Float32Array sans      │ FPS en peinture   │                   │ BVH)              │ STEP 50k–100k triangles
                    │ .flatMap() + cache     │ de faces sur STEP │                   │                   │
                    │ getBoundingClientRect( │ 100k triangles    │                   │                   │
                    │ ) + BVH raycasting     │                   │                   │                   │
   ui               │ Imports profonds       │ −60 % à −70 % sur │ Mesuré (vite      │ Faible            │ Temps npm --prefix ui run
   (vite.config.ts, │ d’icônes + sous-       │ le temps de build │ build: 3,8 s /    │                   │ build et taille ui/dist
   @lucide/svelte,  │ ensembles latin seuls  │ Vite (3,8 s       │ 5,4 s)            │                   │
   Inter)           │ + chunking three       │ gagnées) et ~85   │                   │                   │
                    │                        │ Ko d’assets en    │                   │                   │
                    │                        │ moins             │                   │                   │
   fabstep-cad      │ Découpage d’occt.cpp + │ 2× à 4× sur la    │ Envisageable      │ Moyen             │ Temps de cargo check -p
   (occt.cpp)       │ .hxx en plusieurs .cpp │ recompilation     │                   │                   │ fabstep-cad après modification
                    │                        │ incrémentale C++  │                   │                   │ d’un .hxx
  ──────
  ## 5 — Suggestions d’amélioration
  • Améliorations de l’existant (besoin avéré) :
      • Unifier Frame (actuellement dupliqué 7× dans fabstep-core) et la validation des tolérances dans un module interne commun geometry.
      rs / options.rs.
      • Remplacer la comparaison de message français en dur dans ui/src/App.svelte:306 par un code d’erreur stable (cad.planarity_exceeded
      ou équivalent).
      • Dans crates/fabstep-core/src/pipeline.rs:43-56, appliquer le raccourci mono-corps à prepare_mesh_bodies_for_sections_controlled
      (déjà en place dans analyze_bodies_with_provenance).
  • Simplifications architecturales (besoin avéré) :
      • Extraire engineKeys/diagnosticKeys (ui/src/lib/messages.ts) et les contrôleurs de workers hors d’App.svelte (1 565 l.).
      • Retirer les 6 composants UI morts (Alerts, AppBar, StatusBar, Tag, Field, Popover) ou décider de leur adoption.
      • Extraire les 4 105 lignes de tests inline de fabstep-cad/src/lib.rs dans crates/fabstep-cad/tests/.
  • Améliorations UX/UI (besoin avéré) :
      • Adopter NumField.svelte pour les 4 champs de tolérance, corriger Escape en peinture de faces, restaurer le focus sur Dialog.svelte,
      ajouter aria-label sur FR/EN et sur le viewport 3D, et corriger .input:focus qui masque :focus-visible.
  • Renforcement des tests et de l’outillage (besoin avéré) :
      • Ajouter un micro-benchmark reproductible (cargo test --release --ignored ou binaire bench dédié) mesurant temps + Work + RSS sur
      la rampe publique (194k triangles) pour suivre les régressions de performance.
      • Ajouter cargo fetch --manifest-path crates/fabstep-worker/Cargo.toml --locked dans .github/workflows/check.yml.
  • Évolutions fonctionnelles facultatives (exploratoire) :
      • Paralléliser les lots indépendants de fabstep-verify (si un modèle de budget Work déterministe par lot est défini et validé).
      • Réduire le backlog de 88 éléments roadmap en statut « À tester » via une session groupée de validation Fusion/bureau afin d’éviter
      la dérive entre développement et qualification utilisateur.
  ──────
  ## 6 — Plan d’action priorisé

  ### A. Corrections critiques (à traiter en priorité)

  1. Refondre atlas_interference::Checker + supprimer le double appel dans fabstep_build_torus_atlas (AUD-001, AUD-002)
      • Dépendances : aucune. Effort : Moyen–Élevé.
      • Bénéfice : Débloque Flansch sous le plafond de 30 min (facteur 10×–40× cumulé sur l’interférence).
      • Critères d’acceptation : Tous les témoins négatifs de qualify-atlas-interference.sh restent refusés ; conversion complète de la
      rampe publique puis de Flansch sous 30 min et 4 Gio.
  2. Alléger report.json du fabstep-worker et typer sa lecture dans supervisor.rs (AUD-004)
      • Dépendances : aucune. Effort : Moyen.
      • Bénéfice : Élimine le risque worker.output_too_large sur gros STL et économise des centaines de Mio en RAM/IPC.
      • Critères d’acceptation : report.json < 1 Mio sur la rampe publique ; real_worker_handoff et tests UI/desktop verts.
  3. Intégrer fabstep-worker dans scripts/package-macos.py (AUD-006)
      • Dépendances : aucune. Effort : Faible–Moyen.
      • Bénéfice : Garantit que le mode gros STL fonctionne dans les futurs paquets release autonomes.
      • Critères d’acceptation : package-macos.py verify --require-no-homebrew valide FabSTEP ET fabstep-worker.

  ### B. Quick wins (fort ratio bénéfice / effort)

  4. Correctifs C++ rapides : StepInputBuffer dans fabstep_tessellate_step, BRepAdaptor_Surface(face, false), pré-filtre atlas_fan::repair
  (AUD-003, AUD-008)
      • Dépendances : aucune. Effort : Faible.
      • Bénéfice : −256 Mio RAM, ~174 s gagnées sur Flansch, suppression du O(E²) dans trim_edge_cache.
      • Critères d’acceptation : cargo test -p fabstep-cad --offline --locked + V-1256/V-1257 verts.
  5. Quick wins UI/UX : $state.raw, boucle Float32Array, Escape peinture, NumField tolérances, renderedPreview, Dialog focus, imports
  profonds Lucide (AUD-007 partiel, AUD-013, AUD-014)
      • Dépendances : aucune. Effort : Faible.
      • Bénéfice : Fluidité UI, correction de 4 bugs UX réels, build Vite 3× plus rapide.
      • Critères d’acceptation : npm --prefix ui run test && check && build verts ; build Vite < 2,5 s.
  6. Libérer EngineState.inner avant write_all/sync_all + corriger troncature > 64 Mio en CLI + cargo fetch worker en CI (AUD-010, AUD-015,
  AUD-016)
      • Dépendances : aucune. Effort : Faible.
      • Bénéfice : Zéro blocage UI à l’export, erreur input.too_large fiable en CLI, CI pérenne.
      • Critères d’acceptation : Tests desktop, CLI et CI verts.
  ### C. Améliorations structurelles
  7. Optimisation fabstep-core : MeshTopology plate partagée, planar::segment 1-tri, suppression clone partial_backup, weld_controlled
  déterministe, partial::prove local (AUD-005, AUD-012)
      • Dépendances : benchmark de référence sur la rampe publique. Effort : Moyen–Élevé (par lots).
      • Bénéfice : Division par 3–6 du temps core et forte réduction du pic mémoire sur 540k triangles.
      • Critères d’acceptation : Parité bit à bit des rapports/STEP sur le corpus de régression + mesure temps/RSS améliorée.
  8. Optimisation fabstep-verify : select_nth_unstable_by / SAH, normales précalculées, pile (id, dist), HashSet midpoints (AUD-009)
      • Dépendances : aucune. Effort : Faible–Moyen.
      • Bénéfice : Audit de fidélité 2–4× plus rapide et moindre consommation du budget Work.
      • Critères d’acceptation : Tests fabstep-verify verts + accord exact avec recherche exhaustive.
  9. BVH de raycasting dans Viewport.svelte + factorisation de parameter_search et fabstep-worker sans provenance inutile (AUD-007, AUD-
  011)
      • Dépendances : Quick wins UI (action 5). Effort : Moyen.
      • Bénéfice : Peinture de faces fluide sur STEP 100k triangles ; recherche assistée et worker plus rapides.
      • Critères d’acceptation : Frame-time < 16 ms en peinture sur STEP dense ; tests parameter_search et fabstep-worker verts.
  ### D. Évolutions facultatives
  10. Découpage progressif des monolithes (occt.cpp en plusieurs TU, fabstep-cad/src/lib.rs, App.svelte, rounded.rs) et mise à jour
  documentaire (STATUS.md, FEATURES.md, README.md) (AUD-016)
      • Dépendances : idéalement après stabilisation des chantiers A–C. Effort : Moyen.
      • Bénéfice : Recompilation incrémentale C++ accélérée et meilleure lisibilité long terme.
      • Critères d’acceptation : sh scripts/check.sh intégralement vert sans changement fonctionnel.

  ──────
  ## 7 — Limites de l’audit

  • Modules examinés en profondeur : fabstep-cad (occt.cpp, torus_atlas_*.hxx, trim_edge_cache.hxx, lib.rs, build.rs), fabstep-verify
  (lib.rs, source_embedding.rs), fabstep-core (pipeline.rs, planar.rs, automatic.rs, rounded.rs, partial.rs, partial/chain.rs, revolved.rs,
  complete_torus.rs, complete_sphere.rs, mesh_section.rs, torus_atlas/), fabstep-import (lib.rs, model.rs, xml.rs, ZIP/OPC), fabstep-app
  (lib.rs, document.rs, parameter_search.rs, source_proof_cache.rs), fabstep-desktop (main.rs, supervisor.rs, section.rs, tauri.conf.json),
  fabstep-worker (main.rs), fabstep-cli (main.rs), ui (App.svelte, Viewport.svelte, workflow.ts, workers, composants, locales, vite.
  config.ts), scripts CI/packaging et documentation principale.
  • Modules seulement échantillonnés / non examinés en détail : les reconnaisseurs géométriques spécialisés récents (cross_drill.rs,
  oblique_pocket.rs, unequal_pocket.rs, height_patch.rs, rounded/linear_blend.rs, rounded/smooth_revolution.rs, rounded/octant.rs), les 44+
  fichiers individuels docs/adr/*, les scripts de qualification FreeCAD (scripts/validate-*.py, scripts/qualify-*.cpp), et les tests
  d’intégration individuels.
  • Vérifications réellement exécutées dans cet environnement (toutes réussies, exit code 0) :
      • python3 -m unittest discover -s scripts/tests : 12 réussis, 11 ignorés (dépendances locales/FreeCAD), 0 échec.
      • npm --prefix ui run test : 238/238 réussis (555 ms).
      • npm --prefix ui run check (svelte-check) : 0 erreur, 0 avertissement.
      • npm --prefix ui run build : réussi en 5,42 s (4 262 modules, chunk principal 877 Ko).
      • cargo fmt --check (workspace, fabstep-desktop, fabstep-worker) : 0 diff.
      • cargo test --workspace --offline --locked : 707 réussis, 0 échec, 18 ignorés sur 147 suites.
      • cargo clippy --workspace --all-targets --offline --locked -- -D warnings : 0 avertissement.
      • cargo test --manifest-path crates/fabstep-desktop/Cargo.toml --offline --locked : 50 réussis, 0 échec, 1 ignoré
      (real_worker_handoff).
      • cargo test --manifest-path crates/fabstep-worker/Cargo.toml --offline --locked : 8 réussis, 0 échec.
  • Vérifications NON exécutées :
      • Les tests compilés avec --features large-stl (scripts/check.sh L. 24–41) et real_worker_handoff --ignored (L. 42–45) ; cargo
      clippy sur fabstep-desktop et fabstep-worker ; le test autonome occt_version.rs ; l’exécution sur le fichier privé Flansch ; les
      réimports FreeCAD/Fusion ; les essais interactifs dans l’application Tauri (FabSTEP.app).
  • Risques nécessitant une validation manuelle : ergonomie réelle de NumField et de la peinture de faces sous WebKit macOS, conformité
  des 88 jalons post-v1 dans Fusion/FreeCAD, et mesure réelle du pic mémoire physique macOS sur Flansch après optimisation.
  ──────
  ## Top 10 des actions apportant le plus de valeur au projet

  1. Corriger atlas_interference::Checker (torus_atlas_interference.hxx) — Séparer l’auto-intersection individuelle (1× par face) et
  passer au contrôle croisé bi-forme + pré-filtre annulaire des tores : c’est le verrou direct du timeout de 30 min sur Flansch.
  2. Supprimer le double contrôle d’interférence pré-STEP dans fabstep_build_torus_atlas (occt.cpp:4087) — Gain immédiat de 2× en quelques
  lignes tout en conservant le contrôle complet obligatoire sur le solide réimporté.
  3. Éliminer les 14 882 appels IntTools_FaceFace dans atlas_fan::repair (torus_atlas_fan.hxx) — Récupère directement ~174 s sur Flansch
  par pré-filtre de quasi-planéité ou construction rationnelle directe.
  4. Corriger BRepAdaptor_Surface(face, false) (trim_edge_cache.hxx:328, occt.cpp:2866) et StepInputBuffer (occt.cpp:4615) — Quelques
  lignes pour supprimer un O(E²) de UVBounds et 256 Mio de pic RAM.
  5. Alléger report.json de fabstep-worker et typer sa lecture dans supervisor.rs — Supprime un risque réel d’échec worker.
  output_too_large (128 Mio) et économise des centaines de Mio en RAM/IPC sur gros STL.
  6. Partager une MeshTopology plate dans fabstep-core + fast-path 1-triangle dans planar::segment + supprimer le clone partial_backup —
  Réduit drastiquement le temps CPU (5–6× moins de passes) et le pic mémoire sur les maillages de 100k–600k triangles.
  7. Optimiser fabstep-verify (select_nth_unstable_by, normales précalculées sans hypot(), pile (id, dist), HashSet) — Accélère de 2–4×
  l’audit de fidélité et sécurise le budget MAX_WORK sur les maillages denses.
  8. Passer les données de maillage UI en $state.raw + boucle Float32Array + BVH raycasting en peinture de faces — Élimine les gels du
  thread UI sur les résultats STEP complexes.
  9. Embarquer et vérifier fabstep-worker dans scripts/package-macos.py — Indispensable pour que le mode gros STL fonctionne dans le
  prochain paquet macOS autonome.
  10. Quick wins UX & robustesse (Escape peinture, NumField tolérances, renderedPreview, Dialog focus, libération du mutex avant fsync à
  l’export, imports profonds Lucide) — Faible coût, impact quotidien immédiat sur le confort d’utilisation et le temps de build.