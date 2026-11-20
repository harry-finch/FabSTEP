---
id: R-133
titre: "Choisir une intention et ajuster automatiquement la reconstruction courbe"
resume: "Tester en Rust des tolérances bornées, valider le meilleur candidat et rendre les réglages manuels et le résultat lisibles."
statut: livree
jalon: atelier
cree: 2026-11-20
maj: 2026-11-20
livree: 2026-11-20
---
## But

Sur l’octant R-117, un seuil de planéité inadapté peut laisser des centaines de facettes. L’utilisateur veut choisir son intention et obtenir une reconstruction vérifiée sans deviner les tolérances.

## Inclus

Le panneau présente Automatique et Facettes avant le bouton de reconstruction. Les stratégies expertes et les tolérances restent dans Avancé. Un réglage manuel impose les valeurs choisies. Le résultat indique le nombre de faces par famille et l’écart mesuré lorsqu’il est disponible. Les réglages effectifs figurent dans les détails techniques et le JSON du résultat.

La vue Source et la vue STEP utilisent le clic droit pour tourner, le bouton central pour déplacer et la molette pour zoomer.

### Recherche

Le moteur présélectionne un nombre borné de couples en Rust sur les zones courbes restées facettisées. Une réduction de faces avec surface démontrée est nécessaire pour envoyer un candidat à la validation native. Le soudage, l’unité et le budget utilisateur restent fixes. Le choix final passe les contrôles natifs de réimport et de fidélité. L’annulation et le remplacement du fichier invalident l’essai. Le mode gros STL conserve les plafonds existants.

## Hors périmètre

Conversions ordinaires et inspecteur FR/EN. Le mode Facettes et les stratégies explicites gardent leurs valeurs. Le bouton de recherche radiale reste réservé aux stratégies expertes compatibles.

## Dépendances

R-117 pour l’octant et R-121 pour les mesures d’écart.

## Critères d'acceptation

Vérifier une pièce plane, un octant nécessitant un réglage, la saisie manuelle, le refus d’un candidat et l’annulation. Distinguer les distances mesurées des bornes calculées. Recompiler l’application locale.

## Vérifications de l'utilisateur

Essayer Automatique puis Facettes sur l’octant, vérifier les familles de faces et le maximum mesuré, puis saisir une planéité dans Avancé et vérifier que la valeur est respectée. Comparer les gestes de navigation entre les deux vues.
