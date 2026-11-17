---
id: R-072
titre: "Qualifier les résultats très facettisés des gros STL"
resume: "Flansch franchit désormais la reconnaissance avec 4 cylindres et 8 cônes, mais conserve 501 900 plans, soit 501 912 faces au total. Le suivi du résultat est limité à 100 000 faces. Identifier les zones qui restent facettisées et les réductions géométriquement prouvées possibles ; examiner les coûts CAO, mémoire et inspection avant toute proposition de changement de limite ou de simplification."
statut: idee
jalon: post-v1
cree: 2026-11-17
maj: 2026-11-17
livree:
---

Le premier refus observé est dans `rounded_source_faces`. Le constructeur CAO mixte arrondi impose indépendamment `MAX_RESULT_FACES = 100_000` : supprimer la provenance optionnelle du helper ne suffirait donc pas. Aucune construction OCCT ni qualification mémoire d’un résultat de 501 912 faces n’est acquise.

La qualification devra aussi couvrir les consommateurs du résultat : rapport et STEP limités chacun à 128 Mio dans le protocole helper, audit et aperçu avec leurs bornes propres. Une modification isolée du nombre de faces ne constitue pas une qualification de ce parcours.
