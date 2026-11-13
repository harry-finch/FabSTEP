---
id: "R-052"
titre: "DXF1 — exporter une face plane"
resume: "Sélectionner une face du STEP validé, prévisualiser son contour dans son plan, puis **Exporter le profil DXF**"
statut: "a-tester"
jalon: "dxf"
cree: "2026-11-13"
maj: "2026-11-13"
---

| DXF1 — exporter une face plane | Sélectionner une face du STEP validé, prévisualiser son contour dans son plan, puis **Exporter le profil DXF** | Contour extérieur et trous conservés ; échelle 1:1, unité mm et repère 2D explicites ; lignes, arcs et cercles conservés lorsqu’ils sont disponibles dans la géométrie native ; réimport indépendant et contrôle des dimensions | Implémenté ; extraction native lignes/arcs/cercles, aperçu worker, export protégé par identité de résultat/sélection. 39 profils natifs relus par ezdxf sans erreur ni réparation ; 326 tests workspace, 19 bureau, 97 UI et contrôles globaux réussis. Parcours graphique et Fusion restants ; ADR-0024, V-1170/V-1171 |

**Extension du 12 novembre — ellipses exactes.** Après le retour utilisateur sur les faibles inclinaisons du sock-clip, les profils et le DXF prennent en charge les ellipses complètes et partielles, avec aperçu analytique. Les coutures périodiques prouvées ne bloquent plus les coupes centrales régulières ; arêtes entre faces, tangences réelles et courbes non prises en charge restent refusées. Quatorze coupes du fichier privé fraîchement reconverti sont exportées et relues indépendamment ; voir V-1175 pour les réglages et limites. La coupe du maillage source et les intersections toriques obliques générales restent distinctes de cette extension.

Source : [ancienne feuille de route](../../docs/ROADMAP-avant-fabboard.md). Les validations historiques sont conservées ; aucun essai nouveau n’est déduit de la migration.

La livraison de ce lot attend toutes ses vérifications utilisateur rattachées ; une réponse positive à une seule action ne clôt pas les autres.
