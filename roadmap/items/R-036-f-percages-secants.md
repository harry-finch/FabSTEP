---
id: R-036
titre: "F — perçages sécants"
resume: "Perçages orthogonaux centrés ou décentrés. Axes obliques entre eux dans un prisme à quatre bouches circulaires."
statut: en-cours
jalon: post-v1
cree: 2026-11-13
maj: 2026-11-19
livree:
---

## But

Reconstruire les limites communes de deux perçages cylindriques orthogonaux de rayons distincts à partir du STL.

## Inclus

Première tranche centrée avec quatre entrées complètes, branches transverses, preuve source et certificat natif ; détail dans R-083.

## Hors périmètre

Tangences, axes obliques/décentrés, triples intersections, trous borgnes ou branches ambiguës.

## Dépendances

Accord A-045, laboratoire R-063 ; intégration R-083 et ADR-0038.

## Critères d'acceptation

Source, domaines, courbes communes, volume, solide réimporté et provenance prouvés dans les budgets ; aucun export partiel.

## Vérifications de l'utilisateur

Les essais de chaque tranche sont portés par ses sous-jalons et leurs actions FabBoard ; ne pas les dupliquer au niveau de ce chantier global.

## État consolidé

R-096 qualifie un décentrage des axes orthogonaux. R-105 démontre deux axes centrés à 60° et 75° avec quatre entrées circulaires et deux branches certifiées. Les croisements multiples, bouches obliques elliptiques et combinaisons d’excentration et d’obliquité restent ouverts.

Les essais dans Fusion sont suivis dans les actions des sous-jalons.

## Historique et décisions

| F — perçages sécants | Deux trous cylindriques orthogonaux de rayons distincts, intersection transverse | Courbes de découpe partagées, domaines et incidence prouvés ; sondes matière/vide ; refus tangences et triples intersections | Contrat de découpe à concevoir ; contacts entre corps toujours refusés |

Source : [ancienne feuille de route](../../docs/ROADMAP-avant-fabboard.md). Les validations historiques sont conservées ; aucun essai nouveau n’est déduit de la migration.

## Décision structurelle préparée le 17 novembre

La poursuite des pièces complexes demandée par l’utilisateur inclut l’étude des perçages sécants. Le modèle courant représente des limites droites/circulaires ; l’intersection de deux cylindres transverses demande une courbe partagée et un domaine sur chacun des supports. La première tranche proposée contient deux trous orthogonaux de rayons distincts dans un même bloc, avec entrées circulaires planes complètes. Les tangences, triples intersections, contacts entre corps et branches ambiguës restent refusés.

Option recommandée : contrat additif interne de courbe d’intersection et de domaines tronqués, conservant identité source, incidence de deux faces et choix matière/vide prouvé par le core. Le CAD construit seulement les supports/domaines démontrés et contrôle la courbe commune, les limites et le STEP réimporté. Conserver les anciens DTO/parcours compatibles ; aucun nouvel assouplissement des tolérances ni opération booléenne décidant seule quelle matière garder. Définir avant implémentation les plafonds de branches/segments et le certificat continu de proximité aux limites source. Cette option ouvre le domaine utile, mais touche les frontières core/CAD, la provenance et les tests natifs ; elle nécessite l’arbitrage architectural de l’utilisateur.

Alternative : poursuivre uniquement les combinaisons de primitives aux frontières planes/circulaires, ce qui minimise les changements du modèle mais laisse les perçages sécants facettisés ou refusés. Aucun code de production modifié pour ce lot à ce stade.

Accord utilisateur reçu le 17 novembre (A-045) : « Oui, étendre le modèle pour les perçages sécants (recommandé) ». Conception détaillée du contrat lancée pendant la validation de R-060 à R-062. Cet accord architectural ne constitue pas une preuve géométrique ni une validation de code encore absent.


Qualification préalable : [R-063](R-063-f0-qualifier-les-courbes-des-percages-secants.md), laboratoire initial PARTIAL (V-1187), désormais certifié dans le budget après reprise (V-1200). Les courbes communes et leur couverture passent ; la reconnaissance source et l’intégration du contrôle de volume restent à développer. L’extension est autorisée, pas encore intégrée à la conversion.


## Poursuite globale autorisée

