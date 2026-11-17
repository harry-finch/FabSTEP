# ADR-0026 — Sections polygonales du maillage source

Date : 12 novembre 2026. Statut : DXF2b implémenté ; validation et limites consignées en V-1176.

## Objectif et contrat

L’outil de coupe doit fonctionner dans la vue Source sans exiger une reconstruction STEP réussie. Il produit la section polygonale du maillage préparé, sans reconnaître ni ajuster des lignes, arcs ou autres primitives. La coupe du STEP conserve son parcours et ses courbes natives. Les deux résultats sont distingués dans l’interface et les noms proposés à la sauvegarde.

La préparation source réutilise l’import borné STL/3MF, les unités explicites/déclarées, les placements, le soudage configuré, la topologie et l’orientation existants. Ces étapes sont séparées de la segmentation des faces planes et de la reconnaissance. Les diagnostics et nombres de sommets soudés sont conservés ; « source » ne signifie pas que le prétraitement topologique est ignoré. Le document reste limité à 32 corps, 100 000 triangles et 300 000 sommets. Aucun corps invalide ne peut être omis pour permettre un export partiel.

`fabstep-app::PreparedSourceSection` conserve des maillages privés et immuables, sans objet OCCT. Sa construction exige la validation géométrique native de chaque corps puis la séparation de tout le document. Le core seul ne fournit pas cette autorisation d’export. L’aperçu importé reste disponible en cas de refus.

## Géométrie pure

Le module `fabstep-core::mesh_section` reçoit les maillages indexés en mm et le `SectionRequest` existant. Les identifiants de corps correspondent à l’ordre autoritatif de la préparation. Les intersections sont assemblées par identité de sommet ou d’arête source, jamais par soudage de points proches. Chaque arête traversée fournit un seul point partagé. Les cas dans le plan sont examinés par incidences ; les triangles coplanaires, tangences, embranchements et décisions numériquement ambiguës sont refusés explicitement.

Les contours sont des cycles fermés vérifiés. Contacts/croisements et inclusions sont contrôlés avec budget de travail ; les trous, composantes disjointes et corps demandés vides restent représentés. Aucun contour ouvert n’est refermé artificiellement. Le résultat réutilise `SectionProfile`, exclusivement avec des segments `Line`, et l’exporteur DXF pur existant. Les appels contrôlés vérifient régulièrement l’annulation ; aucun déplacement automatique du plan n’est utilisé pour contourner un refus.

## Préparation et fraîcheur

La commande `prepare_source_section` reçoit les identités de requête/source, une époque monotone de préparation et les options. Des options nulles invalident immédiatement la préparation. Le résultat fournit les corps, noms/provenances, bornes et diagnostics autoritatifs ; les indices de corps ne sont pas déduits du simple aperçu importé.

La préparation validée est réutilisée pendant les déplacements du plan. Les options, l’unité STL, les octets source, la révision et les époques participent à son identité. Les contextes de section distinguent `target: "step"` et `target: "source"` ; ce dernier exige l’identifiant de préparation correspondant. La résolution du plan d’une face STEP n’est pas proposée dans la coupe source. Les profils source ne deviennent jamais des STEP exportables et n’effacent pas un résultat STEP validé.

L’interface borne chaque file à un travail actif et une dernière demande, ignore les réponses tardives et invalide l’export dès un changement. L’export relit uniquement le profil natif du contexte courant et conserve les gardes avant/après dialogue jusqu’au remplacement atomique. La préparation et la section s’exécutent hors du fil UI ; le worker d’aperçu reste partagé avec les profils STEP.

## Validation

Comparaison avant/après du refactoring de préparation sur dix analyses complètes, dont orientations inversées ; corpus de sections polygonales, cas invalides, unités et placements 3MF ; annulation, cache, changement de source/options/cible et sauvegarde tardive. Un maillage valide dont la reconnaissance échoue doit néanmoins produire une coupe. Les DXF sont relus par un lecteur indépendant. Les essais graphiques natifs et Fusion sont distingués des tests automatisés dans V-1176.

Les prédicats utilisent des filtres arithmétiques et refusent un signe indéterminé ; ils ne prétendent pas fournir une arithmétique exacte générale. Les produits trop petits pour garantir leur résidu sont également refusés, y compris certains produits encore normaux dont la partie résiduelle pourrait subir un sous-flux. Cette précaution suit la limite connue des transformations arithmétiques en présence de sous-flux/sur-flux ; voir [Nanevski et al., Automatic Generation of Staged Geometric Predicates](https://www.cs.cmu.edu/~rwh/papers/geompred/icfp01.pdf). Aucun code de cette publication ni nouvelle bibliothèque n’est incorporé.
