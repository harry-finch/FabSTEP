---
id: R-065
titre: "UX7 — suivre les étapes réelles de reconstruction"
resume: "Afficher les étapes observées du moteur et le corps en cours, sans pourcentage ni temps restant inventés."
statut: a-tester
jalon: atelier
cree: 2026-11-17
maj: 2026-11-17
livree:
---

## But

Donner une indication honnête de l’avancement de la reconstruction, conformément à la recommandation acceptée par l’utilisateur.

## Inclus

Trois étapes documentaires : analyse, reconstruction des corps, validation finale. Sous-étape réellement observée dans le moteur (reconnaissance, construction/réimport, fidélité…), corps en cours et temps écoulé. Activité indéterminée lorsque le noyau ne donne pas de mesure plus fine. Observation légère, bornée, protégée par la révision et la source, pour conversions ordinaires et guidées.

## Hors périmètre

Pourcentage du temps restant, compteur de triangles/zones inventé, interruption d’un appel natif en cours, modification des algorithmes de reconnaissance. Le nombre total de corps analysés et les sous-compteurs internes ne sont affichés que s’ils sont réellement disponibles ; la première tranche utilise l’indice de corps fourni par le moteur.

## Dépendances

Checkpoints coopératifs existants ; observation bureau en lecture seule. Aucun changement du noyau géométrique ni nouvelle dépendance.

## Critères d'acceptation

Les phases affichées proviennent de l’exécution courante. Aucune progression ancienne après nouveau fichier, options, refus ou succès. Une seule observation en vol ; échec du suivi sans effet sur la conversion. Tests natifs et UI de fraîcheur, disparition et mapping d’étapes ; build bureau réussi. Pas de pourcentage de temps affiché.

## Vérifications de l'utilisateur

- Lancer une reconstruction avec artifacts/FabSTEP.app et vérifier que l’étape détaillée évolue, que l’activité reste visible pendant les appels longs et que l’indicateur disparaît au résultat ou au refus.
