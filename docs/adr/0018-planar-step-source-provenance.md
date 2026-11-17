# ADR-0018 — provenance exacte des faces planes STEP

Date : 9 novembre 2026. Statut : accepté et implémenté pour le mode Plans.

## Contexte

I1b permet de sélectionner des faces natives du STEP réimporté. La correction guidée doit retrouver leur zone source sans assimiler les indices des faces, des solides et des corps importés. Une proximité géométrique ne constitue pas une provenance.

## Décision

Le premier domaine couvert est le mode Plans, présenté comme **Conserver les facettes**. Le pipeline conserve l’appartenance exacte des triangles aux régions planes ainsi que leur correspondance aux triangles de l’aperçu importé après séparation des composantes, soudage et orientation. Les identifiants des instances 3MF placées restent distincts.

Le constructeur CAO suit chaque face à travers l’historique de couture. Les tables de transfert OCCT relient les faces construites aux entités `AdvancedFace` écrites, puis aux faces réimportées. Des noms réservés `FABSTEP_SOURCE_FACE_V1_<token>` transportent les identifiants dans le STEP, y compris lors de la combinaison des racines. Leur lecture seule ne suffit jamais à affirmer une correspondance source : elle doit être jointe aux données de provenance privées produites avec ces mêmes octets de résultat.

La correspondance doit être exhaustive et non ambiguë. Toute perte de suivi rend la provenance indisponible pour le document ; elle ne transforme pas un solide valide en échec de conversion. Les validations géométriques et les budgets existants demeurent obligatoires. Aucune attribution par rang de parcours, support géométrique ou plus proche triangle n’est admise.

Une API compagnon conserve les octets STEP et leur provenance dans un objet opaque `DocumentStep`. L’API historique de conversion reste disponible et peut abandonner ces métadonnées. Le cache bureau conserve un snapshot atomique du résultat ; les contrôles de source, révision et identité du résultat rejettent les réponses périmées.

L’interface reçoit soit `unavailable` avec un code stable, soit `exact` avec, pour chaque face native, les identifiants de corps source/analysé et les indices de triangles source. Le worker vérifie les limites, la couverture et la partition. Une métadonnée invalide désactive le lien source sans supprimer le graphe STEP valide ni l’export.

**Voir la zone source** affiche l’union des triangles correspondant aux faces sélectionnées, éventuellement dans plusieurs corps. **Retour au STEP** conserve la sélection native. Cette inspection n’applique aucune correction géométrique.

## Limites et suite

Les reconstructions analytiques restent sans correspondance STEP/source jusqu’à l’ajout d’un suivi exact de leurs constructeurs. Les noms STEP ne constituent ni un format de projet persistant ni une autorité lors de l’import d’un fichier externe. La modification manuelle du STEP et la persistance intersessions ne sont pas couvertes.

Ce lot apporte uniquement la liaison entre la sélection du résultat et la source. L’[ADR-0019](0019-guided-complete-cylinder.md) définit ensuite la première hypothèse guidée : un cylindre complet, ses frontières, ses preuves et ses refus.

## Validation

Voir [V-1137 à V-1139](../VALIDATION.md) : suivi natif sous permutations et altération de métadonnées, partition STL/3MF, identité du cache, union multi-corps dans le worker et suite globale réussie. Le nouveau parcours graphique reste à qualifier en fenêtre visible.
