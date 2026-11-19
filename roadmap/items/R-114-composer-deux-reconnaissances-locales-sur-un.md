---
id: R-114
titre: "Composer deux reconnaissances locales sur un même solide"
resume: "Qualifier un témoin mêlant deux familles déjà reconnues et un témoin à revendications incompatibles. Définir la propriété des triangles et des arêtes communes avant de modifier Auto."
statut: idee
cree: 2026-11-19
maj: 2026-11-19
livree:
---

## Témoin qualifié

Bloc de 24 × 20 × 18 mm centré à l’origine. Les perçages traversants ont pour axes X et Z, centrés à l’origine, et pour rayons 6 mm et 3 mm. Le congé longitudinal de rayon 1,5 mm est sur l’arête parallèle à X en y = −10 mm et z = 9 mm. Les surfaces courbes des deux opérations sont séparées de 5,5 mm au minimum. Générer les trois références (perçages seuls, congé seul, pièce combinée), les réimporter en conservant les courbes paramétriques, puis les mailler avec une déflexion linéaire de 0,025 mm et des déflexions angulaires de 0,20 et 0,10 rad.

Auto reconnaît les opérations isolées aux deux densités. Sur la pièce combinée, il conserve le cylindre du congé et facettise les perçages. Le contrôle de continuité avancé de FreeCAD sur les perçages isolés est à instruire. Les mesures détaillées figurent en V-1260. Une fraisure conique et un congé constituent le témoin de composition qui fonctionne déjà.

## Qualification des revendications concurrentes

Le banc `scripts/qualify-r114-composition-freecad.py` compare les triangles de petits STL aux faces de deux dessins de référence. Une opération possède ses triangles source. À la frontière, deux usages d’une même arête doivent être complets et orientés en sens opposés. Le banc refuse les candidats qui revendiquent un même triangle, ainsi que les frontières manquantes ou inversées. Il vérifie le verdict dans les deux ordres d’insertion.

Le témoin disjoint couvre le congé et les perçages sur deux densités. Le contre-exemple contient deux trous transversaux qui rejoignent le même trou longitudinal. Les deux opérations qui voudraient chacune reconstruire ce trou revendiquent 524 puis 978 triangles en commun. La découverte de ces domaines directement dans Auto et leur construction native restent à développer.
