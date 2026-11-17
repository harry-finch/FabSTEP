# ADR-0013 — Cône complet et cylindre ouvert en Surfaces

Statut : implémenté, qualification consignée dans VALIDATION
Date : 9 novembre 2026

## Contexte

Le modèle `RoundedModel` et le pont OCCT savent déjà réunir des cônes et des cylindres ouverts. Le parcours Surfaces exigeait cependant une sphère, un tore ou deux chanfreins opposés avant de chercher les arcs. Une fraisure complète et une encoche ouverte distinctes ne bénéficiaient donc pas ensemble de ces preuves, malgré leur représentation disponible.

## Décision

Étendre uniquement l’admission du parcours Surfaces à un candidat comportant exactement un cône complet, puis exiger qu’au moins un cylindre ouvert soit effectivement prouvé par `partial::augment`. Le nombre de cônes est compté dans les patchs classés ; aucune chaîne de révolution supplémentaire n’est admise.

Pour ce candidat, la preuve UV des sections est différée jusqu’à la preuve du cylindre ouvert. Sans cylindre ouvert, `recognition.unsupported` conserve le repli historique Primitives/Cylindres. Les orientations provisoires du modèle ne sont jamais exportées : toutes les sections sont prouvées et leurs orientations remplacées avant calcul du volume et retour du modèle. Les propriétaires sentinelles empêchent la recherche d’arcs de reprendre des triangles de sections complètes.

La recherche d’arcs ajoute du travail avant le repli des cônes seuls ; une source particulièrement complexe peut donc atteindre le budget pendant cette recherche. Le coût historique n’est pas garanti identique. La recherche d’arcs et les preuves de sections partagent le même compteur de travail. Les erreurs de preuve, de budget et de déviation restent bloquantes ; aucune tolérance n’est augmentée. Tous les triangles et sommets source, les limites et les raccords restent contrôlés. Les contrôles natifs de la source, du STEP réimporté et de la fidélité s’appliquent sans changement.

Aucun type public, ABI, mode, préférence ou dépendance supplémentaire n’est introduit. Les modes Plans, Cylindres et Primitives conservent leurs règles. Un arc trop peu échantillonné pour constituer un candidat peut rester facettisé via le parcours historique ; cela ne constitue pas une reconnaissance cylindrique. Un candidat d’arc dont la paroi n’est pas prouvée reste refusé.

## Corpus et qualification

Le générateur indépendant `fixtures/generate_post_v1_combinations.py` construit une plaque de 44 × 24 × 10 mm, une encoche semi-cylindrique R4 et un perçage R3 avec fraisure de hauteur 4 mm et rayon supérieur R6. Le volume nominal est calculé à partir de ces paramètres : 9 875,132801517426 mm³. Le résultat attendu comporte 7 plans, 2 cylindres (dont un ouvert) et 1 cône.

Les variantes tournées et miroir, le témoin sans encoche, le cône déformé, la paroi bombée, le sous-échantillonnage, les features qui se coupent et l’arc dense couvrent l’admission et ses limites. Les tests core/app contrôlent la provenance, les paramètres, le volume, les dimensions, la fidélité, les codes stables et les replis historiques. Le script FreeCAD indépendant vérifie les exports CLI. Les preuves effectivement exécutées et les essais natifs restant ouverts sont consignés dans [VALIDATION](../VALIDATION.md).

## Limites

Cette tranche ne reconnaît pas les intersections de surfaces, les cônes partiels, les nouvelles chaînes multi-cônes ni les congés variables. Les autres lots de la [recherche post-v1](../POST_V1_RESEARCH.md) conservent leurs contrats et étapes de validation distincts. Un build de développement qualifié ne remplace pas automatiquement le paquet 1.0.0 déjà livré.
