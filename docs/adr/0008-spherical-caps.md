# ADR-0008 — Calottes sphériques à limites source

Statut : acceptée et implémentée dans 0.8.0-alpha.1
Date : 7 novembre 2026

## Contexte

M5a reconnaît les hémisphères et certains congés toriques. Un dôme plus plat ou plus haut qu’un hémisphère demande une limite circulaire située hors de l’équateur. Le modèle `MixedSphere` et le pont OCCT savent déjà représenter cette limite ; la reconnaissance et sa preuve doivent être étendues.

## Décision

Étendre le mode explicite `surfaces` aux calottes sphériques convexes ou concaves, délimitées par un anneau circulaire plan et un pôle présents dans la source. Les sphères entières, les zones sphériques entre deux anneaux sans pôle et les congés généraux restent hors de ce sous-ensemble. Les seuils initiaux et les modes précédents sont conservés.

Le rayon de l’anneau et la hauteur axiale du pôle contraignent le rayon de la sphère : pour un rayon de bord `a` et une hauteur `h`, `R = (a² + h²) / (2h)`. Le centre est déterminé sur l’axe. Ces deux limites seules ne démontrent pas une sphère : elles sont également compatibles avec un cône. La reconnaissance doit donc trouver une courbure intérieure résolue, puis vérifier tous les sommets et triangles. Pour `tol = max(plane, fit)`, la hauteur dépasse `4·tol` ; au moins douze sommets intérieurs sont à plus de `4·tol` des limites et s’étendent sur des hauteurs séparées par plus de `4·tol`. Leur distance normale à la génératrice conique dépasse `fit`. Le conditionnement exige `R·ε·128 ≤ fit`. Une hauteur non résolue, un pôle décalé, un sphéroïde ou une couverture insuffisante restent refusés.

Les sommets de l’anneau et du pôle sont contraints aux limites idéales ; leur déplacement vectoriel est inclus dans le résidu. La frontière et la topologie de disque doivent être démontrées. La projection radiale des triangles doit conserver une orientation cohérente et couvrir une seule fois le domaine bordé par les arcs géodésiques issus du contour source.

Hors de l’équateur, ce polygone sphérique n’a pas exactement l’aire de la calotte limitée par un petit cercle. La couverture doit être comparée à l’éventail géodésique reliant le pôle au contour, et non directement à `2πh/R`. Le contour effectue exactement un tour, avec des pas azimutaux de signe constant d’au plus 45° ; les arêtes des triangles projetés respectent aussi un angle central maximal de 45°. L’écart entre ce bord géodésique et la limite circulaire est borné en plus de la flèche des facettes. Pour chaque intervalle azimutal Δu, la frange est bornée par `2R·sin(|atan2(zb, a·cos(Δu/2)) − atan2(zb, a)|/2)`, où `zb` est la hauteur du bord par rapport au centre. Cette valeur est nulle à l’équateur. Le budget composé et l’audit natif bidirectionnel restent distincts.

Le contrat sérialisé et l’ABI restent inchangés : anneaux, centre, rayon et orientation intérieure/extérieure. Les latitudes de construction se déduisent des anneaux. Le volume provient du flux analytique existant, également valable lorsque la calotte dépasse l’équateur. Les dimensions d’une grande calotte peuvent atteindre le diamètre de la sphère à l’intérieur de la surface, indépendamment du diamètre de son anneau de fermeture.

## Validation

Le corpus indépendant de petites et grandes calottes, rotation/translation, cavité concave et instances 3MF dont un miroir produit cinq STEP contenant six solides. Les réimports OCCT puis FreeCAD passent : fermeture, orientation, centres, rayons, domaines, dimensions et volumes. Des solides de référence construits séparément avec `Part.makeSphere` confirment ces mesures. Les cas déformés, peu échantillonnés, sans pôle, en contact et hors budget sont refusés. La deuxième pièce utilisateur STL et 3MF conserve ses résultats ; les tests historiques des hémisphères M5a passent.

Les 152 tests workspace et le test desktop passent. La conversion native d’une grande calotte a été observée ; l’export par dialogue macOS reste à confirmer, le pilotage ayant ensuite cessé de répondre. Les contrôles réellement exécutés et leurs limites sont consignés dans [VALIDATION](../VALIDATION.md).
