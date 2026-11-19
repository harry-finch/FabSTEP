---
id: R-084
titre: "C2b — reconstruire une révolution à profil cubique"
resume: "Un profil lisse cubique borné, deux disques source et un tour complet ; preuve continue, STEP contrôlé et famille Révolution explicite."
statut: livree
jalon: post-v1
cree: 2026-11-18
maj: 2026-11-19
livree: 2026-11-19
---

## But

Reconstruire une paroi de révolution lisse qui ne se réduit pas aux lignes et arcs déjà reconnus. L’utilisateur approuve explicitement le nouveau type après le prototype natif et demande de terminer le jalon.

## Inclus

- Un profil Bézier cubique non rationnel, rayon positif, hauteur monotone, tour complet, deux disques plans source.
- Ajustement déterministe avec témoins indépendants, couverture et écart continu, ressources bornées selon ADR-0039.
- Contrat additif, famille Révolution distincte, validation du solide entier avant/après STEP et provenance.
- Auto et Surfaces, conservation de la priorité des primitives historiques et des anciens formats, corpus indépendant et non-régression.

## Hors périmètre

Splines multisegments, pôles, retours axiaux, cavités, assemblages arbitraires de surfaces libres, correction guidée couplée et DXF de courbes non coniques. Aucun gain promis sur Flansch, aucune hausse de limite.

## Dépendances

R-033 ; ADR-0039 approuvé dans la conversation du 18 novembre. Base F1/V-1205 validée.

## Critères d'acceptation

- Corpus source fermé/orienté et variantes placées ; négatifs de couverture, fit, budget et domaine refusés sans faux succès.
- STEP valide à deux plans et une révolution, génératrice contrôlée après réimport, volume et fidélité dans les budgets.
- Provenance exhaustive, corps séparés conservés, contacts refusés ; ancien JSON lisible.
- Contrôles core/CAO/app/UI/bureau, revue de preuve et suite complète réussis ; limites documentées.

## Vérifications de l'utilisateur

- Examiner les profils cubiques du corpus, leur reconstruction et la sélection de la paroi dans FabSTEP.
- Vérifier le solide, le profil et le volume des STEP cubiques dans Fusion.

## Autorisation

« Tu as mon accord pour intégrer le nouveau type de surface qu’il faut ajouter […] tu peux aller jusqu’au bout du jalon. » Accord reçu après présentation du domaine cubique unique et du laboratoire à cinq références.
