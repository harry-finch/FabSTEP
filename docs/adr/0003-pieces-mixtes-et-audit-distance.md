# ADR-0003 : Pièces mixtes, fonds prouvés et audit de distance

Statut : accepté
Date : 2026-11-06

## Contexte

Une extrusion complète ne représente pas un bloc percé partiellement ou muni d'un bossage. Reconnaître seulement des cercles ne prouve ni la paroi qui les relie ni le fond d'un trou. Le jalon précédent reste disponible ; les formes locales exigent une représentation supplémentaire.

## Décision

Ajouter `MixedModel` : faces planes, parois cylindriques complètes et anneaux circulaires partagés. Chaque anneau appartient à un plan et à un cylindre. Les points source sont conservés et chaque triangle est attribué à une face reconnue ou conservée. Un fond borgne est une région plane source ; aucun fond n'est ajouté.

Les candidats circulaires sont appariés uniquement si toute la paroi entre eux est prouvée. Le parcours des triangles vérifie géométrie, orientation et frontière exacte du patch. Tous les candidats détectés doivent avoir une preuve unique ; les intersections, variations de section et bouts non démontrés sont refusés. Les autres régions restent planes ou facettisées, sans prétention de reconstruction analytique.

L'adaptateur OCCT construit les cylindres finis et partage leurs arêtes circulaires avec les plans adjacents. Il coud une coque, vérifie le solide et ses auto-interférences, puis écrit et réimporte le STEP. Le volume attendu est calculé par flux sur les faces analytiques, sans correction empirique du volume du STL.

Le mode utilisateur reste **Cylindres**, explicitement activé. L'orchestrateur essaie la reconnaissance d'extrusion complète, puis la reconnaissance locale seulement sur `recognition.unsupported`. Les erreurs de budget ou de complexité ne sont pas masquées. Le mode Plans reste inchangé.

Ajouter une crate pure `fabstep-verify` pour comparer deux maillages. OCCT tesselle le STEP réimporté ; l'audit mesure dans les deux sens les distances aux triangles depuis tous les sommets, milieux d'arêtes et barycentres. Un arbre de boîtes accélère les requêtes. Il n'y a ni sélection aléatoire ni rapport partiel en cas de dépassement du budget.

## Alternatives considérées

- Construction par différences/unions de primitives : écartée à ce stade ; elle impose de retrouver une décomposition volumique et risque de fabriquer une géométrie non prouvée.
- Reconnaissance locale silencieuse après n'importe quelle erreur : rejetée ; un budget dépassé doit rester bloquant.
- Présenter les maxima échantillonnés comme une distance de Hausdorff : rejeté. Le paramètre de déflexion du mailleur n'est pas une borne certifiée ; la surface continue peut présenter un maximum entre les échantillons.

## Conséquences

API additive : `recognition` du jalon précédent est conservé, avec `mixed_recognition` et `fidelity` supplémentaires. La comparaison est un contrôle additionnel, pas un remplacement de la preuve locale ni de la validation BRep. La limite de refus inclut le budget utilisateur, la déflexion demandée et une marge numérique explicite dans l'orchestrateur.

Périmètre : cylindres complets entre contours plans, fonds plans, bossages et trous non intersectants ; pas de cylindres partiels, raccords complexes, fond conique reconstruit ni congé. Une pièce peut conserver des zones facettisées non candidates. La validation native peut refuser des plans approximatifs conservés.

## Validation

Corpus analytique indépendant et fixture exportée par FreeCAD, fonds manquants, parois coniques, saillies intermédiaires, trous orthogonaux, rotation et subdivisions. Audit numérique sur retessellation, distance à l'intérieur des triangles, triangles très fins et surfaces présentant une partie manquante. Résultats effectivement exécutés dans `../VALIDATION.md`.
