---
id: R-033
titre: "C2 — profils de révolution courbes"
resume: "Révolutions toriques ou cubiques, une ou deux travées, pointe axiale, maillage irrégulier et retour axial vérifié."
statut: a-tester
jalon: post-v1
cree: 2026-11-13
maj: 2026-11-19
livree:
---

## But

Étendre la reconnaissance des profils de révolution courbes, depuis les compositions exactes jusqu’aux méridiennes plus générales.

## Inclus

Lignes et arcs circulaires composés, limites source, axe et couverture démontrés ; étude puis contrat borné pour les méridiennes lisses.

## Hors périmètre

Surfaces libres arbitraires, fermeture inventée, approximation sans borne ; un nouveau contrat de méridienne doit être qualifié avant intégration.

## Dépendances

R-061, R-075, R-076, R-084 ; les choix de contrat supplémentaires seront présentés dans leur tranche.

## Critères d'acceptation

Profils reconstruits avec bornes continues, volume et réimport contrôlés ; les variantes hors domaine restent refusées ou facettisées selon le mode.

## Vérifications de l'utilisateur

Les essais de chaque tranche sont portés par ses sous-jalons et leurs actions FabBoard ; ne pas les dupliquer au niveau de ce chantier global.

## État consolidé

R-095 couvre deux travées à tangente continue et nœud observé. R-099 couvre la pointe axiale. R-104 prend en charge une mono-travée sur trois maillages FreeCAD sans anneaux réguliers. Les retours axiaux, les raccords vers une famille voisine et les combinaisons supplémentaires restent ouverts.

Les essais dans Fusion sont suivis dans les actions des sous-jalons.

## Historique et décisions

| C2 — profils de révolution courbes | Lignes et arcs circulaires, puis méridienne lisse de complexité bornée | Profil simple, axe et couverture prouvés, borne continue et volume contrôlé ; trims source | Étude ; méridienne libre soumise à contrat et décision d’interprétation |

Source : [ancienne feuille de route](../../docs/ROADMAP-avant-fabboard.md). Les validations historiques sont conservées ; aucun essai nouveau n’est déduit de la migration.

## Poursuite demandée le 17 novembre

Première tranche exacte lancée dans R-061 : deux quarts de tore et une portée cylindrique. Les profils généraux restent ici.


## Qualification des grandes zones de Flansch

R-074 mesure quatre chaînes composées et confirme nativement la représentation tore–cône–cylindre. R-075 intègre la première extension bornée de reconnaissance, sans nouveau format ; les profils à deux cônes et les arcs de 100° restent distincts. Voir V-1198 pour les preuves, les témoins absents et les limites de la conversion complète.


## Poursuite globale autorisée

L’utilisateur confirme que l’objectif est de réaliser l’ensemble C1b2, C2, D, E, F et G et autorise la poursuite du chantier. Les tranches déjà développées restent à tester ; cet accord ne remplace pas leur acceptation ni les preuves géométriques. Les travaux sont décomposés en extensions bornées, corpus indépendants et vérifications natives. Les changements structurants encore indéfinis seront présentés lorsqu’un contrat concret est établi ; l’accord sur les perçages sécants A-045 reste acquis.

Priorité immédiate : prolonger les raccords de R-075 vers les chaînes à deux cônes et les arcs de 100° identifiés dans R-074. Mesurer les attributions et les limites restantes sur Flansch après chaque tranche, sans relèvement implicite des plafonds. Les méridiennes libres restent un contrat ultérieur à qualifier.


## Intégration des tranches

R-076 ajoute les deux profils toriques composés et obtus mesurés sur Flansch ; validation automatique V-1201 réussie, essais utilisateur A-069/A-070 ouverts. La méridienne libre reste à qualifier.

R-095 — Reconstruire une paroi de révolution dont la méridienne comporte deux segments cubiques à tangente commune. Voir V-1223.

R-099 — Reconstruire une méridienne cubique entre un disque et une pointe présente dans le maillage. Voir V-1227.

R-104 — Reconstruire une méridienne cubique lorsque le maillage FreeCAD ne présente pas d’anneaux intérieurs complets. Voir V-1234.

R-106 — Reconstruire une méridienne à deux segments tangents dont le raccord se trouve entre deux hauteurs du STL. Voir V-1236.

R-111 — Reconnaître un retour axial sur un profil de rayon monotone et conserver les deux disques mesurés. Voir V-1248.
