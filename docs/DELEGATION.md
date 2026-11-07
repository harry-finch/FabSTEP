# Délégation et revue — 6 novembre 2026

L'architecte conserve les contrats entre modules, les décisions produit, l'intégration et la validation finale. Les sous-agents ont chacun rendu leur rapport avant intégration.

| Mission | Modèle | Livrable et décision de l'architecte |
| --- | --- | --- |
| Audit géométrique puis moteur core | GPT-6 Astra, effort high | Risques du brainstorming identifiés ; parsing, topologie, contours et tests implémentés. Contrat pur retenu, aucun code de référence repris comme preuve de validité. |
| Audit FabApps puis adaptateur OCCT | GPT-6.1 Sol, effort high | SDK local confirmé, ABI C++ étroite, validation et réimport STEP. Intégré après essais sur trois formes. |
| Interface | GPT-6.1 Terra Codex, effort high | Kit Fab adapté, aperçu 3D et commandes natives. Revue : corrections des défauts d'unité/tolérance, lectures concurrentes, limites de fichiers, traductions et champs à 1100 px. |
| Revue croisée core par Sol | GPT-6.1 Sol, effort high | Trois défauts supplémentaires corrigés : fin ASCII, précision de grille, signalement des sommets déplacés ; tests de régression ajoutés. |
| Revue croisée CAD/app par Astra | GPT-6 Astra, effort high | ABI et contrôles de solide validés ; calcul de norme robuste et refus des budgets non finis ajoutés par l'architecte. |

L'architecte a construit app/CLI/Tauri, ajouté les essais bout en bout et de protection des fichiers, réalisé l'export natif, le réimport OCCT séparé et la validation par l'API de FreeCAD. Il a repris les exigences des quatre skills de FabWeave. Les règles de fork ivaCAM du skill Git n'ont pas été transposées à ce nouveau dépôt.

## M2a — extrusions cylindriques

| Mission | Modèle | Rapport et décision de l'architecte |
| --- | --- | --- |
| Reconnaissance d'extrusions | GPT-6 Astra, effort high | Ajustement circulaire, validation de toute la paroi et provenance des bouchons. 27 tests core. Retenu avec mode explicite, borne composée et limites strictes. |
| BRep analytique + revue adversariale core | GPT-6.1 Sol, effort high | Prismes OCCT à surfaces analytiques, coutures périodiques et réimport. 8 tests CAD. A trouvé une saillie près d'un bouchon faussement acceptée : correction Astra et régression confirmées par Sol. |
| Corpus et interface + revue native | GPT-6.1 Terra Codex, effort high | Générateur indépendant, manifeste de références, contrôles et diagnostic FR/EN. A corrigé les nombres non finis cachés et l'invalidation du cache natif ; test d'ordonnancement des révisions. |
| Intégration et décision finale | Architecte principal | Validation native des triangles source avant reconstruction, options CLI, contrôles géométriques, 6 tests app et 3 CLI. Contrôle FreeCAD des cinq cas et de l'export UI, revue visuelle, version/documentation. |

Les rapports ont conduit à scinder le jalon : M2a ne reconstruit que des extrusions complètes. Les cylindres locaux au sein de pièces mixtes et les trous borgnes demandent une étape M2b distincte. Aucun succès sur le corpus synthétique n'est présenté comme une validation sur fichiers utilisateur.

## M2b — pièces mixtes et trous borgnes

| Mission | Modèle | Rapport et décision de l'architecte |
| --- | --- | --- |
| Reconnaissance locale et preuve géométrique | GPT-6 Astra, effort high | Modèle mixte et attribution de tous les triangles. Appairage unique des anneaux, frontière exacte de la paroi et fond plan démontré. Volume analytique calculé sur les surfaces. A quantifié la fragmentation des bouchons en binaire f32. |
| Construction OCCT, tessellation et contre-essais | GPT-6.1 Sol, effort high | Arêtes circulaires partagées entre cylindres et plans, auto-interférences, réimport STEP, tessellation bornée et contrôle de la source sans export intermédiaire. A reproduit le refus natif des plans conservés en binaire f32. 15 tests CAD. |
| Corpus, interface et revue numérique | GPT-6.1 Terra Codex, effort high | Générateur M2b indépendant, variantes positives et négatives, présentation des maxima dans les deux sens et exemple mixte. A trouvé une perte de précision dans la distance à l'intérieur des triangles très fins, corrigée par l'architecte avec un test de régression. |
| Orchestration et validation | Architecte principal | Maintien de M1 et M2a, intégration de la reconnaissance locale, crate pure d'audit par arbre de boîtes, contrôles de volume et de budget, CLI et application. Réimport FreeCAD Part des huit sorties CLI et du STEP enregistré via le dialogue natif. |

