# ADR-0024 — Profil DXF d’une face STEP plane

Date : 10 novembre 2026. Statut : implémenté et validé automatiquement ; essais graphiques et Fusion restants.

## Besoin et périmètre

DXF1 permet de sélectionner une seule face plane du STEP courant, de prévisualiser son contour dans son propre plan et d’exporter ce profil en millimètres, à l’échelle 1:1. Contour extérieur et trous sont conservés. Il n’y a ni projection de face courbe, ni dépliage, ni section du solide : ces usages relèvent d’autres lots, notamment DXF2.

## Décisions

- `fabstep-core::profile` contient uniquement les DTO scalaires et l’écriture DXF pure. Le repère est orthonormé direct : origine 3D en mm, axes X/Y et normale ; les courbes 2D restent en mm. Les seules entités sont les lignes, arcs de cercle à balayage signé et cercles entiers.
- `fabstep-cad::planar_face_profile` relit le STEP immuable et partage le parcours de racines/faces de `result_faces`. Les identifiants sélectionnés désignent ainsi la même face et le même solide. Aucun objet OCCT ne traverse l’ABI ; aucun contour n’est déduit des triangles d’affichage.
- Le noyau vérifie le plan, le contour extérieur, les trous, les orientations, les sommets partagés et la totalité des arêtes visitées. `WireExplorer` peut omettre des arêtes dans un contour incorrect : son parcours est confronté à l’inventaire direct. Les raccords sont contrôlés à 1e-7 mm ; aucune fermeture ou correction de coordonnées n’est ajoutée. Une ellipse, spline, arête dégénérée ou frontière incohérente est refusée.
- Le repère prend l’origine et l’axe X du plan situé dans le document ; sa normale respecte l’orientation de la face. Y = normale × X. Les placements et miroirs sont donc pris en compte sans changer les dimensions. Un arc de plus de 180° conserve son domaine paramétrique ; le cercle entier exige également une fermeture topologique.
- L’exporteur produit un DXF ASCII AutoCAD 2000 (`AC1015`, `$INSUNITS = 4`), avec entités `LINE`, `ARC`, `CIRCLE`, calque `OUTER` et calques `HOLE_n`. Les arcs DXF sont antihoraires : un balayage source négatif inverse les angles de début/fin sans changer le lieu géométrique. Le fichier contient le profil dans son XY local, pas le placement du solide en 3D ; le repère de correspondance est consultable dans l’aperçu.

## Fraîcheur et accès aux fichiers

Le bureau reçoit des identifiants, jamais des courbes ou du texte DXF fournis par l’interface. Un contexte de sélection monotone lie requête, source, face, époque moteur et identité exacte `Arc<DocumentStep>`. La géométrie native affichée est conservée dans ce seul contexte ; l’exporteur utilise cette géométrie immuable. Changer de sélection, même A → B → A, abandonne les réponses précédentes.

Extraction et écriture travaillent hors fil UI. La fraîcheur est vérifiée avant/après extraction, avant le dialogue de sauvegarde, puis après le dialogue et jusqu’au remplacement atomique du fichier temporaire. Une réponse tardive de sélection ne peut réactiver un profil ancien. Un échec d’extraction ou d’export ne retire pas le STEP validé ni sa provenance. Les conversions, corrections guidées et recherches de paramètres existantes gardent leurs contrats.

## Limites

Le STEP lu est limité à 128 Mio, 32 racines et 100 000 faces parcourues. Une face comporte au plus 4 096 contours et 100 000 arêtes ; la sortie DXF est limitée à 16 Mio. Le worker d’aperçu applique une limite plus basse de 1 024 contours et 16 384 segments pour conserver une taille d’affichage bornée ; il refuse l’ensemble du profil plutôt qu’un aperçu partiel. Les erreurs ont des codes `dxf.*` stables ; les gardes d’identité utilisent `engine.stale`.

Aucune nouvelle dépendance de l’application. Ezdxf est utilisé uniquement comme lecteur indépendant de validation dans un environnement Python déjà disponible. Le parcours Fusion et les dialogues natifs doivent être distingués des tests automatisés et des contrôles de composant.

## Vérification et références

Tests de profils polygonaux, trous circulaires/polygonaux, arcs signés et majeurs, faces obliques/retournées, placements et plusieurs solides, refus et budgets, fraîcheur pendant les opérations asynchrones. Un corpus natif exporté est relu avec un lecteur DXF indépendant ; voir V-1170/V-1171 dans [VALIDATION](../VALIDATION.md).

Format vérifié dans les références Autodesk : [HEADER et unités](https://help.autodesk.com/cloudhelp/2020/ENU/AutoCAD-DXF/files/GUID-A85E8E67-27CD-4C59-BE61-4DC9FADBE74A.htm), [LINE](https://help.autodesk.com/cloudhelp/2020/ENU/AutoCAD-DXF/files/GUID-FCEF5726-53AE-4C43-B4EA-C84EB8686A66.htm), [ARC](https://help.autodesk.com/cloudhelp/2024/ENU/AutoCAD-DXF/files/GUID-0B14D8F1-0EBA-44BF-9108-57D8CE614BC8.htm), [CIRCLE](https://help.autodesk.com/cloudhelp/2020/ENU/AutoCAD-DXF/files/GUID-8663262B-222C-414D-B133-4A8506A27C18.htm).
