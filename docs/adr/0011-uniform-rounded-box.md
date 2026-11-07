# ADR-0011 — Boîtier convexe entièrement arrondi à rayon uniforme

Statut : accepté ; implémenté dans le build de développement après 0.10
Date : 7 novembre 2026

## Contexte

Le cas 04 du corpus mécanique possède six plans, douze quarts de cylindre et huit octants sphériques. Les reconnaisseurs de surfaces de révolution à contours circulaires complets ne représentent pas les trois arcs orthogonaux de chaque coin. Une boîte englobante ou une faible distance implicite aux sommets ne démontre ni ces raccords, ni la présence de toutes les faces.

Ce jalon vise un corps convexe complet à rayon unique. Il ne doit pas généraliser implicitement les congés, fermer une coque ouverte ou remplacer un perçage par de la matière.

## Décision

### Modèle et place dans le pipeline

[rounded_box.rs](../../crates/fabstep-core/src/rounded_box.rs) expose `recognize_rounded_box`. Son contrat distinct `RoundedBoxModel` contient les sommets source, le centre, un repère direct orthonormal, les trois demi-extensions positives du noyau et le rayon. La géométrie est la somme de Minkowski d’un pavé orienté B et d’une boule de rayon R. Les demi-extensions extérieures valent celles du noyau plus R ; aucune primitive de fermeture n’est ajoutée au maillage.

`RoundedBoxRecognition` fournit le rapport commun et une attribution exhaustive des triangles. `RoundedBoxFaceRef` distingue un plan par axe et côté, un cylindre par axe et signes des deux axes transverses, et une sphère par les trois signes de son coin. Les signes transverses sont ordonnés par indice croissant d’axe. Les sommets partagés restent uniques ; les modèles historiques `RoundedModel` et leurs anneaux ne sont pas détournés pour représenter les octants.

Le [pipeline app](../../crates/fabstep-app/src/lib.rs) essaie ce contrat seulement après l’absence de candidat dans tout le parcours historique Surfaces → Primitives → Cylindres. Son `recognition.unsupported` autorise le tore complet, puis la sphère complète selon l’[ADR-0010](0010-resolved-revolutions.md). Un candidat dont la preuve échoue produit `recognition.surface_unproven` ; les refus de déviation ou de calcul sont également bloquants. Il n’y a aucun repli automatique vers Plans.

### Ancrages et domaines source

La recherche conserve les sept plus grandes régions planes, sans combinaison exhaustive. Les six premières doivent être séparées de la septième par un facteur d’aire d’au moins quatre. Cette condition ne prouve pas le modèle ; elle identifie un candidat de manière bornée. Chaque support doit ensuite avoir un seul contour rectangulaire, ses quatre coins attestés et son plan opposé dans un repère orthogonal. Le rayon commun provient des retraits des contours aux six supports. Tous leurs sommets sont revérifiés après canonicalisation. Le rayon et les demi-extensions du noyau doivent dépasser huit fois max(fit, planéité, soudage).

Chaque sommet reçoit une unique projection dans ce repère. Les coordonnées proches d’une couture sont ramenées à la même limite de domaine ; la norme vectorielle complète du déplacement, y compris ces corrections, doit rester sous le fit. Chaque triangle idéal doit appartenir entièrement à un seul des 26 domaines convexes : rectangle plan, quadrant cylindrique borné ou octant sphérique. Un triangle traversant une couture sans frontière source est refusé.

Tous les domaines doivent être présents. Les cylindres demandent chacun au moins huit sommets témoignant d’une courbure intérieure ; les octants en demandent six. Les pas angulaires locaux ne dépassent pas 45°. Les déterminants des projections planes, cylindriques et sphériques doivent être positifs : les points sur la bonne surface ne suffisent pas à accepter un pli.

### Couverture et borne

Pour un point q du triangle idéal, la projection est :

```text
P(q) = clamp_B(q) + R × (q − clamp_B(q)) / ||q − clamp_B(q)||
```

Les domaines convexes maintiennent toute l’image dans le patch prévu. Cette projection est identique sur les coutures communes : ligne pour plan–cylindre, quart de cercle pour cylindre–sphère. Elle définit donc une application continue sur la coque entière.

La source est une variété fermée orientée avec χ = 2. L’orientation de chaque triangle idéal vue depuis le centre est positive, et la somme des angles solides vaut 4π dans la marge numérique fixée. L’homotopie du triangle idéal vers P reste hors du noyau, qui contient le centre. Elle conserve le degré un de la projection radiale. Comme la frontière du boîtier convexe se projette radialement de manière bijective sur la sphère, chaque point de cette frontière est couvert. Cette preuve complète les contrôles locaux ; la seule formule de distance au pavé arrondi n’est pas un critère d’acceptation.

