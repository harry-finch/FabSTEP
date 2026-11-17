# ADR-0028 — Fusion plane guidée sans déplacement des sommets

Date : 15 novembre 2026. Statut : acceptée pour le lot R-056, suite Plan puis Tore approuvée par l’utilisateur.

## Contexte

Le regroupement automatique exige des normales presque identiques. Une sélection de facettes peut décrire un plan dans la tolérance sans satisfaire ce critère angulaire, notamment sur des triangles minces. La correction guidée doit vérifier cette hypothèse sans forcer la géométrie.

## Décision

Ajouter la famille Plane au contrat additif du guidage ; l’absence de famille conserve Cylindre. Fusionner uniquement une sélection connectée par arête et couvrant intégralement ses faces sources. Déterminer le plan par somme compensée des normales pondérées par aire, dans l’ordre canonique des triangles. Vérifier orientation, écarts de tous les sommets, contours et trous avec les contrôles bornés existants. Conserver les coordonnées ; ne pas projeter les sommets de frontière.

Pour la coexistence, prouver les indications courbes via le parcours existant, puis fusionner les faces polygonales indiquées. Refuser tout contour courbe ou recouvrement de paroi courbe. Ne pas modifier les reconnaisseurs historiques lorsqu’aucun Plan n’est demandé. Remapper les attributions exhaustives, recalculer le volume attendu et la borne conservative, puis valider source, résultat réimporté et fidélité. Un résultat entièrement plan utilise le constructeur planaire natif existant, sans relâcher le contrat du constructeur mixte.

## Conséquences

Une faible variation intérieure peut disparaître dans la représentation plane malgré la conservation du tableau de coordonnées ; le résidu est signalé et borné. Un refus ne republie aucun export précédent. Les sessions restent limitées à seize indications. Plan ne répare pas nécessairement un STL qui échoue avant la sélection. Tore et les plans à frontières courbes restent hors de ce lot.
