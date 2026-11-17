# ADR-0025 — Coupe du STEP et export DXF

Date : 10 novembre 2026. Statut : étape STEP implémentée et validée automatiquement ; parcours graphique et Fusion restants.

## Périmètre

Première étape DXF2 : définir un plan sur le STEP validé courant, choisir les corps, calculer les contours de section et les exporter en DXF 1:1 en millimètres. Le plan est XY, XZ, YZ ou celui d’une face plane sélectionnée, avec décalage signé selon sa normale. L’origine et les axes du repère sont explicites. L’outil ne découpe pas le solide et ne crée aucun fond. La coupe directe du maillage source, même après refus de conversion, constitue l’étape suivante distincte.

## Représentation et géométrie

`SectionRequest` contient un repère orthonormé direct et la liste triée, unique et non vide des identifiants natifs des corps. `SectionProfile` conserve ce repère et chaque corps demandé ; un corps sans intersection démontrée possède une liste vide. Ses contours portent un identifiant de composante, le rôle extérieur/trou et des lignes/arcs/cercles/ellipses. Chaque composante a exactement un extérieur. Plusieurs composantes disjointes d’un même corps sont possibles.

`fabstep-cad::section_profile` utilise OCCT `BRepAlgoAPI_Section`, sans approximation, indépendamment pour chaque corps. Les identités proviennent du même lecteur que l’inspection STEP. Les contours sont reconstruits par un graphe des sommets natifs partagés : incidence deux, parcours complet et fermeture. Aucun soudage par distance, arête de raccord ou ajustement de coordonnées n’est ajouté. Les points isolés, embranchements et arêtes communes ambiguës sont refusés. Les distances entre contours sont contrôlées avant classement par inclusion ; les faces temporaires utilisées pour ce classement ne sont pas des bouchons ajoutés au modèle.

Les niveaux critiques des surfaces analytiques support sont vérifiés avant de se fier à un résultat vide. Cela évite de confondre une tangence ou singularité omise par l’opération avec une absence d’intersection. Les familles de courbes admises sont limitées aux intersections analytiques prouvées ; les contrôles de normales ponctuels restent secondaires et ne constituent pas une preuve globale de transversalité. Les tests couvrent notamment tangence sphérique oblique éloignée d’une couture, niveau tangent du tore et sommet conique.

`planar_face_frame` permet de reprendre le plan situé d’une face, indépendamment de la possibilité d’exporter ses propres contours en DXF1. Un contour elliptique sur une face plane ne rend donc pas son plan inutilisable. Les coordonnées restent celles du STEP en mm, y compris après unités STL/3MF, placements et miroirs.

## Export et limites assumées

L’export réutilise le format DXF1 et ses entités exactes. Les calques `BODY_{solid_id}_COMPONENT_{component_id}_OUTER/HOLE_n` distinguent corps et composantes ; les identifiants sont natifs à partir de zéro, les trous sont numérotés à partir de un. Un document entièrement vide est prévisualisable mais non exportable (`dxf.section_empty`). Si un seul corps choisi est invalide ou non pris en charge, toute la section est refusée : aucun export partiel.

Le premier lot refuse notamment :

- Une coupe coplanaire avec une face native, ou contenant une arête native commune, à l’exception des coutures périodiques prouvées sur une unique face (extension du 12 novembre).
- Un niveau critique de la surface support lorsque la boîte conservatrice de la face ne permet pas d’exclure une intersection, même si le point tangent est hors de la face tronquée. Tout plan passant par le sommet support d’un cône est refusé ; certaines coupes axiales d’un tronc de cône régulier sont donc également refusées.
- Les coupes de tore autres que les cercles admis des plans normaux à l’axe ou des plans méridiens contenant l’axe ; le tore doit être annulaire. Les coutures périodiques prouvées du plan médian sont désormais admises.
- Les splines, paraboles, hyperboles et autres courbes non prises en charge. Aucune approximation n’est proposée silencieusement.

Ces restrictions sont des refus conservateurs ; elles ne signifient pas nécessairement que la section visible demandée serait géométriquement incorrecte.

Budgets : STEP 128 Mio, 32 corps, 100 000 faces ; 100 000 arêtes et 4 096 contours au total, parcours source limité à 600 000 incidences et classement plafonné à un million d’unités de travail sur les paires d’arêtes. Export 16 Mio. Aperçu : 1 024 contours et 16 384 segments cumulés sur tous les corps ; au-delà, refus complet. Le décalage UI/natif est fini et limité à ±1 000 000 000 mm. L’appel OCCT engagé n’est pas interrompu au milieu.

## Fraîcheur et interface

Le module bureau `section.rs` associe une époque de section monotone à la source, la révision et l’identité exacte du STEP. Le contexte contient uniquement la requête résolue puis le profil natif immuable ; aucune géométrie arbitraire ne provient de l’interface pour l’export. Les commandes sont `set_section_context`, `section_profile`, `export_section_dxf`. La définition du contexte renvoie le repère avant calcul, pour afficher le plan même si la coupe est ensuite refusée. La sauvegarde revérifie la fraîcheur avant et après le dialogue puis jusqu’à l’écriture atomique.

