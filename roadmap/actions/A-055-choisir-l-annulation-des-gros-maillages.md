---
id: A-055
type: decider
titre: "Choisir l’annulation des gros maillages"
element: R-067
statut: fait
cree: 2026-11-17
par: agent
fait: 2026-11-17
---

Recommandation : isoler le calcul des gros maillages dans un processus séparé, pour pouvoir l’arrêter pendant un appel natif long. Alternative plus limitée : annulation aux frontières des étapes, avec attente possible de la fin de l’appel CAO courant. Le premier choix touche l’architecture, à valider selon INT-AUDIT-01 §12. Dans les deux cas, avertissement au-delà de 100 000 et qualification du fichier 540 170 triangles avant fixation d’un nouveau plafond.

## Réponse

Accord explicite dans la conversation : ok je valide ta proposition tu peux y aller. Processus isolé pour annulation réelle.
