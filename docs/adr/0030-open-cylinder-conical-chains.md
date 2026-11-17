# ADR-0030 — Raccords ouverts cylindriques et coniques

Date : 16 novembre 2026. Statut : décision acceptée par l’utilisateur ; qualification suivie dans R-058.

## Contexte

Les extrémités des rainures de body 26mm.stl contiennent une bande cylindrique, un chanfrein conique et une seconde bande cylindrique. La bande supérieure seule comprend 144 triangles. Le reconnaisseur de cylindres partiels exigeait deux arcs portés par des faces planes : le raccord intermédiaire conique ne satisfaisait pas ce contrat. Une sélection manuelle ne pouvait pas fournir cette preuve manquante.

## Décision

Étendre de manière additive `RoundedModel` avec des cônes partiels entre deux arcs circulaires source et deux génératrices source. Les rayons restent strictement positifs. Les cylindres et cônes dépendants partagent les mêmes arcs, avec exactement deux incidences par arc. Les surfaces voisines nécessaires sont reconstruites ensemble ; aucune frontière circulaire ne remplace isolément un bord polygonal d’une face voisine.

La reconnaissance reste déterministe, bornée et sans OCCT dans le core. Les tolérances ne changent pas. La recherche dédiée possède un budget de dix millions d’unités de travail, en complément du budget du parcours arrondi existant ; les refus de budget restent bloquants. Le guidage et la sélection de région incluent les bandes dépendantes ; les régions indépendantes restent conservées. Une attribution exhaustive distingue les cônes partiels et les autres surfaces. La construction suivie native ajoute un groupe de jetons après les groupes existants, tout en conservant les API historiques.

Le réimport STEP, les nombres de surfaces, le volume attendu, les dimensions et l’audit échantillonné restent des critères obligatoires. Les refus de correction sont affichés comme « Correction non appliquée » ; ils ne restaurent pas d’ancien export et proposent un recalcul validé de la session conservée.

## Limites

Le premier reconnaisseur cible les chaînes semi-circulaires de quatre à neuf niveaux axiaux source, limitées par deux supports plans. Les contours complets des supports sont vérifiés avant ajout des arcs ; une ambiguïté ou une déformation conserve les facettes. Les API historiques restent disponibles ; les nouveaux champs désérialisés manquants valent une liste vide. Il ne s’agit pas d’une reconnaissance générale de surfaces coniques libres : pas de pointe partielle, de raccord inventé ni de réparation implicite d’un maillage ouvert. Les preuves et les limites constatées de l’implémentation seront consignées dans VALIDATION et R-058.
