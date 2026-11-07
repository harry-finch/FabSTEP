# ADR-0004 : Arcs cylindriques ouverts et raccords ancrés

Statut : accepté et implémenté
Date : 2026-11-06

## Contexte

Le premier STL utilisateur présente un évidement circulaire ouvert sur deux faces du bord. Les deux contours de son extrusion sont des chaînes ouvertes dans les contours des plans, et non des anneaux. M2b conserve donc ses 36 facettes alors que le trou et le bossage complets sont reconstruits. Les raccords verticaux contiennent aussi des sommets collinéaires intermédiaires.

## Décision

Étendre le modèle mixte avec des arcs circulaires orientés, des contours composés de segments et d'arcs, et des parois cylindriques partielles. Conserver les anneaux et cylindres complets existants. Chaque arc référence ses deux sommets source d'extrémité ; le cercle ajusté doit passer par ces extrémités. Aucun raccord n'est déplacé pour faire coïncider des surfaces.

Une paroi partielle demande deux chaînes planes correspondantes, une paroi intégralement vérifiée et deux génératrices source. Les quatre chaînes forment exactement sa frontière. La compression d'une génératrice subdivisée exige collinéarité, continuité et mêmes faces adjacentes. Tous les triangles gardent une attribution unique. Les limites d'échantillonnage, de balayage et de travail restent explicites.

Un arc demande au moins huit points, un balayage de 30 à 330° et des intervalles d'au plus 45°. Un contour entièrement co-circulaire ne peut contourner les exigences des cercles complets en devenant un arc avec quelques facettes restantes. Le cas d'un profil en D peut être distingué par une unique ouverture d'au moins 90° matérialisant la corde, les autres intervalles restant au plus à 45°. Cette convention reste une hypothèse de reconnaissance dans le mode explicitement choisi, pas une récupération certaine de l'intention de conception.

Une extrusion M2a réussie peut encore contenir un arc laissé polygonal dans son profil. L'orchestrateur tente donc aussi la reconnaissance mixte après cette réussite et la préfère si un cylindre partiel est prouvé. Seule l'absence de reconnaissance (`recognition.unsupported`) permet de garder l'extrusion précédente. Un candidat partiel dont la paroi n'est pas prouvée (`recognition.partial_unproven`), un budget dépassé ou une complexité excessive restent bloquants.

OCCT construit une face cylindrique limitée en angle et réutilise ses deux arcs, ses deux génératrices et leurs sommets dans les contours des plans adjacents. Les plans restent des plans, les cylindres des surfaces analytiques. Le contrôle natif de la source, des incidences, du solide, des auto-interférences et du STEP réimporté reste obligatoire, ainsi que l'audit échantillonné de fidélité.

Le volume analytique intègre les secteurs circulaires des plans et le flux du cylindre partiel, y compris le terme lié à la position de son centre. Une correction empirique du volume maillé ne constitue pas une validation.

## Alternatives considérées

- Compléter le contour en cercle ou soustraire un cylindre d'un bloc supposé : rejeté, car la géométrie absente et la décomposition volumique ne sont pas démontrées.
- Simplement lisser l'affichage : rejeté, les faces STEP resteraient planes.
- Déplacer les extrémités vers un cercle ajusté librement : rejeté à ce stade ; cela modifierait les raccords avec les plans.

## Conséquences

Extension additive du rapport mixte : `arcs`, `partial_cylinders`, contours `segments` et provenance `partial_cylinder`. Le nombre de cylindres du rapport inclut les surfaces complètes et partielles. Les lecteurs de rapports et le dessin des contours doivent comprendre les nouveaux types.

Le fichier utilisateur reste dans `artifacts/`, exclu de Git. Un corpus synthétique indépendant assure les régressions reproductibles sans redistribuer sa pièce. L'extension ne couvre pas les intersections entre cylindres, les congés ou les surfaces libres. Une chaîne bruitée peut encore être refusée si un ajustement local échoue, même si un ajustement global satisferait le seuil ; les arcs denses peuvent dépasser le budget de recherche. Aucun seuil n'est relâché implicitement.

## Validation

Comparer le cas utilisateur avant/après, contrôler les mesures et les types de surfaces dans FreeCAD. Ajouter des références indépendantes avec arcs seuls ou mêlés à des cercles complets, subdivisions, orientations et translations, ainsi que des parois défectueuses et des budgets insuffisants. Les résultats exécutés, les cas de revue contradictoire et la comparaison au STEP utilisateur sont consignés dans [VALIDATION](../VALIDATION.md).
