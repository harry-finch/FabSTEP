---
id: R-116
titre: "Estimer les normales sur copie Taubin en lecture seule"
resume: "Calculer des normales lissées sur une copie temporaire sans déplacer les sommets source, uniquement pour stabiliser la segmentation et la découverte de candidats sur les STL quantifiés ou bruités."
statut: livree
jalon: v1.0
cree: 2026-11-19
maj: 2026-11-20
livree: 2026-11-20
---


## But

Sur les STL binaires (`f32`) ou légèrement bruités, la quantification des sommets dévie les normales des petits triangles de bordure et fragmente les régions planes ou les zones candidates. L’utilisateur obtient alors des facettes au lieu de surfaces reconnues, même quand la pièce respecte les tolérances. Calculer des normales sur une copie temporaire lissée par filtre de Taubin, sans jamais modifier les sommets source, stabilise la découverte des régions tout en gardant intactes les preuves géométriques et la fidélité du STEP.

## Inclus

- Calcul déterministe en Rust pur (`fabstep-core`, `f64`, sans `HashMap`) d’une copie temporaire lissée par filtre de Taubin (sans rétrécissement de volume, nombre d’itérations fixe et borné, coût débité sur le budget de travail existant).
- Utilisation des normales lissées uniquement comme signal auxiliaire pour la segmentation initiale et la proposition de candidats (plans, bandes courbes), jamais pour la géométrie exportée.
- Conservation stricte des coordonnées des sommets source : tous les résidus, bornes continues, déplacements de soudage, attributions de triangles et constructions BRep OCCT restent évalués sur le maillage source non lissé.
- Comparaison mesurée avant/après sur un corpus de STL binaires `f32` tournés et de maillages légèrement bruités, en vérifiant l’absence de régression sur le corpus existant.
- Signalement par diagnostic stable uniquement si une seconde passe à normales lissées est activée en recours, sans modifier les tolérances choisies par l’utilisateur.

## Hors périmètre

- Remplacement, lissage ou projection des sommets du maillage source ou du STEP construit.
- Relâchement automatique des tolérances de soudage, de planéité, d’ajustement radial ou du budget d’écart.
- Débruitage de scans 3D organiques ou reconstruction de surfaces libres arbitraires.
- Modification de l’ABI C++/OCCT ou des formats JSON/IPC existants (hors diagnostic éventuel).

## Dépendances

- Complémentaire de `R-115` (récupération des plans fragmentés par graine de grande aire) : évaluer le gain propre des normales Taubin sur les cas que `R-115` seul ne suffit pas à récupérer.

## Critères d'acceptation

- Les sommets du maillage source et du solide STEP construit restent strictement identiques aux coordonnées soudées d’origine (aucun déplacement dû au lissage Taubin).
- Un témoin STL binaire `f32` tourné et un témoin synthétique à micro-bruit sous la tolérance retrouvent leurs faces planes et surfaces analytiques prouvées, avec réimport STEP, volume et audit de fidélité valides.
- Un maillage dont la déformation dépasse réellement la tolérance ou le budget d’écart reste refusé avec son code stable habituel (aucun faux positif créé par le lissage).
- Le calcul des normales lissées est déterministe (résultat identique d’un passage à l’autre) et borné dans le budget de travail existant, y compris sur les maillages proches de 100 000 triangles.
- `sh scripts/check.sh` passe sans erreur ni avertissement.

## Vérifications de l'utilisateur

- Tester la reconstruction sur un STL binaire réel précédemment fragmenté dans FabSTEP et vérifier le STEP exporté dans Fusion.
