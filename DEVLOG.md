# Journal d'ingénierie

## 2026-11-06 — Cadrage et première verticale STL → STEP

Décisions utilisateur : moteur C++ OCCT accepté derrière Rust, priorité bureau macOS puis Windows/Linux, solide BRep sans historique d'esquisses au premier jalon. Licence et conventions FabWeave reprises. L'audit du brainstorming a révélé notamment un faux succès d'export de coque ouverte et des appairages de trous insuffisamment prouvés.

Trois sous-agents ont développé le core, l'adaptateur CAO et l'interface ; les revues croisées ont corrigé parsing ASCII, précision du soudage et signalement des adaptations. L'architecte a intégré CLI/Tauri, sauvegarde atomique, protection des sorties et contrôles source/résultat. Voir [rapport](docs/DELEGATION.md).

Validation : 25 tests Rust, Clippy strict, compilation du bureau, contrôle Svelte et build. Vérifications visuelles sombre/clair à 1440 et 1100 px ; valeurs numériques élargies après revue. Export natif du cube 20 mm, réimport DRAWEXE puis API FreeCAD 1.1.1 réussis. L'automatisation de la GUI FreeCAD s'est arrêtée dans QMacAccessibilityElement ; pas de succès manuel graphique revendiqué. Les essais et limitations sont consignés dans `docs/VALIDATION.md`.

La version produite est une alpha locale liée au SDK OCCT installé, pas une distribution autonome. Les cylindres analytiques et 3MF restent les étapes suivantes.

## 2026-11-06 — M2a : extrusions à cylindres analytiques

Après validation du premier jalon par l'utilisateur, développement réparti entre Astra (reconnaissance géométrique), Sol (construction OCCT et revue contradictoire), Terra Codex (corpus indépendant, interface puis revue du contrat natif). L'architecte a conservé l'intégration app/CLI, les tests bout en bout et la validation finale.

Choix documenté dans ADR-0002 : mode explicite strict pour distinguer l'intention circulaire des polygones. Reconnaissance d'extrusions complètes seulement ; cercle ajusté, bouchons correspondants, vérification de chaque triangle latéral, borne composée affichée. Cylindre à trois faces, tube à quatre faces, plaque percée à sept faces. Le chemin Plans reste disponible et conserve les facettes.

Deux résultats de revue ont modifié l'implémentation. L'arrondi f32 d'un STL tourné rend certaines régions fusionnées légèrement non planes : la validation source en mode Cylindres utilise ses triangles exacts et la recherche des bouchons utilise la tolérance d'ajustement. Une saillie radiale près d'un bouchon échappait au test d'altitude : la provenance des triangles remplace cette exemption ; cas permanent de régression. La borne additionne les contributions de paroi, de bouchon et de correspondance.

La revue UI/native a corrigé la sérialisation des champs numériques vides et l'invalidation immédiate du cache natif. Un test couvre les révisions arrivant dans le désordre. Les valeurs obsolètes ne deviennent pas exportables après une modification.

Validation : 44 tests workspace et un test desktop ; Clippy strict, formatage, Svelte et build. Cinq STEP du corpus validés par FreeCAD Part, types de surfaces, rayons, volume et axe tourné compris. Export réel depuis le dialogue macOS puis réimport FreeCAD : cylindre valide, deux plans et un cylindre. Interface revue en sombre/clair à 1440/1100 px, refus du budget 0,001 mm et récupération en mode Plans après champ vide vérifiés. Le raccourci synthétique `super+a` de l'outil de contrôle quitte l'application sur ce poste ; la saisie par API d'accessibilité fonctionne. Aucun crash du moteur n'en a été déduit.

La distribution reste locale et dépendante du SDK Homebrew. Les fichiers de l'utilisateur et Fusion restent à tester ; trous borgnes et pièces mixtes reportés à M2b.

## 2026-11-06 — M2b : cylindres locaux, pièces mixtes et fonds borgnes

