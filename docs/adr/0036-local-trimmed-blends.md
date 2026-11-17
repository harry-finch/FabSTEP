# ADR-0036 — Domaines locaux pour les raccords constants et à rayon linéaire

Statut : extension additive des deux domaines approuvée explicitement par l’utilisateur (A-068). Les preuves source et l’intégration de production restent à réaliser.

## Besoin et preuves préparatoires

Le modèle actuel représente des calottes/zones sphériques par anneaux complets. Il ne représente pas l’octant sphérique local reliant trois arrondis cylindriques. Les cônes partiels existants utilisent des arcs circulaires coaxiaux ; ils ne représentent pas les sections elliptiques d’un cône incliné coupé par les plans terminaux de la pièce.

R-079 prépare deux références indépendantes, bornées et distinctes du reconnaisseur : un coin orthogonal à trois arrondis uniformes et une enveloppe de sphères à rayon linéaire entre deux plans orthogonaux. Les résultats natifs et leurs limites sont consignés dans VALIDATION à l’issue du laboratoire. Ils ne prouvent pas encore la reconnaissance depuis un STL.

## Proposition de contrat additif

Conserver les modèles et parcours existants. Ajouter uniquement les domaines nécessaires aux deux familles, avec limites partagées et orientées et attribution de chaque triangle source. Aucune fermeture ni propagation d’arrondi décidée par OCCT.

- **D : octant sphérique local.** Centre, rayon positif, trois directions orthogonales démontrées, choix de côté matière, et trois usages orientés d’arcs circulaires partagés avec les cylindres voisins. Chaque arc porte ses extrémités et son identité de frontière source. La couverture doit être exactement le domaine compris entre ces arcs, avec les trois tangences prouvées. Les sphères partielles arbitraires ne sont pas admises.
- **G : portion de cône inclinée.** Sommet/axe/demi-angle démontrés, deux génératrices de tangence aux plans et deux arcs elliptiques partagés avec les plans terminaux. Chaque conique conserve centre, demi-axes, repère, intervalle orienté, extrémités et identité source. La portée est limitée au rayon linéaire positif et aux coupes elliptiques régulières ; les extrémités paraboliques/hyperboliques et les singularités restent refusées.
- Les objets restent des données scalaires dans le core. La frontière CAO construit seulement le domaine prouvé, contrôle les limites partagées puis le STEP réimporté. Types, volume, dimensions, provenance et fidélité sont obligatoires avant export.
- Définir avant activation les plafonds de patches/arcs et le certificat continu de projection et de couverture de chaque triangle. Les seuils existants, le refus des corps en contact et l’absence d’export partiel restent inchangés.

Pour G, la famille analytique de référence est C(t)=(r(t),r(t),t), r(t)=a+bt>0. Son enveloppe est un cône circulaire droit incliné lorsque b≠0 ; les plans horizontaux d’extrémité donnent des ellipses pour |b|<1. Un empilement de quarts de cercle de rayon variable ne remplace pas cette enveloppe. Le cas b=0 conserve le cylindre existant.

## Options et recommandation

1. **Étendre les deux domaines bornés** : recommandé pour poursuivre D et G sans introduire d’emblée une représentation générale de surfaces libres. Risque : les incidences, orientations et trims sont sensibles et demandent des tests indépendants source/natifs.
2. **Étendre seulement l’octant de D** : intégration plus petite, G reste au laboratoire ; même besoin de preuve des limites circulaires partagées.
3. **Conserver les modèles actuels** : aucune extension de contrat, ces zones restent facettisées ou refusées ; C1/C2/E et les perçages sécants déjà autorisés peuvent continuer.

La décision porte sur la représentation et sa portée. Elle ne vaut ni acceptation d’un futur code ni validation automatique des fichiers utilisateur. Les références natives ne sont pas exposées dans l’application avant la preuve source et les contrôles de conversion.

## Décision reçue

L’utilisateur approuve « Oui, étendre les deux domaines D et G (recommandé) ». L’option 1 est retenue ; aucun nouvel accord n’est nécessaire pour implémenter ce périmètre. La qualification native seule ne vaut pas preuve source ni acceptation du futur code.
