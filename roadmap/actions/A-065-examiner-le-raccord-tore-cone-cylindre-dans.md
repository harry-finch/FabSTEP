---
id: A-065
type: tester
titre: "Examiner le raccord tore–cône–cylindre dans FabSTEP"
element: R-075
statut: fait
cree: 2026-11-17
par: agent
fait: 2026-11-19
---

Ouvrir artifacts/FabSTEP.app, charger fixtures/torus_cone_cylinder/nominal.stl puis cliquer Reconstruire le STEP. Attendre un résultat STEP validé : deux faces planes et trois faces latérales (tore, cône, cylindre), sans facettes sur le raccord. Répéter avec mirrored.stl et opposite_branch.stl. Vérifier la sélection des faces et exporter le STEP. Le guidage Tore sur ce raccord avec voisin conique reste hors périmètre. Flansch entier reste refusé au plafond de faces ; ce refus ne signifie pas que le raccord de référence a échoué.

## Réponse

Accepté par l’utilisateur lors de la validation groupée des tests et des jalons du 19 novembre 2026.
