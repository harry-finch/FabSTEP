# ADR-0038 — Domaines des perçages sécants centrés

Statut : accepté sous A-045 ; intégré par R-083, validation automatisée V-1205. Essais utilisateur distincts.
Date : 18 novembre 2026.

## Contexte

Le laboratoire F0 certifie les courbes d’intersection réellement réimportées de deux cylindres orthogonaux. Il ne prouve pas qu’un STL donné représente ce domaine. La reconstruction doit conserver les ouvertures, le choix de matière, les placements et l’attribution source, sans faire d’une opération booléenne native une preuve de reconnaissance.

## Décision

Ajouter à `RoundedModel` un domaine optionnel alloué séparément `CrossDrill`. Il contient un centre, un repère direct (x : grand trou, z : petit trou), trois demi-dimensions, les rayons petit/grand, quatre références d’anneaux d’entrée et deux cycles d’identifiants de sommets source pour les branches négative/positive en z. Un résidu mesuré `witness_tolerance_mm`, déjà compté dans la borne source, permet à la CAO de recontrôler les témoins bruts sans les déplacer. Les plans suivent l’ordre −x, −y, −z, +x, +y, +z ; les trois propriétaires cylindriques sont petit trou négatif, petit trou positif, grand trou. Le dixième groupe de provenance s’ajoute sans déplacer les groupes existants.

Le domaine est limité à un bloc, quatre entrées circulaires complètes, des axes centrés orthogonaux et un grand rayon au moins double du petit, avec incertitudes et marges résolues. Le core doit prouver les supports, les neuf régions orientées, leurs incidences opposées et la couverture autour des deux ouvertures du grand cylindre. Les branches source portent leurs identités avant la construction native. Une couture périodique peut les découper en plusieurs arêtes OCCT ; leur nombre ne définit pas leur identité.

La borne finale inclut l’erreur source et une réserve native de 4e−7 mm : branches et paramètres certifiés, plus correspondance des supports avant/après transfert. La CAO généralise les certificats F0 aux paramètres/repères prouvés, vérifie les branches, leurs courbes paramétriques et leur couverture après STEP, puis les contrôles habituels du document. Les arêtes spline ne deviennent admissibles à la tessellation que dans ce domaine certifié et borné. L’arithmétique d’intervalles conserve ses contraintes de compilation ; les plafonds globaux ne sont pas relevés.

Le volume idéal est calculé indépendamment de la construction OCCT. Le volume natif adaptatif est une mesure, distincte de l’encadrement analytique. Les anciennes formes restent lisibles lorsque le champ est absent. Le guidage local du groupe couplé reste refusé tant qu’il n’a pas sa propre preuve. La famille affichée est Cylindre, sans nouveau choix utilisateur.

## Alternatives envisagées

- Garder des facettes conserve un repli utile, mais ne reconstruit pas les perçages autorisés par A-045.
- Introduire un BRep générique et des splines arbitraires élargirait inutilement le contrat. Le domaine dédié conserve des invariants explicites.
- Utiliser seulement une soustraction native ou des sondes échantillonnées ne prouve ni la source ni la couverture des limites ; ces opérations ne remplacent pas les preuves exigées.

## Conséquences

La couverture d’une paroi cylindrique à deux ouvertures et les coutures périodiques demandent des contrôles supplémentaires. L’attribution source demeure exhaustive ; la correspondance native conserve la politique tout-ou-rien d’ADR-0018. Les sections DXF contenant ces intersections non coniques restent refusées ; aucun contour polygonal n’est substitué silencieusement.

## Validation

Corpus paramétrique indépendant, variantes de placement/rayon/tessellation, négatifs de topologie et de domaine, tests core et CAO, documents à plusieurs corps, réimport STEP, volume, fidélité et provenance. Réutiliser les adversaires du certificat F0. Une preuve incomplète, une régression historique ou un budget dépassé interdit de déclarer le lot validé. Les résultats seront consignés dans VALIDATION ; aucun succès de production n’est revendiqué par cette décision seule.

## Extension F2 du 18 novembre 2026

R-091 conserve les quatre anneaux, les deux branches et les trois propriétaires de ce contrat. Les mesures permettent d’étendre le rapport petit/grand rayon strictement sous deux tiers si la marge couvre l’incertitude des deux rayons. Les contrôles du certificat et les essais au seuil sont référencés dans [V-1219](../VALIDATION.md#v-1219--r-091-perçages-sécants-de-rayons-plus-proches).
