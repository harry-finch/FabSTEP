---
id: R-026
titre: "Qualifier le garde de cocircularité des anneaux fermés"
resume: "Le défaut de conditionnement a été reproduit puis corrigé dans R-070. Les anneaux clairsemés et les profils en D sont couverts par les tests de V-1194."
statut: a-tester
jalon: audit
cree: 2026-11-13
maj: 2026-11-18
livree:
---

| Qualifier le garde de cocircularité des anneaux fermés | AUD-017 | P2, preuve préalable | Construire une reproduction du défaut supposé ; conserver la protection des anneaux peu échantillonnés. Aucun remplacement direct par `fit_circle` sans cette preuve |

Source : [ancienne feuille de route](../../docs/ROADMAP-avant-fabboard.md). Les validations historiques sont conservées ; aucun essai nouveau n’est déduit de la migration.

## Qualification

Le contour réel de 362 sommets de Flansch et 48 variantes synthétiques quantifiées reproduisent la sensibilité du cercle contraint par ses deux extrémités adjacentes. R-070 complète ce garde par un ajustement sur tous les sommets uniquement lorsque le premier calcul échoue. Les profils en D et les anneaux clairsemés conservent leurs décisions. Les tests de `partial::tests` et le protocole de V-1194 permettent de reproduire la qualification. L’essai applicatif associé reste suivi par A-060.