La flèche est nulle sur un plan. Sur un cylindre, elle vaut R moins la norme minimale exacte du triangle projeté dans le plan transverse. Sur un octant, elle vaut R moins la distance minimale exacte du triangle à son centre sphérique. Ces minima bornent tout l’intérieur du triangle et son image. Le maximum des déplacements vectoriels source–sommets idéaux et le budget de soudage sont ajoutés. Le résultat doit respecter le budget de déviation inchangé.

Pour les longueurs x, y, z du noyau, le volume attendu est :

```text
xyz + 2R(xy + xz + yz) + πR²(x + y + z) + 4πR³/3
```

### Construction native et multi-corps

[Le pont CAD](../../crates/fabstep-cad/src/occt.cpp) construit un pavé aux dimensions extérieures puis applique un congé R à ses douze arêtes. OCCT reste exclusivement dans `fabstep-cad` ; `TKFillet` est inclus dans les dépendances natives et dans leur contrôle de distribution.

Avant et après réimport STEP, la validation exige exactement 26 faces : six plans, douze cylindres à domaine angulaire d’un quart de tour et huit octants sphériques. Les types seuls ne suffisent pas : les supports, axes, centres, rayons, domaines, aires, volume et dimensions doivent correspondre au modèle. Le solide doit avoir une coque fermée valide, des incidences cohérentes et aucune auto-interférence. Les 48 raccords sont vérifiés comme 24 lignes plan–cylindre et 24 cercles cylindre–sphère, avec normales tangentes aux trois points de contrôle de chaque arête. Les pôles dégénérés ne sont pas comptés comme raccords supplémentaires.

La permission des octants n’est pas générale. Elle n’est accordée qu’au solide présentant la signature géométrique complète du boîtier. Le regroupement de STEP conserve son repère attendu et ses comptes d’octants **par corps**, puis les revérifie après réimport de chaque racine. Un octant isolé ou une face sphérique partielle arbitraire ne devient pas admissible dans un autre modèle. Les contrôles de séparation et l’interdiction d’export partiel restent inchangés.

## Alternatives examinées

- Réutiliser les deux anneaux d’un `MixedSphere` : ne représente pas les trois limites orthogonales d’un octant sans faux contours.
- Ajuster seulement la boîte englobante et le rayon : ne prouve pas les supports, les raccords ni la couverture, et pourrait effacer des cavités.
- Vérifier seulement une distance implicite aux sommets : peut accepter des plis ou des intérieurs triangulaires trop éloignés.
- Introduire immédiatement des congés généraux : exigerait d’autres topologies, domaines et preuves de raccordement, sans être nécessaire à ce cas mécanique borné.

## Conséquences

La reconnaissance reste pure, déterministe et linéaire dans les scans de source, avec des recherches de taille fixe et le compteur de travail existant. Une forte disproportion, six supports insuffisamment dominants, un rayon mal résolu ou une triangulation qui franchit les coutures peut entraîner un refus conservateur. La normalisation et la garde de translation évitent d’affirmer une précision que les coordonnées ne permettent pas.

Les trous, bossages, méplats supplémentaires, arrondis partiels, rayons variables, zones sphériques générales et congés d’un corps arbitraire restent hors de ce contrat. Le choix explicite du mode et `recognition.rounded_box_intent_assumed` rendent l’interprétation déclarée. `recognition.rounded_box_coverage` décrit l’attribution complète ; les audits de fidélité demeurent distincts de la borne analytique.

## Validation

[rounded_boxes.rs](../../crates/fabstep-core/tests/rounded_boxes.rs) génère indépendamment rectangles, quarts de cylindre et octants. Les tests couvrent rotation, miroir, f32, échelle, volume, 26 attributions, déformations locales, rayons variables, arrondi partiel, échantillonnage insuffisant, pli cosphérique refusé par orientation, déplacements vectoriels aux coutures, coque ouverte, rayon non résolu et budget indépendant. Des points intérieurs aux triangles et des points analytiques dans le sens inverse vérifient la borne. L’absence de six supports laisse disponibles les reconnaisseurs terminaux historiques.

Le cas 04 indépendant FreeCAD, les références synthétiques, les documents multi-corps et leurs contacts complètent ces tests au niveau CAD/app/CLI. Un contrôle séparé doit vérifier les 26 faces et les 48 tangences après réimport indépendant. Les résultats exécutés et la qualification du paquet sont consignés dans [VALIDATION](../VALIDATION.md), [RELEASE](../RELEASE.md) et le [corpus mécanique](../../fixtures/complex_v1/README.md). La réussite de ce cas ne constitue pas une promesse de congés généraux ni une déclaration de v1 achevée.
