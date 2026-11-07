# ADR-0006 — Cônes et chanfreins circulaires explicites

Statut : accepté ; implémenté en 0.6.0-alpha.1
Date : 6 novembre 2026

## Contexte

Le mode Cylindres refuse les cônes et les transitions coniques. Les fichiers STL et 3MF ne donnent pas l'intention d'un chanfrein ; la présence de deux cercles de rayons différents ne prouve pas à elle seule une paroi conique. Une forme sphérique ou un congé tessellé ne doit pas devenir une suite de petits cônes.

## Décision

Ajouter un mode explicite `primitives`, présenté comme « Cônes et cylindres ». Les modes `planar` et `cylinders`, leurs seuils et leurs refus restent disponibles. Les chanfreins droits entre faces planes sont déjà représentés par des plans ; ce volet ajoute les chanfreins circulaires et fraisures démontrables par des bandes coaxiales.

`fabstep-core` retourne un `RevolvedRecognition` séparé : son `RevolvedModel` contient un `MixedModel` et des `MixedCone`. Chaque cône référence deux anneaux ; un rayon nul représente uniquement une pointe présente et prouvée dans la source, jamais un sommet inventé. Cette exception appartient au nouveau modèle : l'API historique des modèles mixtes reste stricte. Les interfaces restent des données Rust sérialisables, sans objet OCCT.

Les bandes de triangles doivent démontrer l'axe, les rayons, la correspondance angulaire et leur variation affine. Les bandes coniques collinéaires peuvent être regroupées. Une seule plage conique est admise par chaîne connectée de bandes, même si des bandes interprétables comme cylindriques séparent plusieurs cônes. La revue a montré que de très petits changements de rayon pouvaient autrement contourner le refus d’une méridienne non affine. Ce choix conservateur évite l’interprétation de courbes en cônes par morceaux ; des chaînes comportant plusieurs chanfreins réels peuvent donc être refusées. Les transitions cône–cylindre et cône–plan sont admises lorsqu'elles sont prouvées ; les portions coniques ouvertes sont différées.

La reconnaissance fournit une attribution exhaustive des triangles, les résidus, la flèche des cordes, une borne composée et le volume analytique attendu. Un candidat conique dont la preuve échoue est bloquant (`recognition.conic_unproven`) ; seule l'absence de candidat (`recognition.unsupported`) autorise l'essai de la reconnaissance cylindrique historique. Les dépassements de budget ne sont jamais masqués par un repli.

`fabstep-cad` construit et réimporte le STEP, vérifie les solides et compte séparément les surfaces coniques. `fabstep-app` conserve la validation exacte des triangles source, les contrôles de volume/dimensions, l'audit de distance échantillonné et la séparation des corps. Tous les corps doivent réussir avant export.

## Alternatives examinées

- Élargir silencieusement le mode Cylindres : évité pour conserver un choix explicite et les régressions existantes.
- Reconstruire directement un profil de révolution global : trop restrictif pour une fraisure dans une plaque polygonale.
- Ajuster un cône aux seuls sommets ou aux deux contours terminaux : insuffisant pour exclure un renflement, une torsion, un raccord ou une paroi manquante.
- Assimiler tout chanfrein à un cône : incorrect ; les chanfreins droits restent des faces planes.

## Conséquences

Le nouveau modèle ajoute des transitions analytiques partagées sans modifier les API de reconstruction historiques. Les sphères, tores, congés, cônes partiels et chaînes comprenant plusieurs plages coniques ne sont pas pris en charge par ce volet. La deuxième pièce utilisateur contient plusieurs de ces surfaces et ne constitue pas un critère de réussite intégrale de M4a. L'aperçu reste celui du maillage source et de ses frontières reconnues, pas une tessellation de la BRep exportée.

## Validation

Corpus analytique indépendant : cône tronqué, pointe, chanfrein circulaire, fraisure et fond conique, rotations, dimensions et volumes nominaux ; négatifs bruités, non affines, peu échantillonnés, incohérents et budgets dépassés. Réimport FreeCAD avec surfaces `Cone`, rayons, angles, longueurs et fermeture. Vérifier les modes historiques, le multi-corps, la traçabilité des paramètres, l'invalidation après changement et l'export natif. Les résultats effectivement exécutés figurent dans `docs/VALIDATION.md`. Le cas à pentes alternées est une régression de la contre-revue, pas un succès accepté.
