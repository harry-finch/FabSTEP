# fabstep-core

Analyse géométrique Rust pure et déterministe, sans dépendance CAO. Les API
`analyze_stl` et `analyze_mesh` analysent un corps ; `analyze_stl_bodies` et
`analyze_mesh_bodies` séparent plusieurs composantes. Les reconnaisseurs explicites
produisent des modèles analytiques et la provenance des triangles ; leur périmètre
actuel est détaillé dans la dernière section. Toutes les coordonnées et tolérances
internes sont en millimètres.

Le pipeline lit les STL ASCII ou binaires, même lorsque l'en-tête binaire
commence par `solid`. Les normales déclarées dans le fichier sont ignorées.
Les sommets sont soudés à distance euclidienne réelle dans les 27 cellules
voisines ; le représentant admissible d'indice minimal est retenu. Ce soudage
est déterministe pour un même fichier et n'effectue pas une fermeture transitive.

`analyze_stl` et `analyze_mesh` acceptent un seul composant fermé. Leurs variantes
`*_bodies` admettent jusqu’à 32 composantes, sous les limites globales décrites en M3.
Les triangles dégénérés ou dupliqués, les
arêtes ouvertes ou non manifold, les sommets pincés et les orientations locales
incohérentes sont refusés. Une inversion globale est corrigée avec un diagnostic.
Le volume utilise une origine locale et une sommation compensée.

Les régions planes se développent autour du plan du triangle initial, avec une
distance maximale configurable et un produit scalaire des normales supérieur ou
égal à `1 - 1e-10`. Il n'y a ni projection des sommets ni simplification locale
des arêtes : les limites partagées restent identiques. Les contours doivent être
fermés et simples ; le premier est extérieur, les suivants sont des trous non
imbriqués. Leur orientation est relative à la normale sortante de la face.

## Limites explicites

- 64 Mio, 100 000 triangles et 5 millions de comparaisons pour les contours.
- Coordonnées et tolérances finies, strictement positives pour les tolérances.
  Une étendue incompatible avec la grille de soudage est refusée.
- Aucun remplissage de trou, aucune suppression silencieuse de triangle.
- Pas d'import 3MF ni d'export STEP dans cette crate. Le mode `Planar`, activé
  par défaut, conserve les facettes des surfaces courbes.
- Les intersections entre faces dans l'espace ne sont pas vérifiées ici. Le
  rapport porte toujours `validation.native_required` : le noyau CAO doit encore
  valider le solide. Une topologie fermée ne suffit pas à garantir sa validité.
- Les prédicats 2D emploient des calculs flottants conservateurs, pas de
  prédicats arithmétiques exacts. Les cas ambigus peuvent être refusés.
- Les faces presque planes conservent leurs sommets originaux et peuvent être
  refusées par un noyau plus strict ; `planar.approximate` signale ces régions.
- Les API monocomposants refusent plusieurs coques. Les API `*_bodies` les
  séparent sans prouver leur séparation spatiale : une coque de cavité ne doit
  pas devenir un solide positif imbriqué. Le contrôle natif de l’application
  refuse contacts, recouvrements et imbrications avant tout export.

Les erreurs portent des codes stables (`mesh.open`, `mesh.non_manifold_edge`,
`mesh.inconsistent_orientation`, etc.) et un message lisible en français.

## Reconnaissance M2, explicitement activée

`Options.reconstruction = ReconstructionMode::Cylinders` active l'interprétation
de profils polygonaux circulaires en cercles. Le résultat est une
`AnalyticExtrusion` indépendante du modèle planaire source : origine, axe unitaire,
axe X perpendiculaire, hauteur, contour extérieur et trous. Les polygones sont
orientés extérieurement CCW et intérieurement CW dans le repère
`(x_axis, axis × x_axis)` ; seuls les sommets colinéaires de ce nouveau profil
sont simplifiés. Les sommets et contours d'`Analysis` sont conservés.

Les deux bouchons doivent être parallèles et leurs contours correspondre, avec
le même nombre de sommets par boucle, à un décalage cyclique près. Une
resegmentation interne utilise `max(plane_tolerance_mm, cylinder_fit_tolerance_mm)`
pour retrouver les bouchons, notamment après quantification binaire f32.
Elle conserve la provenance exacte de chaque triangle. Un triangle proche d'un
bouchon en altitude mais appartenant à une paroi est toujours vérifié comme une
paroi : les petites collerettes ne sont pas supprimées.

