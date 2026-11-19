---
id: R-032
titre: "C1b1 — chaînes coniques de même signe avec portées"
resume: "Reconnaître deux à huit cônes de signes libres séparés par des portées cylindriques prouvées ; conserver les garde-fous contre les fausses ruptures."
statut: livree
jalon: post-v1
cree: 2026-11-13
maj: 2026-11-19
livree: 2026-11-19
---

## But

Reconnaître les suites de chanfreins coniques dont les rayons augmentent ou diminuent dans le même sens, avec des portées cylindriques séparatrices prouvées. L’utilisateur demande le 17 novembre de poursuivre la reconnaissance géométrique puis la simplification de l’interface ; cette tranche précède la reprise UX4.

## Inclus

- Deux à huit cônes complets par chaîne, pentes de signe libre, séparés par des portées cylindriques résolues.
- Deux anneaux source intérieurs distincts par portion du nouveau domaine, y compris les éventuelles portées terminales.
- Séparation des pentes supérieure au bruit et marge existante conservée, sans assouplir les tolérances.
- Intégration aux modes Auto, Cônes et cylindres et Surfaces arrondies, avec contrôle natif complet et provenance applicable au parcours.
- Corpus positif transformé et cas négatifs de courbure, témoins insuffisants, portée ambiguë tardive et dépassements de budgets.

## Hors périmètre

Cônes adjacents sans portée : conservés dans R-059, car ils demandent une nouvelle preuve de segmentation. Pas de profil libre, nouveau format de données, dépendance ou réglage implicite.

## Dépendances

R-031 fournit la preuve des chaînes alternées. Les cas historiques gardent leur contrat. R-042 reprend ensuite l’interface à la demande de l’utilisateur.

## Critères d'acceptation

- Une chaîne suffisamment échantillonnée avec cônes de même signe produit les surfaces et le volume analytiques attendus, après réimport natif.
- Les méridiennes courbes, portées trop petites et preuves insuffisantes ne deviennent pas de faux cônes.
- Les anciens cas non admis ne changent que s’ils satisfont explicitement le nouveau contrat complet de témoins.
- Tests core/app, contrôles de régression pertinents, formatage, analyse statique et build bureau réussis.

## Vérifications de l'utilisateur

- Essayer le corpus de chaînes de même signe dans le build de développement et examiner les surfaces reconstruites.
- Réimporter un STEP du corpus dans Fusion et vérifier le solide, les portées et les dimensions.

## Historique

| C1 — chaînes de révolution | Plusieurs chanfreins/portées, puis ruptures de pente sans portée | Chaque rupture issue de la source et séparée du bruit ; refus des méridiennes douces découpées artificiellement ; provenance et borne | C1a implémenté : 3 à 8 cônes alternés et portées résolues avec témoins intérieurs ; 7 exports/8 solides OCCT et suite globale réussis : 228 workspace, 5 bureau, 7 UI (V-1125–V-1127). C1b sans portée ou de même signe reste à étudier |

Source : [ancienne feuille de route](../../docs/ROADMAP-avant-fabboard.md). Les validations historiques sont conservées ; aucun essai nouveau n’est déduit de la migration.

## Implémentation du 17 novembre

Preuve locale étendue aux signes libres, avec portées résolues et témoins intérieurs sur toutes les plages. Les paires historiques court-circuitent ce nouveau calcul pour préserver leur budget et leur contrat. Corpus reproductible, cas négatifs et contrôles natifs/provenance ajoutés ; revue indépendante acceptée. Architecture documentée dans ADR-0031 ; preuves et limites dans V-1184.