Le jalon conserve une distinction entre preuve locale de reconnaissance, validité native et audit de distance échantillonné. Les résultats concernent le corpus synthétique, dont une pièce créée indépendamment avec FreeCAD. Le corpus utilisateur reste à constituer.

## M2c — arcs ouverts

| Mission | Modèle | Rapport et décision de l’architecte |
| --- | --- | --- |
| Reconnaissance des arcs et preuve de paroi | GPT-6 Astra, effort high | Contours ouverts, raccords et provenance des triangles ; refus des candidats dont la preuve est incomplète. 40 tests core passent ; cas en D et contours sous-échantillonnés couverts. |
| Construction BRep et contre-essais | GPT-6.1 Sol, effort high | Surfaces cylindriques partielles et arêtes partagées ; 18 tests CAD, réimport DRAWEXE. Revue croisée : contournement des seuils et différence liée à la subdivision de corde reproduits, corrigés par Astra puis contre-vérifiés. |
| Corpus, interface et analyse du STL réel | GPT-6.1 Terra Codex, effort high | Générateur M2c indépendant, variantes et négatifs, affichage des arcs et messages FR/EN. Lecture directe des plans du STL et comparaison avec le STEP par FreeCAD ; le STEP original confirme la hauteur de 18 mm ; une arête intermédiaire à Z ≈ 17,13256 mm pourrait expliquer la mesure rapportée, sans preuve de ce lien. |
| Orchestration et validation | Architecte principal | Intégration app/CLI et maintien des régressions ; 78 tests workspace et un test desktop passent ; formatage et Clippy strict passent. Neuf STEP M2c validés dans FreeCAD, dont l’export natif du fichier utilisateur ; comparaison au STEP original, revue visuelle et documentation terminées. |

## M3 — 3MF Core et corps distincts

| Mission | Modèle | Rapport et décision de l'architecte |
| --- | --- | --- |
| Import structuré et API core | GPT-6 Astra, effort high | Crate pure d'import ZIP/XML, unités, transformations et occurrences ; API d'analyse en mm et de séparation par arêtes. 43 tests core et 11 import. Corrections des trois défauts remontés par la revue et budgets avant DOM. |
| Document STEP et revue import | GPT-6.1 Sol, effort high | Multi-racines OCCT, contrôle de séparation source/résultat, noms et réimport racine par racine ; 23 tests CAD. A trouvé les structures XML trop permissives, les descripteurs ZIP incohérents et le coût de copie des espaces de noms. |
| Corpus, interface et précision | GPT-6.1 Terra Codex, effort high | Générateur 3MF indépendant, aperçu puis conversion par révision, inspecteur par corps, textes FR/EN. A quantifié l'arrondi de l'export FreeCAD et isolé son effet en restaurant uniquement les coordonnées exactes du STL dans un témoin privé. |
| Intégration et décision finale | Architecte principal | App/CLI/Tauri, contrôle de la source avant reconstruction, maintien du contrat mono-STL, tests bout en bout, corpus complémentaire affine et export FreeCAD. Contre-vérification des trois défauts, 105 tests workspace + 1 desktop, douze réimports FreeCAD, revue visuelle et export natif. |

L'architecte a retenu un refus conservateur des contacts et recouvrements : conserver des solides superposés ne restituerait pas l'union géométrique prescrite par 3MF pour la fabrication. Les placements et identifiants source sont conservés sans arbre XCAF. Le 3MF FreeCAD arrondi reste un refus documenté en mode Cylindres ; le témoin exact confirme que le moteur n'a pas besoin d'un relâchement global des tolérances. Les constats et preuves figurent dans [VALIDATION](VALIDATION.md).

## Consolidation M3 — précision et pièces utilisateur

| Mission | Contribution |
| --- | --- |
| Corpus et reconnaissance | Générateur synthétique exact, arrondi et aberrant en 3MF, avec une variante STL ASCII. Régressions core et app pour le seuil strict, l'ajustement explicite, le sommet aberrant et le budget d'écart. Premiers essais de conversion de la pièce arrondie. |
| Validation indépendante | Lecture directe du ZIP/XML et du STL SolidCraft, comparaison des coordonnées en f32 et des empreintes, réimport FreeCAD des STEP de référence et des conversions privées et synthétiques. Vérification des rayons, longueurs, dimensions, volumes et surfaces. |
| Interface | Présentation des réglages appliqués, du résidu radial et de la borne composée. Libellés français et anglais, distinction entre ajustement et budget, maintien de l'invalidation après modification d'un réglage. |
| Intégration | Conservation des seuils par défaut, ajout des réglages appliqués dans le rapport commun et le diagnostic `precision.custom_tolerances`. Vérification des parcours CLI et bureau. |

