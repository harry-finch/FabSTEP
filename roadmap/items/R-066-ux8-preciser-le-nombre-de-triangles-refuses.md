---
id: R-066
titre: "UX8 — préciser le nombre de triangles refusés"
resume: "Afficher le nombre exact de triangles d’un STL trop dense et le maximum autorisé."
statut: livree
jalon: atelier
cree: 2026-11-17
maj: 2026-11-19
livree: 2026-11-19
---

## But

Rendre utile le refus d’un STL trop dense : afficher sa taille géométrique et la limite actuelle, sur demande de l’utilisateur.

## Inclus

Compte exact des enregistrements d’un STL binaire dont la longueur est cohérente ; comptage des facettes d’un STL ASCII syntaxiquement valide sans stocker de triangles au-delà du plafond. Message français/anglais avec séparateurs de milliers. Code d’erreur inchangé et anciens messages compatibles.

## Hors périmètre

Relever le plafond de 100 000 triangles, simplifier automatiquement le maillage ou modifier l’import 3MF. Les limites de taille du fichier et de stockage restent actives.

## Dépendances

Lecteur STL et présentation des erreurs existants ; aucune nouvelle dépendance ni nouveau champ de format.

## Critères d'acceptation

Le fichier privé FlanschTurboLuftfilter_REV02.STL est refusé avec 540 170 triangles et limite 100 000. Les longueurs binaires incohérentes et structures ASCII invalides ne produisent pas un faux compte exact. Tests du lecteur et des messages, contrôle UI et build réussis.

## Vérifications de l'utilisateur

- Relancer artifacts/FabSTEP.app et ouvrir FlanschTurboLuftfilter_REV02.STL ; vérifier que le message affiche 540 170 triangles et une limite de 100 000.
