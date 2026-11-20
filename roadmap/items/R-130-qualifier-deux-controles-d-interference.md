---
id: R-130
titre: "Qualifier deux contrôles d’interférence simultanés"
resume: "F2 · Mesurer le temps et la mémoire cumulée du helper avant d’adopter la concurrence entre groupes."
statut: specifiee
jalon: audit
cree: 2026-11-20
maj: 2026-11-20
livree:
---
## But

Réduire l’attente sur les grands solides dont le temps de contrôle domine encore après simplification des faces. Le résultat doit rester aussi fiable lorsque plusieurs groupes sont contrôlés simultanément.

## Inclus

- Comparer un et deux groupes simultanés sur les mêmes solides publics. Donner à chaque contrôle son propre contexte natif et borner le nombre total de threads, y compris le parallélisme interne OCCT.
- Mesurer le temps mural, le CPU, la mémoire totale du helper et de ses éventuels enfants, ainsi que la latence d’annulation. Comparer ensuite l’ordre habituel à un ordre lançant les groupes estimés coûteux en premier.
- Conserver les diagonales, les contrôles mono-forme, ainsi que les validations indépendantes avant et après transfert.

## Hors périmètre

Passage direct à quatre ou huit groupes, accélération garantie sur Flansch et modification des plafonds.

## Dépendances

R-122 et R-123. Utiliser la composition de faces résultante et la référence mono-forme de R-085. Démarrer uniquement si les groupes restent un coût dominant mesurable.

## Critères d'acceptation

- Sur la rampe, le secteur et un témoin public riche en colliers, enregistrer les mêmes groupes couverts et les mêmes verdicts avant et après transfert. Les triangles sécants et le fil ouvert gardent leurs refus.
- Répéter trois fois chaque configuration sur la même machine. Retenir le mode concurrent seulement si le gain mural dépasse la variabilité observée, avec un pic mémoire agrégé sous 4 Gio. Sinon consigner le résultat du banc et conserver le mode courant.
- Annuler en cours de contrôle puis provoquer un refus dans un groupe. Tous les calculs se terminent et aucun STEP partiel ou ancien ne peut être exporté.

## Vérifications de l'utilisateur

Aucune.
