---
id: R-071
titre: "Accélérer les fenêtres d’arcs des contours complexes"
resume: "Après G4, Flansch franchit le cercle dense mais épuise encore le budget sur la face 7 278, contour de 300 points réellement non circulaire. Étudier une réduction prouvée du coût des fenêtres, sans supposer la monotonie des ajustements ni augmenter le budget ; une nouvelle preuve numérique exige une qualification distincte."
statut: livree
jalon: post-v1
cree: 2026-11-17
maj: 2026-11-19
livree: 2026-11-19
---

## But

Réduire les essais redondants lors de la recherche d’arcs sur les contours complexes, pour permettre au moteur de progresser sur les gros STL sans perdre de candidats valides. L’accord utilisateur « tu peux poursuivre » valide la démarche présentée après G4 ; la conversion de Flansch reste à vérifier, sans promesse de succès.

## Inclus

- Mesurer séparément les ajustements des fenêtres et leurs comparaisons de couverture.
- Éliminer des calculs répétés ou inutiles avec une équivalence démontrée et un repli conservateur quand les préconditions ne sont pas réunies.
- Préserver les coordonnées, tolérances, fenêtres non monotones, candidats maximaux, provenance et refus géométriques.
- Comparer aux parcours de référence, mesurer le fichier réel et publier le résultat exact.

## Hors périmètre

Hausse des plafonds, décimation, nouvelles primitives, contrat public modifié, export partiel, changement d’interface. Une nouvelle preuve numérique ou un changement architectural substantiel nécessite un arbitrage distinct.

## Dépendances

G4/R-070 intégré (a49c715). Fichier privé Flansch disponible localement, non versionné.

## Critères d'acceptation

- Travail supprimé mesuré et justifié ; comptabilisation conservatrice du travail restant.
- Identité des décisions et des valeurs géométriques avec l’oracle, hors refus de ressources explicitement documentés.
- Cas circulaires, bruités, ambigus, seuils, non-monotonie et budgets couverts.
- Tests et validations natives passants, paquet reconstruit, limitations annoncées.

## Vérifications de l'utilisateur

- Réessayer le gros STL et son annulation dans le build mis à jour ; signaler le résultat ou le refus final.

## Mise en œuvre et portée

Le profilage avant modification attribue 2 919 238 unités à la maximalité, contre 1 398 664 aux fenêtres ajustées. L’optimisation conserve les fenêtres et les ajustements ; elle remplace uniquement l’inclusion d’ensembles d’arêtes par une inclusion d’intervalles cycliques après preuve d’unicité des sommets du contour. Préparation et comparaisons sont débitées, allocation faillible, parcours historique conservé si l’unicité ne tient pas. Un budget épuisé reste bloquant, sans repli.

La reconnaissance du fichier réel termine après cette modification : 4 cylindres, 8 cônes et 501 900 plans. Le prochain refus se situe dans le suivi des faces du résultat (100 000 au maximum), avant construction du STEP. Le message est précisé pour afficher compte et limite, sans changer le code stable ni le plafond. Cette suite est suivie dans R-072.

Le garde d’unicité peut coûter davantage pour un grand contour avec très peu de candidats ; aucun gain uniforme ni équivalence des refus de ressources n’est promis. Les preuves d’équivalence concernent les candidats et leurs valeurs géométriques.

## Validation

V-1195 consigne les oracles, le benchmark et la qualification : 496 tests workspace, 49 bureau, 8 helper, 68 ciblés avec `large-stl`, 1 transfert réel, 229 UI, 12 Python et 4 SDK réussis ; formatage, Clippy, Svelte et builds passent. Le paquet macOS est reconstruit et ses exécutables vérifiés. Le rejeu final du helper embarqué affiche bien le refus avec 501 912 faces et la limite de 100 000, sans STEP. Essai manuel A-061 ; aucun essai graphique ni Fusion revendiqué.
