---
id: R-132
titre: "Qualifier un délai prolongé sur demande pour les gros STL"
resume: "F5 · Option CLI conditionnée au choix A-136 si le budget ordinaire demeure insuffisant après les travaux géométriques."
statut: specifiee
jalon: audit
cree: 2026-11-20
maj: 2026-11-20
livree:
---
## But

Permettre à un utilisateur qui en fait le choix de laisser tourner une pièce coûteuse et d’interrompre le calcul. Le budget courant de 30 minutes reste la référence pour un lancement ordinaire.

## Inclus

- Si A-136 retient cette option, spécifier une borne supérieure finie et proposer le délai prolongé explicitement en CLI pour le seul helper gros STL.
- Indiquer la phase avant ou après transfert et le nombre de groupes terminés sur le total connu. Distinguer dépassement du délai, annulation et dépassement des 4 Gio agrégés.
- Évaluer une reprise persistante dans une décision ultérieure. Elle demanderait une preuve d’identité de l’entrée, de la version du moteur et d’OCCT, des options, du solide et de la phase ainsi qu’un journal intègre des groupes achevés.

## Hors périmètre

Reprise sur disque dans ce lot, allongement silencieux dans le bureau, changement des plafonds d’entrée ou de mémoire.

## Dépendances

Décision produit A-136 rattachée à R-085. Si le délai ordinaire convient après les développements de reconnaissance ou si A-136 conserve la limite, clore la piste en documentant ce choix. L’accord sur la feuille de route ne décide pas du délai.

## Critères d'acceptation

- Sur un témoin public et avec une horloge ou un budget de test contrôlé, vérifier le délai ordinaire, l’option explicite dans ses bornes et le refus des valeurs hors plage.
- Vérifier la progression, l’annulation, la fin des processus et le nettoyage des fichiers temporaires. Un échec de délai, de mémoire ou de contrôle avant ou après transfert ne laisse aucun STEP exportable.
- Comparer la sortie d’une pièce qualifiée dans les deux modes. Exiger les mêmes familles, la même fermeture, le même volume et les mêmes contrôles de fidélité.

## Vérifications de l'utilisateur

Si le mode est retenu, lancer une conversion prolongée sur une pièce publique et l’annuler depuis la CLI.