Chaque triangle latéral doit correspondre entièrement à un segment de profil
extrudé, dans la tolérance. Tous ses sommets et son orientation sont vérifiés,
y compris les anneaux intermédiaires. Les variations de section dépassant cette
tolérance, cônes, étranglements, épaulements et trous borgnes sont refusés.

Un cercle exige au moins 12 échantillons, un écart angulaire maximal de 45°,
une couverture complète et monotone du tour, ainsi qu'un résidu radial maximal
inférieur à `cylinder_fit_tolerance_mm` (10⁻⁵ mm par défaut). L'ajustement est
centré et normalisé ; aucun point aberrant n'est éliminé silencieusement. La
flèche maximale est calculée sur les angles réellement présents, y compris les
échantillonnages non uniformes. Les remplacements ne doivent pas créer de
contact entre trous ou entre un trou et le contour extérieur.

`max_deviation_bound_mm` additionne la tolérance de soudage, la borne du résidu
d'extrusion, le résidu radial et la flèche des cordes. Le résidu d'extrusion
additionne le maximum des distances mesurées aux parois associées et les
allocations de résidu de planéité et de correspondance des bouchons. Cette borne
conservatrice doit rester inférieure à `max_deviation_mm` (0,05 mm par défaut).
Il ne s'agit pas d'une mesure Hausdorff globale bidirectionnelle. Le noyau CAO
doit valider la coque source, puis le résultat analytique et son réimport STEP.
Le volume attendu est celui du profil analytique extrudé, et non celui du
polygone tessellé ; leurs différences de volume sont normales.

Un polygone régulier peut être intentionnel. Le mode est donc opt-in et produit
`recognition.circular_intent_assumed`. Il n'existe aucun repli implicite vers le
mode planaire : formes non prises en charge, budget dépassé et complexité
excessive donnent respectivement `recognition.unsupported`,
`recognition.deviation_exceeded` et `recognition.too_complex`. Le budget de
reconnaissance est limité à 10 millions d'opérations de recherche/comparaison,
en plus des limites déjà appliquées à la segmentation.

## M2b : pièces mixtes, trous borgnes et bossages

`recognize_mixed` est additive : la reconnaissance M2a d'extrusion complète reste
inchangée. Le modèle mixte contient des plans conservés, des anneaux circulaires
partagés et des parois cylindriques complètes. Chaque anneau appartient à
exactement un plan réel et un cylindre ; les axes communs sont unitaires, les
rayons identiques et les centres ordonnés positivement selon l'axe. `inner`
indique une paroi intérieure, dont la normale pointe vers l'axe.

La détection extrait les boucles circulaires des régions planes avec provenance.
L'appairage exige des plans parallèles, une même section et des contours
correspondants. Une exploration par adjacence construit ensuite le patch de
paroi : tous ses triangles sont contrôlés géométriquement, son orientation
radiale doit être constante et sa frontière doit être exactement égale aux deux
anneaux source. Aucun triangle voisin refusé ne peut être ignoré dans cette
preuve de frontière. Les régions planes partiellement consommées sont refusées.

Un fond borgne n'est jamais créé : sa face et son contour doivent exister dans
la source. Le même mécanisme reconnaît les bossages et les trous traversants.
Tous les anneaux candidats doivent avoir une preuve unique ; une paroi conique,
tronquée, intersectée ou munie d'un détail intermédiaire ne devient pas une
conversion partielle silencieuse. Les autres faces/facettes sont conservées
comme plans, sans prétendre y reconnaître une surface courbe.

`triangle_faces` donne une attribution exhaustive et disjointe pour tous les
triangles source. Le rapport conserve les résidus d'ajustement et de cordage ;
il ajoute à la borne d'extrusion le résidu mesuré de tous les triangles des
plans conservés. Le rayon commun final respecte strictement
`cylinder_fit_tolerance_mm`, indépendamment d'une tolérance de planéité plus
grande. Le volume attendu est calculé sur la frontière analytique par le théorème
de divergence, avec une origine locale : contributions planes `A·d/3` et
cylindriques signées `2πr²h/3`. Il ne s'agit pas d'une correction heuristique du
volume du maillage.

La construction CAO doit partager les cercles entre surfaces et contrôler la
couture, les incidences, les auto-intersections et le réimport. Les pièces
mixtes quantifiées en f32 peuvent être refusées lorsque leurs plans conservés
dépassent la tolérance native : aucune projection locale de sommets n'est faite.
Les congés et intersections cylindre/cylindre ne sont pas reconstruits par ce
jalon. Les arcs partiels disposent de l'extension conservatrice décrite ci-dessous.