L’utilisateur confirme que l’objectif est de réaliser l’ensemble C1b2, C2, D, E, F et G et autorise la poursuite du chantier. Les tranches déjà développées restent à tester ; cet accord ne remplace pas leur acceptation ni les preuves géométriques. Les travaux sont décomposés en extensions bornées, corpus indépendants et vérifications natives. Les changements structurants encore indéfinis seront présentés lorsqu’un contrat concret est établi ; l’accord sur les perçages sécants A-045 reste acquis.


## Plan initial de la tranche F1, avant intégration

A-045 autorise déjà le contrat : aucun nouvel arbitrage n’est requis pour le domaine centré, orthogonal, avec grand rayon au moins double du petit et quatre entrées circulaires complètes. A-067 attend une lecture du laboratoire, pas une seconde autorisation d’architecture.

1. Construire un corpus STL paramétrique indépendant avec dimensions/rayons variés, rotations, miroirs et retessellisations. Les deux branches idéales sont Γ±(t) = (r cos t, r sin t, ±√(R² − r² sin²t)). Ajouter des refus opposés : décentrage, rapports de rayons hors domaine, tangence, branche incomplète ou parcourue plusieurs fois, carte repliée, troisième trou et fond non prouvé.
2. Retrouver six supports et quatre anneaux source ; démontrer deux paires coaxiales orthogonales, leur intersection centrée, les marges et le choix de matière. Exiger neuf propriétaires : six plans, deux parois du petit trou et une paroi du grand trou à deux ouvertures. Cette dernière a quatre limites et une caractéristique d’Euler de −2 ; les deux branches doivent être disjointes, parcourues une fois et utilisées en sens opposés par leurs voisins. L’aire seule ne prouve pas cette couverture.
3. Introduire un domaine scalaire dédié et additif avec quatre références d’anneaux, deux branches analytiques et trois propriétaires cylindriques. Une branche peut être découpée en plusieurs arêtes natives à une couture périodique : ne pas assimiler branche source et arête OCCT. Toute construction native reste subordonnée aux domaines et au choix de matière démontrés par le core.
4. Généraliser le certificat F0 aux paramètres et repères mesurés, conserver ses budgets, comptabiliser l’écart natif dans la borne finale. Le contrôle de volume utilise un intervalle indépendant et une mesure native adaptative avec les seuils existants. N’admettre les arêtes spline dans le précontrôle de tessellation que pour les intersections certifiées, sans accepter les splines arbitraires ni relever le plafond de triangles.

Les difficultés ouvertes sont la couverture source autour des deux ouvertures, les transformations d’intervalles, les coutures paramétriques natives et leur tessellation bornée. Les trous décentrés/obliques, tangences, triples intersections, budgets modifiés ou contrat BRep générique seraient des extensions distinctes à présenter ; ils ne sont pas promis par F1. Aucun code de production F n’est encore activé.

Le certificat d’intervalles devra conserver les options numériques du laboratoire (`-fno-fast-math -ffp-contract=off`, ou équivalents plateforme) dans son unité de compilation. Un rapport exactement 2:1 peut être indéterminé après mesure STL : privilégier des témoins avec marge (par exemple R=6,5 mm et r=3 mm), ou refuser l’incertitude ; aucune canonicalisation non comptée pour faire passer le corpus.

## Domaine intégré au 18 novembre 2026

R-083 a intégré F1. R-091 élargit le rapport de rayons aux couples dont l’enveloppe de mesure reste strictement sous deux tiers. Un contrôle supplémentaire couvre les maillages reliant directement bouches et jonctions. Le corpus contient une pièce dessinée et maillée dans FreeCAD. Les résultats, y compris le repli au seuil, figurent dans V-1219. Les axes décentrés ou obliques et les intersections multiples restent à qualifier.

R-092 traite le contour fragmenté du bloc incliné décrit dans V-1219. Les deux densités de maillage dans les deux poses sont reconstruites aux seuils initiaux. Qualification et témoin de non-régression du mode Plans dans V-1220. Pour étendre F, restent les axes décentrés ou obliques entre eux, les rapports hors du domaine prouvé et les intersections multiples.

R-096 — Reconstruire les deux intersections partagées lorsque le petit perçage est décentré par rapport au grand. Voir V-1224.

R-105 — Reconstruire les deux intersections transverses de perçages dont les axes se croisent dans un prisme à quatre bouches circulaires. Voir V-1235.
