---
id: R-067
titre: "G1 — gros maillages sur accord et calcul annulable"
resume: "Autoriser un essai explicite au-delà de 100 000 triangles, avec avertissement, limites distinctes et arrêt du calcul ; qualifier d’abord le STL de 540 170 triangles."
statut: livree
jalon: post-v1
cree: 2026-11-17
maj: 2026-11-19
livree: 2026-11-19
---

## But

Permettre à l’utilisateur d’essayer un STL dense après avertissement, en conservant un moyen réel d’arrêter un calcul trop long. Demande du 17 novembre ; mode proposé à qualifier d’abord sur FlanschTurboLuftfilter_REV02.STL (540 170 triangles, 25,76 Mio).

## Inclus

- 100 000 devient un seuil d’avertissement dans le parcours explicitement accepté, pas une suppression générale des gardes.
- Consentement lié au fichier courant, nombre de triangles et risques mémoire/durée visibles avant calcul ; aucune simplification automatique.
- Qualification mesurée du fichier cible : durée par phase, mémoire maximale, résultat validé ou refus explicite.
- Limites d’entrée, de faces produites, de tessellation d’aperçu et de travail traitées séparément. Plafond supérieur arrêté après mesures, sans valeur supposée déjà sûre.
- Solution recommandée : processus de calcul isolé pour les gros maillages, arrêt du processus sur Annuler, source conservée, aucun résultat incomplet exportable.

## Hors périmètre

Acceptation illimitée, garantie de conversion de tout maillage dense, suppression des audits de source/résultat, décimation silencieuse, multiplication générale des budgets. Les 64 Mio et la conservation des corps restent des contraintes tant qu’aucune qualification ne justifie un changement.

## Dépendances

Processus séparé validé explicitement par l’utilisateur le 17 novembre (A-055). L’annulation actuelle aux checkpoints ne peut interrompre un appel natif engagé. Plusieurs plafonds de 100 000 protègent des objets différents ; les remplacer globalement serait incorrect. Worker compilé séparément avec capacité source élargie ; rapport JSON et STEP bornés dans un répertoire privé, marqueur de fin publié en dernier. Le bureau ne récupère aucun objet natif.

## Critères d'acceptation

- Avertissement explicite et consentement remis à zéro au nouveau fichier.
- Mesures reproductibles et rapport honnête sur le fichier cible ; les contrôles géométriques restent obligatoires.
- Arrêt vérifié pendant une phase longue, source réutilisable, reprise possible, aucune réponse tardive exportable.
- Limites finies, nettoyage des fichiers temporaires et absence d’export partiel ; parcours ordinaires sous le seuil inchangés.

## Vérifications de l'utilisateur

- Essayer le fichier dense après avertissement, vérifier les étapes puis annuler pendant un calcul long et confirmer que l’application reste utilisable.
- Si la reconstruction aboutit, inspecter le STEP exporté et vérifier ses dimensions et ses ouvertures dans Fusion.

## Périmètre de la première tranche

STL binaires de 100 001 à 600 000 triangles, avec arrêt à 30 minutes ou au-delà de 4 Gio de mémoire résidente observée sur macOS. Les STL ASCII et le 3MF gardent le parcours habituel. Le processus séparé conserve les validations et budgets existants ; résultat et tessellation restent limités à 100 000 faces/triangles. La reconstruction et l’export sont disponibles, mais la provenance guidée n’est pas transférée : correction guidée, recherche de paramètres et coupe source restent indisponibles pour ces nouveaux fichiers denses. Aucun fichier auparavant pris en charge ne perd ces fonctions.

## Vérifications préalables

Lecture ciblée confirmée : `fabstep-app/src/lib.rs` contrôle la source via `triangle_model` avant le STEP final ; une source de 540 170 triangles peut donc créer autant de faces natives de validation, même si le résultat reconnu est compact. Les plafonds source apparaissent aussi dans `fabstep-verify` et la sélection/provenance UI ; les plafonds de faces STEP et de tessellation résultat sont distincts. Ces vérifications initiales précédaient toute modification des seuils ou du code. Inspection binaire privée : 270 077 enregistrements de sommets distincts, coordonnées finies, bornes [0,0,0] à [42.5,100.5,93.5] dans l’unité STL non déclarée ; cela ne vaut pas validation topologique ni CAO.


## Résultat du développement

Implémentation et contrôles automatisés achevés, preuves dans V-1191 de `docs/VALIDATION.md`. Helper et bureau construits séparément, source/résultat bornés, annulation réelle, cleanup et publication protégés ; aperçu dense hors fil UI. Le cube synthétique de 101 568 triangles est reconstruit après un essai annulé. Le fichier privé de 540 170 triangles franchit la limite d’entrée mais reste refusé par le budget des contours : suite éventuelle R-068, sans promesse de conversion. Le contrôle graphique complet reste à confirmer par l’utilisateur (blocage du pilote de dialogue).
