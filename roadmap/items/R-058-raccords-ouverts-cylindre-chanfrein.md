---
id: R-058
titre: "Raccords ouverts cylindre–chanfrein"
resume: "Reconstruire ensemble les parois cylindriques ouvertes et leurs chanfreins coniques partiels, avec contours partagés et validation du solide entier."
statut: livree
jalon: post-v1
cree: 2026-11-16
maj: 2026-11-17
livree: 2026-11-17
---

## But

Reconstruire les extrémités cylindriques des rainures chanfreinées de body 26mm.stl, aujourd’hui laissées facettisées car leur raccord conique partiel n’est pas représenté. Extension du modèle interne explicitement approuvée par l’utilisateur le 16 novembre 2026.

## Inclus

- Représentation additive des cônes partiels entre deux arcs source coaxiaux et deux génératrices source ; reconnaissance initiale des chaînes demi-circulaires de quatre à neuf niveaux axiaux.
- Preuve conjointe des bandes cylindriques et coniques, contours partagés et attribution exhaustive des triangles.
- Intégration à Auto et au guidage local avec conservation des régions indépendantes.
- Construction OCCT suivie, validation du solide réimporté, volume, dimensions et fidélité.
- Régression synthétique et reproduction sur le fichier privé utilisateur, sans publier ce fichier.

## Hors périmètre

Cônes libres arbitraires, pointes partielles, intersections inventées, fermeture de maillage ouvert, augmentation automatique des tolérances et historique paramétrique.

## Dépendances

R-057 pour Auto, guidage et provenance. Accord utilisateur reçu pour l’extension du modèle de données.

## Critères d'acceptation

- Une extrémité de rainure du fichier body 26mm est reconstruite avec son chanfrein, sans détacher de face ou inventer de fermeture.
- Les surfaces natives et la provenance correspondent aux régions prouvées ; retrait/recalcul d’indication restent valides.
- Les chaînes déformées, ambiguës ou incomplètes ne sont jamais acceptées artificiellement.
- Les tests historiques, tests du nouveau raccord, Clippy et builds passent.

## Vérifications de l'utilisateur

- Ouvrir body 26mm.stl dans le build local, examiner une extrémité de rainure et essayer sa correction.
- Réimporter le STEP dans Fusion et vérifier le raccord, la continuité du solide et les dimensions.

## Validation du développement

Revue indépendante acceptée. Body 26mm : Auto et indication Cylindre produisent un solide validé, 14 faces cylindriques et 7 coniques, volume 17 053,652458875804 mm³ ; sélection de 144 triangles étendue aux 432 triangles du raccord, provenance exacte. Tolérances inchangées. Tests synthétiques : rotations, concavité, pentes croissantes/décroissantes, raccords déformés, supports non plans, ordre des sélections, provenance et rejeu/retrait.

423 tests workspace, 32 bureau, 208 UI, 12 Python et 4 SDK réussis ; analyse statique, formatage et builds passent. Le conflit d’artefacts des doctests initiaux a été résolu en séquence ; voir V-1183 pour les journaux et compléments finaux. Bundle artifacts/FabSTEP.app reconstruit et empreinte vérifiée. Graphique et Fusion restent à qualifier dans A-040/A-041. Aucun commit ni push.

## Acceptation utilisateur

Le 17 novembre 2026 : « c’est parfait, je valide ! tu peux continuer. » Correctif accepté. Le réimport indépendant dans Fusion n’est pas explicitement confirmé et reste suivi par A-041.
