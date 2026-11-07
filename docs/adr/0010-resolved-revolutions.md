# ADR-0010 — Doubles chanfreins résolus et tores complets

Statut : accepté ; implémenté dans le build de développement après 0.10
Date : 7 novembre 2026

## Contexte

Le corpus mécanique indépendant contient une bague à deux chanfreins opposés séparés par un cylindre. La restriction de l’[ADR-0006](0006-cones-et-chanfreins-explicites.md) la refusait volontairement : de petites variations de rayon pouvaient être classées comme cylindres et séparer artificiellement plusieurs cônes le long d’une méridienne courbe. La régression `m4_alternating_taper` doit rester bloquante.

Un tore annulaire complet n’a pas de frontière plane ou de pôle à fournir aux reconnaisseurs historiques. Ses sommets ajustés ne suffisent pas à démontrer la forme : des tores déformés, des projections repliées et des enroulements multiples doivent être refusés. Les modes, tolérances, budgets et priorités existants restent applicables.

## Décision

### Deux chanfreins opposés avec portée résolue

La règle d’une portion conique par chaîne est amendée uniquement pour une chaîne de trois portions **cône–cylindre–cône**, avec pentes opposées. Chaque portion conserve ses preuves de paroi, de loi affine du rayon, d’orientation, de couverture et de provenance. Il ne s’agit pas de découper librement un profil en cônes.

Le garde partagé de [revolved.rs](../../crates/fabstep-core/src/revolved.rs), également employé par [rounded.rs](../../crates/fabstep-core/src/rounded.rs), utilise ε = max(fit, planéité, soudage). La portée cylindrique a une hauteur H > 8ε. Pour chacun des cônes, de hauteur h et variation radiale Δr, h > 2ε, |Δr| > 2ε et :

```text
(|Δr| − 2ε) / (h + 2ε) > 4 × 2ε / (H − 2ε)
```

Le membre gauche minore la pente conique compatible avec les incertitudes ; celui de droite impose une marge par rapport à la pente maximale compatible avec le cylindre. Le rayon du cylindre reste vérifié sur sa source. Ce garde restreint l’admissibilité ; il n’augmente aucune tolérance.

Le mode Primitives conserve sa correspondance entre anneaux. Le mode Surfaces accepte aussi cette nouvelle chaîne par la preuve UV existante, qui tolère des anneaux échantillonnés différemment. Elle peut donc produire deux cônes sans sphère ni tore ; `recognition.conic_intent_assumed` porte alors l’intention déclarée. Les anciens cas à zéro ou un cône gardent leurs replis. Les pentes de même signe, les portées ambiguës, les chaînes supplémentaires et les variations non affines restent refusées.

### Tore annulaire complet isolé

[complete_torus.rs](../../crates/fabstep-core/src/complete_torus.rs) expose `recognize_complete_torus`. L’app ne l’essaie qu’en mode Surfaces, après `recognition.unsupported` de tout le parcours historique et du boîtier arrondi terminal de l’[ADR-0011](0011-uniform-rounded-box.md). Seul son propre `unsupported` permet ensuite la sphère complète. Une topologie fermée différente de χ = 0 indique l’absence de candidat torique ; un candidat de genre un non prouvé produit un refus bloquant.

Le modèle contient exactement un tore extérieur et aucune autre primitive. `MixedTorus.rings = None`, `axis = Some(direction unitaire)` et les deux paramètres périodiques couvrent un tour complet. Les congés historiques conservent `Some([i, j])` et n’émettent pas le champ `axis`, donc leur JSON reste identique. Aucun faux anneau n’est ajouté. Les sommets source restent présents ; tous les triangles portent `RoundedFaceRef::Torus { index: 0 }`. Le rapport distingue ce tore (`tori = 1`) d’un congé prouvé (`toroidal_fillets = 0`).

