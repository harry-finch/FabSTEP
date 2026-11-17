---
id: "R-036"
titre: "F — perçages sécants"
resume: "Deux trous cylindriques orthogonaux de rayons distincts, intersection transverse"
statut: "idee"
jalon: "post-v1"
cree: "2026-11-13"
maj: "2026-11-13"
---

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
