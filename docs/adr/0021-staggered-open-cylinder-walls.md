# ADR-0021 : Parois cylindriques ouvertes à triangulation décalée

Statut : accepté et implémenté dans le build de développement
Date : 2026-11-09

## Contexte

Un STL utilisateur comporte des arcs ouverts dont les sommets intermédiaires en hauteur ne suivent pas les mêmes angles que ceux des contours plans. La preuve historique exige chaque triangle dans l'extrusion d'une seule corde du contour : une paroi proche d'un cylindre peut donc être refusée malgré des contours compatibles.

La recherche des arcs présente aussi une difficulté numérique distincte. L'ajustement contraint par les deux extrémités change lorsque la chaîne s'allonge ; le refus d'un préfixe ne démontre pas celui de la chaîne entière. Des sous-arcs chevauchants peuvent ainsi masquer un arc admissible. Certaines chaînes du fichier réel dépassent néanmoins réellement le seuil radial initial : corriger la recherche ne justifie pas de modifier ce seuil.

## Contrat de l'extension

La reconnaissance doit conserver deux contours plans source correspondants, les extrémités partagées et deux génératrices source. Elle n'ajoute ni fond, ni fermeture, ni raccord. Les coordonnées source restent inchangées. Les cylindres complets et le guidage manuel ne changent pas de contrat.

Une preuve de paroi complémentaire peut admettre une triangulation décalée si elle établit :

- le respect du seuil radial explicite par tous les sommets de la paroi ;
- un domaine angulaire cohérent, des triangles suffisamment résolus et une projection cylindrique sans repli ;
- la couverture complète, de degré un, entre les deux arcs et les deux génératrices ;
- une borne tenant compte des intérieurs des triangles, notamment de leur portée angulaire, et des résidus des contours ;
- l'attribution exclusive de tous les triangles et la conservation des régions voisines.

Une aire totale proche de l'aire attendue ne suffit pas à démontrer cette couverture. La preuve doit aussi contrôler l'orientation de la projection et la frontière exacte. Le travail reste plafonné par le budget existant ; son dépassement refuse la conversion.

Deux parois cylindriques partielles peuvent se rejoindre sur une génératrice déjà présente dans la source. Leurs chaînes doivent être identiques, éventuellement parcourues en sens inverse, et chaque segment source doit séparer exactement ces deux parois prouvées. Les triangles des parois sont disjoints. Une génératrice a soit un cylindre et un plan voisins, soit deux cylindres et aucun plan voisin. Une troisième incidence est refusée. Chaque arc reste partagé par un seul cylindre et un seul plan.

Le constructeur natif associe les sommets et les génératrices aux indices source. Les deux cylindres réutilisent une même arête native, munie de ses représentations paramétriques sur les deux surfaces ; leurs parcours de cette arête doivent être opposés. La couture n'est pas utilisée pour deviner ce raccord. Cette extension n'autorise ni intersection de cylindres à calculer, ni courbe de découpe inventée. Le modèle et l'ABI existants suffisent à décrire ces incidences.

Les seuils par défaut restent inchangés. Une tolérance personnalisée figure dans les réglages et diagnostics du rapport. Un refus analytique ne déclenche aucun export facettisé implicite. La construction native, le contrôle du solide réimporté, son volume et ses dimensions, ainsi que l'audit bidirectionnel échantillonné restent obligatoires.

## Validation

Le corpus synthétique indépendant couvre les triangulations décalées, les changements de repère, la quantification des coordonnées et les parois défectueuses. Les tests vérifient aussi les refus pour budget insuffisant et la conservation des résultats historiques. Le fichier utilisateur et ses dérivés restent dans `artifacts/`, hors du corpus publié.

Les résultats exécutés sont consignés en V-1149 à V-1152 dans [VALIDATION](../VALIDATION.md) : cinq nouveaux tests core, deux CAD et deux app, quinze conversions synthétiques, puis trois conversions analytiques de la pièce privée avec un seuil radial choisi explicitement. La suite globale passe avec 277 tests workspace, quinze bureau et trente-deux UI. Le réimport FreeCAD indépendant et le parcours graphique restent non qualifiés pour cette correction.