L'ADR-0003 introduit des anneaux partagés entre chaque cylindre complet et ses deux faces planes adjacentes. Le moteur attribue chaque triangle à une face. Il reconnaît un fond borgne uniquement si la région plane existe dans le STL. Le mode Cylindres conserve le chemin d'extrusion M2a et essaie les cylindres locaux lorsque celui-ci renvoie `recognition.unsupported`. Les autres erreurs restent bloquantes.

L'adaptateur natif réutilise les arêtes circulaires des cylindres dans les faces planes, contrôle les auto-interférences et réimporte le STEP final. Une validation native distincte contrôle la source triangulée sans effectuer d'export intermédiaire. Les tests CAD incluent deux perçages qui se croisent sans arête commune et vérifient leur refus.

L'architecte a ajouté `fabstep-verify`, qui compare les deux maillages par arbre de boîtes. La revue numérique de Terra a révélé une perte de précision dans la distance à l'intérieur de triangles très fins. Le calcul a été corrigé avec un prédicat fondé sur le produit vectoriel et un test de régression. Le rapport distingue la borne locale de reconnaissance, la déflexion demandée à OCCT et les maxima échantillonnés dans les deux sens.

Les huit STEP du corpus ont passé les contrôles de solide fermé, types de surfaces, dimensions, volumes, rayons, profondeurs, hauteurs et axes attendus au réimport via l'API Part de FreeCAD 1.1.1. Le corpus comprend sept variantes générées par Python et une pièce FreeCAD à axes orthogonaux de 3 364 triangles. Les conversions mesurées en debug prennent de 1,46 à 4,33 s. Les parois coniques, fonds non plans, bossages bombés et contours à huit points du corpus sont refusés. [Validation](docs/VALIDATION.md) · [Rapport FreeCAD](artifacts/m2b-freecad-validation.json)

La conversion du STL binaire f32 dérivé de la pièce mixte inclinée échoue avec les réglages par défaut. La segmentation à 0,00001 mm fragmente un bouchon. À `--plane 0.00002`, le moteur reconnaît la pièce, mais OCCT refuse un contour conservé au-delà de sa tolérance d'environ 0,0000001 mm. Le modèle garde les points source et aucune projection globale n'a été introduite.

La version 0.3.0-alpha.1 compte 67 tests workspace et un test bureau. L'exemple mixte dans l'application locale affiche dix faces, dont huit plans et deux cylindres, et des maxima échantillonnés d'environ 0,010096 mm et 0,010221 mm. L'enregistrement via le dialogue natif a réussi. Le fichier constitue le neuvième réimport M2b et FreeCAD confirme les mêmes types de surfaces et le volume attendu. Les essais dans Fusion et sur des fichiers utilisateur restent à mener.

## 2026-11-06 — M2c : arcs ouverts et premier STL utilisateur

Le premier STL réel, conservé localement, comporte une encoche latérale dont le contour est un arc ouvert. La reconnaissance précédente conservait ses 36 facettes planes. Le modèle mixte prend désormais en charge des portions cylindriques dont les deux arcs et les raccords latéraux sont démontrés ; les candidats incomplets sont refusés. Un corpus synthétique indépendant couvre variantes tournées, subdivisées, mixtes et négatives. Le résultat local mesuré sur ce STL comporte 14 faces (11 plans, 3 cylindres) et un volume de 52 480,08325658634 mm³. La validation finale comprend 78 tests workspace (40 core, 18 CAD, 12 app, 4 CLI, 4 verify) et 1 test desktop ; formatage et Clippy strict passent pour les deux. Svelte check ne signale ni erreur ni avertissement et le build UI réussit (bundle principal 998,30 ko).

