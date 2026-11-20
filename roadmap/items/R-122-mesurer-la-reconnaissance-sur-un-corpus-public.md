---
id: R-122
titre: "Mesurer la reconnaissance sur un corpus public reproductible"
resume: "G7 · Établir une référence sur une douzaine de pièces de plusieurs familles, puis la réutiliser à chaque extension."
statut: en-cours
jalon: post-v1
cree: 2026-11-20
maj: 2026-11-20
livree:
---
## But

Une bride seule ne permet pas de savoir si la reconstruction progresse sur les pièces des utilisateurs. Le banc doit permettre de comparer deux versions du moteur à partir des mêmes STL et solides de référence.

## Inclus

- Commencer avec une petite bride à congé torique et une pièce d’une autre famille pour R-123. Compléter le pilote avec environ douze pièces de sources indépendantes couvrant profils extrudés, pièces tournées, perçages et congés.
- Figer pour chaque pièce la source, les droits d’utilisation et de redistribution, l’empreinte, l’unité, le STEP de référence, les paramètres et deux densités de tessellation. ABC et Fusion 360 Gallery sont des sources candidates sous réserve des droits de chaque pièce.
- Séparer les pièces de mise au point des pièces réservées à la vérification. Consigner aussi les quasi-primitives bruitées et les refus attendus.

## Hors périmètre

Extension à 50–100 références après examen de la diversité et du coût du pilote. Le STL privé de Flansch est exclu du corpus.

## Dépendances

Aucune. Les deux premiers témoins suffisent pour démarrer R-123.

## Critères d'acceptation

- Depuis un checkout neuf, reconstruire les entrées par une commande documentée et vérifier leurs empreintes. Un fichier indisponible est signalé.
- Pour chaque pièce et densité, enregistrer la couverture par famille pondérée par aire, les faux positifs et les familles manquées, le nombre de faces et de colliers, les deux écarts maximaux échantillonnés, le temps et le pic mémoire. Préciser machine, version, échantillonnage et budget.
- Rejouer deux fois la même version et obtenir les mêmes résultats géométriques. Distinguer dans le bilan les mesures variables, les refus et les entrées absentes.

## Vérifications de l'utilisateur

Aucune.
