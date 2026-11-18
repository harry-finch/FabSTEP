# ADR-0039 — Révolution à méridienne cubique bornée

Statut : accepté par l’utilisateur le 18 novembre 2026 ; intégré dans R-084 et qualifié automatiquement sous V-1206 ; acceptation graphique/Fusion encore ouverte.

## Contexte

C2 couvre déjà les profils composés de lignes et d’arcs circulaires. Une méridienne plus générale demande un nouveau domaine scalaire, une famille de surface honnête et une preuve couvrant tout le maillage. Le laboratoire `scripts/qualify-smooth-revolution.sh` confirme la représentation OCCT et le réimport de cinq références synthétiques ; il ne reconnaît aucun STL.

Après présentation du premier domaine cubique et des résultats natifs, l’utilisateur autorise explicitement l’ajout du type de surface et demande d’aller au bout du jalon pendant son absence. Les splines arbitraires et les profils multisegments ne sont pas autorisés par cette tranche.

## Décision

Ajouter un domaine optionnel `SmoothRevolution` à `RoundedModel` : origine au centre du disque inférieur, axe unitaire vers le disque supérieur, hauteur positive H, quatre rayons de contrôle b₀…b₃, deux références d’anneaux source et résidu mesuré des témoins. La génératrice est r(t)=Σ bᵢ Bᵢ³(t), z(t)=Ht, 0≤t≤1. Les quatre rayons doivent être strictement positifs avec marge résolue ; cette condition suffisante exclut certains profils pourtant positifs. Un seul tronçon non rationnel, un tour complet et deux disques plans présents dans le STL. Aucun pôle, retour axial, cavité ou raccord à une autre famille n’est inventé.

Le profil est **ajusté sous une borne**, pas présenté comme une récupération de l’intention de conception. Une adaptation au code stable le signale. Les primitives historiques gardent la priorité ; un cône ou un tore ne devient pas une révolution libre pour masquer une preuve refusée. Auto et Surfaces peuvent employer ce domaine lorsque sa source est démontrée. Le guidage local couplé et les sections DXF non coniques restent refusés.

La source doit fournir au moins cinq niveaux complets suffisamment séparés, dont un témoin indépendant de l’ajustement. La recherche est plafonnée à 128 niveaux, 32 000 triangles et 8 192 régions. Chaque triangle est attribué à un disque ou à la paroi. Après ouverture de la couture, le pavage angle–hauteur doit couvrir exactement une fois le rectangle, sans lacune, recouvrement ni inversion. Les bords et disques sont vérifiés séparément. Le seul résidu aux sommets ne suffit pas.

Pour un triangle de largeur normalisée a et d’ouverture angulaire w, une borne d’interpolation proposée est e + M₂a²/8 + M₁aw/4 + Rw²/8, avec résidu spatial e, R=max bᵢ, M₁=3max|Δbᵢ| et M₂=6max|Δ²bᵢ|. Le terme mixte est obligatoire. La correspondance bijective des domaines fournit les deux sens de l’écart. Les déplacements de limites, l’arrondi numérique et une réserve native de 1e−6 mm s’ajoutent sans relever le budget demandé. Cette preuve doit être qualifiée sur le corpus avant intégration validée.

Le volume idéal est πH/7 Σᵢⱼ bᵢbⱼ C(3,i)C(3,j)/C(6,i+j). La mesure native adaptative est comparée à cette référence indépendante de la construction ; elle n’est pas elle-même un certificat d’intervalles.

## Frontière CAO et compatibilité

Le noyau construit seulement le domaine prouvé. Avant/après STEP et lors des documents multisolides, vérifier le solide, la génératrice mono-travée cubique non rationnelle, ses pôles physiques (paramètre éventuellement inversé), l’axe, le tour complet, les deux cercles partagés, les disques, les dimensions, le volume et la provenance. Le volume seul ne distingue pas deux profils renversés axialement.

Une famille Révolution et un compteur distinct sont ajoutés ; ne pas compter cette surface comme un cône. Les champs optionnels absents conservent la lecture des anciens documents. La couture spline et la paroi obtiennent un contrôle de ressources spécifique ; la tessellation de ce domaine certifié est construite sur une grille angle–hauteur déterministe, car le mailleur générique a produit des triangles trop éloignés sur un cas en miroir. Les points proviennent des faces natives réimportées et l’erreur d’interpolation comprend le terme mixte. aucune admission générale des splines ni hausse du plafond de tessellation/résultat. Les anciennes entrées CAO restent disponibles et le onzième groupe de provenance s’ajoute à la fin.

## Réserve native et discrétisation

Le garde natif ε=5e−8 mm contrôle les pôles physiques, les centres, les coefficients de rayon, l’axe et les limites paramétriques. Une dérive canonique inférieure à 4ε par sérialisation, deux sérialisations (corps puis document) et les supports/limites finaux tiennent dans la réserve chargée de 20ε=1e−6 mm. La réserve est soustraite du budget disponible pour la reconnaissance, jamais ajoutée à la tolérance autorisée.

Le contrôle des disques exige des courbes paramétriques circulaires dont la géométrie physique est vérifiée. La paroi exige quatre limites paramétriques linéaires, leurs usages opposés et leur correspondance aux courbes spatiales. Les bornes de grille de tessellation proviennent des quatre pôles effectivement réimportés, avec une marge sur la normalisation des nœuds. Le plafond de 100 000 triangles est contrôlé avant allocation puis à l’extraction. Les autres familles conservent leur mailleur habituel.

## Alternatives et risques

- Une B-spline multisegment ajoute choix des nœuds, multiplicité, raccords et risque de surajustement ; différée.
- Conserver les facettes reste un repli utile hors domaine, mais n’étend pas C2.
- Quatre niveaux quelconques interpolent quatre coefficients : ils ne prouvent pas une forme cubique. Les témoins supplémentaires, la couverture complète et la borne continue restent requis.
- Détecter cette famille trop tôt pourrait intercepter les primitives existantes. Les tests de priorité et de refus font partie du jalon.
- Un réimport peut normaliser la courbe. La comparaison porte sur sa géométrie physique, pas sur un nom de type seul.

## Validation et retour arrière

Le corpus indépendant doit couvrir formes bombées, resserrées, inflexions, placements, tailles et retessellisations, ainsi que plis, niveaux manquants, source ouverte, profils hors famille et budgets insuffisants. Tests core, CAO, application, multisolide et UI ; suite complète après intégration. Les résultats exécutés sont consignés dans VALIDATION. Revenir sur le commit du jalon retire le reconnaisseur et ses champs additifs ; aucune migration de données ni nouvelle dépendance n’est nécessaire.
