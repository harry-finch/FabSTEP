---
id: R-114
titre: "Composer deux reconnaissances locales sur un même solide"
resume: "Qualifier un témoin mêlant deux familles déjà reconnues et un témoin à revendications incompatibles. Définir la propriété des triangles et des arêtes communes avant de modifier Auto."
statut: a-tester
cree: 2026-11-19
maj: 2026-11-20
livree:
---

## But

Reconstruire sur le même bloc un perçage sécant et un congé extérieur dont les volumes et les surfaces sont disjoints.

## Inclus

Un bloc orthogonal, deux perçages traversants et un congé longitudinal extérieur de rayon constant. Le raccord étend le contrat privé core/CAO approuvé le 19 novembre. Les triangles proviennent des deux opérations et des six plans porteurs.

## Hors périmètre

Les opérations dont les frontières ou les revendications se recouvrent gardent le traitement de repli. La composition arbitraire de reconnaisseurs dépasse cette première configuration.

## Dépendances

Les deux chaînes d’intersection des perçages, la construction des arcs et génératrices du congé et la traçabilité source du corps doivent être vérifiées ensemble.

## Critères d'acceptation

Dix faces STEP sur deux densités, volume vérifié indépendamment, contrôle des quatre arêtes partagées et des deux branches de perçage après relecture, attribution exhaustive. Refus du contre-exemple à revendications concurrentes.

## Vérifications de l'utilisateur

Ouvrir les STL du bloc composé, lancer Auto et sélectionner le congé puis les trois parois de perçages dans la vue STEP.

## Témoin qualifié

Bloc de 24 × 20 × 18 mm centré à l’origine. Les perçages traversants ont pour axes X et Z, centrés à l’origine, et pour rayons 6 mm et 3 mm. Le congé longitudinal de rayon 1,5 mm est sur l’arête parallèle à X en y = −10 mm et z = 9 mm. Les surfaces courbes des deux opérations sont séparées de 5,5 mm au minimum. Générer les trois références (perçages seuls, congé seul, pièce combinée), les réimporter en conservant les courbes paramétriques, puis les mailler avec une déflexion linéaire de 0,025 mm et des déflexions angulaires de 0,20 et 0,10 rad.

Auto reconnaît les opérations isolées aux deux densités. Sur la pièce combinée, il conserve le cylindre du congé et facettise les perçages. Le contrôle de continuité avancé de FreeCAD sur les perçages isolés est à instruire. Les mesures détaillées figurent en V-1260. Une fraisure conique et un congé constituent le témoin de composition qui fonctionne déjà.

## Qualification des revendications concurrentes

Le banc `scripts/qualify-r114-composition-freecad.py` compare les triangles de petits STL aux faces de deux dessins de référence. Une opération possède ses triangles source. À la frontière, deux usages d’une même arête doivent être complets et orientés en sens opposés. Le banc refuse les candidats qui revendiquent un même triangle, ainsi que les frontières manquantes ou inversées. Il vérifie le verdict dans les deux ordres d’insertion.

Le témoin disjoint couvre le congé et les perçages sur deux densités. Le contre-exemple contient deux trous transversaux qui rejoignent le même trou longitudinal. Les deux opérations qui voudraient chacune reconstruire ce trou revendiquent 524 puis 978 triangles en commun. La découverte de ces domaines directement dans Auto et leur construction native restent à développer.

## Intégration dans Auto

L’extension approuvée construit maintenant les dix faces sur les deux maillages disjoints. La qualification de la revendication commune, les résultats FreeCAD et les avertissements des trois arêtes des perçages figurent en V-1272.
