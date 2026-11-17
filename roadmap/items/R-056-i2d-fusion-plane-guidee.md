---
id: R-056
titre: "I2d — fusion plane guidée"
resume: "Ajouter Plan aux familles guidées : fusionner une sélection contiguë presque plane, sans déplacer ses sommets, après contrôle du solide."
statut: a-tester
jalon: atelier
cree: 2026-11-15
maj: 2026-11-15
livree:
---

## But

Corriger une zone plane à partir des faces STEP sélectionnées, au même endroit que les familles Cylindre et Cône. Suite approuvée dans la conversation du 15 novembre : Plan puis Tore.

## Inclus

- Choix Plan et action Essayer dans la barre contextuelle, français et anglais.
- Fusion de faces polygonales contiguës d’un seul corps ; toute la sélection est vérifiée contre un même plan déterministe.
- Conservation des sommets et des contours source, trous compris ; aucune projection ni fermeture inventée.
- Refus explicite des sélections non planes, disjointes, chevauchantes ou contenant des raccords courbes non pris en charge.
- Indications retirables et recalculables ; cohabitation avec Cylindre/Cône dans les zones indépendantes.
- Provenance exacte, borne d’écart et validation native du document entier avant export.

## Hors périmètre

Projection ou déplacement des frontières, fusion de plans bordés de courbes, tore et sphère guidés (R-050). Le lot ne garantit pas de réparer tout STL refusé à l’ouverture.

## Dépendances

R-048 pour la session guidée, R-042 pour la barre contextuelle. Capacités déjà développées ; qualification utilisateur toujours ouverte.

## Critères d'acceptation

- Un cas à facettes distinctes presque planes est fusionné avec provenance exhaustive et sommets inchangés.
- Les trous sont préservés ; les sélections déconnectées, déformées et chevauchantes sont refusées avec code stable.
- Ajouter puis retirer une indication Plan conserve le parcours des indications Cylindre/Cône indépendantes.
- Le STEP réimporté est un solide validé ; dimensions, volume et audit passent les contrôles existants.
- Tests Rust, bureau et UI, formatage, Clippy et builds réussis ; limites de vérification visuelle déclarées.

## Vérifications de l'utilisateur

- Essayer Plan sur une zone sélectionnée et vérifier que le résultat ou le motif de refus est compréhensible.
- Réimporter un STEP accepté dans Fusion et vérifier ses faces, ses dimensions et ses trous.

## Validation du développement

Build de développement `artifacts/FabSTEP.app` reconstruit le 15 novembre. Revue indépendante acceptée ; scripts/check.sh passe : 400 tests workspace, 32 bureau, 184 UI, 12 Python et 4 SDK, formatage, Clippy, Svelte et builds. Preuves détaillées V-1180 et ADR-0028. Contrôle graphique automatisé indisponible ; vérification utilisateur A-035/A-036 encore ouverte. Aucun commit ni push.
