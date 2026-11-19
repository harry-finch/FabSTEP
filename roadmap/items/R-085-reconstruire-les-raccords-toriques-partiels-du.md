---
id: R-085
titre: "Reconstruire les raccords toriques partiels du pourtour de Flansch"
resume: "Les 46 671 faces passent le contrôle géométrique en laboratoire. Le rejeu de Flansch est reporté après les développements de reconnaissance pour limiter la RAM."
statut: en-cours
jalon: v1.0
cree: 2026-11-18
maj: 2026-11-19
livree:
---

## But

Reconstruire les congés toriques partiels d’une bride lorsque leur contour et les faces qui le partagent sont démontrés.

## Inclus

Identifier les limites dans les deux bandes de Flansch. Introduire un secteur à quatre arcs partagés, accepté explicitement le 18 novembre 2026. Qualifier une première configuration fermée à deux plans radiaux, ses variantes placées et ses contre-exemples. Étendre ensuite le raccordement aux congés cylindriques tangents réellement présents sur Flansch.

## Hors périmètre

Approximation de contours irréguliers, hausse des plafonds actuels, export de coque ouverte ou de partie du document.

## Dépendances

Diagnostic R-080 et contrat ADR-0040. Les autres raccords du pourtour dépendent de la qualification des voisins tangents.

## Critères d'acceptation

Attribution exhaustive, bornes dans les deux sens, limites partagées et contrôle natif après STEP. La première configuration doit conserver sept faces sélectionnables sur le corpus public en Auto. Pour étendre le raccordement entre une paroi courbe et les facettes voisines, qualifier aussi un solide indépendant de Flansch en mode Auto. Vérifier les faces reconnues, le partage du bord, le volume après réimport et le comportement sur une limite déformée. Pour Flansch, vérifier les voisins et leurs autres bords, assembler le corps complet, rejouer le helper et consigner le premier refus ou la validation du solide.

## Vérifications de l'utilisateur

Essayer le secteur public dans FabSTEP et vérifier le solide exporté dans Fusion. La qualification du pourtour réel reste ouverte dans ce jalon.

## État vérifié

Le secteur public à plans radiaux est intégré dans Auto et Surfaces. Les essais FabSTEP et Fusion A-084 et A-085 ont été acceptés le 19 novembre. Le helper Auto convertit aussi la rampe dense indépendante de 194 694 triangles en un solide de 659 faces (V-1250).

Sur le STL Flansch original, 43 surfaces toriques couvrent 493 542 des 540 170 triangles. Le solide construit directement en laboratoire présente 46 671 faces. Ses arêtes passent le contrôle géométrique exact après transfert. Le témoin fermé est tessellé à 0,005 mm en 98 576 triangles ; les écarts maximaux échantillonnés sont de 0,011553 et 0,010018 mm (V-1252/V-1253). Le premier refus d’auto-intersection a été reproduit puis corrigé sur les six raccords concernés. Les 14 882 raccords passent le contrôle individuel. Le rejeu du paquet corrigé atteint la limite de trente minutes pendant le contrôle complet (`worker.timeout`, V-1258). Aucun STEP de Flansch n’est exportable. Le volet Flansch reste ouvert sur le coût du contrôle entre faces, avec les plafonds actuels.


Dans la contre-expertise Argon, le contrôle parallèle a isolé sur le STEP de laboratoire six paires arête/face invalides à la diagonale 3 094. Le helper expérimental sur le STL original obtient `cad.invalid_solid` en 17 min 36 s et 3,12 Gio, avant l’échéance. Six interférences arête/face sont désormais isolées sur la coque construite depuis le STL, avant transfert. La feuille native 3 088 et les six petits témoins échouent en séquentiel. Reprendre sur leurs courbes de bord et les raccords voisins, puis rejouer le contrôle entier et les deux directions de fidélité. Mesures, témoins et versions figurent en V-1259. Le contrôle de production reste séquentiel.

Le secteur public à deux bords décalés reproduisait le refus du noyau. Il passe maintenant depuis le STL dense de 166 848 triangles jusqu’au STEP réimporté dans FreeCAD. Sur Flansch, les 26 refus ciblés du raccord ont disparu. Les 1 907 nouveaux raccords conservent leurs supports et leurs bords après transfert par lots. Les qualifications publiques et privées sont détaillées en V-1261.

Le rejeu complet en Auto du 19 novembre atteint trente minutes pendant la construction, avec un pic de 3,03 Gio et aucun STEP publié. R-085 reste en cours. Le filtre expérimental de distance au tore diminue le nombre de groupes coûteux sur le banc. Sa qualification sur le solide entier avant et après transfert reste à faire (V-1261).

### Qualification du corps relu (V-1262)

FabSTEP retrouve les 46 671 faces après écriture et relecture du corps Flansch en laboratoire. Le contrôle géométrique accepte le solide, dont les 29 traces circulaires corrigées. La tessellation de contrôle à 0,005 mm reste sous le plafond avec 98 468 triangles. Les distances échantillonnées sont de 0,011552 mm vers le STEP et de 0,010018 mm en retour.

Les contrôles indépendants couvrent les 6 793 groupes natifs et les 6 763 groupes du STEP relu. Les 704 groupes modifiés par la correction de bord ont été revérifiés directement. Tous passent. Le helper du paquet atteint encore la garde de trente minutes dans le contrôle des intersections. Son pic échantillonné est de 3,04 Gio. La prochaine qualification porte sur le coût des groupes contenant des raccords B-splines sur les deux états du solide. Les témoins publics et les commandes sont consignés en V-1262.

### Calendrier de reprise

À la demande de l’utilisateur, reporter tout nouvel essai du STL Flansch et de ses captures jusqu’à la fin des développements de reconnaissance géométrique. Pendant ces développements, utiliser les petits témoins indépendants.
