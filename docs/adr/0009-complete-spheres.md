# ADR-0009 — Corps sphériques complets

Statut : acceptée et implémentée dans 0.9.0-alpha.1
Date : 7 novembre 2026

## Contexte

Les calottes M5b1 sont délimitées par un anneau plan et un pôle présents dans le maillage. Un corps sphérique complet ne possède aucune frontière géométrique. Sa triangulation peut ne contenir ni anneau plan ni paire de sommets antipodaux. Les pôles et la couture d’une sphère OCCT sont des éléments de paramétrisation, pas des détails retrouvés dans le fichier source.

## Décision

Étendre uniquement le mode explicite Surfaces arrondies aux corps sphériques entiers, avec une preuve portant sur l’ensemble du maillage fermé. Les zones sphériques tronquées sans pôle, tores complets et raccords généraux restent différés.

`MixedSphere.rings` devient `Option<[usize; 2]>`. `Some` conserve la représentation JSON historique des deux limites d’une calotte. `None`, sérialisé `null`, désigne une sphère complète sans frontière. Ce dernier cas impose un `RoundedModel` constitué d’une seule sphère extérieure, sans plan, anneau ou autre surface. Les sommets source et l’attribution de tous les triangles à `RoundedFaceRef::Sphere` restent conservés. Plusieurs corps sphériques séparés passent par le pipeline documentaire existant, avec placements et contrôles de séparation à la source et au résultat.

La nouvelle reconnaissance est essayée seulement lorsque le parcours historique Surfaces → Primitives → Cylindres a conclu `recognition.unsupported`. Un succès, un budget dépassé ou une preuve échouée de ce parcours reste prioritaire. Certains cylindres à deux niveaux ont eux aussi des sommets cosphériques : cette priorité conserve leur interprétation existante.

Le centre et le rayon sont ajustés dans des coordonnées recentrées et normalisées. Le résidu de chaque sommet, recalculé à partir du centre final, doit respecter la tolérance radiale. Le conditionnement exige notamment `max(|coordonnées|)·ε·32 ≤ fit` et `R·ε·128 ≤ fit`. Les budgets de travail restent explicites. La preuve exige une surface source fermée connexe de caractéristique d’Euler 2, une projection radiale orientée positivement pour chaque triangle, des arêtes centrales d’au plus 45° et une somme compensée des angles solides égale à 4π. Les régions planes dont un contour comporte au moins cinq sommets sont conservativement exclues pour ne pas effacer un bouchon circulaire. La borne composée inclut le soudage, les déplacements de projection et l’écart entre chaque facette idéale et la sphère.

Le noyau CAO construit le cas complet par une entrée C ABI dédiée utilisant `BRepPrimAPI_MakeSphere`. Le STEP doit être réimporté avec un solide fermé valide et une face sphérique, un volume de `4πR³/3`, des bornes `centre ± R` et un audit bidirectionnel conforme. Les contrôles de couture et des deux pôles de tessellation restent requis.

## Alternatives examinées

- Inventer deux anneaux de rayon nul permettrait de réutiliser le contrat des calottes, mais confondrait des pôles paramétriques avec des limites source.
- Étendre d’abord les zones entre deux anneaux réutiliserait le pont natif ; leur couverture et leurs deux franges demandent une preuve différente, reportée à un jalon propre.
- Essayer la sphère avant les reconnaissances historiques modifierait des interprétations déjà validées, notamment certains cylindres cosphériques.

## Conséquences

Le type Rust et le type TypeScript changent ensemble ; les anciennes calottes conservent leurs données sérialisées. Aucun mode, réglage ou bouton supplémentaire n’est nécessaire. La reconnaissance analytique reste une interprétation explicite du maillage, sans prétendre retrouver l’intention de conception. Le STEP reste un BRep sans historique paramétrique.

Une petite face plane de trois ou quatre sommets cosphériques peut être indiscernable d’une facette ou d’un quadrilatère de tessellation sphérique. L’admettre dans la tolérance conserve la prise en charge des maillages UV ; la reconnaissance ne prouve pas l’intention sphérique originale. Les modes historiques restent prioritaires et le diagnostic d’interprétation reste affiché.

## Validation

Le corpus indépendant couvre des icosphères irrégulières sans pôles imposés, une tessellation UV, rotation/translation, échelle et instances 3MF dont un miroir. Les six STEP positifs contiennent sept solides ; le témoin tronqué conserve sa calotte et son plan. Ellipsoïde, sommet déplacé, coque ouverte, faible densité, budget insuffisant et contacts source/résultat sont refusés. Les 165 tests workspace et le test desktop passent. Deux exports natifs et les deux sources de la deuxième pièce utilisateur sont également réimportés dans FreeCAD : dix sorties contrôlées au total, hors références et sonde facettisée. Les mesures et limites des essais sont consignées dans [VALIDATION](../VALIDATION.md).
