# ADR-0017 — Sélection des faces du résultat STEP

Date : 9 novembre 2026. Lot I1b, préalable à la correction guidée I2.

## Motivation

L’utilisateur souhaite désigner les faces du résultat à améliorer. La sélection de triangles source I1 reste utile lorsqu’une conversion échoue, mais ne représente pas les faces du BRep exporté. Le parcours de correction privilégiera donc le résultat STEP, avec accès explicite à la source.

## Décision

Une commande additive `result_faces(requestId, sourceId)` lit uniquement les octets du résultat validé courant. Le noyau CAO réimporte ce STEP et produit une tessellation d’affichage, accompagnée des faces natives, de leurs familles et de leurs voisins par arête partagée. Le calcul reste hors fil UI ; les objets OCCT ne quittent pas `fabstep-cad`.

Le DTO comporte un maillage global et des plages de triangles contiguës et exhaustives par face : `id`, `solid_id`, `family`, `triangle_start`, `triangle_count`, `adjacent_face_ids`. Les identifiants commencent à zéro et n’existent que dans cette révision du résultat. Les solides STEP ne sont pas assimilés aux corps source par leur ordre. Les familles proviennent des surfaces natives ; une tessellation de cylindre reste une face cylindrique, tandis que les faces planes d’un résultat facettisé restent des plans.

L’aperçu est borné à 32 solides, 100 000 triangles et 300 000 sommets. Les limites du graphe et du travail sont explicites. Un dépassement refuse l’aperçu complet, sans exporter ou afficher un sous-ensemble présenté comme complet. La déflexion d’affichage est fixée à 0,01 mm et indiquée ; elle n’est ni le budget de reconstruction, ni une certification de distance globale.

Le cache STEP n’est jamais invalidé par une demande ou une erreur d’aperçu. La commande vérifie la source, la requête, l’époque et l’identité du résultat avant et après calcul. Changer le fichier ou les options rend l’ancien aperçu et ses sélections inutilisables. Une erreur d’inspection n’enlève pas l’autorisation d’export d’un STEP qui reste validé et courant.

L’interface propose Source / Résultat STEP, avec chargement à la demande. Le clic sélectionne une face entière, les modificateurs permettent une sélection multiple, et l’extension suit les arêtes du BRep sans franchir les solides. Les calculs de sélection et de voisinage sont bornés et exécutés dans un worker. Le surlignage n’est pas une cible du lancer de rayon. L’inspection ne modifie aucune géométrie.

## Correspondance source : limite explicite

Cette section décrit la limite initiale d’I1b. L’[ADR-0018](0018-planar-step-source-provenance.md) ajoute ensuite la correspondance exacte pour le mode Plans ; les autres reconstructions restent soumises à cette limite.

Le pipeline actuel ne conserve pas une chaîne d’identité entre patch reconnu, face construite et face du STEP final réimporté. Le même numéro, la même famille ou une proximité spatiale ne constituent pas cette preuve. La réponse indique donc `source_correspondence: { status: "unavailable", reason: "step.provenance_unavailable" }`. Aucun surlignage source approximatif n’est présenté comme exact.

Avant I2, il faudra conserver cette provenance à la construction, à la couture et au transfert STEP, puis vérifier qu’elle est exhaustive et non ambiguë au réimport. Les modèles Plans et extrusion historique devront aussi fournir leurs attributions source. Le regroupement multi-racines et les changements de topologie ne devront ni perdre ni inventer d’appartenance. Une correspondance ambiguë devra rester indisponible. Les API de transfert OCCT permettent de relier des entités STEP aux formes réimportées, mais ne fournissent pas à elles seules la provenance vers le maillage d’origine.

Les actions de correction viendront ensuite : une indication de famille devra guider une nouvelle reconnaissance, passer toutes les preuves, puis présenter les écarts avant remplacement du résultat. Aucun bouton de correction inactif n’est ajouté dans I1b.

## Références et validation

- [Documentation officielle STEPControl_Reader](https://www.occt3d.com/dev/doc/refman/html/class_s_t_e_p_control___reader.html), correspondance entité STEP / forme ; API locale OCCT 7.9 utilisée comme référence d’implémentation.
- [I1 : inspection source](0016-source-zone-inspection.md).
- [Validation](../VALIDATION.md), où les tests exécutés et les essais graphiques sont distingués.
