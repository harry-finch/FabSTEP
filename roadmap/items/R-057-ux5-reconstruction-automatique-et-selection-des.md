---
id: R-057
titre: "UX5 — reconstruction automatique et sélection des parois"
resume: "Ouvrir et obtenir le meilleur STEP prouvé, sélectionner une paroi en un geste et corriger localement sans perdre les surfaces reconnues."
statut: livree
jalon: atelier
cree: 2026-11-16
maj: 2026-11-19
livree: 2026-11-19
---

## But

Simplifier la conversion et la correction des STL complexes : ne plus demander à l’utilisateur de choisir entre facettes et formes courbes, ni sélectionner une paroi trouée facette par facette. Périmètre approuvé le 16 novembre : « fait tout d’un coup » après proposition de sélection rapide et reconstruction automatique locale.

## Inclus

- Reconstruction Auto par défaut dans le bureau : recherche bornée des familles compatibles, régions prouvées conservées et régions incertaines facettisées avec diagnostic.
- Modes historiques et réglages explicites conservés dans Avancé ; aucune augmentation silencieuse des tolérances.
- Double-clic sur une face STEP pour sélectionner sa paroi prouvée, limites géométriques respectées ; Maj-clic pour ajouter/retirer une face.
- Cliquer-glisser avec le bouton gauche maintenu ajoute les faces STEP visibles survolées, sans dépendre de la reconnaissance ; le passage répété sur une face ne la retire pas. Rotation au bouton droit pendant le mode sélection.
- Extension topologique par voisinage accessible dans Avancé ; absence de paroi prouvée n’entraîne pas la sélection du corps entier.
- Correction Auto, Plan, Cylindre, Cône et Tore depuis le résultat à provenance exacte, sans retour obligatoire aux facettes ni perte des régions automatiques indépendantes.
- Pour Tore, la zone comprend le raccord et sa portée cylindrique nécessaire, explicités dans l’interface ; aucun raccord libre inventé.
- Provenance native des modèles arrondis, incluant sphères, tores et cylindres partiels ; sessions retirables/recalculables.
- Repli des regroupements plans non constructibles vers les triangles source, sans déplacement des frontières ; contrôle natif final du document entier.

## Hors périmètre

Reconnaissance universelle des formes libres, raccords toriques arbitraires et reconstruction d’un historique paramétrique. Les boîtiers arrondis utilisent un modèle dédié incompatible avec le modèle local guidé : conserver leur mode explicite existant, sans promettre leur correction locale dans ce lot. Toute limite observée est consignée.

## Dépendances

R-042, R-048 et R-056 fournissent atelier et sessions existantes. R-050 pour la famille Tore intégrée à ce parcours. Les qualifications utilisateur précédentes restent distinctes.

## Critères d'acceptation

- Une pièce mêlant région reconnaissable et région non prouvée produit un solide complet, avec reconnaissance seulement de la région démontrée ; source invalide et budgets de travail restent refusés.
- Double-clic sur une paroi à nombreuses facettes sélectionne cette paroi sans ses bouchons ni tout le corps ; réponse périmée ignorée et export intact.
- Ajouter/retirer une correction locale préserve les régions indépendantes ; provenance exhaustive, réimport solide, dimensions, volume et audit réussis.
- Les tests des modes historiques restent verts ; tests nouveaux Auto, Tore, provenance native et gardes UI/bureau.
- Contrôles globaux et build macOS réussis ; limitations graphiques ou de corpus déclarées.

## Vérifications de l'utilisateur

- Ouvrir body 26mm.stl et sock_clip.stl, sélectionner un trou par double-clic et corriger une zone sans perdre les autres formes reconnues.
- Vérifier l’interface simplifiée et le parcours Tore, puis ouvrir le STEP dans Fusion pour contrôler surfaces, dimensions et solide.

## Validation du développement

Revue indépendante acceptée. `scripts/check.sh` passe : 415 tests workspace, 32 bureau, 197 UI, 12 Python et 4 SDK, formatage, Clippy strict, Svelte et builds. Le paquet `artifacts/FabSTEP.app` est reconstruit le 16 novembre ; binaire vérifié identique au build. V-1181 contient les résultats privés et limites de validation. Les essais graphiques et Fusion restent A-037/A-038. Aucun commit ni push.

## Retour utilisateur — sélection manuelle

Le double-clic n’étend pas certaines zones facettisées. Le regroupement actuel ne sait sélectionner que les régions prouvées ; aucune nouvelle preuve géométrique n’est déduite de ce retour. L’utilisateur demande de sélectionner en maintenant le clic : ajout d’un balayage visible et additif, indépendant de la reconnaissance, sans mutation du STEP. Un double-clic sans extension doit désormais afficher une aide explicite. Gestes, annulation du pointeur et commandes worker sont testés séparément du rendu natif.

Validation du correctif de sélection : 204 tests UI, Svelte et build macOS réussis ; revue du raccordement par l’orchestrateur. V-1182 détaille les preuves et la limite de vérification graphique. Essai du balayage demandé dans A-039.

## Retour utilisateur — refus sur une paroi cylindrique

Sur body 26mm.stl, une sélection de 144 triangles au bout arrondi d’une rainure est refusée avec `guidance.family_unproven`. Investigation autorisée : reproduire la zone, vérifier la preuve du cylindre partiel et ses raccords, corriger sans assouplir les tolérances. Le refus d’un essai guidé doit être distingué d’un échec de conversion initiale ; les règles de validité de l’export restent inchangées.

Le refus est désormais présenté comme « Correction non appliquée », avec recalcul directement accessible et sans restauration d’un export ancien. La cause géométrique du fichier privé est traitée dans R-058 : chaîne demi-circulaire cylindre–chanfrein–cylindre. La sélection s’étend aux bandes dépendantes ; 208 tests UI et tests natifs ciblés passent. V-1183 conserve les preuves détaillées et les limites graphiques.