Les mesures confirment deux cas distincts. Pour le premier jeu, un ajustement radial explicite suffit sur l'export FreeCAD arrondi. Pour le second, le 3MF est déjà un paquet Core simple et les courbes restent facettisées dans les exports validés. Les données utilisateur et leurs dérivés restent dans `artifacts/`. [Validation](VALIDATION.md#consolidation-m3--pièces-utilisateur-et-réglages-explicites)


Complément de revue UI 0.5.1 : la vérification native par l’architecte a révélé la conservation des erreurs dans l’ancienne langue après basculement FR/EN. Le sous-agent UI a remplacé le stockage du texte traduit par celui de l’erreur structurée ou de la clé locale. Le refus de planéité de la seconde pièce a ensuite été traduit sans nouveau calcul, puis la conversion Plans à 1e-7 mm a été validée dans le build final (V-507). Le build final conserve les mêmes contrôles géométriques ; sa taille JS minifiée est de 1 016,85 ko.


## M4a — cônes et chanfreins circulaires

| Mission | Modèle | Rapport et décision de l’architecte |
| --- | --- | --- |
| Reconnaissance et preuve des bandes | GPT-6 Astra, effort high | Modèle de révolution séparé, anneaux et pointes source, attribution exhaustive, volume analytique et borne composée. 52 tests core dont six nouveaux tests M4. Revue croisée CAD sans défaut bloquant. |
| Noyau, corpus indépendant et contre-revue | GPT-6.1 Sol, effort high | Surfaces coniques OCCT, anneaux contrôlés avant couture, pointes et compte des surfaces au réimport multi-racines. 26 tests CAD. Dix STEP du corpus réimportés dans FreeCAD, sept négatifs et budget insuffisant refusés. |
| Interface et documentation d’usage | GPT-6.1 Terra Codex, effort high | Mode explicite, exemple chanfreiné, compte conique et diagnostics FR/EN. Sélecteur élargi après revue à 1100/1440 px, audit des révisions/cache et traduction de l’erreur de cohérence analytique. |
| Intégration et validation finale | Architecte principal | App/CLI, cinq tests app et un test CLI supplémentaires, maintien des modes historiques, 3MF placé et multi-corps, protection des sorties. Relecture des preuves, deux réimports des fichiers réels et export conique depuis le dialogue natif. |

Deux problèmes ont été corrigés avant livraison. L’audit de distance a détecté un triangle effondré près d’une pointe : seules les copies de l’apex topologique dans la tessellation OCCT sont désormais réunies, sans modifier le STEP ni la source. La contre-revue Sol a aussi montré que de très courtes bandes presque cylindriques pouvaient séparer artificiellement plusieurs cônes de pentes différentes. L’architecte a retenu une limite conservatrice d’une seule plage conique par chaîne connectée ; Astra a ajouté la reproduction et Sol a confirmé le refus par la CLI.

L’inspection temporaire du rendu natif a établi que WebKit suspend les frames lorsque la fenêtre lancée par l’outil est masquée (`visibility hidden`, zéro frame, contexte valide et 192 triangles chargés). Le passage à la visibilité a ensuite produit une frame et montré le chanfrein et ses anneaux. L’instrumentation a été retirée ; aucun changement de rendu n’a été appliqué pour contourner cette condition de test. Voir les observations exécutées et leurs limites dans [VALIDATION](VALIDATION.md).

## M5a — hémisphères et congés toriques

