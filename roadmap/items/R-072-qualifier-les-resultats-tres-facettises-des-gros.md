---
id: R-072
titre: "Qualifier les résultats très facettisés des gros STL"
resume: "Flansch franchit désormais la reconnaissance avec 4 cylindres et 8 cônes, mais conserve 501 900 plans, soit 501 912 faces au total. Le suivi du résultat est limité à 100 000 faces. Identifier les zones qui restent facettisées et les réductions géométriquement prouvées possibles ; examiner les coûts CAO, mémoire et inspection avant toute proposition de changement de limite ou de simplification."
statut: livree
jalon: post-v1
cree: 2026-11-17
maj: 2026-11-17
livree: 2026-11-17
---

## But

Qualifier le résultat très facettisé des gros STL et réduire les subdivisions inutiles lorsque les preuves géométriques existantes le permettent. L’accord « c’est parti pour R-072 » autorise cette analyse et les corrections locales, sans promettre que Flansch deviendra convertible.

## Inclus

- Mesurer la répartition des faces avant et après sécurisation des plans.
- Vérifier les possibilités de regroupement strict sans déplacement des sommets ni altération des frontières analytiques.
- Intégrer les réductions locales démontrées, avec provenance complète, bornes et tests de non-régression.
- Qualifier le parcours réel jusqu’au prochain refus ou jusqu’au STEP validé, et décrire les limites des consommateurs du résultat.

## Hors périmètre

Hausse implicite des plafonds, décimation, projection des sommets, nouvelle primitive ou modification de contrat public. Toute simplification géométrique ou extension importante de capacité nécessitera une proposition mesurée et un arbitrage utilisateur.

## Dépendances

R-071 intégré (e29f1ad). Flansch disponible localement ; aucune donnée privée à versionner.

## Critères d'acceptation

- Origine du grand nombre de faces expliquée par des mesures reproductibles.
- Toute modification conserve les sommets, les triangles attribués et les frontières nécessaires ; les contrôles de planéité native restent obligatoires.
- Temps, mémoire et travail bornés ; refus sans export partiel et annulation préservés.
- Tests adaptés, compilation, revue et documentation cohérentes ; les limitations non résolues restent explicites.

## Vérifications de l'utilisateur

- Valider les conclusions de qualification et la suite proposée ; aucun nouveau build à essayer pour ce lot.

## Constat initial

Le premier refus observé est dans `rounded_source_faces`. Le constructeur CAO mixte arrondi impose indépendamment `MAX_RESULT_FACES = 100_000` : supprimer la provenance optionnelle du helper ne suffirait donc pas. Aucune construction OCCT ni qualification mémoire d’un résultat de 501 912 faces n’est acquise.

La qualification devra aussi couvrir les consommateurs du résultat : rapport et STEP limités chacun à 128 Mio dans le protocole helper, audit et aperçu avec leurs bornes propres. Une modification isolée du nombre de faces ne constitue pas une qualification de ce parcours.

## Résultat de qualification

Mesures et limites : V-1196 dans `docs/VALIDATION.md`. Le grand nombre de plans précède leur sécurisation (480 024 plans ; seulement 21 876 faces supplémentaires dues au garde). Aucun regroupement local utile pour franchir le plafond n’est démontré. Quatre grandes zones courbes, totalisant 278 642 triangles, sont refusées faute de raccord cylindre–plan démontré ; leur famille exacte reste à déterminer.

Aucune modification de production ni hausse de plafond retenue. Les limites du suivi, de la construction CAO, de l’audit et de l’inspection sont indépendantes. La validation native source traite toujours les 540 170 triangles originaux, même si le résultat est réduit.

Suite proposée : qualifier la surveillance mémoire macOS (R-073), puis les méridiennes des quatre zones et profiler les étapes de validation native source, puis soumettre une extension géométrique ou architecturale précisément délimitée. Cette proposition ne vaut pas autorisation de modifier les contrats ni de simplifier le maillage.

La sonde native source a été arrêtée manuellement à 241,622 s après observation d’une empreinte physique `sample` de 6.9G, pour un pic RSS échantillonné de 2,35 Gio. Validité native finale inconnue ; détail et portée dans V-1196.
