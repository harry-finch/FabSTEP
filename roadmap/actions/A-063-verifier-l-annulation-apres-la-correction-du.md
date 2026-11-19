---
id: A-063
type: tester
titre: "Vérifier l’annulation après la correction du suivi mémoire"
element: R-073
statut: fait
cree: 2026-11-17
par: agent
fait: 2026-11-19
---

Ouvrir artifacts/FabSTEP.app après le build R-073, charger un STL binaire de plus de 100 000 triangles, accepter Essayer quand même, puis Annuler pendant le calcul. Vérifier que le calcul s’arrête, que le maillage source reste visible et qu’aucun ancien STEP ne devient exportable. Répondre avec le résultat. Ne pas provoquer volontairement un dépassement mémoire : les seuils et le nettoyage sont testés automatiquement. Flansch reste susceptible de refuser ses 501 912 faces si le calcul termine avant l’annulation.

## Réponse

Accepté par l’utilisateur lors de la validation groupée des tests et des jalons du 19 novembre 2026.