Le cas `m2b_boss_blind_rotated` quantifié en STL binaire f32 illustre deux
limitations distinctes. Avec les valeurs par défaut, la resegmentation produit
144 régions et seulement trois des quatre anneaux nécessaires : le plan initial
d'un triangle fragmente le dessus du bossage. Les résidus radiaux des anneaux
détectés sont pourtant inférieurs à 1 µm ; les défauts de coaxialité mesurés
sont inférieurs à 0,8 µm. Choisir explicitement une tolérance de planéité de
`2e-5` mm retrouve les quatre anneaux, avec la tolérance radiale inchangée et
une borne composée d'environ 0,010263 mm. Ce succès de reconnaissance ne garantit
pas l'export : les plans conservés légèrement non plans peuvent ensuite être
refusés par la tolérance native d'environ `1e-7` mm. Aucun seuil n'est augmenté
silencieusement et aucune projection globale des sommets partagés n'est faite.

## Arcs cylindriques ouverts (M2c)

Le mode explicite `Cylinders` reconnaît aussi une paroi circulaire extrudée
entre deux contours plans, avec au moins huit sommets, une ouverture de 30 à
330 degrés et des pas angulaires d'au plus 45 degrés. Le cercle est ajusté sous
la contrainte de passer par les deux extrémités source : aucun sommet partagé
n'est déplacé. Les deux profils doivent correspondre, et les quatre frontières
de la paroi sont prouvées : deux chaînes circulaires et deux génératrices.
Toutes les facettes intermédiaires sont contrôlées et reçoivent une provenance
`PartialCylinder`. Les cônes, variations intermédiaires et parois incomplètes
sont refusés par `recognition.partial_unproven`, sans tentative opportuniste
sur un sous-arc plus court.

Un contour entièrement cocirculaire n'est pas découpé pour contourner les
critères d'échantillonnage des cylindres complets. Une exception explicite
permet les profils en D : un unique intervalle angulaire d'au moins 90 degrés
désigne la corde, tous les autres intervalles étant d'au plus 45 degrés.
L'hypothèse d'intention circulaire reste propre au mode `Cylinders`.
L'ajout d'un sommet collinéaire sur la corde ne change pas cette reconnaissance.

`MixedLoop::Segments` partage les arcs avec `MixedPartialCylinder`. Les rails
peuvent perdre leurs sommets intermédiaires uniquement s'ils sont collinéaires,
strictement monotones et adjacents aux deux seules régions attendues. Leurs
incidences sont préindexées ; la recherche de candidats, les comparaisons de
maximalité et les remplacements de contours consomment le budget de travail.
Une géométrie très dense peut renvoyer `recognition.too_complex` (par exemple
un arc de 512 segments). La croissance d'un candidat s'arrête au premier
ajustement refusé : un arc bruité dont un préfixe dépasse la tolérance peut
être refusé même si un ajustement plus long aurait réussi. Aucun seuil n'est
relâché pour ces cas.

La borne locale combine flèche des cordes, résidus radiaux, correspondance des
profils, résidus de paroi et déplacement de soudage ; ce n'est pas une preuve
de distance de Hausdorff globale. L'aire plane inclut les segments circulaires
signés et le flux du cylindre partiel inclut le terme dépendant de son centre.
Le volume analytique reste ainsi indépendant d'une correction approximative
du volume source. La construction et le réimport natifs restent obligatoires.

## Validation

`cargo test -p fabstep-core`

Les fixtures ASCII sont générées indépendamment du convertisseur par
`fixtures/generate.py`. Résultats attendus : cube, 12 triangles, 6 faces, volume
1 mm³ ; prisme concave, 20 triangles, 8 faces, volume 6 mm³ ; plaque à trou
rectangulaire traversant, 32 triangles, 10 faces, volume 24 mm³, deux faces
possédant chacune un contour intérieur.

Le corpus indépendant `fixtures/generate_m2.py` couvre cylindre, tube et plaque
percée, subdivisions axiales, rotation/translation, cône et étranglement.
Les tests ajoutent la quantification binaire f32, un budget insuffisant, les
arcs partiels, les points aberrants et une saillie radiale de 1 mm cachée dans la
tranche de tolérance d'un bouchon. Cette dernière doit être refusée.

Le corpus indépendant `fixtures/generate_m2b.py` couvre les fonds borgnes, les
bossages, leurs combinaisons, deux perçages, subdivisions et rotations. Les
négatifs portent sur cône, fond non plan, saillie intermédiaire et faible
échantillonnage. Un test indépendant échantillonne les triangles source vers
chaque cylindre, puis 128 angles × 5 hauteurs par cylindre vers les triangles
source avec un calcul de distance point/triangle séparé. Ce contrôle local
bidirectionnel est un test échantillonné, pas une garantie Hausdorff globale.

