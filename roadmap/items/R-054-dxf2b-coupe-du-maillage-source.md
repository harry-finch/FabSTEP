---
id: R-054
titre: "DXF2b — coupe du maillage source"
resume: "Couper le maillage préparé sans exiger un STEP réussi ; profils polygonaux et export DXF distinct."
statut: livree
jalon: dxf
cree: 2026-11-13
maj: 2026-11-19
livree: 2026-11-19
---

**DXF2b — coupe source implémentée.** La vue Source propose une coupe du maillage préparé, même après refus de reconnaissance STEP. Préparation topologique indépendante des contours plans, validation native de tous les corps et de leur séparation, puis extraction polygonale pure bornée. Aucun ajustement de courbe ni fermeture par proximité ; trous, composantes et corps vides restent explicites. Le contrôleur distingue les identités source/STEP et préserve le STEP existant. Les prétraitements sont consultables et les exports source utilisent le suffixe `-mesh-section.dxf`. Quatorze DXF de corpus, dont le sock-clip, sont relus indépendamment ; détails et contrôles globaux en V-1176, architecture dans ADR-0026. Le parcours natif et Fusion restent à qualifier.

Le retour au guidage I2 est réalisé par I2c : choisir Cylindre/Cône sur les facettes STEP et vérifier une paroi complète entre deux contours plans source. La suite reste l’élargissement des familles guidées et des raccords prouvables, puis l’édition/historique I3. La coupe de surfaces toriques obliques générales reste une extension distincte, sans conditionner ce guidage.

Ordre proposé : **DXF1 → DXF2**, en réutilisant le même exporteur 2D. Ce chantier peut avancer indépendamment des futures surfaces libres et des congés variables ; il ne remplace pas les lots géométriques en cours.

La coupe est une extraction de section et un outil de visualisation : elle ne découpe pas définitivement le solide et ne crée pas de fond. Le plan possède une origine et deux axes explicites ; le passage en coordonnées DXF 2D préserve les dimensions, avec une orientation et un emplacement du repère visibles avant export. Le choix des corps concernés est explicite, et aucun corps choisi en échec n’est omis silencieusement.

L’export d’une face vise d’abord les **faces planes**. Projeter ou déplier une face courbe constitue un autre besoin, hors de ces premiers lots. Les contours viennent des arêtes natives, pas de la tessellation d’affichage. Une section du maillage reste polygonale tant qu’un ajustement de lignes/arcs n’est pas demandé et validé avec une tolérance explicite. Les courbes non prises en charge par le premier exporteur sont signalées ; toute discrétisation éventuelle est annoncée et bornée. Aucun contour ouvert n’est fermé artificiellement ni présenté comme un profil fermé valide.

Validation prévue : profils avec trous, plusieurs boucles et corps, plans obliques, unités STL/3MF, placements et miroirs, sections vides/tangentes/coplanaires et contours ouverts. Contrôler le réimport DXF, les dimensions, les rayons et la fermeture attendue dans un lecteur indépendant, puis le parcours dans Fusion. Calcul hors fil UI, limites de travail explicites et invalidation des profils après changement de source, de résultat ou de plan.

Source : [ancienne feuille de route](../../docs/ROADMAP-avant-fabboard.md). Les validations historiques sont conservées ; aucun essai nouveau n’est déduit de la migration.

La livraison de ce lot attend toutes ses vérifications utilisateur rattachées ; une réponse positive à une seule action ne clôt pas les autres.
