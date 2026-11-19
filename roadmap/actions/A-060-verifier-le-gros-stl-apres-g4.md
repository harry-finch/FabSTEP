---
id: A-060
type: tester
titre: "Vérifier le gros STL après G4"
element: R-070
statut: fait
cree: 2026-11-17
par: agent
fait: 2026-11-19
---

Quitter puis rouvrir /Users/mugen/Documents/01_Projets/FabSTEP/artifacts/FabSTEP.app. Ouvrir FlanschTurboLuftfilter_REV02.STL, autoriser l’essai dense et lancer la reconstruction. Le build G4 franchit le premier contour circulaire, mais le test automatisé atteint encore recognition.too_complex sur un contour plus loin : aucun STEP ne doit être exportable. Vérifier que la source reste consultable. Relancer puis annuler pendant le calcul et vérifier le retour à un état utilisable. Répondre avec le message observé et le résultat de l’annulation ; signaler tout blocage.

## Réponse

Accepté par l’utilisateur lors de la validation groupée des tests et des jalons du 19 novembre 2026.
