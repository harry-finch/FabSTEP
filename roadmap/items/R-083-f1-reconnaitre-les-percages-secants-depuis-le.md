---
id: R-083
titre: "F1 — reconnaître les perçages sécants depuis le STL"
resume: "Prouver deux perçages centrés orthogonaux, leurs limites communes et leur reconstruction STEP, dans le domaine déjà autorisé par A-045."
statut: livree
jalon: post-v1
cree: 2026-11-18
maj: 2026-11-19
livree: 2026-11-19
---

## But

Reconnaître depuis le STL deux perçages traversants centrés et orthogonaux dans un bloc, reconstruire leurs intersections communes et proposer un STEP uniquement après preuve complète. Lancement explicitement demandé après le cadrage R-036 ; contrat autorisé par A-045.

## Inclus

- Corpus STL indépendant : dimensions/rayons variés, placements, miroirs et retessellisations ; négatifs de topologie, domaine et budget.
- Quatre anneaux d’entrée complets, six supports plans, petit rayon positif et grand rayon au moins double avec marge résolue ; deux branches transverses.
- Partition orientée exhaustive en six plans et trois parois cylindriques, frontières opposées, couverture des deux ouvertures de la grande paroi, bornes continues et travail plafonné.
- Contrat scalaire additif, construction des seuls domaines prouvés, certificat natif des courbes 3D et paramétriques après réimport, volume indépendant et fidélité du document entier.
- Parcours Automatique et Surfaces, compatibilité et provenance ; comportement guidé explicitement refusé si non pris en charge.

## Hors périmètre

Axes décentrés ou obliques, tangences, rayons presque égaux indéterminés, triples intersections, trous borgnes, surfaces libres, export DXF des courbes non coniques. Aucun relèvement de plafond ni export partiel.

## Dépendances

A-045, R-036 et certificat de laboratoire R-063/V-1200. G1/R-082 constitue la base validée ; ses domaines restent inchangés.

## Critères d'acceptation

- Corpus indépendant fermé et orienté, preuve source acceptée sur les variantes du domaine et refus explicite des négatifs.
- STEP réimporté : un solide, six plans, trois cylindres, deux branches partagées sans invention de matière ; dimensions/volume/audit cohérents.
- Attribution source exhaustive et provenance native vérifiée sur le corpus, y compris plusieurs corps séparés ; contacts refusés.
- Certificat des intersections réellement réimportées, continuité des paramètres et budget respectés ; aucune simple mesure échantillonnée présentée comme preuve continue.
- Anciennes familles et formats préservés, compilation/tests/Clippy/bureau/UI validés et revue indépendante.

## Vérifications de l'utilisateur

- Examiner les perçages sécants du corpus et la sélection de leurs faces dans FabSTEP.
- Réimporter leurs STEP dans Fusion et vérifier les deux trous, les intersections et le volume.


## Intégration et limites — 18 novembre 2026

Reconnaissance source, contrat additif ADR-0038, construction native, certificat après STEP, sélection/provenance et contrôle multisolide intégrés. Le corpus et les tests versionnés reproduisent les preuves ; V-1205 consigne les vérifications automatisées et leurs limites. La régression de priorité sur quatre trous borgnes est corrigée et couverte en core et application. Le précontrôle public refuse les cycles insuffisants et les témoins dont le résidu dépasse les marges, petit rayon compris.

Les domaines hors périmètre restent dans R-036 ; F1 ne généralise ni les intersections ni les corrections guidées. Flansch n’a pas été rejoué. Arrêt après validation, documentation et publication Git demandé par l’utilisateur ; aucun nouveau développement lancé à la clôture.
