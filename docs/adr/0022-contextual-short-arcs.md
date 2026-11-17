# ADR-0022 : Petits arcs entre parois cylindriques prouvées

Statut : accepté et implémenté dans le build de développement
Date : 2026-11-09

## Contexte

Après la consolidation des parois décalées, certains petits raccords restent facettisés : leurs contours n'ont que quatre points et leur balayage est souvent inférieur à 30°. La tolérance radiale peut déjà être respectée ; élargir ce seuil ne fournit ni les échantillons manquants ni une meilleure détermination du rayon.

Un très petit résidu d'ajustement ne garantit pas la stabilité du centre d'un arc court. Les contraintes d'échantillonnage des arcs historiques restent donc inchangées. L'extension vise des intervalles entiers encadrés par des parois déjà prouvées, dans le mode analytique explicitement choisi.

## Contrat

La recherche complémentaire part uniquement de deux arcs historiques prouvés sur un même contour plan. Elle examine l'intervalle source complet qui les sépare, sans supprimer de points gênants ni utiliser un nouvel arc court comme appui pour une recherche récursive.

L'intervalle doit comporter quatre à sept points et couvrir au plus 45°. Tous les points intérieurs doivent être du même côté de la corde, à une distance supérieure au seuil d'ajustement explicite. Au moins deux de ces témoins doivent se situer entre 20 et 80 % de la corde, avec une séparation d'au moins 20 %. Le cercle ajusté passe par les extrémités source, considérées comme fixes pour ce calcul.

Les contraintes de tous les points définissent un intervalle de centres possibles sur la médiatrice de la corde. Cet intervalle doit être borné, rester du côté opposé au bombé et avoir une largeur d'au plus 10 % du rayon ajusté. La variation du rayon sur cet intervalle est également bornée par cette largeur. Ce seuil est un critère conservateur de résolution ; il ne constitue pas une incertitude statistique ni une connaissance du rayon du modèle original. Aucun rayon nominal n'est déduit des autres raccords. Les bornes intermédiaires et décalages tangents non finis sont refusés.

Les tangentes des arcs voisins doivent être compatibles avec les centres admissibles. Ce contrôle peut exclure un intervalle incompatible ; il ne déplace ni le centre ajusté ni les extrémités pour imposer une tangence. Les deux contours source correspondants, la paroi complète, les génératrices et leurs incidences restent soumis aux preuves existantes. Les limites de travail et le budget d'écart ne changent pas.

Un intervalle ne satisfaisant pas les critères de candidature reste facettisé. Après admission comme candidat, une preuve de paroi ou de raccord échouée reste bloquante. Aucun résultat analytique précédent n'est rendu exportable pour masquer cet échec.

Quatre points cocirculaires d'un polygone intentionnel peuvent être indiscernables d'une tessellation. La reconnaissance reste une interprétation circulaire explicite, comptée dans les diagnostics existants ; elle ne retrouve pas l'intention de conception. L'ajustement automatique des paramètres et le guidage manuel de petits arcs ne font pas partie de ce lot.

## Bornage du centre

Dans le repère de la corde, les extrémités sont `(-h, 0)` et `(h, 0)`. Un centre admissible s'écrit `(0, t)` et son rayon vaut `R(t) = sqrt(h² + t²)`. Pour un témoin `(u, v)` et le seuil radial `e`, on exige `h > 4e` et `v > e`. La contrainte `abs(distance((u,v),(0,t)) - R(t)) <= e` équivaut alors à une inégalité quadratique, sans ambiguïté de signe lors des élévations au carré.

Avec `D = u² + v² - h² - e²` et `q = v² - e² > 0`, l'intervalle est délimité par `(vD ± e sqrt(D² + 4h²q)) / (2q)`. L'intersection porte sur tous les points intérieurs. Le cercle ajusté doit appartenir à cet intervalle, sous une marge de calcul flottant explicite. Puisque `R(t)` est 1-Lipschitz, la largeur de l'intervalle des centres borne aussi celle des rayons compatibles.

La normale radiale du voisin fixe, si elle existe, la position sur cette médiatrice qui donnerait une tangence à l'extrémité. Cette position doit être dans l'intervalle admissible et le sens de parcours des tangentes doit être cohérent. Ce témoin de compatibilité ne remplace pas le centre ajusté : la tangence exacte du résultat final n'est pas promise.

## Validation

Le corpus indépendant couvre des raccords courts convexes et concaves, des changements de repère, les coordonnées quantifiées, les contours incompatibles, l'absence d'appuis, le conditionnement insuffisant, les parois défectueuses et les budgets. Les essais natifs vérifient les familles de surfaces, le volume, les dimensions et la fidélité après réimport. Les fichiers utilisateur restent exclusivement dans `artifacts/`.

Les résultats exécutés sont consignés en V-1153 à V-1156 dans [VALIDATION](../VALIDATION.md) : cinq nouveaux tests core, deux app, dix-huit conversions synthétiques et sept essais CLI privés. La suite globale passe avec 284 tests workspace, quinze bureau et trente-deux UI. Le build de développement est reconstruit ; parcours graphique, Fusion et réimport FreeCAD indépendant restent ouverts. Le paquet autonome 1.0.0 n'est pas remplacé par cette extension.
