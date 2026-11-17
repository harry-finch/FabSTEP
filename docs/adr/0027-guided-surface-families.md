# ADR-0027 — familles de surfaces guidées : cylindre et cône

Date : 12 novembre 2026. Statut : implémenté, validation consignée dans V-1177.

## Besoin et périmètre I2c

L’utilisateur souhaite sélectionner des facettes du STEP puis indiquer quelle forme chercher pour corriger cette zone. I2a/I2b proposent déjà les cylindres complets et les indications successives. I2c ajoute un choix contextuel **Cylindre / Cône**, sans changer le mode global ni les tolérances.

Le cône de ce lot est une paroi complète de tronc de cône, extérieure ou intérieure, entre deux contours circulaires de rayon positif réellement portés par des plans source. Les pointes, cônes partiels et raccords directs vers une autre surface courbe sont refusés. Une chaîne cône–cylindre reconnue automatiquement n’est pas une autorisation de reconstruire des voisins non indiqués. Les sphères, tores et cylindres ouverts guidés restent des extensions distinctes.

## Preuve géométrique

`GuidanceFamily` et `recognize_guided_surfaces` ajoutent un parcours pur produisant un `RevolvedRecognition`. La recherche des contours plans, les candidats cylindriques et la preuve historique de révolution sont réutilisés. Une paroi conique doit être une composante bornée exactement par les deux contours source ; ses triangles, frontières et régions consommées sont contrôlés. La preuve de révolution est limitée à cette paroi et exige exactement un cône, sans cylindre ni autre anneau. Les raccords circulaires, résidus, flèche et volume analytique restent soumis aux contrôles existants.

Chaque sélection doit appartenir entièrement à une seule paroi prouvée de la famille demandée. Les indications ambiguës, dupliquées ou se recouvrant sont refusées. Une recherche par corps partage un budget de travail ; les limites de 16 indications et 100 000 triangles sélectionnés par document restent inchangées. Les autres régions restent facettisées. Le parcours historique cylindre seul garde son `MixedRecognition` et ses contrats.

## Provenance et validation native

L’API CAD additive `build_revolved_step_with_provenance` reçoit des jetons exhaustifs et uniques pour les tableaux plans, cylindres et cônes. Chaque jeton est associé à la face effectivement construite avant la couture, puis suivi à travers les transferts STEP. L’ordre natif de parcours des faces ne sert jamais de correspondance source. Les anciens points d’entrée ABI restent disponibles.

L’application conserve la partition source complète du résultat guidé. Les triangles sélectionnés et la paroi reconstruite sont tracés séparément, avec la famille et l’identifiant stable de chaque indication. Tous les corps source et le résultat complet sont validés ; le réimport, le volume, les dimensions, la séparation et l’audit de fidélité ne sont pas contournés.

## Commande et session

`DocumentStep::guided_surface` ajoute l’entrée par famille ; `guided_cylinder` reste un wrapper compatible. La commande bureau `guided_convert` accepte un champ facultatif `family` : son absence conserve le cylindre historique. Une famille inconnue est refusée avec `guidance.unsupported_family` dans la nouvelle révision consommée ; l’ancien résultat ne reste pas exportable après cette tentative.

La source, les options et les indications acceptées restent privées au moteur. Ajouter, retirer ou recalculer revalide le document entier. Un refus conserve la session acceptée, sans republier ses anciens octets STEP. Les identités source/révision et les gardes d’annulation restent actives.

L’interface envoie la famille au moment de l’action et nomme les indications depuis le rapport natif. Le sélecteur ne modifie aucune géométrie seul. La sélection conserve les règles de provenance exacte, de faces planes et d’un seul corps. Les parcours français et anglais restent complets.

## Compatibilité et limites

Aucune dépendance ajoutée, aucun format de projet persistant modifié. Les rapports existants acceptaient une famille textuelle ; `cone` s’ajoute aux valeurs produites. Les anciennes API cylindriques et les conversions automatiques restent disponibles. Retirer la dernière indication conique peut ramener un corps au parcours cylindrique historique ; les identifiants des autres indications sont conservés.

Les tests automatisés et la revue sont consignés dans [VALIDATION](../VALIDATION.md). Ils ne remplacent pas les essais graphiques natifs et dans Fusion. Le paquet autonome 1.0.0 reste distinct du build de développement.
