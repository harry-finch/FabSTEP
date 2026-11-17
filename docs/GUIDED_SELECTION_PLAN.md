# Préparation de la sélection guidée — I1

Date : 9 novembre 2026. Statut : préparation historique du lot I1, désormais implémenté dans le build de développement. Le contrat livré est décrit dans l’[ADR-0016](adr/0016-source-zone-inspection.md) et les validations réellement exécutées dans [VALIDATION](VALIDATION.md). Cette préparation précise les lots I1–I3 de la [roadmap](../ROADMAP.md). I1 sélectionne et renseigne ; seules les étapes ultérieures transmettront des hypothèses au moteur.

## Constats avant implémentation

`ui/src/components/Viewport.svelte` affiche les triangles du `DocumentPreview` dans leur ordre source, sous forme de géométrie Three.js non indexée. Les résultats servent aux contours reconnus et à l’inspecteur. Il n’y a ni détection de clic sur surface, ni table de sélection, ni surlignage de zone.

L’aperçu STL porte l’identifiant `stl` ; les analyses portent `stl/component/{index}`. Le même dédoublement existe pour les instances 3MF. `analyze_bodies`, dans `fabstep-core/src/pipeline.rs`, connaît les indices source `ids` de chaque composante, puis les trie et réanalyse les triangles correspondants. Le soudage modifie les sommets et une inversion globale échange deux coins, mais aucun triangle n’est supprimé silencieusement. Ces indices fournissent une correspondance exacte ; rapprocher les coordonnées, les centres de triangles ou les noms ne serait pas fiable.

Les reconnaissances mixtes, de révolution, arrondies et de boîtier disposent de `triangle_faces`. Le modèle planaire et l’extrusion historique n’exposent pas tous cette attribution. Après un échec CAO, l’aperçu est conservé, mais un diagnostic de corps ne localise pas automatiquement le défaut dans une zone.

## Contrat proposé

Une sélection source porte l’identité des octets chargés, une révision de géométrie source (incluant l’unité STL), l’identifiant du corps **placé** dans l’aperçu et une liste bornée d’indices de triangles source. La requête de conversion reste distincte : changer le mode ne change pas l’identité des triangles.

Une fonction compagnon d’analyse peut retourner une table supplémentaire, sans modifier les signatures publiques existantes :

```text
preview_body_id
analysis_body_id
analysis_to_source_triangles[i] = indice source du triangle d’analyse i
```

Les indices viennent directement de la liste `ids`, jamais d’une recherche géométrique. Côté rendu, chaque mesh conserve son identifiant de corps et la table triangle rendu → triangle source, même lorsque cette table vaut actuellement l’identité. Pour 3MF, l’identifiant d’instance placée distingue deux usages du même objet ; `object_id` ou le nom ne suffisent pas.

La topologie de sélection doit pouvoir être calculée même si la conversion analytique échoue. Une commande IPC additive, hors fil UI, peut produire la correspondance et l’adjacence avec les gardes source/requête/époque existantes. Une analyse topologique elle-même invalide doit rester signalée ; aucune adjacence n’est inventée. Le contrat finalement implémenté expose `selection_topology` avec les champs `request_id`, `source_id` et `components` ; chaque composante ajoute `source_triangle_neighbors` à la correspondance décrite ci-dessus.

## Premier parcours utilisateur

1. Activer explicitement la sélection ; cliquer sur le triangle source visible le plus proche. Le lancer de rayon considère tous les corps visibles, pour respecter l’occlusion ; les lignes et surlignages ne sont pas des cibles.
2. Utiliser ce triangle comme graine d’une zone topologique ajustable : agrandir/réduire d’un anneau d’adjacence, effacer, choisir un autre corps. Cette croissance ne prétend pas reconnaître une surface et peut traverser plusieurs familles géométriques.
3. Montrer la zone sur les positions source, avec les jetons clair/sombre, sans recadrer la caméra à chaque clic. Distinguer le clic d’un glissement d’orbite, d’un pan, d’un geste multipoint et d’un `pointercancel`.
4. Lorsque la correspondance et l’attribution sont disponibles, afficher les familles reconnues dans la zone. Sinon afficher l’indisponibilité de cette information, sans attribuer à la zone un diagnostic seulement connu à l’échelle du corps.