L’axe, le centre et les rayons proviennent d’un ajustement quartique structuré en coordonnées normalisées, suivi d’une diagonalisation déterministe. Ce résultat n’est qu’un candidat. Un raffinement géométrique à sept paramètres réduit la somme des distances quadratiques à la surface, même si le candidat satisfait déjà la tolérance : le biais algébrique des coordonnées f32 peut autrement déplacer inutilement les rayons et le volume. Le raffinement est limité à six itérations et huit réductions de pas par itération ; son travail est compté. Il s’arrête au bruit numérique lié à l’échelle et à la translation. Chaque pas doit améliorer l’objectif ; un résidu maximal déjà admissible doit le rester. L’acceptation finale vérifie toujours tous les résidus physiques et toutes les preuves, sans utiliser le seul objectif d’ajustement.

La source doit être une variété fermée orientée de genre un. Les régions planes significatives sont exclues. Le petit rayon r et l’ouverture R − r doivent dépasser quatre fois max(fit, soudage), avec R > r. Pour chaque triangle, les deux coordonnées périodiques ont un relèvement court unique, de portée au plus 45°. Les déterminants UV sont strictement positifs et la somme des aires vaut (2π)², dans la marge numérique fixée : la continuité entre triangles et le degré un excluent trous de couverture et enroulements multiples.

Pour des portées Δu et Δv, la borne intérieure est :

```text
((R + r) Δu² + 2r Δu Δv + r Δv²) / 8
```

Elle découle des dérivées secondes de la paramétrisation et de la variance des coordonnées barycentriques. La même correspondance couvre l’image analytique ; elle borne les deux sens dans ce domaine prouvé. Le maximum des résidus vectoriels et le budget de soudage sont ajoutés avant comparaison au budget indépendant de déviation. Le volume attendu est 2π²Rr².

Le pont natif construit le tore par une entrée dédiée, contrôle le solide et ses deux périodes, puis réimporte le STEP. Ses coutures sont normalisées lors de l’extraction de tessellation pour l’audit. Les contrôles natifs de la source et du résultat, la séparation des corps et l’audit bidirectionnel restent obligatoires.

## Alternatives examinées

- Supprimer la limite de cônes par chaîne : réintroduit le contournement des séparateurs presque cylindriques.
- Utiliser la covariance ou un axe imposé pour le tore : ne prouve pas son axe sur des échantillons irréguliers.
- Accepter le résidu quartique, le fit des sommets ou χ = 0 seuls : ne démontre ni la fidélité géométrique des intérieurs ni la couverture.
- Ne raffiner qu’un candidat hors tolérance : conserve un biais algébrique mesuré sur les STL f32 pourtant déjà admissibles.

## Conséquences

Les modes Plans et Cylindres restent inchangés. Les nouvelles interprétations sont explicites, bornées et déterministes ; aucun seuil n’est adapté à un fichier. Les refus de preuve, de conditionnement, de déviation et de calcul restent bloquants. Un tore trop mince, une région plane démontrée ou un échantillonnage inadéquat peut être refusé même si la pièce nominale est un tore. Les tores cornus, auto-intersectants, elliptiques, partiels ou raccordés à d’autres primitives ne sont pas couverts par ce nouveau contrat terminal.

Les anciens ADR restent le relevé de leurs périmètres initiaux ; cette décision amende la limite de l’ADR-0006 et l’ordre terminal de l’ADR-0009. Les formules de reconnaissance ne dispensent pas du contrôle du STEP natif.

## Validation

Les régressions dédiées sont [double_chamfers.rs](../../crates/fabstep-core/tests/double_chamfers.rs) et [complete_tori.rs](../../crates/fabstep-core/tests/complete_tori.rs). Elles couvrent anneaux non appariés, rotation, miroir, f32, échelles, résidus aux intérieurs, distances inverses, portées courtes, fit explicite plus large, méridiennes non affines, tubes elliptiques, déformations, plis et double enroulement. Le test du biais f32 compare les deux STL indépendants au volume nominal sans relâcher le fit. Les budgets et les anciennes interprétations sont rejoués.

La bague 02 et les tores indépendants doivent également conserver leurs surfaces, dimensions, volume et matière après réimport FreeCAD. Les résultats exécutés et la qualification du paquet sont suivis dans [VALIDATION](../VALIDATION.md), [RELEASE](../RELEASE.md) et le [corpus mécanique](../../fixtures/complex_v1/README.md), sans assimiler les tests core à une validation native ou à une v1 achevée.