L’utilisateur signale une hauteur attendue de 17,13 mm dans sa CAO. Lecture directe du STL binaire, sans le moteur FabSTEP : base Z = 0 ; plan principal à Z = 18 mm avec 174 triangles horizontaux et aire 2 489,08295993 mm² ; Z = 17,13256073 comporte des sommets mais aucun triangle horizontal. FreeCAD Part confirme Z = 0, 18 et 28 mm sur le STEP précédent. Le STEP d’origine fourni ensuite confirme lui aussi les plans Z = 0, 18 et 28 mm : l’écart de hauteur n’a pas été reproduit. Une arête B-spline intermédiaire du petit trou est située à Z ≈ 17,13256 mm ; elle pourrait expliquer la mesure rapportée, mais ce lien reste une hypothèse. [Mesures locales et empreinte](artifacts/body-test-height-validation.json).

La comparaison FreeCAD du STEP d’origine et du résultat M2c montre respectivement 15 faces (11 plans, 4 cylindres) et 14 faces (11 plans, 3 cylindres), le petit trou étant fusionné dans le résultat. Le volume passe de 52 480,08321216366 à 52 480,08325658634 mm³. Pour l’encoche, rayon original 17,61391495380856 mm et balayage 2,514231927641763 rad ; résultat 17,613915009037 mm et 2,514231894625 rad, hauteur 18 mm dans les deux cas. [Comparaison locale](artifacts/body-original-comparison.json) · [Arête intermédiaire](artifacts/body-original-intermediate-edges.json).

Huit STEP issus de la CLI, dont sept cas synthétiques et le STL réel, ont été réimportés et validés via FreeCAD Part. Le volume reconstruit du cas réel correspond à l’original à une tolérance relative de 10⁻⁸ ; les écarts de rayon sont inférieurs à 0,00001 mm et l’écart angulaire de l’arc multiplié par son rayon est inférieur à 0,00001 mm. Les rapports locaux sont [FreeCAD](artifacts/m2c-freecad-validation.json) et [conversions](artifacts/m2c-conversions.json). Les conversions mesurées en debug prennent environ 1,4 à 6 s sur ces cas : ce n’est pas une garantie de performance. La revue adversariale a conduit à corriger les cas de profils en D et de contours entièrement cocirculaires sous-échantillonnés ; les régressions core passent. La revue visuelle couvre les thèmes sombre/clair à 1440/1100 px et le résultat natif du fichier utilisateur. Le STEP enregistré depuis le dialogue macOS a été réimporté et validé dans FreeCAD : neuf sorties M2c contrôlées au total.

## 2026-11-06 — M3 initial : documents 3MF et corps distincts

L'architecte a réparti le travail entre Astra (import 3MF et API core), Sol (STEP multi-racines, séparation native puis revue croisée) et Terra Codex (corpus indépendant, interface et analyse de précision). Le contrat retenu dans ADR-0005 conserve unités, placements et provenance, limite le document à 32 corps et 100 000 triangles et refuse tout contact ou imbrication. Le contrôle natif est appliqué aux triangles source avant reconstruction, aux solides reconstruits et au document STEP réimporté. Il évite notamment de remplir une cavité composée de coques disjointes ou de masquer un recouvrement par ajustement analytique.

Nouvelle crate `fabstep-import` : lecture ZIP/XML en mémoire, relation OPC StartPart, unités Core, transformations composées, miroirs et occurrences du build. Les extensions requises inconnues et les références Production sont refusées. Les budgets s'appliquent avant et après expansion. L'interface distingue aperçu et conversion, garde la source visible après refus et exige tous les corps validés pour exporter ; l'inspecteur sélectionne un corps sans restreindre le document exporté. Les commandes historiques mono-STL restent disponibles dans les API internes ; la CLI et le bureau utilisent le pipeline document.

La revue Sol a trouvé trois défauts de lecture : structures XML trop permissives, descripteurs ZIP contradictoires et coût de copie des espaces de noms dans le DOM. Astra a corrigé les trois et ajouté des régressions. La contre-vérification finale a été réalisée par l'architecte : les fichiers concernés sont refusés, le descripteur valide reste accepté, et le cas à 4 000 redéclarations passe de 4,16 s à un refus en environ 0,036 s. Aucun temps garanti n'est déduit de ces mesures.