Le corpus `fixtures/generate_m2c.py` couvre les arcs ouverts, subdivisions,
rotations, combinaisons avec trous et bossages, puis cônes, saillies, profils
discordants et sous-échantillonnage. Les régressions core construisent aussi
un cercle complet à deux intervalles de 50 degrés (refusé quel que soit le
départ du contour) et un profil en D avec et sans subdivision de sa corde.

## Maillages indexés et corps multiples (M3)

`read_stl_mesh(bytes, input_unit)` fournit un aperçu borné en millimètres sans
validation topologique. `analyze_mesh` et `analyze_mesh_bodies` reçoivent des
coordonnées déjà en mm : `Options.input_unit` est ignorée uniquement pour ces
API, les tolérances restant actives. Les indices et coordonnées sont vérifiés
avant accès. `analyze_stl_bodies` conserve la conversion d'unité STL explicite.
Les appels monocomposants historiques continuent de refuser plusieurs coques.

Le découpage suit les adjacences d'arêtes après soudage, avec un maximum de
32 corps et 100 000 triangles. Chaque corps est réanalysé depuis les coordonnées
originales pour conserver ses propres diagnostics de déplacement et orientation.
Les occurrences 3MF doivent être fournies séparément : aucun soudage entre
instances. Le test topologique ne prouve pas la séparation spatiale. Les corps
qui se touchent, se recouvrent ou s'emboîtent doivent encore être refusés par le
noyau natif avant export ; notamment une coque de cavité ne doit pas devenir
silencieusement un second solide positif.


## Cônes et chanfreins circulaires (M4a)

Le mode explicite `Primitives` ajoute `RevolvedRecognition`. Son `RevolvedModel` contient un modèle mixte et des cônes référençant deux anneaux. Un anneau de rayon nul représente uniquement une pointe existante prouvée ; l’API mixte historique conserve son interdiction. Les axes viennent de contours circulaires plans source. Les anneaux exigent au moins douze points, un pas maximal de 45°, des échantillons angulaires compatibles et une couverture triangulaire complète. Les bandes coniques collinéaires sont regroupées. La limite initiale d’une plage conique par chaîne admet une exception prouvée : exactement deux cônes de pentes opposées séparés par une portée cylindrique résolue. La hauteur de cette portée et les intervalles d’incertitude des pentes doivent satisfaire le garde partagé de l’[ADR-0010](../../docs/adr/0010-resolved-revolutions.md). Les séparateurs presque cylindriques ambigus, pentes de même signe et chaînes arbitraires restent refusés.

Chaque triangle reçoit une face de provenance. La correspondance des sommets avec les facettes idéales, les résidus de paroi, la flèche et le soudage composent la borne ; le volume analytique provient du flux des surfaces. `recognition.conic_unproven` bloque tout repli pour un candidat non démontré ; seule l’absence de candidat (`recognition.unsupported`) autorise l’appel historique par l’orchestrateur. Les budgets restent explicites. Ce reconnaisseur ne construit pas de cône partiel, sphère, tore ou congé. Les arcs cylindriques ouverts ne sont pas promus par ce modèle ; ils restent disponibles dans le chemin historique sans cône.

Les tests M4 et `double_chamfers.rs` couvrent formes indépendantes, pointes source, incidences, provenance, volumes, budgets, opt-in, séparation des chanfreins et refus non affines. Le noyau natif doit encore vérifier source, résultat, réimport et fidélité. Une conicité très faible répartie sur de nombreux niveaux peut être refusée par ces critères conservateurs.

## Hémisphères et congés toriques (M5a)

Le mode explicite `Surfaces` ajoute `RoundedRecognition`. `RoundedModel` contient
un `RevolvedModel`, des portions sphériques et des portions toriques. Les anneaux
partagés et les pôles prouvés restent des données Rust ; aucun fichier de
référence CAO n'intervient dans la reconnaissance.

Le sous-ensemble initial couvre un hémisphère délimité par un équateur source et
un pôle source, ainsi qu'un congé torique quart de cercle raccordant un cylindre
et un plan. Il peut être combiné aux plans, cônes et cylindres complets ou
partiels du même corps. Ce socle M5a a depuis été étendu aux calottes et complété
par des reconnaisseurs terminaux distincts, décrits ci-dessous. Les raccords
à rayon variable et les surfaces libres restent hors des contrats actuels.

