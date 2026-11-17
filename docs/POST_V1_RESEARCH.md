# Recherche géométrique après la v1

Recherche du 9 novembre 2026, préalable au développement. Point de départ : code de la v1 locale `1d8c26c`, [architecture](../ARCHITECTURE.md), [ADR géométriques](adr/) et [corpus mécanique indépendant](../fixtures/complex_v1/README.md). Ce document distingue les capacités observées dans ce code des propositions. Les critères ci-dessous ne constituent pas des tests exécutés ; l'avancement des implémentations et leurs résultats appartiennent à [ROADMAP](../ROADMAP.md) et [VALIDATION](VALIDATION.md).

La suite recommandée étend les contrats existants par familles vérifiables. Une refonte du moteur, un changement de noyau ou une dépendance de reconnaissance externe ne sont pas nécessaires pour commencer. Les premières tranches réutilisent les modèles mixtes, la provenance des triangles, les budgets et le contrôle natif. Les courbes d'intersection et les surfaces libres demandent ensuite des contrats supplémentaires, sans déplacer OCCT dans le core.

## Ce qui existe et ce qui manque

| Observation dans la v1 | Conséquence pour la suite |
| --- | --- |
| `rounded.rs::recognize_rounded` assemble plans, cylindres, cônes, sphères et tores, puis appelle `partial::augment` pour les cylindres ouverts. L'admission initiale exige une sphère, un tore ou la chaîne à deux cônes opposés. | La combinaison d'un cône seul et d'une encoche ouverte dispose déjà des données et des constructeurs nécessaires ; son admission peut être élargie de manière ciblée en Surfaces. |
| `revolved.rs::resolved_double_chamfer` prouve un séparateur cylindrique et des pentes opposées. | Supprimer simplement la limite du nombre de cônes transformerait potentiellement des courbes en petits cônes. Les chaînes supplémentaires nécessitent une preuve de changements de pente résolus. |
| `MixedSphere.rings: Some([i,j])`, le flux dans `rounded.rs::volume` et `occt.cpp::spherical_face` représentent déjà deux limites de latitude. | Une zone sphérique sans pôle peut réutiliser le contrat sérialisé et l'ABI, mais la preuve actuelle `prove_sphere` dépend d'un pôle et d'une topologie de disque. |
| Le boîtier utilise un contrat isolé `RoundedBoxModel`. Le contrôle natif réserve les huit octants sphériques à ses 26 faces. | Un trou ou un arrondi manquant ne doit pas être accepté en relâchant globalement les vérifications de ce modèle. |
| Le modèle mixte décrit principalement segments, cercles et arcs ; les cônes et tores partiels sont refusés nativement. | Les intersections de perçages et certains congés demandent des courbes de découpe et des domaines paramétriques supplémentaires. |
| `measure_solid` exige une unique coque fermée. La séparation des composantes refuse les corps imbriqués. | Un bac ouvert avec une paroi d'épaisseur réelle est distinct d'une cavité totalement enfermée à plusieurs coques. Le second cas exige une décision explicite sur la sémantique des corps. |
| Les surfaces natives autorisées sont planes, cylindriques, coniques, sphériques et toriques. | Une B-spline ou une surface de révolution générale nécessite un modèle, des mesures et un contrôle natif dédiés ; elle ne peut pas être ajoutée comme simple tolérance de reconnaissance. |

Les fichiers de code principaux sont [rounded.rs](../crates/fabstep-core/src/rounded.rs), [revolved.rs](../crates/fabstep-core/src/revolved.rs), [partial.rs](../crates/fabstep-core/src/partial.rs), [types du core](../crates/fabstep-core/src/lib.rs), [dispatch et validation app](../crates/fabstep-app/src/lib.rs), [frontière Rust du CAD](../crates/fabstep-cad/src/lib.rs) et [pont OCCT](../crates/fabstep-cad/src/occt.cpp).

## Enseignements des sources primaires