Validation 0.5.0-alpha.1 : 105 tests workspace (43 core, 23 CAD, 19 app, 5 CLI, 11 import, 4 verify) et un test desktop ; formatage et Clippy strict verts ; Svelte zéro erreur/avertissement, bundle JS 1 013,37 ko avec avertissement de taille. Neuf exports du corpus sont validés par FreeCAD Part, y compris un 3MF produit par FreeCAD Mesh, ainsi que le STL utilisateur, son témoin 3MF à coordonnées exactes et le STEP enregistré depuis l'interface : douze sorties distinctes. Le test natif de contact conserve l'aperçu et bloque l'export, puis le rechargement d'un document valide rétablit le résultat.

Le STL utilisateur conserve 14 faces et ses dimensions. Son réexport 3MF par FreeCAD 1.1.1 arrondit les 1 075 sommets à six chiffres significatifs, avec écart maximal d'environ 0,0000800 mm. Les résidus circulaires indépendants dépassent alors le seuil strict de 0,00001 mm et le mode Cylindres refuse le fichier. Le remplacement des seules coordonnées par les valeurs f32 source exactes, sans changer indices ni paquet, rétablit les 14 faces et les mesures validées dans FreeCAD. Aucun seuil n'a été relâché. [Comparaison](artifacts/body-test-quantization.json) · [Validation complète](docs/VALIDATION.md).

M3 reste un sous-ensemble Core sans Production multi-modèles, fusion ni arbre d'assemblage XCAF. La précision des exports, les fichiers utilisateur plus variés, Fusion et le conditionnement autonome restent ouverts. L'application locale a été reconstruite et laissée ouverte sur l'exemple 3MF valide, en français et sombre ; serveur de développement et onglet temporaire fermés.


## 6 novembre 2026 — consolidation 0.5.1 : précision explicite et fichiers réels

Le diagnostic séparé par trois sous-agents a distingué deux causes : arrondi à six chiffres significatifs du premier 3MF FreeCAD, et géométries réellement hors périmètre de la seconde pièce SolidCraft. Aucun algorithme de reconstruction ni seuil initial n’a été relâché. Le premier fichier passe avec un ajustement radial choisi à 0,0001 mm ; le deuxième contient cône, sphère et tore et reste refusé en Cylindres. Sa conversion Plans devient possible avec planéité 0,0000001 mm, en conservant 8 211/8 360 faces facettisées selon STL/3MF.

`ProcessingSettings` rend les réglages des requêtes traçables dans l’API commune, la CLI et le bureau, sans confondre préférence d’unité STL et unité du document 3MF. Diagnostic stable `precision.custom_tolerances` pour les seuils actifs personnalisés. L’UI distingue tolérance radiale, budget composé et résidu mesuré, affiche les réglages appliqués, accepte des valeurs positives sous 1e-6 et explique le refus générique sans en inventer la cause. La revue à 1100 px a corrigé un débordement des champs et unités.

Six tests synthétiques supplémentaires prouvent le refus strict, le succès sur arrondi avec réglage explicite, la conservation des coordonnées et de la provenance, le refus d’un point aberrant et le maintien indépendant du budget d’écart. Deux tests supplémentaires et des assertions dans les tests existants couvrent la traçabilité des paramètres et les unités. Validation globale : 113 tests workspace et un test desktop réussis, Clippy strict réussi. Les réimports indépendants, chiffres et limites sont consignés dans docs/VALIDATION.md. Sources utilisateur inchangées sous artifacts, aucune publication distante.


La vérification native a également révélé que les erreurs déjà traduites restaient dans la langue précédente après basculement FR/EN. L’UI conserve désormais l’erreur structurée ou sa clé de traduction et la traduit réactivement, sans nouveau calcul. Le refus de planéité OCCT rencontré sur la seconde pièce dispose d’un texte précis FR/EN. Ces deux comportements ont été vérifiés dans le build final. Le résidu radial est affiché à six chiffres significatifs, sans transformer un petit résidu non nul en zéro.


## 2026-11-06 — M4a : cônes et chanfreins circulaires, 0.6.0-alpha.1