La sélection est conservée après un échec analytique ou CAO. Une nouvelle source, même de nom identique, ou un changement d’unité STL l’efface. Une modification des tolérances conserve les triangles source sélectionnés mais invalide l’adjacence et l’interprétation ; les réponses obsolètes ne peuvent ni restaurer une sélection effacée ni modifier celle d’une autre source. Les identifiants de corps source et de composantes analysées doivent être séparés dans l’état UI.

I1 ne modifie ni géométrie, ni options de reconstruction, ni autorisation d’export. Sans table topologique exploitable, une sélection élémentaire peut rester possible, avec croissance désactivée et expliquée. Aucun calcul de graphe quadratique ne doit être exécuté à chaque clic : l’adjacence est bornée, mise en cache pour la bonne révision et calculée hors fil UI.

## Validation avant livraison

- Correspondance exacte pour STL à triangles de plusieurs composantes entrelacés, soudage avec déplacement, inversion globale et unités mm/inch.
- Instances 3MF répétées, transformations composées et miroir ; aucune confusion entre occurrences du même objet.
- Conservation après refus, rechargement du même nom, changement de tolérance pendant calcul, effacement pendant une requête, réponses obsolètes de toutes les étapes.
- Clic/orbite/pan/gestes multipoints, corps occulté, clic dans le vide, redimensionnement ; aucune sélection au relâchement d’un glissement.
- Croissance/réduction déterministes, provenance exhaustive et indices valides ; plafonds 32 corps et 100 000 triangles conservés, allocations et travail explicites.
- Absence de faux classement quand `triangle_faces` n’est pas disponible ; export toujours conditionné au résultat validé courant.
- Essais visuels et clavier en français/anglais, clair/sombre, aux largeurs de référence.

## Passage à I2 et décisions ultérieures

I2 devra définir une première famille d’hypothèse bornée (par exemple une zone cylindrique candidate) et son effet sur la recherche. Une indication guide ou désambiguïse ; elle n’autorise jamais à forcer une surface, une fermeture ou un fond absent. Toute la paroi et ses raccords restent soumis à la preuve, aux budgets, à la provenance et au réimport STEP. La sélection visuelle seule n’améliore donc pas encore la géométrie exportée.

La segmentation automatique par normales/surfaces, la persistance intersessions et les contraintes contradictoires dépassent I1. Elles nécessitent des contrats distincts. Le choix recommandé pour le premier lot est la croissance topologique explicite et réversible ; les paramètres exacts du geste et les commandes clavier seront fixés avec le composant UI et sa validation.


## Suivi après I1b : provenance Plans (I1c)

L’[ADR-0018](adr/0018-planar-step-source-provenance.md) définit le suivi exact des faces planes construites à travers la couture et les transferts STEP. Le résultat en cache conserve sa partition source privée. L’interface propose **Voir la zone source** puis **Retour au STEP**, avec union multi-corps et sélection native conservée. Ce parcours ne corrige aucune géométrie ; les modes analytiques restent sans liaison source jusqu’à la conservation de leur propre historique. I2 doit encore borner la première hypothèse guidée et son contrat de contraintes.


## Première indication I2a

L’[ADR-0019](adr/0019-guided-complete-cylinder.md) borne **Essayer un cylindre** à une paroi complète contenant toute la sélection, entre deux contours plans circulaires prouvés. Les autres zones restent facettisées. La sélection de faces est résolue côté natif depuis le résultat et sa source privés ; le nouveau calcul retire immédiatement l’ancien export. Le rapport distingue la zone indiquée de la paroi reconstruite. Retirer l’indication recalcule le résultat de base dans cette première tranche. La tranche I2b ci-dessous ajoute les indications successives ; leur historique reste à développer.


## Indications successives I2b

L’[ADR-0020](adr/0020-consecutive-cylinder-guidance.md) conserve la provenance native des plans et cylindres complets guidés. Les indications peuvent être ajoutées depuis le résultat courant et retirées individuellement, avec un plafond de 16 par document et un budget de preuve partagé par corps. Après refus, la session acceptée est conservée sans export et peut être recalculée. L’édition, l’historique annuler/rétablir et la persistance restent séparés de cette première gestion de plusieurs indications.


## Choix de famille I2c

L’[ADR-0027](adr/0027-guided-surface-families.md) ajoute le choix Cylindre/Cône sur les facettes STEP à provenance exacte. Les troncs de cône sont bornés par deux contours circulaires plans source ; pointes et raccords directs vers une surface courbe restent refusés. Plusieurs indications des deux familles peuvent coexister, avec retrait individuel et recalcul complet. Les autres familles et l’historique I3 restent dans la roadmap.
