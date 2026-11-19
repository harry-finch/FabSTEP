---
id: R-063
titre: "F0 — qualifier les courbes des perçages sécants"
resume: "Vérifier dans un laboratoire natif borné les courbes et paramètres de deux cylindres orthogonaux après réimport STEP, avant intégration du contrat R-036."
statut: livree
jalon: post-v1
cree: 2026-11-17
maj: 2026-11-19
livree: 2026-11-19
---

## But

Préparer l’extension des perçages sécants approuvée explicitement le 17 novembre (A-045), en prouvant d’abord la précision des courbes du STEP réellement réimporté. Ce laboratoire précède l’intégration R-036 dans l’application.

## Inclus

- Bloc et deux perçages centrés orthogonaux, rayon du grand trou au moins deux fois celui du petit, avec marges résolues.
- Générateur natif de laboratoire, export/réimport STEP, contrôle d’un solide, de six plans et trois faces cylindriques.
- Bornes continues sur les courbes d’intersection et leur correspondance paramétrique sur les deux cylindres ; travail et tailles explicitement limités.
- Volume indépendant, sondes matière/vide, refus de données ou de certificats hors domaine.

## Hors périmètre

Reconnaissance du STL, nouveau contrat de production, raccordement à l’interface et export DXF. Une construction booléenne de référence ne prouve pas la reconstruction depuis un maillage. Les intersections exportées ne sont pas annoncées comme des courbes mathématiquement exactes.

## Dépendances

Accord A-045 obtenu. SDK OCCT existant ; aucune nouvelle dépendance. R-060 à R-062 restent indépendants. L’intégration R-036 attend les preuves sur les courbes réimportées et les paramètres de surface.

## Critères d'acceptation

- Laboratoire reproductible et isolé des chemins de production.
- Contrôles positifs et négatifs, fermeture, topologie et volume vérifiés après réimport.
- Les bornes annoncées portent sur les courbes réellement obtenues ; aucun échantillonnage seul présenté comme certificat continu.
- Tout échec d’une preuve reste explicite et empêche de déclarer R-036 prêt pour l’export.

## Vérifications de l'utilisateur

- Examiner le bilan du laboratoire et ses limites avant l’essai de l’intégration dans l’application.

## Avancement — 17 novembre 2026

Laboratoire implémenté, **PARTIAL** (V-1187). Réimport : un solide, six plans et trois faces cylindriques ; sondes et tests élémentaires positifs passent. Le certificat de la première arête épuise le budget global avant de couvrir la branche ; les splines périodiques demandent également un traitement borné. Aucune activation dans l’application, aucun contrat de production ajouté. Reste à réduire le coût des bornes continues, traiter le périodique et qualifier le calcul du volume avant R-036. Il n’y a pas de nouvel arbitrage utilisateur bloquant : A-045 est obtenu.


## Résultat de la reprise

V-1200 remplace le constat courant PARTIAL de V-1187 : les trois courbes réimportées, y compris la périodique, et leurs cycles passent les certificats à 1e-5 et 1e-7 mm dans le budget inchangé. Référence analytique de volume encadrée ; volume OCCT adaptatif mesuré séparément. Revue indépendante et rejeu natif identique par l’orchestrateur PASS. Aucun chemin de conversion activé ; R-036 doit encore prouver le maillage source et intégrer le contrat et le contrôle de volume.
