# Fonctionnalités

Identifiants stables ; preuves et limites dans [VALIDATION](VALIDATION.md). Candidate mécanique 1.0.0-rc.1.

| ID | Fonction | État |
| --- | --- | --- |
| FS-001 | STL ASCII et binaire, unité explicite | Implémenté et testé |
| FS-002 | Soudage borné et diagnostics topologiques | Implémenté ; source native contrôlée dans tous les modes |
| FS-003 | Régions planes, contours extérieurs et trous polygonaux | Implémenté et testé |
| FS-004 | Construction OCCT, STEP et réimport contrôlé | Solides, types de surfaces, volume et dimensions vérifiés |
| FS-005 | CLI check/convert et rapports JSON | Implémenté ; réglages, provenance et codes d’erreur stables |
| FS-006 | Interface FabApps et aperçu 3D | Thèmes sombre/clair, FR/EN, aide et indicateur de calcul |
| FS-007 | Bureau Tauri et export du résultat courant | Révisions, source binaire en cache et sauvegarde atomique ; dernière qualification native en cours |
| FS-008 | Cylindres et trous traversants dans une extrusion constante | Mode explicite, réimports FreeCAD |
| FS-009 | Import 3MF Core, composants et placements | Implémenté ; 32 corps / 100 000 triangles, contacts refusés. Production multi-modèles non pris en charge |
| FS-010 | Historique paramétrique reconstruit | Hors périmètre ; export BRep |
| FS-011 | Parois cylindriques locales, bossages et trous borgnes | Fonds plans présents dans la source, plusieurs axes séparés, réimports indépendants |
| FS-012 | Attribution de chaque triangle à une face conservée ou reconstruite | Implémenté dans les contrats analytiques |
| FS-013 | Audit bidirectionnel source / tessellation du STEP réimporté | Tous les modes analytiques, maxima échantillonnés et déflexion explicites |
| FS-014 | Cylindres partiels et encoches ouvertes | Contours et raccords prouvés ; cas utilisateur validé |
| FS-015 | Cônes et chanfreins circulaires | Bandes complètes, pointes source ; double chanfrein opposé avec portée résolue |
| FS-016 | Calottes et sphères entières | Couverture et courbure démontrées ; réimports et faux positifs testés |
| FS-017 | Congés toriques cylindre–plan et tores entiers | Contrats distincts ; rayons, couverture, raccords et refus vérifiés |
| FS-018 | Boîtes convexes entièrement arrondies à rayon uniforme | 6 plans, 12 cylindres partiels, 8 octants sphériques et 48 raccords contrôlés |
| FS-019 | Paquet macOS autonome | Apple Silicon/macOS 26 ; signature ad hoc, notices et isolation Homebrew. Candidate en qualification |
| FS-020 | Congés arbitraires, intersections de perçages, surfaces libres | Recherche, non annoncés dans la v1 mécanique |