Ajout du mode explicite « Cônes et cylindres » (`primitives`) sans élargir silencieusement Cylindres. Astra a implémenté la reconnaissance pure des bandes à rayon affine et des pointes source ; Sol le pont conique OCCT, le corpus analytique et la validation FreeCAD ; Terra l’interface, l’exemple intégré et les documents d’usage. L’architecte a intégré le pipeline et la CLI, relu les preuves et réalisé les tests bout en bout et natifs. [ADR-0006](docs/adr/0006-cones-et-chanfreins-explicites.md) fixe le contrat.

Les cônes tronqués et pointus, chanfreins circulaires, fraisures et fonds de perçage coniques démontrables produisent de vraies surfaces coniques dans le STEP réimporté. Le volume analytique, la provenance et la borne composée restent distincts de l’audit de distance échantillonné. Le 3MF placé et les documents multi-racines conservent les contrôles de séparation et l’export indivisible. Le refus d’un candidat non prouvé est bloquant ; seule l’absence de candidat autorise la reconnaissance cylindrique historique.

La revue a trouvé une succession non affine faussement acceptée grâce à des séparateurs presque cylindriques. Le moteur impose maintenant une seule portion conique par chaîne connectée ; le contre-exemple est conservé dans le corpus et refusé. La tessellation d’audit normalise uniquement les copies d’une pointe conique topologique sous la tolérance native ; source et BRep ne sont pas modifiées.

128 tests workspace passent (52 core, 26 CAD, 28 app, 7 CLI, 11 import, 4 verify). Dix sorties du corpus, deux conversions de la première pièce réelle et un STEP exporté depuis l’application ont été réimportés dans FreeCAD : treize fichiers distincts. La première pièce garde 14 faces et ses dimensions ; la seconde reste refusée analytiquement car ses courbes dépassent M4a. Les cônes partiels, chaînes à plusieurs portions coniques, sphères, tores, congés et aides manuelles restent ouverts. Détails et limites des essais dans [VALIDATION](docs/VALIDATION.md).

## 6 novembre 2026 — M5a, hémisphères et congés toriques

Version 0.7.0-alpha.1. Travail réparti entre les mêmes sous-agents : Astra pour la reconnaissance et ses preuves, Sol pour OCCT, le corpus indépendant et FreeCAD, Terra Codex pour l’interface et les documents d’usage. L’architecte a fixé le contrat, intégré app/CLI, ajouté les tests de bout en bout et vérifié le bureau.

Le mode `surfaces` reconstruit des hémisphères et des congés toriques quart de cercle entre cylindre et plan, avec triangulations irrégulières et combinaison d’un cône et d’une encoche cylindrique ouverte. `RoundedRecognition` conserve les triangles source et leur attribution exhaustive. Les comptes sphériques et toriques traversent l’ABI et le réimport multi-racines ; les congés prouvés restent un compte distinct. Les bornes comprennent les déplacements de projection et l’écart des facettes, sans promesse de distance globale de Hausdorff.

La contre-revue a corrigé la distinction des arêtes équatoriale et méridienne d’une sphère par leurs axes, puis le comptage des déplacements des sommets contraints à l’équateur et aux pôles. Le coût des recherches et celui de la tessellation dans deux directions de courbure sont bornés. Aucun seuil initial n’a été augmenté et aucun STEP de référence n’intervient dans la conversion.

Les deux sources privées de la deuxième pièce passent aux seuils initiaux. FreeCAD retrouve 19 faces analytiques (12 plans, quatre cylindres, un cône, une sphère et un tore), une demi-sphère R10, un congé R2 tangent à ses deux voisins, et les dimensions 60,5 × 59,5 × 28 mm. Les écarts au volume CAO original restent inférieurs à 0,000056 mm³. Les données privées et sorties restent sous `artifacts/` ; les fixtures publiques sont indépendantes.

