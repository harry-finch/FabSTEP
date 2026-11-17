# ADR-0032 — Deux cônes adjacents à rupture source unique

Date : 17 novembre 2026. Statut : retenue pour R-060, validation automatisée réussie (V-1186), essais utilisateur ouverts.

## Contexte

Les chaînes avec portées disposent d’une séparation démontrée entre les lois affines. Sans portée, fusionner toutes les bandes coniques refuse les changements de pente réels ; découper arbitrairement chaque anneau pourrait au contraire transformer une courbe en succession de cônes.

## Décision

Conserver le chemin historique à une seule loi affine en premier. Pour un patch à deux extrémités circulaires planes, rechercher ensuite une unique rupture sur un anneau source complet. La première tranche admet exactement deux troncs de cône de rayons positifs, sans portée terminale, pôle ou tore. Les chaînes plus longues restent R-059.

Chaque côté possède deux anneaux source intérieurs séparés, selon le contrat existant, et tous ses anneaux satisfont la loi affine de ses extrémités. Pour une portion de hauteur h et variation d de rayon, avec epsilon maximal des tolérances de soudage, planéité et ajustement, la pente m=d/h possède la marge conservative delta=(2epsilon+2epsilon|m|)/(h−2epsilon). Exiger h>2epsilon, |d|>2epsilon, valeurs finies et une différence de pentes strictement supérieure à quatre fois la somme des marges. Cela constitue un garde d’admissibilité supplémentaire, pas une tolérance d’ajustement élargie.

Deux ruptures distinctes admissibles entraînent un refus, même si elles sont proches. Aucun classement au moindre résidu ni regroupement approximatif d’anneaux. Le parcours débite chaque candidat et chaque échantillon au budget commun de reconnaissance ; il ne lance pas plusieurs constructions natives pour sélectionner une hypothèse.

L’anneau commun garde une seule géométrie. Les preuves complètes de paroi, couverture et orientation, la borne composée, l’attribution de tous les triangles et la validation native restent obligatoires. Les modèles de cônes et les incidences natives existants suffisent ; aucun DTO, ABI ou type de surface ajouté.

## Conséquences et limites

Les sources trop peu échantillonnées et les ruptures ambiguës peuvent rester facettisées en Auto, ou être refusées en mode explicite. Une interprétation dans la tolérance ne reconstitue pas une intention de conception perdue. Le corpus historique no_land avec témoins intérieurs devient intentionnellement positif ; les courbes et profils sans témoins restent des adversaires. Validation exécutée dans V-1186 de VALIDATION, distincte des essais utilisateur.
