# ADR-0015 — Chaînes coniques alternées résolues (C1a)

Date : 9 novembre 2026
Statut : implémenté dans le build de développement ; qualification consignée dans VALIDATION

## Domaine admis

Étendre les modes Primitives et Surfaces aux chaînes axiales de **trois à huit cônes complets**, séparés par des portions cylindriques résolues. Les pentes coniques successives sont opposées. Une portion cylindrique terminale peut précéder ou suivre la chaîne, à condition d’avoir elle aussi ses témoins source intérieurs.

Le cas historique exact cône–cylindre–cône conserve son garde et ses critères de l’[ADR-0010](0010-resolved-revolutions.md), sans exiger rétroactivement les nouveaux témoins. Les autres suites à deux cônes ne sont pas ajoutées implicitement. Le plafond de huit cônes est **par chaîne**, pas un nouveau plafond global par document ; les limites globales existantes restent applicables.

Cette première tranche ne reconnaît ni cônes successifs sans portée, ni pentes voisines de même signe, ni profils de révolution arbitraires. C1b devra définir séparément ces domaines. Les cônes partiels restent hors périmètre.

## Preuves réutilisées et complétées

Le classement continue de fusionner toutes les bandes coniques consécutives dans une **même loi affine de rayon**. Une variation de pente dans une portion n’entraîne pas un nouveau découpage opportuniste. Les cercles intermédiaires et frontières sont issus de la source ; affinité, résidus, orientation, couverture, incidences et attribution des triangles restent contrôlés par les preuves existantes.

Pour chaque nouvelle portion (cône, portée intérieure ou cylindre terminal), exiger deux anneaux source intérieurs distincts. Avec la coordonnée axiale normalisée t entre les extrémités :

- un témoin est dans `[1/4, 1/2[` ;
- un autre est dans `]1/2, 3/4]` ;
- leur séparation axiale dépasse `4ε`, avec `ε = max(fit, planéité, soudage)`.

Un anneau au milieu n’est pas compté deux fois ; les points d’interpolation ne sont pas des témoins. La présence de témoins ne remplace pas la preuve de leur rayon ni de la paroi triangulée entière.

Chaque fenêtre cône–cylindre–cône de la chaîne reprend le garde de l’ADR-0010. Pour une portée de hauteur H et chaque cône adjacent de hauteur h et variation Δr :

```text
H > 8ε ; h > 2ε ; |Δr| > 2ε
(|Δr| − 2ε) / (h + 2ε) > 4 × 2ε / (H − 2ε)
```

Toutes les portées sont vérifiées, y compris les dernières de la chaîne. Cette séparation des intervalles de pente empêche qu’un séparateur presque cylindrique au niveau du bruit suffise à autoriser une nouvelle portion. Les pentes opposées et les témoins imposent un domaine conservateur. Ils prouvent une interprétation géométrique bornée des échantillons, **pas l’intention de conception** ni l’impossibilité d’une courbe lisse passant par les mêmes points.

## Intégration et budgets

Les fonctions partagées `resolved_run_samples` et `resolved_conical_chain` de `revolved.rs` sont utilisées par Primitives et Surfaces. La recherche des témoins débite le compteur de travail existant. Aucun seuil n’est augmenté ; dépasser huit cônes dans une chaîne produit `recognition.too_complex`. Une chaîne non prouvée reste bloquante avec le code du parcours (`recognition.conic_unproven` ou `recognition.surface_unproven`).

Primitives conserve sa correspondance entre anneaux ; Surfaces emploie sa preuve paramétrique existante et peut accepter des tessellations différentes entre niveaux. L’admission du parcours Surfaces reconnaît les chaînes classées à plusieurs cônes ; les anciens cas à zéro ou un cône et les lots A/B conservent leurs règles. Le nouveau chemin C1a n’admet pas de portion torique dans la chaîne.

Aucune structure publique, ABI, option ou dépendance nouvelle n’est nécessaire. Les plans et cylindres non concernés restent représentés selon leur parcours. Source native, réimport STEP, volume, dimensions, familles de surfaces et audit bidirectionnel restent obligatoires ; aucune coque ouverte ni aucun résultat partiel n’est exportable.

## Corpus et limites de qualification

Le corpus analytique indépendant doit fixer les profils nominaux, volumes et nombres de surfaces avant conversion. Il couvre chaînes d’au moins trois cônes, transformations, placements, quantification, témoins absents ou insuffisants, portées ambiguës, défaut tardif dans une chaîne, non-affinité observable, dépassements de budget et du plafond par chaîne. Les anciennes régressions `m4_alternating_taper` et `m4_piecewise_taper` doivent rester bloquantes.

Les résultats effectivement exécutés sont consignés dans [VALIDATION](../VALIDATION.md). La validation des échantillons et le réimport natif ne valent pas validation de toutes les révolutions. Les profils sans portée, même-signe, libres et les aides manuelles conservent leurs propres étapes dans la [roadmap](../../ROADMAP.md).