L’outil est contextuel et fermé au retour Source ou après invalidation du résultat. La vue 3D montre un plan translucide non sélectionnable, son contour et la normale, sans masquer ni couper le solide. Le panneau offre le repère, le décalage, les corps, le profil 2D et l’export. Une nouvelle édition retire immédiatement l’ancien export par un contexte nul léger ; une paire d’époques distinctes empêche qu’une invalidation tardive efface la nouvelle coupe. La chaîne complète résolution/calcul/préparation n’a qu’une demande active et une dernière demande en attente ; les éditions intermédiaires sont remplacées, avec regroupement de 150 ms. Les coordonnées SVG sont préparées hors fil UI par le worker partagé avec DXF1, dans une instance indépendante.

## Validation et références

Tests natifs des contours avec trous, composantes disjointes, corps vides, placements/miroirs, repère oblique, refus sans export partiel, tangences/singularités et limites. Tests bureau/UI de changement de plan/corps/source/résultat, messages désordonnés, sauvegarde et travail borné. Corpus relu par ezdxf indépendamment, suite globale et limites visuelles : V-1172/V-1173 dans [VALIDATION](../VALIDATION.md).

Les APIs sont vérifiées dans les headers OCCT 7.9 installés ; documentation de principe : [BRepAlgoAPI_Section](https://www.occt3d.com/dev/doc/refman/html/class_b_rep_algo_a_p_i___section.html), [opérations booléennes](https://www.occt3d.com/dev/doc/overview/html/specification__boolean_operations.html). La documentation en ligne courante ne remplace pas le contrat des headers 7.9 utilisés pour compiler.

### Manipulation directe du plan

Extension du 10 novembre : translation selon la normale et rotation autour des deux axes locaux par poignées Three.js, sans nouvelle dépendance. Le pivot est le centre des bornes de la pièce projeté sur le plan, capturé au début du geste. La première manipulation recale l’origine du repère DXF sur ce pivot : le lieu du plan et les dimensions sont conservés, mais les coordonnées XY locales du profil peuvent changer ; le nouveau repère reste affiché dans l’aperçu. Le décalage numérique reste signé selon la normale courante ; les plans prédéfinis réinitialisent le repère personnalisé. Le contrat IPC ajoute `basis: "custom"` et `custom_frame` optionnel ; les anciennes entrées restent valides. Le bureau refuse les repères ambigus, non orthonormaux, indirects ou hors limites. Cette étape conservait initialement le refus des ellipses ; cette limite est levée par l’extension ci-dessous.

L’affichage suit le geste localement, sans lancer de section native à chaque mouvement. Le contexte exportable est invalidé dès le début ; seul le dernier plan relâché est recalculé. Annulation du geste et fermeture de l’outil doivent libérer les contrôles de navigation et ne jamais rétablir un export périmé.


### Ellipses exactes et coutures périodiques — 12 novembre

L’utilisateur autorise explicitement l’extension du format de courbes après reproduction du refus d’une faible inclinaison sur le sock-clip. `Ellipse` contient `center_mm`, `major_axis_mm`, `ratio`, `start_param_rad` et `sweep_param_rad`. Le petit axe local vaut `ratio * [-major_y, major_x]` ; le balayage signé conserve le parcours natif. Un tour entier n’est admis que seul dans une boucle fermée native. Le writer DXF inverse les bornes des parcours horaires pour préserver le lieu géométrique dans l’entité ELLIPSE, sans polyligne ni fermeture inventée. L’aperçu SVG et ses dimensions sont calculés analytiquement dans le worker.

Les sections elliptiques exactes de cylindres et de cônes sont admises. Pour les cônes, le refus global des plans passant par le sommet support reste actif ; une tangence conique contient ce sommet. Les intersections toriques obliques générales restent hors périmètre. Les profils DXF1 de faces planes bénéficient également de l’extraction elliptique partagée.

Les boîtes OCCT de géométrie native, sans triangulation et élargies par les tolérances, permettent seulement de démontrer l’absence d’intersection. Toute boîte inutilisable conserve les contrôles précédents. Une arête de couture doit appartenir à une seule face distincte, avoir deux courbes paramétriques selon `BRep_Tool::IsClosed`, et être retrouvée par identité topologique exacte si l’opération ne fournit pas d’ancêtre. Les familles analytiques et la transversalité restent contrôlées. Aux pôles sphériques, la normale géométrique radiale remplace la normale paramétrique indéfinie. Aucun raccord entre deux faces n’est assimilé à une couture.

Références : [entité ELLIPSE Autodesk](https://help.autodesk.com/cloudhelp/2023/ENU/AutoCAD-DXF/files/GUID-107CB04F-AD4D-4D2F-8EC9-AC90888063AB.htm), [définition des coutures OCCT](https://occt3d.com/dev/doc/refman/html/class_b_rep___tool.html). Contrats vérifiés également dans les headers du SDK utilisé. Preuves et limites : V-1175.