Validation : 142 tests workspace et un test desktop réussis ; formatage, Clippy strict workspace/bureau, check desktop, Svelte check et build passent. Six STEP synthétiques (sept solides), les deux sources réelles et l’export du dialogue macOS sont relus dans FreeCAD. Deux sondes en placement miroir passent aussi. Le navigateur a été inspecté à 1100×800 et 1440×900, en anglais clair et français sombre. Le bureau affiche la pièce réelle ; un budget insuffisant invalide le résultat et son export, avec traduction réactive de l’erreur.

Une première tentative d’import via accessibilité a nécessité de redémarrer uniquement l’instance de test. La sélection par clic dans la liste a ensuite permis l’import ; une observation complète a résolu l’état périmé du bouton Save. Le STEP natif est validé indépendamment. Ces difficultés de pilotage sont consignées sans les assimiler à des défauts du moteur. Voir [VALIDATION](docs/VALIDATION.md) et [DELEGATION](docs/DELEGATION.md).

Limites conservées : autres calottes, sphères/tores complets, congés généraux ou variables, surfaces libres, Production 3MF et distribution autonome. Le dépôt reste sans commit initial ni publication distante.


## 2026-11-06 — 0.7.1 : calcul visible

À la demande utilisateur, un indicateur compact est ajouté dans l’aperçu : pictogramme géométrique, barre indéterminée, phase réelle et durée depuis la dernière demande. Aucun pourcentage ni temps restant n’est inventé. Les textes existent en français et anglais, les couleurs utilisent les jetons et les animations respectent `prefers-reduced-motion`.

Le verrou d’exécution reste séparé de la demande visible. Une modification pendant un ancien calcul affiche l’attente du suivant ; la réponse obsolète ne peut effacer cet état. Les saisies invalides et les refus quittent le traitement, y compris un refus 3MF avant aperçu. Le minuteur est nettoyé au démontage et la préparation du tableau IPC se trouve dans le bloc protégé pour libérer le verrou même en cas d’échec d’allocation. Le moteur géométrique n’a pas changé. La délégation et les vérifications sont consignées dans les documents dédiés.

Validation de cette version : 142 tests workspace et un test desktop, formatage, Clippy strict, Svelte et builds réussis. Le banc IPC temporaire a vérifié les phases et les requêtes remplacées en deux langues et deux thèmes. Le test natif sur la deuxième pièce a confirmé l’attente après changement, le refus à 0,001 mm puis la réussite à 0,05 mm : 19 faces, dimensions et volume inchangés, indicateur retiré et export disponible.


## 2026-11-07 — M5b1 : calottes sphériques, 0.8.0-alpha.1

Le mode Surfaces arrondies s’étend aux petites et grandes calottes convexes ou concaves, avec anneau plan, pôle source et courbure suffisamment prouvée. Les domaines non résolus, zones sphériques sans pôle et sphères complètes ne sont pas promis. Les bornes intègrent la frange au bord hors équateur ; aucun seuil initial n’a été augmenté. Les libellés FR/EN sont adaptés sans ajouter d’exemple à la colonne source.

Cinq STEP CLI synthétiques, représentant six solides, ont été réimportés et validés dans FreeCAD : calottes petites et grandes, cavité concave, rotation et instances miroir. Les deux sources privées de la seconde pièce gardent 19 faces et les dimensions 60,5 × 59,5 × 28 mm au réimport FreeCAD. [Corpus CLI](artifacts/m5b1/verified/corpus-cli-summary.json) · [Corpus FreeCAD](artifacts/m5b1/verified/corpus-freecad.json) · [Régression réelle](artifacts/m5b1/verified/real-regression.json).

152 tests workspace et un test desktop réussissent ; formatage et Clippy strict du workspace, check desktop, Svelte check et build UI passent. Dans le bureau 0.8, la grande calotte affiche deux faces, dont une sphérique, un volume de 837,758041 mm³ et des dimensions de 12 × 12 × 10 mm. L’export natif n’a pas été effectué : après observation du résultat et du bouton Exporter actif, les lectures CUA ont expiré à répétition. Un essai manuel reste à faire ; ce blocage ne démontre pas un défaut du moteur. La revue navigateur prévue pour les libellés n’a pas abouti ; aucune validation visuelle navigateur M5b1 n’est revendiquée.