Les liens OCCT de cette étude ciblent le code officiel **V7_9_3**, cohérent avec le SDK du projet. Les anciennes URL de documentation 7.9 consultées redirigent vers une documentation plus récente ; leurs signatures ne servent donc pas de preuve de compatibilité avec le SDK local.

- OCCT peut construire une révolution à partir d'une géométrie génératrice. Une face balayée donne un solide, tandis qu'un fil donne une coque. Cette capacité suppose une génératrice déjà définie ; elle ne retrouve ni son existence ni son intention dans un STL. [BRepPrimAPI_MakeRevol, V7_9_3](https://github.com/Open-Cascade-SAS/OCCT/blob/V7_9_3/src/BRepPrimAPI/BRepPrimAPI_MakeRevol.hxx)
- `Geom_SurfaceOfRevolution` définit les coordonnées angulaire et méridienne. Son constructeur ne vérifie pas que la courbe et l'axe soient coplanaires ni que la courbe ne s'auto-intersecte. Ces conditions doivent appartenir à la preuve FabSTEP. [Geom_SurfaceOfRevolution, V7_9_3](https://github.com/Open-Cascade-SAS/OCCT/blob/V7_9_3/src/Geom/Geom_SurfaceOfRevolution.hxx)
- `BRepFilletAPI_MakeFillet` accepte un rayon constant, une évolution linéaire et une loi de rayon. Il propage un contour d'arêtes tangentes et expose des échecs de construction. `HasResult` peut désigner un résultat partiel troué : cette valeur ne suffit jamais à autoriser un export FabSTEP. [BRepFilletAPI_MakeFillet, V7_9_3](https://github.com/Open-Cascade-SAS/OCCT/blob/V7_9_3/src/BRepFilletAPI/BRepFilletAPI_MakeFillet.hxx)
- Les opérations booléennes distinguent les intersections et la construction des morceaux résultants. Leur notion d'interférence dépend aussi des tolérances des entités. La reconnaissance d'un perçage traversant une autre paroi doit fournir les limites et la matière conservée avant d'utiliser ces outils. [Spécification des opérations booléennes, V7_9_3](https://github.com/Open-Cascade-SAS/OCCT/blob/V7_9_3/dox/specification/boolean_operations/boolean_operations.md)
- `BRepAlgoAPI_Section` produit des arêtes et sommets d'intersection ; cela ne prouve pas à lui seul la fermeture d'un solide. Ses courbes paramétriques sur les supports requièrent aussi une attention explicite. [BRepAlgoAPI_Section, V7_9_3](https://github.com/Open-Cascade-SAS/OCCT/blob/V7_9_3/src/BRepAlgoAPI/BRepAlgoAPI_Section.hxx)
- `GeomAPI_PointsToBSplineSurface` ajuste une surface à un tableau organisé de points dont les indices correspondent aux directions paramétriques. Un maillage triangulaire arbitraire n'est pas directement ce tableau. La tolérance sur les points fournis ne constitue pas une borne globale sur les triangles et les raccords. [GeomAPI_PointsToBSplineSurface, V7_9_3](https://github.com/Open-Cascade-SAS/OCCT/blob/V7_9_3/src/GeomAPI/GeomAPI_PointsToBSplineSurface.hxx)
- GeomPlate construit une surface sous contraintes de points et de courbes. `GeomPlate_MakeApprox` distingue l'erreur d'approximation de cette surface de celle évaluée sur ses contraintes ; aucune n'est par définition une preuve de fidélité au maillage entier. [GeomPlate_BuildPlateSurface](https://github.com/Open-Cascade-SAS/OCCT/blob/V7_9_3/src/GeomPlate/GeomPlate_BuildPlateSurface.hxx), [GeomPlate_MakeApprox](https://github.com/Open-Cascade-SAS/OCCT/blob/V7_9_3/src/GeomPlate/GeomPlate_MakeApprox.hxx)
- Efficient RANSAC reconnaît des primitives à partir de points et normales, avec une recherche probabiliste et des points éventuellement non attribués. Cette technique peut inspirer une recherche de candidats, mais ne remplace pas une attribution complète et une preuve déterministe. Aucun ajout de CGAL au projet n'est proposé ici. [Manuel primaire CGAL Shape Detection](https://doc.cgal.org/latest/Shape_detection/index.html)
- Le contrôle BRep vérifie notamment la fermeture, l'orientation, les contours et la correspondance des courbes 3D avec leurs représentations sur les faces. Le mode exact de contrôle courbe/surface est distinct du contrôle par points et concerne les arêtes `SameParameter`. Les auto-interférences restent un contrôle complémentaire. [BRepCheck_Analyzer](https://github.com/Open-Cascade-SAS/OCCT/blob/V7_9_3/src/BRepCheck/BRepCheck_Analyzer.hxx), [BOPAlgo_ArgumentAnalyzer](https://github.com/Open-Cascade-SAS/OCCT/blob/V7_9_3/src/BOPAlgo/BOPAlgo_ArgumentAnalyzer.hxx)

Les preuves et restrictions proposées dans les jalons suivants sont des choix de conception FabSTEP déduits de ces capacités et du code existant, pas des garanties fournies par ces bibliothèques.

## Séquence proposée

| Lot | Première géométrie concrète | Dépendances | Porte de sortie |
| --- | --- | --- | --- |
| A — combiner les preuves existantes | Fraisure conique complète et encoche cylindrique ouverte séparées dans une plaque | V1 | Deux surfaces réellement analytiques, provenance complète et compatibilité des cas historiques |
| B — zones sphériques sans pôle | Segment sphérique entre deux plans parallèles ; variante concave avec parois présentes | A indépendant | Preuve annulaire, deux limites circulaires, courbure intérieure et deux franges bornées |
| C1 — chaînes de révolution résolues | Bague à trois chanfreins et portées, puis profil à changements de pente sans portée | V1, bénéfice de B pour les profils courbes | Chaque rupture et chaque bande prouvées ; aucun lissage ou découpage automatique d'une courbe ambiguë |
| C2 — révolutions plus générales | Profil à lignes et arcs circulaires, puis méridienne lisse bornée | C1 et contrat de profil | Couverture périodique, génératrice simple, borne continue et contrôle des trims |
| D — congés partiels constants | Une ou deux arêtes arrondies d'une boîte, extrémités planes source ; ensuite coin triédrique | A, contrat local de supports/raccords | Rayon, domaines, tangence et arrêts prouvés, sans fillet ajouté sur des arêtes voisines |
| E — boîtiers ouverts creux ou percés | Bac ouvert arrondi, puis perçages éloignés des raccords | D | Épaisseur et fond source conservés, vide accessible conservé, une coque fermée |
| F — intersections de perçages | Deux trous cylindriques orthogonaux sécants de rayons différents dans un bloc | Contrat local de courbes de découpe | Intersections transverses, graphe de limites, matière/vide et deux supports prouvés |
| G — congés à rayon variable | Une arête droite, deux appuis plans, rayon linéaire positif et arrêts source | D et bornes de surfaces paramétriques | Loi identifiable, régularité de l'enveloppe, continuité et erreur contrôlées |
| H — surfaces libres bornées | Une surface bicubique de type hauteur, bordée par quatre courbes présentes, sur un solide fermé | Contrat B-spline et validation continue | Paramétrisation injective, bords partagés, borne et volume contrôlés ; mode explicite |

Cet ordre privilégie les résultats vérifiables. D et les travaux préparatoires de F peuvent être étudiés sans attendre une révolution à méridienne libre. G et H restent de la recherche tant que leurs certificats ne sont pas établis.

### A — première tranche verticale

Le cas doit fonctionner en **Surfaces** sans demander de sphère ni de tore artificiel. Exemple de référence à construire indépendamment : plaque `60 × 40 × 12 mm`, encoche semi-cylindrique `R6` sur un côté, trou intérieur `R3` avec fraisure supérieure `R5` de hauteur `2 mm`, suffisamment éloigné de l'encoche. Les deux faces terminales, les limites latérales de l'encoche et toutes les parois sont présentes dans le maillage. Le manifeste fixe les surfaces et le volume depuis cette construction de référence, avant conversion.

Le point d'entrée est `recognize_rounded`, où l'admission précède aujourd'hui `partial::augment`. Réutiliser `prove_section`, la preuve des arcs, `RoundedModel`, `RoundedFaceRef`, le flux analytique et `build_rounded_step`. L'extension doit être admise seulement avec un cône complet et au moins un cylindre partiel effectivement prouvés. Un simple contour qui ressemble à un arc ne suffit pas.

Préserver le parcours des cônes sans arc et les modes Plans, Cylindres et Primitives. La nouvelle recherche ne doit ni consommer sans borne le budget des cas historiques ni rendre récupérable un refus de preuve ou de budget. Fixer la précédence par des régressions portant sur le modèle et le code d'erreur, pas seulement sur un succès final. Si le chemin supplémentaire est tenté, son coût doit être compté.

Les tests nécessaires couvrent : positif nominal, rotation/translation, triangulation différente, 3MF placé et miroir ; cône déformé, arc peu échantillonné, faux raccord latéral, patchs se coupant, budget d'écart et budget de travail. Vérifier les sources et triangles conservés, les comptes de surfaces et les paramètres après réimport, puis l'audit bidirectionnel. Un cône seul, les doubles chanfreins, la pièce réelle mixte et les anciens refus doivent garder leur contrat.

Cette tranche ne nécessite a priori ni nouveau type public, ni nouvelle ABI, ni mode UI supplémentaire. L'annonce reste « combinaison cône complet et cylindre ouvert prouvés en Surfaces », sans annoncer des révolutions arbitraires.

### B — zone sphérique entre deux anneaux

Commencer par une zone de sphère extérieure, azimut complet, entre deux cercles coaxiaux de rayons strictement positifs, fermée par deux plans réellement présents. Ajouter une zone concave seulement avec les autres parois du solide source. Tester une zone au-dessus de l'équateur et une zone le traversant, par exemple une sphère `R10` coupée à `z = -4` et `z = 6`.

Pour deux anneaux `(z0,r0)` et `(z1,r1)`, le centre axial candidat satisfait `zc = (r1²-r0²+z1²-z0²)/(2(z1-z0))`. Calculer dans un repère local normalisé et refuser un écart axial mal conditionné. Les anneaux seuls ne démontrent pas une sphère : exiger une courbure intérieure résolue, incompatible avec la génératrice conique et un cylindre dans les tolérances.

Remplacer la preuve de disque à pôle par une preuve d'anneau : deux contours simples, couverture de degré un, orientation cohérente, aucun triangle replié, aucun pôle ajouté. Borner la projection aux deux latitudes, compter le déplacement complet des sommets contraints et ajouter les franges géodésiques des deux petits cercles à la flèche des triangles. La formule d'aire sphérique continue seule ne prouve pas la couverture du polygone géodésique source. Préserver les calottes et sphères entières actuelles.

Refus dédiés : zone trop mince, contours presque confondus, sphéroïde, deux anneaux reliés par des facettes coniques, couverture manquante ou double, mauvaise orientation, frange hors budget. Le type `MixedSphere` et le constructeur natif semblent suffisants ; cette observation ne dispense pas d'un test natif à deux anneaux sans pôle.

### C — révolutions et combinaisons plus générales

**C1 : profils linéaires par morceaux.** Étendre d'abord aux portées et chanfreins supplémentaires séparés par des changements de pente démontrés. Une frontière de changement doit provenir d'une boucle source complète et d'échantillons intérieurs des bandes adjacentes. Comparer des intervalles d'incertitude des pentes et imposer une séparation résolue ; ne pas découper une méridienne douce en autant de cônes que ses niveaux tessellés. Refuser les séparateurs minuscules, les changements proches du bruit et les interprétations concurrentes non départagées. Le cas actuel Cône–Cylindre–Cône reste prioritaire.

**C2 : profils courbes.** Commencer par une succession bornée de lignes et d'arcs de cercle, en prouvant rayon, centre et raccords de chaque portion. Puis étudier une méridienne B-spline à degré et nombre de segments plafonnés. Un premier contrat peut imposer `r(z) > 0`, avec `z` monotone ; les profils avec retour axial et les pôles nécessitent une extension distincte. Pour tous, prouver l'axe, le tour complet, la correspondance à la source, l'absence d'auto-intersection et les bords partagés. Une surface tournée de 270° doit avoir ses deux faces latérales présentes et prouvées : le fait qu'OCCT accepte cet angle ne justifie pas leur invention.

L'ajustement d'une méridienne ne doit pas se limiter à quelques coupes. Attribuer chaque triangle, prouver l'injectivité du domaine et une borne continue incluant le profil, l'azimut et les trims. Une intégrale de volume à erreur contrôlée remplace les formules élémentaires lorsqu'elles ne s'appliquent plus. La surface exportée doit être nommée selon sa nature réelle ; une révolution B-spline ne devient pas un cône dans le rapport.

### D — congés partiels à rayon constant

Commencer par une arête droite à deux appuis plans : le raccord est cylindrique, de rayon constant, avec limites de tangence et arrêts plans issus du maillage. Tester un parallélépipède dont une seule arête est arrondie, puis deux arêtes disjointes. Introduire ensuite un coin entre trois congés compatibles, avec son domaine sphérique et ses trois coutures prouvés. Les coins à plus de trois branches restent exclus du premier contrat.

Décrire explicitement les supports, la ligne des centres, les domaines remplacés, les limites et l'orientation convexe/concave. Une surface cylindrique détectée n'est appelée congé que si ses tangences sont démontrées. Le rayon doit être mesuré dans la source, pas déduit d'un « rayon courant » global. Une version initiale peut exiger des raccords analytiques construits directement ; si `MakeFillet` est utilisé, contrôler ses propagations et les domaines résultants, sans exporter de résultat partiel.

Le corpus doit distinguer un chanfrein étroit d'un arrondi, un congé réellement arrêté d'une paroi manquante, et un raccord tangent d'une juxtaposition presque tangente. Vérifier les normales, le rayon et la continuité sur l'ensemble du raccord avec la borne prévue ; les quelques échantillons de contrôle indépendant restent une mesure séparée.

### E — boîtiers creux et percés

Premier cas : un **bac ouvert** à coins arrondis dont le fond, les parois intérieures/extérieures et le rebord sont présents. Sa frontière matérielle reste une coque fermée, même si l'intérieur est accessible. Ne pas appliquer automatiquement une épaisseur à un boîtier plein et ne pas inventer le fond. Distinguer les rayons intérieur et extérieur, mesurer les jeux, prouver que les parois ne se rencontrent pas et conserver la cavité.

Ajouter ensuite des perçages qui restent dans les plages planes et n'atteignent aucun congé. Un trou traversant un raccord relève des domaines d'intersection du lot F. Le `RoundedBoxModel` historique à 26 faces reste inchangé ; le nouveau contrat décrit des patchs et leurs adjacences réelles.

Une **cavité totalement enfermée** peut demander plusieurs coques d'un même solide. Elle ne doit pas être traitée comme deux corps imbriqués acceptés par exception. La distinction composante/coque/corps et le traitement des cavités constituent une décision d'architecture séparée, avec ADR, limites et qualification. Cette recherche ne l'autorise pas implicitement.

### F — perçages sécants

Premier sous-ensemble : deux cylindres orthogonaux de rayons différents dans un bloc, intersection transverse suffisamment éloignée des tangences et des autres limites. Conserver des anneaux d'entrée plans complets pour déterminer chaque cylindre. Les portions supprimées par l'autre perçage doivent être identifiées depuis les limites et la matière de la source ; aucune paroi ni fond ne peut être prolongé arbitrairement.

Le modèle devra représenter une courbe 3D partagée et ses domaines sur les deux surfaces, avec provenance des arêtes source et incidence de deux faces. Les opérations de section ou de découpe OCCT peuvent construire ce graphe dans `fabstep-cad`, à partir des supports et limites prouvés par le core. Elles ne doivent pas choisir seules quels morceaux représentent la pièce. Les segments d'intersection et les faces produites sont plafonnés et contrôlés après STEP.

Refuser initialement : tangence, quasi-tangence à la tolérance, courbes trop courtes, triple intersection, rayons/axes indistinguables, slivers non résolus et branchement ambigu. Les intersections entre trous d'un même solide n'assouplissent pas le refus de contact entre corps distincts. Contrôler des points de vide et de matière de part et d'autre de chaque branche, en plus du volume global qui pourrait masquer une erreur locale.

### G — rayon variable : décision conditionnelle

Le premier essai peut se limiter à un congé le long d'une arête droite, entre deux plans, à loi de rayon linéaire positive, avec deux arrêts présents. L'algorithme doit retrouver et borner la loi depuis plusieurs sections, distinguer les limites de tangence, et établir que l'enveloppe ne devient ni singulière ni auto-intersectante. Un empilement de quarts de cercle de rayons différents ne suffit pas à démontrer un congé variable correct.

Une capacité de génération OCCT ne suffit pas au passage en production. La décision de poursuivre exige : loi identifiable dans les tolérances, stabilité à une autre tessellation, limite explicite de paramètres et d'itérations, domaine régulier, borne de l'intérieur des triangles et du raccord, volume avec erreur contrôlée, réimport STEP et corpus indépendant positif/négatif. Des lois différentes indiscernables dans le bruit entraînent un refus ou un choix utilisateur explicitement documenté, jamais un rayon inventé.

Une loi générale à spline, des arrêts à rayon nul et les coins où plusieurs congés variables se rencontrent constituent des extensions ultérieures. Un nouveau mode ou une option d'approximation devra être explicite, consigné dans `ProcessingSettings` et accompagné de diagnostics stables. La v1 ne promet pas ces formes.

### H — formes libres : approximation explicite et bornée

Premier laboratoire : un patch bicubique de type hauteur, sans surplomb ni trou, entouré de quatre limites présentes et joint à des parois source fermant un solide. Son maillage doit provenir d'une surface nominale construite indépendamment. Éviter de commencer par un objet organique entier : segmentation, paramétrisation, topologie et raccords y seraient tous inconnus simultanément.

Séparer la segmentation et le domaine de l'ajustement de surface. Conserver les arêtes vives, prouver une paramétrisation injective et ses limites, fixer degré, nœuds, nombre de pôles et poids positifs si rationnels. Refuser les plis, singularités, trous absents et conditions numériques non résolues. L'ajustement et sa provenance restent des données sérialisables ; tout objet OCCT reste côté CAD.

La porte technique est une **borne continue** démontrée, par exemple via subdivision de domaines Bézier et majoration des dérivées/écarts, avec arrêt lorsque le budget de travail est épuisé. Ce mécanisme reste à concevoir et à faire relire : l'erreur aux points d'ajustement, un bon rendu ou le maximum échantillonné de l'audit actuel ne le remplacent pas. Le volume doit aussi avoir une précision indépendante de l'ajustement ; les coutures doivent partager la même géométrie et leur continuité annoncée doit être contrôlée.

La décision produit doit distinguer « approximation libre contrôlée » et reconnaissance d'une primitive mécanique. Choix explicite, paramètres enregistrés, déplacement total annoncé, compte B-spline réel et diagnostic d'interprétation sont nécessaires. Aucun passage automatique vers ce mode pour faire réussir un refus analytique. Si une famille ne permet pas de preuve suffisante à coût borné, elle reste refusée et l'aperçu source reste disponible.

## Preuves et ressources communes à chaque lot

1. **Contrat avant reconnaissance.** Définir domaine admis, observations source requises, erreurs stables et cas ambigus. Une recherche de candidats ne constitue pas la preuve. Chaque triangle est attribué une fois ; toute nouvelle limite ou modification possède sa provenance et son déplacement compté.
2. **Travail borné.** Garder les limites globales de 64 Mio, 32 corps et 100 000 triangles après placement, et les budgets existants : reconnaissance 10 millions d'unités, contours 5 millions, distances 50 millions. Plafonner aussi candidats, patchs, branches d'intersection, paramètres et subdivisions avant allocation. Un nouveau contrat peut réduire le domaine admis ; il ne relève pas silencieusement les seuils. Les appels natifs supplémentaires doivent avoir un nombre et une taille d'entrées bornés, une interruption contrôlée lorsque disponible et une qualification de leur coût ; ces bornes ne constituent pas une promesse de durée murale constante.
3. **Refus cohérents.** `recognition.unsupported` reste réservé à l'absence de candidat admissible. Les erreurs de preuve, topologie, déviation ou budget ne déclenchent pas un autre interpréteur permissif. Les nouvelles familles peuvent avoir des codes dédiés, à décider dans leur ADR et à traduire en français/anglais ; aucun code proposé dans cette recherche n'est présenté comme déjà implémenté.
4. **Solidité native.** Vérifier les triangles source, construire les seules surfaces prouvées, contrôler fermeture/incidences/orientation et auto-interférences, sérialiser puis réimporter le STEP. Vérifier types, paramètres, limites, volume et dimensions après réimport ; préserver tous les corps et placements, sans résultat partiel.
5. **Fidélité séparée.** Conserver la borne composée de reconnaissance et les maxima bidirectionnels de l'audit, accompagnés de la déflexion. L'audit actuel est échantillonné. Ajouter un nouveau type de surface exige une borne adaptée, pas un changement du libellé de l'audit en « Hausdorff certifié ».
6. **Corpus indépendant.** Avant d'ajuster le reconnaisseur, fixer dimensions, surfaces, volumes ou intégrales indépendantes, points matière/vide, limites et refus attendus. Générer depuis FreeCAD Part ou un générateur analytique distinct, sans appeler la reconnaissance et sans utiliser le STEP attendu pour aider le convertisseur. FreeCAD constitue un parcours indépendant, mais partage la famille de noyau OCCT ; il n'est pas une preuve d'indépendance de noyau.
7. **Variantes et adversaires.** Au minimum : résolution grossière/nominale/dense, triangulation irrégulière, rotation/translation, miroir 3MF, STL binaire quantifié, bruit autour du seuil, quasi-dégénérescence et dépassement de budget. Des variantes non représentables à la précision native peuvent légitimement être refusées ; ne pas augmenter les tolérances automatiquement pour les faire passer.
8. **Qualification complète.** Tests core, tests app avec réimport et audit, contrôle indépendant des STEP CLI, puis import/export dans le paquet natif. Vérifier les anciens corpus, l'invalidation immédiate après changement, la dernière requête gagnante et l'aperçu préservé après échec. Rejouer les contrôles de `AGENTS.md` et consigner ce qui a réellement été exécuté. Les contrôles manuels restant ouverts doivent rester identifiables.

L'installation sur un autre Mac et l'utilisation des STEP dans Fusion ont été confirmées par l'utilisateur. Les versions, matériels et fichiers n'étant pas fournis, cette confirmation d'usage ne fournit pas de mesures géométriques supplémentaires ni une qualification générale des futurs lots. Pour ces lots, archiver les fichiers effectivement testés, leurs empreintes et les résultats mesurés ; une qualification Fusion ciblée complète utilement les réimports FreeCAD.

## Décision recommandée

Lancer A comme première tranche, puis B et C1, en maintenant les anciens modèles et modes. Définir un contrat local de supports et limites pour D, E et F lorsqu'un corpus concret le rend nécessaire. Garder C2 à méridienne libre, G et H conditionnels à une preuve bornée et à une décision explicite sur leur mode d'interprétation. Les cavités fermées à plusieurs coques restent un arbitrage distinct. Aucune publication distante, modification de licence ou migration de noyau n'est requise par cette séquence.
