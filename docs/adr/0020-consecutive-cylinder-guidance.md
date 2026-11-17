# ADR-0020 — indications cylindriques successives

Date : 9 novembre 2026. Statut : accepté et implémenté.

## Objectif

I2a reconstruit une paroi cylindrique, puis impose de repartir des facettes pour en essayer une autre. Ce lot permet d’ajouter une indication depuis le résultat courant, de retirer une indication acceptée et de recalculer celles conservées après un refus.

Il reste limité aux cylindres complets prouvés entre deux contours plans source. Les autres familles, la persistance et l’historique annuler/rétablir restent différés. Les indications ne constituent pas un historique d’esquisses CAO.

## Provenance du résultat guidé

Le constructeur mixte conserve l’identité des faces planes et des cylindres complets construits. Il reprend le suivi exact de couture et les tables de transfert STEP de l’ADR-0018. Les jetons de plans et de cylindres sont fournis séparément selon les indices du modèle, sans déduction depuis un ordre de parcours natif.

L’application conserve l’attribution exhaustive des triangles aux `MixedFaceRef`, puis la correspondance aux indices de la source placée. La table privée et les octets STEP restent indissociables. Une perte de suivi rend la provenance indisponible pour tout le document sans invalider une géométrie validée. Le retrait d’une indication reste possible grâce à la source et aux indications privées conservées.

Les modèles contenant des cylindres ouverts ne sont pas couverts par ce suivi. Une face cylindrique déjà reconstruite peut être inspectée et reliée à sa source ; l’ajout depuis l’interface porte sur les faces encore planes/facettisées.

## Preuves et budgets

Chaque indication contient une sélection source d’une seule composante analysée. Le moteur doit trouver exactement une paroi prouvée qui contient toute cette sélection. Deux indications ne peuvent désigner la même paroi, même avec des sélections initiales distinctes ; les recouvrements sont refusés.

Le document admet au maximum 16 indications. Les sélections et les attributions restent dans le plafond global de 100 000 triangles. Pour chaque corps, segmentation, recherche de candidats et preuve partagent un seul budget de travail ; le plafond n’est pas réinitialisé par indication.

Chaque ajout, retrait ou recalcul repart de la source immuable avec les options capturées. Toutes les parois conservées sont prouvées ensemble. Le résultat passe les mêmes validations natives, de fidélité et de séparation des corps qu’I2a ; aucun export partiel n’est permis.

## Session et refus

Le cache distingue le résultat exportable de la session des indications acceptées. Une opération capture atomiquement la bonne session et retire l’export avant le calcul. Le succès remplace la session et le résultat ; le refus ne modifie pas la liste acceptée et ne restaure aucun export.

Un ajout exige un STEP courant dont la provenance exacte est disponible. Un retrait ou **Recalculer les indications conservées** peut repartir de la session acceptée après un refus. Cette récupération effectue une nouvelle validation complète ; elle ne republie pas des octets anciens.

Changer la source, l’unité ou les options efface la session. Les gardes de source, révision et époque s’appliquent aux réussites et aux échecs. Les identifiants d’indication sont attribués côté natif, stables au retrait d’une autre indication et non réutilisés dans la session.

## Interface et rapport

La liste des indications acceptées indique le corps concerné et la taille de la paroi reconstruite. Chaque ligne possède une action de retrait. Le rapport ajoute `guidance_hints` tout en conservant le champ historique `guidance` ; les indices de faces sélectionnées restent des traces du résultat de départ, jamais une identité réinterprétée dans un STEP ultérieur.

Après un refus, la liste affichée reste celle de la dernière session acceptée. L’interface signale qu’un recalcul est nécessaire avant export et permet de relancer cette session. Une sélection ou une tentative seule ne devient jamais une indication acceptée.

## Validation

Voir [V-1145 à V-1148](../VALIDATION.md) : provenance native mixte, indications successives, retrait, récupération, partition source, instances miroir et suite globale. Les résumés de parois sont reconstruits à chaque calcul ; un test privé refuse leur réutilisation dans un état incohérent. Les essais graphiques restent à qualifier.
