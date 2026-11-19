---
id: A-033
type: tester
titre: "Vérifier la correction contextuelle et la lisibilité de FabSTEP"
element: R-042
statut: fait
cree: 2026-11-15
par: agent
fait: 2026-11-19
---

Quitter puis relancer /Users/mugen/Documents/01_Projets/FabSTEP/artifacts/FabSTEP.app, reconstruit le 15 novembre. Ouvrir le sock_clip.stl et utiliser le résultat automatique habituel. Vérifier que les diagnostics courants sont repliés dans Détails techniques. Sélectionner une zone STEP : la barre près de la pièce doit proposer Préparer la correction. Cette action remplace le résultat automatique par des facettes : après le recalcul, sélectionner à nouveau une zone, puis choisir Cylindre ou Cône et Essayer dans la barre. Replier Faces STEP doit laisser la barre accessible ; changer la sélection doit rouvrir le panneau. Indiquer si la prochaine action est claire et visible, en joignant le geste précis si elle ne l’est pas. Les familles guidées restent limitées aux parois complètes déjà prises en charge.


Mise à jour du 16 novembre (UX5/R-057) : le build courant démarre en Auto et propose directement la correction si la provenance est exacte ; le retour général aux facettes n’est plus nécessaire. Préparer la correction reste seulement pour un résultat ancien/non suivi issu d’une stratégie avancée. La barre comporte Auto/Plan/Cylindre/Cône/Tore. Les essais du nouveau parcours sont détaillés dans A-037/A-038.

## Réponse

Accepté par l’utilisateur lors de la validation groupée des tests et des jalons du 19 novembre 2026.
