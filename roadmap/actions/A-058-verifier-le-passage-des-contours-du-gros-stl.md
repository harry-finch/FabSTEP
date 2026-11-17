---
id: A-058
type: tester
titre: "Vérifier le passage des contours du gros STL avec G2"
element: R-068
statut: a-faire
cree: 2026-11-17
par: agent
fait:
---

Quitter l’ancienne instance et ouvrir /Users/mugen/Documents/01_Projets/FabSTEP/artifacts/FabSTEP.app. Charger /Users/mugen/Downloads/FlanschTurboLuftfilter_REV02.STL, puis lancer l’essai explicite du gros fichier. Le moteur doit franchir l’analyse et atteindre la reconnaissance ; dans la qualification automatisée, il refuse ensuite sur recognition.too_complex après environ 109 s, sans STEP exportable. Relancer puis utiliser Annuler pour vérifier que le fichier reste disponible et que l’interface redevient utilisable. Indiquer le message observé et si l’annulation fonctionne ; les durées peuvent varier.
