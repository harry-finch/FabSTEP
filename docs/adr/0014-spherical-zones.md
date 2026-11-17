# ADR-0014 — Zones sphériques entre deux anneaux source

Date : 9 novembre 2026
Statut : implémenté dans le build de développement ; qualification consignée dans VALIDATION

## Domaine

Le mode Surfaces peut reconnaître une zone sphérique annulaire entre deux contours circulaires plans réellement présents. Les deux rayons sont strictement positifs ; aucun pôle n’est créé dans le maillage ni dans le modèle. Les autres faces doivent former avec cette zone un corps fermé. Une zone concave requiert ses parois et ses limites source, comme une zone convexe.

Le contrat reprend `MixedSphere` avec ses deux anneaux et le constructeur natif existant. Les calottes à pôle et les sphères complètes conservent leurs parcours. Aucun objet OCCT ni IO n’entre dans le core ; aucune nouvelle ABI ou option publique n’est nécessaire.

## Candidat et ambiguïté

Dans un repère local, les anneaux de rayons r0, r1 se trouvent à z0 < z1, avec h = z1 − z0. Le centre candidat est donné par :

```text
zc − z0 = (r1² − r0² + h²) / (2h)
R = hypot(r0, zc − z0)
```

Le calcul est normalisé ; la différence de carrés est factorisée. Une séparation trop petite ou un conditionnement insuffisant au regard de la tolérance ne donne pas de candidat admissible. Tous les sommets doivent satisfaire le résidu sphérique. Les anneaux seuls ne prouvent pas la sphère : au moins douze échantillons intérieurs à plusieurs hauteurs doivent démontrer une courbure résolue, distincte de la méridienne affine reliant les anneaux.

L’absence de candidat laisse le classement historique poursuivre son travail. Une courbure non résolue n’est donc jamais promue en sphère, mais une source indiscernable d’un cylindre peut conserver son interprétation historique ; il ne s’agit pas d’un refus global de toute bande mince nominalement sphérique.

## Preuve du domaine et de la fidélité

Le patch source est une composante connexe à topologie annulaire ; la validation de variété et les incidences de bord restent obligatoires. Les contours correspondent aux deux anneaux, sont disjoints et parcourent chacun un unique tour monotone. Les triangles projetés gardent une orientation cohérente et aucun triangle ne passe par une singularité radiale. Les intervalles angulaires ne dépassent pas 45°.

La projection de chaque sommet sur la sphère est contrainte aux deux latitudes terminales. Les sommets des contours sont projetés sur leur cercle respectif. Le déplacement **vectoriel complet**, incluant un éventuel rabattement vers une latitude, est compté dans le résidu ; une simple erreur radiale ne suffit pas.

Les arêtes droites du maillage se projettent en arcs géodésiques, tandis que les contours analytiques sont des petits cercles. La preuve vérifie que les deux contours géodésiques restent strictement séparés, en bornant leur latitude jusqu’au milieu de chaque arc. Leur aire attendue est la différence de deux éventails géodésiques calculés depuis un pôle auxiliaire nord ou sud choisi pour réduire l’annulation numérique. Ce pôle est uniquement une référence d’intégration : il n’est ajouté à aucune géométrie. L’aire attendue doit être finie, positive et résolue ; la somme des aires orientées des triangles doit lui correspondre. Avec la topologie et les frontières simples, cela établit une couverture de degré un.

La borne additionne le soudage, les résidus de projection, la flèche intérieure des triangles et **les deux franges de bord**. Le contrôle natif des triangles source, la construction, le réimport STEP, les dimensions, le volume et l’audit bidirectionnel restent obligatoires. L’audit échantillonné ne remplace pas cette preuve de reconnaissance.

## Budgets, compatibilité et refus

La recherche, l’attribution et les preuves débitent le compteur de travail existant ; aucun seuil n’est relevé. Les erreurs de preuve, de topologie, de budget et de déviation restent bloquantes. L’admission de zones n’active ni le mode Plans ni une fermeture automatique. Les modes Cylindres et Primitives ne gagnent pas de surface sphérique.

Le cas historique `m5b_spherical_zone.stl`, auparavant explicitement hors périmètre des calottes, est transféré aux positifs du nouveau contrat lorsque les contrôles réussissent. Les autres négatifs des calottes restent en place. Cette modification de périmètre est une nouvelle capacité explicite, pas une suppression de test défaillant.

Le corpus dédié couvre notamment zones traversant l’équateur, nord/sud, rotation, triangulations irrégulière/dense, quantification f32, concavité et placements 3MF, ainsi que sphéroïde, repli, sous-échantillonnage, triangle manquant et budgets. Les résultats réellement exécutés, y compris limites des vérifications externes et visuelles, sont consignés dans [VALIDATION](../VALIDATION.md).

## Limites

Ce contrat exige deux contours circulaires source complets et des latitudes bien conditionnées. Il ne reconnaît pas des patchs sphériques arbitrairement découpés, une surface libre ou une intersection avec un autre perçage. Les bandes trop fines, les franges qui se croisent, la couverture non résolue et les sources insuffisamment échantillonnées ne sont pas promues. Les autres lots de la [roadmap](../../ROADMAP.md) restent distincts.