## 2026-11-07 — M5b2 : corps sphériques complets, 0.9.0-alpha.1

Le mode Surfaces arrondies reconnaît les corps sphériques complets sans imposer d’anneaux ni de pôles source. Le parcours historique garde la priorité ; la reconnaissance complète intervient après `recognition.unsupported`. Les régions planes significatives sont exclues, mais de petites facettes ou des quadrilatères cosphériques restent indiscernables d’une tessellation. Les zones sphériques tronquées sans pôle, tores complets et congés généraux restent différés. Voir [ADR-0009](docs/adr/0009-complete-spheres.md).

165 tests workspace passent (68 core, 33 CAD, 40 app, 9 CLI, 11 import, 4 verify), ainsi qu’un test desktop. Formatage workspace, Clippy strict, check desktop, Svelte check, build UI et bundle local réussissent. Six STEP CLI (sept solides), dont cinq cas de sphères complètes et un contrôle de calotte, ont été réimportés dans FreeCAD ; six témoins négatifs et un budget insuffisant sont refusés. Les deux sources réelles ont été reconverties et réimportées dans FreeCAD avec 19 faces et des dimensions de 60,5 × 59,5 × 28 mm. Les volumes sont inchangés à 54 445,27801611677 mm³ pour le STL et 54 445,278047030406 mm³ pour le 3MF ; rayons et tangences restent conformes et les trois empreintes source intactes.

Dans le bureau 0.9, la sphère affiche une face, 904,778684 mm³ et 12 × 12 × 12 mm ; la calotte affiche deux faces, 837,758041 mm³ et 12 × 12 × 10 mm. Leurs STEP ont été enregistrés respectivement dans `artifacts/m5b2-interface-sphere-0.9.step` et `artifacts/m5b2-interface-cap-0.9.step` ; leurs réimports indépendants dans FreeCAD sont réussis : [sphère](artifacts/m5b2/interface-sphere-freecad.json) et [calotte](artifacts/m5b2/interface-cap-freecad.json). Cet essai de calotte 0.9 complète la vérification native restée ouverte en 0.8. Dix sorties STEP au total ont été réimportées indépendamment : six synthétiques, deux natives et deux réelles, hors références et sonde facettisée. Le budget 0,001 mm a été refusé, avec aperçu conservé, indicateur retiré et export désactivé, puis 0,05 mm a été restauré. L’interface native a été observée à 1440 × 900 en français sombre pour la sphère et anglais clair pour le refus ; 1100 px n’a pas été confirmé.


## 7 novembre 2026 — stabilisation 0.10 et périmètre v1 mécanique

L’utilisateur a précisé que la v1 doit inclure les pièces mécaniques complexes, en privilégiant la fiabilité ; les formes organiques ne sont pas prioritaires. Le travail de distribution a donc été conservé comme alpha 0.10, et non déclaré v1. Un corpus FreeCAD indépendant à quatre positifs et deux négatifs permet de suivre les écarts.

Un faux succès du mode Plans a été reproduit puis corrigé : une source auto-intersectante pouvait perdre son relief lors du regroupement. Les triangles source sont désormais vérifiés dans tous les modes. Cinq régressions permanentes ont été ajoutées. Les préférences UI invalides ne sont plus remplacées silencieusement lors de leur sauvegarde ; l’historique suit les éditions validées. Les erreurs transportent le contexte du corps séparément du texte, en préservant l’ancien message CLI.

Le verrou npm hérité référençait le dossier FabWeave ; une résolution de registre propre et une installation neuve ont supprimé cette dépendance. Le paquet local Apple Silicon macOS 26 relocalise ses 27 bibliothèques, collecte les notices et vérifie signature, checksum et conversions avec accès Homebrew bloqué. CI définie sans exécution distante. 170 tests workspace, un desktop et quatre UI ont passé le contrôle de stabilisation. Voir [qualification](docs/RELEASE.md) et [validation](docs/VALIDATION.md).