| Mission | Modèle | Rapport et décision de l’architecte |
| --- | --- | --- |
| Reconnaissance et preuve des surfaces | GPT-6 Astra, effort high | Modèle `RoundedRecognition`, couverture orientée des triangles, hémisphères et congés toriques quart de cercle, combinaison avec les arcs ouverts. 57 tests core et Clippy strict. Les deux sources utilisateur passent aux seuils initiaux, sans lecture du STEP original. |
| Noyau, corpus et validation indépendante | GPT-6.1 Sol, effort high | Surfaces OCCT sphériques/toriques, anneaux, singularités et budgets de tessellation ; 29 tests CAD. Six STEP synthétiques (sept solides) et les deux sorties réelles réimportés dans FreeCAD, avec rayons, domaines et tangences contrôlés. Deux placements miroir supplémentaires vérifiés. |
| Interface et documentation d’usage | GPT-6.1 Terra Codex, effort high | Mode explicite FR/EN, comptes sphériques/toriques et congés prouvés séparés, exemple local, frontières et diagnostics. Documentation limitée au périmètre démontré ; revue des options, du cache et de l’export. |
| Intégration et revue finale | Architecte principal | App/CLI, cinq tests app et un test CLI, compatibilité historique, placements, protection des sorties et audits. Revue des bornes et contrats, contrôle visuel puis bureau, version 0.7 et documentation. Les 142 tests workspace passent. |

Les rapports ont conduit à deux corrections avant livraison. Les arêtes équatoriale et méridienne d’une sphère peuvent partager centre et rayon : le noyau compare également leurs axes pour retrouver le bon anneau. La contre-revue a aussi exigé de comptabiliser les déplacements imposés aux sommets d’équateur et de pôle dans le résidu vectoriel de projection ; une régression les couvre. Aucun seuil n’a été augmenté.

Le choix retenu reste conservateur : l’hémisphère et le congé torique de la deuxième pièce sont prouvés et reconstruits, sans annoncer les sphères/tores complets, les autres calottes ou les congés généraux. Les modes précédents conservent leurs contrats. Les artefacts privés et le STEP original restent dans `artifacts/`, sans publication distante.


## Indicateur de calcul — 0.7.1

| Mission | Modèle | Rapport et décision de l’architecte |
| --- | --- | --- |
| Composant et traductions | GPT-6.1 Terra Codex, effort high | Indicateur compact, durée monotone, état annoncé sans lecture répétée du chronomètre, animation réduite selon préférence système. Contrôle Svelte réussi. |
| Contre-revue des requêtes | GPT-6.1 Sol, effort high | Séparer verrou moteur et demande visible, protéger la queue et le nettoyage par révision ; réserver la lecture à l’appel aperçu réellement démarré. Aucun blocage après intégration. |
| Intégration et vérification | Architecte principal | Cycle asynchrone, capture d’erreur d’allocation dans le bloc protégé, libellés d’attente, version et bundle local ; banc IPC temporaire et essai natif. Voir VALIDATION. |


## M5b1 — calottes sphériques (7 novembre 2026)

| Mission | Modèle | Rapport et décision de l’architecte |
| --- | --- | --- |
| Reconnaissance, courbure et preuve | GPT-6 Astra, effort high | Calottes à anneau plan et pôle source, distinction d’un cône par courbure intérieure, couverture géodésique et borne de frange. 63 tests core, dont échantillonnage des deux directions et projection au trim. Paramètres et rapports numériques des deux sources utilisateur inchangés. |
| Noyau, corpus et contre-revue | GPT-6.1 Sol, effort high | Pont de production déjà compatible, test CAD petites/grandes calottes. Générateur Python indépendant et références séparées `Part.makeSphere`. Cinq STEP CLI et six solides validés dans FreeCAD ; centres, domaines, dimensions, volumes et orientations contrôlés. Aucun blocage relevé dans la preuve core finale. |
| Textes et revue d’interface | GPT-6.1 Terra Codex, effort high | Périmètre FR/EN et documentation d’usage ; contrat TypeScript et aperçu inchangés. Six boutons d’exemple conservés. Contrôle Svelte réussi ; revue visuelle navigateur non exécutée faute de surface disponible auprès de cet agent. |
| Intégration et livraison | Architecte principal | Périmètre limité à M5b1 sans nouvelle ABI, trois tests app, revue des bornes et du cône ambigu, version 0.8, non-régressions réelles et documentation. 152 tests workspace et un test bureau passent. Conversion native d’une grande calotte observée ; export par dialogue non confirmé après expiration du pilotage macOS. |

Deux points ont conditionné l’intégration. Un anneau et un pôle peuvent également décrire un cône : la courbure intérieure doit être démontrée. Hors équateur, la frontière géodésique des triangles diffère du petit cercle analytique : la couverture et la frange doivent être traitées séparément, avec tous les déplacements de projection comptés. Ces choix sont documentés dans [ADR-0008](adr/0008-spherical-caps.md). Le jalon ne prétend pas traiter les sphères entières, zones sphériques sans pôle ou congés généraux.

## M5b2 — sphères entières (7 novembre 2026)

