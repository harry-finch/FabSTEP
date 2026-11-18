# ADR-0044 — Relief bicubique explicite sur un socle rectangulaire

Décision du 18 novembre 2026, dans le cadre de R-038.

Le nouveau mode avancé ajuste une surface de hauteur bicubique à un corps fermé. Il exige une base rectangulaire parallèle à XY, quatre côtés verticaux et une rive de hauteur constante. Auto conserve ses familles existantes. Le code stable `recognition.height_patch_fitted` signale l’interprétation au résultat.

Le cœur prouve la partition du maillage en six disques orientés, leurs coutures, l’injectivité de la projection du toit et la couverture du rectangle. Les coordonnées XY des seize pôles sont imposées par le rectangle. Les quatre hauteurs intérieures proviennent d’une résolution par moindres carrés avec contrôle du conditionnement. Le résidu vertical est un polynôme de degré six sur chaque triangle. L’enveloppe de ses coefficients de Bernstein majore la distance sur tout le toit dans les deux sens, car les deux surfaces couvrent le même rectangle. Le budget inclut soudage, résidu et arrondi numérique.

OCCT construit le toit et les cinq plans, puis vérifie les douze arêtes et les seize pôles avant et après transfert STEP. Le volume de référence est l’aire de la base multipliée par la moyenne des seize hauteurs. Le document garde six propriétaires de triangles par corps et applique ses contrôles usuels de séparation et de fidélité. La tessellation conserve son plafond et borne la résolution demandée en fonction des dérivées secondes du toit.

La limite spécifique de 32 000 triangles borne le coût du certificat. Les surplombs, inclinaisons du socle, frontières libres, réseaux de patches et reliefs insuffisamment échantillonnés restent hors du domaine reconnu. Les entrées et commandes reproductibles figurent dans [le corpus](../../fixtures/height_patch/README.md).