## 7 novembre 2026 — candidate v1 mécanique

La demande de v1 inclut les pièces mécaniques complexes. Le corpus indépendant FreeCAD a fixé quatre positifs : bloc à perçages sur trois axes, bague à double chanfrein, carter mêlant cylindre/cône/sphère/tore et boîtier à douze congés d’arêtes et huit coins. Les quatre passent désormais jusqu’au réimport ; les négatifs ellipse et coque ouverte conservent leurs refus.

- Astra a implémenté les preuves du double chanfrein résolu, des tores entiers et des boîtes convexes arrondies à rayon uniforme. Sol a construit et contrôlé leurs contrats OCCT ; contre-revue croisée des faux positifs et domaines. Les 26 faces et 48 raccords du boîtier sont contrôlés avant export. Le raffinement géométrique du tore corrige le biais quartique f32 sans relâcher `fit`.
- Sol a optimisé le BVH de l’audit et la réutilisation exacte des milieux d’arêtes. Les distances et échantillons restent identiques ; le carter nominal de 41 290 triangles passe de 195 080 991 à 43 587 709 opérations, sous le budget inchangé.
- L’architecte a intégré les contrats app/CLI/UI et tests bout en bout. Six sorties privées ont été réimportées avec succès ; les mesures de la deuxième pièce restent exactement celles de 0.9.
- Terra a remplacé les tableaux JSON de source par un transfert binaire unique et un cache par identité. Les options ne retransmettent plus le fichier, les réponses obsolètes restent écartées. Les noms 3MF fournis restent littéraux, les noms générés se traduisent. Le transport natif, distinct des tests unitaires, est en qualification.
- `1.0.0-rc.1` : 204 tests workspace, cinq bureau, sept UI ; formatage, Clippy strict, Svelte et build réussis. Paquet autonome de 29 bibliothèques, quatre conversions isolées de Homebrew et signatures/empreinte vérifiées. Vite conserve son avertissement de taille du module JS.

Le premier historique local utilise Stéphane Muller et l’adresse communiquée par l’utilisateur ; branche `codex/v1-mechanical`, aucun distant ni publication. La candidate n’est pas encore déclarée finale : le contrôle du dialogue natif d’import/export doit être terminé. Les congés arbitraires, boîtiers percés/creux, intersections de perçages, surfaces libres, 3MF Production et autres plateformes ne sont pas annoncés.

## 7 novembre 2026 — livraison locale 1.0.0

L’utilisateur confirme l’aperçu et l’export du boîtier tourné à 26 faces depuis RC1. Son fichier exact `artifacts/v1/m6a_rounded_box_rotated.step` est réimporté dans FreeCAD 1.1.1 : un solide fermé valide, 6 plans, 12 cylindres partiels, 8 octants sphériques et 48 raccords tangents. Le volume vaut 7 637,829026775005 mm³ et l’empreinte du fichier reste inchangée. Cette preuve complète la confirmation manuelle après les difficultés de pilotage.

La finale conserve le code fonctionnel du commit candidat `030fd56` ; seules les versions et la documentation évoluent. La revue croisée confirme les sept manifestes/verrous en 1.0.0. Les 204 tests workspace, cinq bureau et sept UI passent à nouveau, ainsi que le formatage, Clippy strict, Svelte et le build. Le README core décrit désormais les APIs actuelles et distingue correctement les analyses mono/multi-corps.

Le paquet Apple Silicon/macOS 26 contient 29 bibliothèques avec leurs notices ; la signature ad hoc, les liens et les quatre conversions de contrôle passent après extraction avec lectures Homebrew bloquées. L’archive ZIP, l’archive des sources comparée au commit et le tag `v1.0.0` restent locaux. Aucune publication ni notarisation. Les périmètres différés restent explicités dans la feuille de route ; aucun historique paramétrique ni support universel des géométries mécaniques n’est annoncé.