| Mission | Modèle | Rapport et décision de l’architecte |
| --- | --- | --- |
| Reconnaissance et preuve globale | GPT-6 Astra, effort high | Ajustement normalisé, topologie χ=2, orientation et couverture 4π, borne des facettes, aucune limite source fictive. Cinq nouveaux tests core ; 68 passent. Recommandation retenue : essayer la sphère seulement après les interprétations historiques sans candidat, pour préserver les cylindres cosphériques. |
| Noyau, corpus indépendant et contre-revue | GPT-6.1 Sol, effort high | Entrée C ABI dédiée à la sphère complète, réimport et deux pôles de tessellation ; trois tests CAD ajoutés. Six références FreeCAD séparées, six STEP du corpus, deux exports natifs et deux sorties réelles contrôlés. Contre-revue de la preuve core sans blocage ; ambiguïté des petites faces cosphériques documentée. |
| Contrat UI, textes et documentation | GPT-6.1 Terra Codex, effort high | Type nullable, libellés FR/EN sans nouveau bouton, usage et architecture, état et journal. Check Svelte réussi ; revue navigateur sombre/clair à environ 1512 px. Aucune revendication d’essai à 1100 px. |
| Intégration et validation finale | Architecte principal | Contrat `rings:null`, ordre de reconnaissance, quatre tests app et un CLI, revue des preuves, version 0.9, contrôles globaux, essais natifs et documentation. 165 tests workspace et un desktop passent. Les exports de sphère et de grande calotte clôturent aussi la réserve native de M5b1. |

La priorité historique et l’absence de frontières source inventées ont déterminé le contrat. Un témoin de contact trop dense a été allégé pour démontrer le contact sous le budget existant, sans augmenter les limites du produit. Les changements restent locaux ; aucune publication distante n’a eu lieu. Voir [ADR-0009](adr/0009-complete-spheres.md) et [VALIDATION](VALIDATION.md).


## Préparation de la v1 mécanique — 7 novembre 2026

L’utilisateur a confirmé que la v1 doit couvrir des pièces mécaniques complexes et que la qualité prime sur le délai. Le paquet autonome est donc un jalon intermédiaire 0.10, pas la v1 achevée. Les sous-agents partagent des périmètres de fichiers séparés, annoncent leurs commandes Cargo et rendent leurs preuves avant intégration.

| Mission | Modèle | Rapport et décision de l’architecte |
| --- | --- | --- |
| Audit moteur et preuves complexes | GPT-6 Astra, high | Faux succès Plans reproduit et corrigé : validation native source pour tous les modes. Puis preuves et tests des tores complets, double chanfrein autour d’une portée cylindrique résolue et pavés arrondis à rayon uniforme. |
| Noyau et paquet macOS | GPT-6.1 Sol, high | Fermeture Mach-O, notices, cible macOS 26 et signature ad hoc. ZIP 0.10 vérifié avec Homebrew bloqué. Construction/relecture native des nouvelles formes, corpus et contrôles FreeCAD indépendants. |
| Interface | GPT-6.1 Terra Codex, high | Éditions numériques regroupées, persistance valide, contexte d’erreur, aide et accessibilité. Sept tests UI et cinq tests bureau ; source binaire en cache, protection des révisions et localisation des seuls noms 3MF générés. Revue statique sans bloqueur ; parcours natif final en qualification. |
| Corpus mécanique et audit de distance | GPT-6.1 Sol, high | Quatre positifs + deux négatifs indépendants, puis optimisation BVH et cache exact des milieux d’arêtes. Le carter de 41 290 triangles passe sous le budget inchangé avec distances et échantillons identiques bit à bit. |
| Intégration | Architecte principal | Contrats, pipeline app/CLI, tests de bout en bout, UI des nouveaux rapports, verrou npm portable, contrôles globaux, documentation et critères de qualification v1. Aucune publication implicite. |

Les quatre positifs mécaniques ont été réimportés avec succès, dont le boîtier aux douze congés d’arêtes et huit coins sphériques. Les tores entiers et leurs variantes passent également, ainsi que les six conversions privées. La contre-revue Astra ne relève aucun blocage dans le contrat CAD du boîtier. Sol a qualifié la candidate autonome RC1 avec 29 bibliothèques et quatre conversions isolées de Homebrew ; l’architecte poursuit le parcours natif et le contrôle global avant la livraison. Les résultats datés et les limites sont conservés dans [VALIDATION](VALIDATION.md), [RELEASE](RELEASE.md) et le [corpus](../fixtures/complex_v1/README.md).