Les contours circulaires isolent des patchs topologiques. La preuve n'exige pas
de correspondance entre les sommets des anneaux : la demi-sphère accepte une
triangulation irrégulière et les autres parois une paramétrisation orientée avec
couverture complète. Tous les triangles ont une provenance disjointe. Les
surfaces candidates non prouvées renvoient `recognition.surface_unproven` ; seul
`recognition.unsupported` permet à l'orchestrateur d'essayer les modes précédents.

La borne sphérique utilise la projection des sommets puis la distance minimale
à l'origine des facettes idéales. La borne torique utilise les dérivées secondes
de la surface paramétrée et les étendues angulaires de chaque triangle. Les
résidus de projection, les plans conservés et le soudage restent comptés. Le
volume attendu est calculé par flux des surfaces, indépendamment du volume du
maillage. L'ajustement radial et le budget d'écart gardent leurs valeurs initiales
et restent deux critères distincts. Ces preuves locales ne remplacent ni les
contrôles natifs ni l'audit de distance échantillonné.

Les diagnostics `recognition.rounded_intent_assumed` et
`recognition.rounded_coverage` décrivent l'interprétation et l'attribution des
triangles. Les comptes `spheres`, `tori` et `toroidal_fillets` sont séparés :
les congés du parcours par patchs ont un raccord cylindre–plan tangent prouvé,
contrairement au contrat distinct du tore complet.
Le corpus et les essais exécutés sont décrits dans [VALIDATION](../../docs/VALIDATION.md).


## Reconnaisseurs actuels en mode Surfaces

| API | Modèle et périmètre démontré |
| --- | --- |
| `recognize_rounded` | `RoundedRecognition` : calottes convexes ou concaves avec anneau et pôle source, congés toriques quart cylindre–plan, parois planes/coniques/cylindriques associées et arcs ouverts. Le double chanfrein opposé résolu peut aussi utiliser sa preuve UV sans anneaux appariés, même sans sphère ni tore. |
| `recognize_rounded_box` | `RoundedBoxRecognition` isolé : six rectangles plans source, douze quarts de cylindre et huit octants sphériques à rayon commun. Attribution des 26 domaines, coutures partagées, orientation locale et couverture globale de degré un. |
| `recognize_complete_torus` | `RoundedRecognition` isolé : tore annulaire complet R > r, sans anneaux source imposés. Ajustement normalisé, raffinement géométrique borné, résidus physiques et couverture UV orientée de degré un. |
| `recognize_complete_sphere` | `RoundedRecognition` isolé : sphère entière sans anneau ni pôle inventé. Ajustement normalisé, topologie χ = 2, orientation et couverture 4π. |

Les calottes sont décrites dans l’[ADR-0008](../../docs/adr/0008-spherical-caps.md),
les sphères complètes dans l’[ADR-0009](../../docs/adr/0009-complete-spheres.md),
les doubles chanfreins et tores complets dans l’[ADR-0010](../../docs/adr/0010-resolved-revolutions.md),
et le boîtier convexe entièrement arrondi dans l’[ADR-0011](../../docs/adr/0011-uniform-rounded-box.md).
Les boîtiers creux ou percés, arrondis partiels, rayons variables et congés
généraux ne sont pas couverts par le dernier contrat.

L’orchestrateur `fabstep-app`, et non le core, décide des replis. En mode
Surfaces, il conserve tous les succès et refus bloquants du parcours historique
Surfaces → Primitives → Cylindres. Seulement après `recognition.unsupported` de
ce parcours, il essaie le boîtier arrondi complet, puis le tore complet, puis
la sphère complète. Chaque nouvel essai exige encore `recognition.unsupported`
du précédent ; une preuve échouée, une déviation excessive ou un budget dépassé
reste bloquant. Aucun passage implicite au mode Plans n’a lieu.

Tous ces contrats conservent la source et attribuent chaque triangle. Les
corrections des sommets, coutures et frontières sont comptées dans les résidus ;
les bornes incluent les intérieurs triangulaires et restent indépendantes du
fit. Les tores complets peuvent être raffinés même sous le fit pour réduire le
biais algébrique f32, mais chaque pas doit améliorer l’objectif géométrique sans
invalider un maximum déjà admissible. Aucun seuil ni budget n’est augmenté.
La construction, le réimport et la séparation natifs, puis l’audit bidirectionnel,
restent obligatoires avant export. Pour le boîtier, le natif vérifie également
les 26 faces et les 48 raccords ; sa permission d’octants est limitée à ce corps.
