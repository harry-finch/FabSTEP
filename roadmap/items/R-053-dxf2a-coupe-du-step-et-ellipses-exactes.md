---
id: R-053
titre: "DXF2a — coupe du STEP et ellipses exactes"
resume: "Placer un plan XY/XZ/YZ ou reprendre le plan d’une face plane, régler son décalage et son orientation, voir la coupe et exporter la section DXF"
statut: livree
jalon: dxf
cree: 2026-11-13
maj: 2026-11-19
livree: 2026-11-19
---

| DXF2 — couper par un plan | Placer un plan XY/XZ/YZ ou reprendre le plan d’une face plane, régler son décalage et son orientation, voir la coupe et exporter la section DXF | Aperçu du plan et du profil ; distinction section du STEP / section du maillage source ; contours multiples, trous et corps distincts conservés ; gestion explicite des tangences, arêtes et faces coplanaires ; réimport indépendant | DXF2a STEP implémenté : plans XYZ/face, décalage, corps explicites, aperçu 3D/2D et DXF ; extension de manipulation directe par flèche et anneaux (translation normale, rotation libre, voir V-1174) ; 347 tests workspace, 23 bureau, 130 UI et contrôles globaux réussis ; 7 DXF natifs relus indépendamment. Parcours graphique/Fusion restants ; restrictions conservatrices dans ADR-0025, V-1172/V-1173. DXF2b source implémenté sans exiger une reconstruction STEP réussie : préparation native, contours polygonaux, même outil de plan et export distinct ; V-1176 |

**Extension du 12 novembre — ellipses exactes.** Après le retour utilisateur sur les faibles inclinaisons du sock-clip, les profils et le DXF prennent en charge les ellipses complètes et partielles, avec aperçu analytique. Les coutures périodiques prouvées ne bloquent plus les coupes centrales régulières ; arêtes entre faces, tangences réelles et courbes non prises en charge restent refusées. Quatorze coupes du fichier privé fraîchement reconverti sont exportées et relues indépendamment ; voir V-1175 pour les réglages et limites. La coupe du maillage source et les intersections toriques obliques générales restent distinctes de cette extension.

Source : [ancienne feuille de route](../../docs/ROADMAP-avant-fabboard.md). Les validations historiques sont conservées ; aucun essai nouveau n’est déduit de la migration.

La livraison de ce lot attend toutes ses vérifications utilisateur rattachées ; une réponse positive à une seule action ne clôt pas les autres.
