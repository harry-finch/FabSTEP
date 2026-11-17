# ADR-0019 — première reconstruction guidée : cylindre complet

Date : 9 novembre 2026. Statut : accepté et implémenté.

## Intention

Après I1c, l’utilisateur peut désigner des faces STEP et retrouver leurs triangles source. I2 doit donner un effet réel à cette sélection sans convertir une hypothèse en primitive forcée.

Le premier outil est **Essayer un cylindre**. Il part d’un résultat **Conserver les facettes** dont la provenance est exacte. Une sélection non vide doit appartenir à une seule composante analysée d’une seule instance source. Elle désigne une paroi ; il n’est pas nécessaire d’en sélectionner toutes les facettes.

## Domaine et preuve

Une seule paroi cylindrique complète, entre deux contours circulaires plans présents dans la source, est reconstruite. Tous les triangles sélectionnés doivent appartenir à cette même paroi prouvée. Le moteur recherche un candidat unique et contrôle toute sa paroi, son orientation, ses deux anneaux, leurs correspondances et les incidences. Les seuils et budgets existants restent inchangés.

Le chemin réutilise la preuve des cylindres complets du reconnaisseur mixte. Il ne reconstruit que le candidat désigné ; les autres régions restent planes ou facettisées. Les contours des supports plans adjacents sont adaptés aux deux anneaux démontrés. La segmentation analytique utilise `max(plane, fit)` : des facettes peuvent être regroupées différemment du résultat Plans, avec leur résidu inclus dans la borne ; aucune identité géométrique absolue des zones non ciblées n’est promise. Les cylindres ouverts, cônes et arrondis ne sont pas des hypothèses admises dans ce premier lot. Une sélection de plan ou de plusieurs parois incompatibles est refusée.

Le modèle final conserve une attribution exhaustive de tous les triangles. La source passe les contrôles CAO, puis le résultat est réimporté et contrôlé : solide fermé, types de surfaces, volume, dimensions et audit bidirectionnel échantillonné. Tous les corps sont reconstruits et leur séparation est recontrôlée ; aucun export partiel n’est autorisé.

## Identité et concurrence

La commande native reçoit la nouvelle requête, l’identité source, la requête du résultat de départ et les identifiants des faces sélectionnées. Elle ne reçoit ni une correspondance source calculée par l’interface ni des options indépendantes du résultat de départ.

Le `DocumentStep` suivi conserve les octets source immuables, le format et les options ; le bureau partage l’`Arc` du téléversement. Le cache capture atomiquement ce résultat de départ, vérifie l’identité de la source, puis avance la révision et retire l’autorisation d’export. La correspondance face → triangles est résolue depuis le `DocumentStep` privé. Les contrôles de révision et d’époque empêchent un calcul périmé de rétablir l’export après un changement de source ou d’options.

Un refus conserve l’aperçu importé et ne rend pas l’ancien résultat exportable. Retirer l’indication déclenche une nouvelle conversion avec les options de base ; il ne restaure pas un cache ancien.

## Interface et traçabilité

L’interface expose une action unique, sans liste de familles encore non prises en charge. Elle explique que toute la paroi et ses contours sont reconstruits. L’indication et son état restent visibles après le calcul ; les erreurs de preuve restent explicites.

Le champ `guidance` du rapport distingue les indices de faces du résultat de départ, les identifiants source, les triangles initialement sélectionnés et ceux de la paroi entière reconstruite. Les paramètres du cylindre figurent dans le modèle mixte du corps ; sa borne et les mesures de fidélité restent accessibles dans le même rapport. Le mode de base ne doit pas masquer la présence de la reconstruction guidée.

Limite initiale d’I2a, levée par l’[ADR-0020](0020-consecutive-cylinder-guidance.md) : une seule indication est prise en charge à la fois. La provenance native du résultat analytique n’est pas encore disponible : une seconde correction demande de retirer l’indication et de repartir du résultat facettisé. L’édition de plusieurs indications, leur historique et la persistance restent dans I3.

## Validation

Voir [V-1140 à V-1143](../VALIDATION.md) : tests de sélection et de preuve du noyau, conversions natives et audit, instance 3MF miroir, cache et requêtes obsolètes, contrôles UI et suite globale réussie. Les essais graphiques restent à qualifier ; ils ne sont pas remplacés par ces tests automatisés.
