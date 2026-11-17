---
id: R-074
titre: "Qualifier les grandes zones courbes de Flansch"
resume: "Mesurer les quatre profils courbes encore refusés, identifier les familles géométriques et les preuves manquantes, puis définir une extension bornée sans relâcher les tolérances."
statut: a-tester
jalon: post-v1
cree: 2026-11-17
maj: 2026-11-17
livree:
---

## But

Comprendre les quatre grandes zones courbes de Flansch qui restent en facettes et définir leur reconstruction prouvable. La demande utilisateur lance cette qualification ; une nouvelle représentation géométrique ou une interprétation du maillage nécessitera un arbitrage précis.

## Inclus

- Mesurer les profils, leurs frontières et les résidus des familles candidates.
- Distinguer défaut de détection, famille non prise en charge et ambiguïté réelle.
- Définir la plus petite extension utile et ses tests ; intégrer seulement un correctif local démontré dans les contrats existants.
- Documenter les mesures et les obstacles restant à la conversion complète.

## Hors périmètre

Décimation, relâchement de tolérances, relèvement de plafonds, contours inventés, extension implicite de contrat public ou nouvelle surface libre. Aucun succès STEP déduit d’un simple ajustement de profil.

## Dépendances

R-072 (V-1196), R-073 (surveillance mémoire), fichier Flansch disponible localement. Profils généraux suivis dans R-033.

## Critères d'acceptation

- Quatre zones identifiées, dimensions et résidus mesurés, hypothèses séparées des preuves.
- Ressources de qualification bornées, données privées non versionnées.
- Toute correction éventuelle possède un corpus indépendant et conserve les contrôles source/résultat.
- Proposition de suite concrète, avec limites et dépendances explicites.

## Vérifications de l'utilisateur

- Valider les conclusions et la portée proposée si une extension dépasse les contrats actuels ; aucun nouveau build à essayer si le lot reste une qualification.

## Résultat de qualification

Les quatre zones sont des profils composés mesurés : tore–cône–cylindre (r≈0,5 mm), tore–cylindre–cône–cylindre–cône (r≈2 mm), puis deux cônes suivis d’arcs de 100° (r≈0,3 mm). Résidus de sommets inférieurs à la tolérance d’ajustement, mais absence de témoins axiaux intérieurs dans toutes les bandes droites. Ces ajustements ne valent pas preuve continue ni conversion. Détails V-1198 de `docs/VALIDATION.md`.

Le prototype synthétique du premier raccord et son reflet se réimportent nativement : un solide de cinq faces, dimensions et volume vérifiés. La représentation actuelle suffit ; la détection et les preuves restent à développer. Un test natif indépendant est conservé. Première extension proposée dans R-075, distincte des profils à deux cônes et des arcs de 100°.

Les quatre zones occupent 270 316 plans exclusifs. Leur remplacement idéal par douze surfaces laisserait encore 231 608 faces : aucun STEP complet de Flansch n’est promis. Code de production et build R-073 inchangés.
