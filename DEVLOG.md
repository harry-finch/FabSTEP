# Journal d'ingénierie

**19 novembre 2026 — Contre-expertise Argon.** Mesure des optimisations du core et de la fidélité sur la rampe publique et Flansch, suppression des listes de sélection inutilisées par le helper en Auto monocorps, ajout de contre-exemples aux interférences OCCT. Les décisions sont dans [le registre d’intégration](docs/INT_AUDIT_01.md#contre-expertise-argon-du-19-novembre). R-113 à R-115 suivent les contours interrompus, la composition locale et la fragmentation des plans en f32.

**19 novembre 2026 — Audit NEON, R-112.** Sept lots intégrés sur `audit/neon`. Le helper gros STL produit un rapport privé de 2 261 octets sur la rampe publique et accompagne la candidate autonome. Les contrôles d’interférence avant et après transfert restent nécessaires pour Flansch. Mesures, décisions, essais de compatibilité et conditions de reprise figurent dans le [registre NEON](docs/INT_AUDIT_01.md#neon--audit-du-19-novembre-2026-r-112).

**19 novembre 2026 — Correction des six raccords et vérification du paquet final.** V-1257 et V-1258 qualifient la représentation rationnelle, le groupe d’interférences qui échouait et le rejeu du superviseur. FabSTEP convertit la rampe publique dense. Flansch atteint désormais la limite de trente minutes au contrôle complet. La suite workspace finale passe 707 tests.


**19 novembre 2026 — Verdict du helper sur Flansch.** La rampe dense publique et la pièce body 26 mm sont converties. Sur les 540 170 triangles du STL privé, le helper du paquet refuse le contrôle d’interférences OCCT après 23 minutes. Aucun résultat n’est publié. Le suivi des pièces et des contrôles pour reprendre figure dans R-085 et V-1256.


## 2026-11-18 et 19 — Domaines géométriques et bride à grand maillage

Les lots R-094 à R-110 étendent Auto aux profils de révolution cubiques avec pointe ou raccord mesuré entre deux hauteurs, aux perçages à axes inclinés et décalés, aux cavités à plusieurs perçages, au rebord torique et aux circuits de congés à une, deux ou quatre jonctions. Deux lois de rayon affines peuvent appartenir à des arêtes gauches d’un même bloc. Les témoins indépendants, mesures, rejets et commandes figurent dans V-1222 à V-1246. R-038 ajoute le relief bicubique sur demande explicite, R-050 les indications sphériques. Le paquet local inclut R-110.

Sur Flansch, la fermeture, le maillage à bord commun et la comparaison au maillage source ont passé leurs contrôles sur le témoin assemblé par étapes. La voie directe depuis le STL a son propre détecteur et un contrôle spatial de la source adapté aux 4 Gio du helper. Le résultat de cette voie doit encore être qualifié après écriture STEP. Les paramètres utilisateur et les limites du document final sont conservés.


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


## 2026-11-13 — état historique conservé lors de la mise d’équerre

Copie intégrale du précédent `STATUS.md`, dont le titre portait encore le 10 novembre mais qui incluait les lots du 12 novembre. Les résultats ci-dessous sont historiques, pas des contrôles relancés par la migration.

# État — 10 novembre 2026

**1.0.0 — v1 mécanique locale.** FabApps, Rust/OCCT, Svelte/Tauri, GPL-3.0-or-later. Cible : Apple Silicon, macOS 26 ou ultérieur. Référence Git : tag local `v1.0.0`, branche `codex/v1-mechanical`, après la candidate `030fd56`.

- STL et 3MF Core, unités et placements conservés ; 32 corps, 64 Mio et 100 000 triangles maximum. Aucun export partiel ni corps en contact.
- Plans, cylindres complets et ouverts, pièces mixtes et borgnes plans, cônes et doubles chanfreins résolus, calottes et sphères entières, tores entiers, congés toriques quart cylindre–plan et boîtiers convexes entièrement arrondis à rayon uniforme.
- Les quatre positifs et deux négatifs du [corpus indépendant](fixtures/complex_v1/README.md) passent. Six STEP issus des deux pièces utilisateur ont été réimportés dans FreeCAD. [Validation](docs/VALIDATION.md)
- 204 tests workspace, cinq bureau et sept UI réussis en 1.0.0 ; formatage, Clippy strict, Svelte et build réussis. La revue indépendante confirme l’absence de changement fonctionnel depuis RC1.
- Transfert natif STL/3MF, aperçu et export de deux cubes observés automatiquement. Aperçu et export du boîtier à 26 faces confirmés par l’utilisateur sur RC1 ; cet export a passé son réimport FreeCAD indépendant, avec 48 raccords tangents.
- Paquet autonome `artifacts/release/1.0.0/FabSTEP.app`, ZIP, notices et 29 bibliothèques, signature ad hoc. Conversions de contrôle exécutées avec lectures Homebrew bloquées. [Livraison](docs/RELEASE.md)

**Validation utilisateur du 9 novembre** — installation sur un autre Mac et utilisation des STEP dans Fusion confirmées. Modèle du Mac, versions et fichiers testés non précisés ; il s’agit d’une validation déclarée, pas d’une nouvelle mesure automatisée. Le dépôt source est public sur GitHub ; la CI macOS du 7 novembre a réussi.

**Développement post-v1 — lot A.** Combinaison cône complet + cylindre ouvert prouvés en Surfaces. 211 tests workspace, cinq bureau et sept UI réussis ; trois STEP CLI validés et réimportés OCCT. Voir [ADR-0013](docs/adr/0013-cone-open-cylinder-combinations.md) et V-1117 à V-1120. FreeCAD indépendant bloqué au démarrage Qt (`neon`) ; parcours graphique et cas 3MF dédié ouverts.

**Lot B et interface.** Zones sphériques convexes/concaves entre deux anneaux source sans pôle, avec placements et miroir 3MF ; neuf exports CLI et dix solides réimportés OCCT. Les choix UI mettent en avant conserver les facettes ou reconnaître les formes courbes, avec reconnaissances ciblées et limites repliables. 221 tests workspace, cinq bureau et sept UI réussis, avec formatage, Clippy strict, Svelte et builds. Voir [ADR-0014](docs/adr/0014-spherical-zones.md) et V-1121 à V-1124. Contrôle visuel UI (serveur local refusé), parcours natif de ces zones et réimport FreeCAD indépendant restent ouverts.

**C1a — chaînes coniques alternées.** Trois à huit cônes par chaîne, portées résolues et témoins source intérieurs, dans Primitives et Surfaces. Admission historique à deux cônes conservée ; pentes de même signe et cônes sans portée restent hors périmètre. Sept exports CLI, huit solides réimportés OCCT ; **228 tests workspace, cinq bureau et sept UI** réussis avec formatage, Clippy strict, Svelte et builds. Voir [ADR-0015](docs/adr/0015-resolved-conical-chains.md) et V-1125 à V-1127. Réimport FreeCAD indépendant et parcours graphique C1a restent ouverts.

**I1 — inspection de zones source.** Sélection au clic, surlignage, croissance/réduction par voisinage et classement des familles lorsqu’une attribution exhaustive existe. Provenance exacte STL/instances 3MF, worker UI et IPC hors fil UI sans mutation du cache STEP. **235 tests workspace, sept bureau et seize UI** réussis, avec formatage, Clippy strict, Svelte et builds. Le build local `artifacts/FabSTEP.app` a été reconstruit. Clic, croissance/réduction, Échap, distinction orbite, classement plan/cône, conservation après refus et effacement au changement de source observés dans le parcours natif. La qualification graphique reste partielle : voir V-1131 et [ADR-0016](docs/adr/0016-source-zone-inspection.md).

**I1b — sélection des faces STEP.** Vue Résultat STEP issue du fichier validé réimporté, faces entières, sélection multiple et extension par arêtes natives. Déflexion d’affichage 0,01 mm, limites inchangées, cache d’export protégé même si l’aperçu échoue. **241 tests workspace, neuf bureau et vingt-trois UI** réussis, avec formatage, Clippy strict, Svelte et builds. Chargement natif observé ; le pilotage maintient la WebView masquée et suspend le rendu, donc le clic visuel sur les nouvelles faces reste à qualifier. Voir [ADR-0017](docs/adr/0017-step-face-inspection.md), V-1132 à V-1135.

**I1c — correspondance STEP/source exacte en Plans.** La sélection de faces du résultat peut afficher ses triangles source avec **Voir la zone source**, puis retrouver la sélection native avec **Retour au STEP**. La partition est conservée à la segmentation et suivie dans la couture et les transferts STEP ; octets et provenance restent liés dans le cache. Les instances 3MF et les sélections multi-corps sont couvertes. **250 tests workspace, dix bureau et vingt-sept UI** réussissent, avec les contrôles globaux de formatage, Clippy strict, Svelte et builds. Les essais graphiques de ce nouveau parcours restent à qualifier en fenêtre visible. Voir [ADR-0018](docs/adr/0018-planar-step-source-provenance.md), V-1136 à V-1139.

**I2a — première reconstruction guidée.** **Essayer un cylindre** utilise les faces du STEP Plans pour désigner une seule paroi cylindrique complète. Le moteur prouve sa paroi et ses contours, conserve les autres zones facettisées et valide tout le document. La source sélectionnée reste visible après refus ; **Retirer l’indication** recalcule Plans. Le résultat et ses réglages/source sont liés côté natif, et l’ancien export est retiré avant calcul. **261 tests workspace, treize bureau et trente et un UI** passent, avec formatage, Clippy strict, Svelte et builds. Voir [ADR-0019](docs/adr/0019-guided-complete-cylinder.md), V-1140 à V-1143. Le nouveau parcours graphique reste à qualifier.

**I2b — indications cylindriques successives.** Le résultat guidé conserve la correspondance exacte de ses plans et cylindres complets vers la source. Jusqu’à 16 indications peuvent être ajoutées sur les facettes restantes, sur un ou plusieurs corps, puis retirées individuellement. Après refus, la session acceptée reste disponible pour recalcul mais aucun ancien STEP n’est exportable. Preuve partagée par corps, identifiants natifs stables et invalidations strictes. La suite globale passe : **267 tests workspace, quinze bureau et trente-deux UI**, avec formatage, Clippy strict, Svelte et builds. Un garde défensif supplémentaire de recalcul est contrôlé séparément en V-1148. Voir [ADR-0020](docs/adr/0020-consecutive-cylinder-guidance.md), V-1144 à V-1148. Parcours graphique restant à qualifier.

**Intégration raisonnée de l’audit — cinq jalons validés.** Le [registre INT-AUDIT-01](docs/INT_AUDIT_01.md) qualifie les 22 constats et trace huit recommandations traitées : archive/SDK, contrôle 3D des coins cylindriques, annulation coopérative, correspondance guidée mise en cache et information/documentation. La suite finale passe **297 tests workspace, 16 bureau, 36 UI, 12 Python et 4 tests SDK**, avec formatage, Clippy, Svelte et builds. Le sock-clip conserve 123 faces et 25 cylindres ; le build de développement est reconstruit. Revue indépendante favorable aux changements sensibles. Les recommandations différées et limites de validation restent explicitement suivies ; aucun commit ni push. Voir V-1161/V-1162.

**UX4 — atelier simplifié.** Vue 3D dominante, inspecteur unique, exemples retirés et détails à la demande. Le contrôleur partagé est couvert par 23 nouveaux tests ; la suite globale passe **297 tests workspace, 16 bureau, 59 UI, 12 Python et 4 SDK**, avec formatage, Clippy, Svelte et builds. Revue indépendante acceptée, défaut mineur d’espacement corrigé. Le build `artifacts/FabSTEP.app` est reconstruit. L’accueil natif FR sombre est observé ; le dialogue d’import puis le pilotage se bloquent, donc le parcours graphique complet, clair/anglais, clavier et petits écrans restent à qualifier. Voir V-1163/V-1164.

**Bascule automatique STEP.** Chaque conversion, recalcul ou correction guidée réussie déclenche le chargement du résultat STEP et son affichage après préparation. Retour manuel à Source et gardes de fraîcheur conservés. **61 tests UI**, Svelte et build local réussissent ; revue indépendante acceptée. Le sélecteur natif reste bloqué lors de l’essai graphique ; voir V-1165.

**Qualification et profilage.** Accueil EN clair observé (V-1166). L’essai de filtre par extensions seules n’a pas résolu le blocage automatisé et a été retiré ; code de production inchangé. L’utilisateur confirme ensuite que la sélection et l’ouverture manuelles fonctionnent : aucun défaut d’import produit n’est établi. Profilage opt-in ajouté : 3 répétitions sur mixte, 3MF et sock-clip, avec équivalence à la recherche réelle vérifiée sur la fixture mixte. Les validations source répétées sont prioritaires pour PERF1 ; PERF1 est désormais implémenté, avec comparaison avant/après décrite ci-dessous. Voir V-1167 et la roadmap.

**PERF1 — preuves source réutilisées.** Cache privé à chaque recherche, exact et borné, conservant reconnaissance, annulation et tous les contrôles propres aux résultats. Rapports complets et refus équivalents sur STL, 3MF et sock-clip ; médianes de recherche réelle réduites de 36,5 %, 48,9 % et 22,5 % dans le profil de développement. 304 tests workspace, 16 bureau, 61 UI, 12 Python et 4 SDK passent, avec formatage, Clippy strict et builds ; `artifacts/FabSTEP.app` reconstruit. Voir V-1168/V-1169.

**DXF1 — profil d’une face plane.** Sélection d’une face STEP plane, aperçu 2D dans son repère et export DXF 1:1 en mm. Arêtes natives de lignes/arcs/cercles et trous conservés, autres courbes refusées ; worker d’aperçu et gardes de source/résultat/sélection jusqu’à la sauvegarde. 39 profils natifs et 5 références de l’exporteur relus par ezdxf sans erreur ni réparation. 326 tests workspace, 19 bureau, 97 UI, 12 Python et 4 SDK passent, avec formatage, Clippy strict et builds ; application de développement reconstruite. Parcours graphique et Fusion restent à qualifier. Voir [ADR-0024](docs/adr/0024-planar-face-dxf.md), V-1170/V-1171.

**DXF2a — coupe du STEP.** Plan XY/XZ/YZ ou repris d’une face plane, décalage signé et choix explicite des corps ; aperçu 3D du plan, profils 2D et export DXF des lignes/arcs/cercles. Corps vides conservés, composantes disjointes et trous distingués, export global refusé en cas d’ambiguïté. Limites conservatrices des tangences/surfaces support documentées dans [ADR-0025](docs/adr/0025-step-section-dxf.md). Sept sections natives relues par ezdxf sans erreur ni réparation ; 347 tests workspace, 23 bureau, 130 UI, 12 Python et 4 SDK passent avec les contrôles globaux. Bundle de développement reconstruit. Essai graphique bloqué au sélecteur (mode Plans laissé actif, restauration impossible à confirmer) ; pas de conversion/section/export graphique validé. Voir V-1172/V-1173 ; la coupe directe du maillage source reste DXF2b.

**Suite** — élargir les familles guidées, puis ajouter l’édition et l’historique annuler/rétablir I3. La gestion de plusieurs indications et leur retrait sont disponibles ; leur persistance reste différée. I2c ajoute les troncs de cône complets entre deux contours plans source. Arcs ouverts, pointes, raccords directs courbes et arrondis ne sont pas encore des hypothèses guidées. DXF1 est désormais implémenté (voir ci-dessus) ; DXF2a ajoute désormais la coupe du STEP ; DXF2b ajoute la coupe directe du maillage source, détaillée en V-1176. La demande de choisir une famille géométrique sur un groupe de facettes pour corriger le STEP est précisée dans la roadmap pour la suite du guidage I2. UX4 est implémenté : vue 3D dominante, inspecteur unique, exemples retirés, détails techniques repliés et outils contextuels. L’import/conversion/guidage/export utilise un contrôleur testé par 23 nouveaux scénarios asynchrones. L’accueil natif FR sombre est observé ; la qualification graphique complète reste ouverte après un blocage du dialogue d’import et du pilotage. Voir V-1163/V-1164. Les autres lots géométriques, 3MF Production multi-modèles, distribution signée/notarisée et Windows/Linux restent ouverts.

**Consolidation des cylindres ouverts.** Recherche des unions d'arcs fragmentés, preuve des parois à triangulation décalée et génératrices source partagées entre deux cylindres prouvés. Une troisième pièce privée passe dans les trois modes analytiques avec une tolérance radiale explicitement choisie à 0,00002 mm : un solide, 17 surfaces cylindriques et 143 plans ; certaines zones restent facettisées. Les seuils initiaux sont inchangés. **277 tests workspace, quinze bureau et trente-deux UI** passent, avec formatage, Clippy strict, Svelte et builds. Voir [ADR-0021](docs/adr/0021-staggered-open-cylinder-walls.md), V-1149 à V-1152. Réimport FreeCAD bloqué au démarrage ; essais graphiques et Fusion de cette correction restent ouverts.

**Petits raccords contextuels.** Reconnaissance d'intervalles entiers de quatre à sept points entre deux parois cylindriques prouvées, avec détermination bornée du centre et contrôle des voisins. La pièce privée gagne huit petits raccords au même seuil explicite de 0,00002 mm : **25 cylindres et 98 plans, soit 123 faces** ; le plus petit raccord reste insuffisamment déterminé. **284 tests workspace, quinze bureau et trente-deux UI** passent avec les contrôles globaux ; `artifacts/FabSTEP.app` est reconstruit. Voir [ADR-0022](docs/adr/0022-contextual-short-arcs.md), V-1153 à V-1156. Aucun ajustement automatique des paramètres n'est ajouté ; essais graphiques, Fusion et réimport FreeCAD indépendant restent ouverts.

**Recherche assistée des réglages.** Une action explicite compare jusqu'à sept tolérances radiales avec conversion complète de chaque essai. Elle conserve le budget d'écart et les autres options, propose le moins de faces puis le seuil le plus strict à égalité, et attend **Appliquer et recalculer** avant de modifier le réglage. Depuis le seuil initial, la recherche native sur la pièce privée propose 0,000012 mm : 123 faces, dont 25 cylindres. Les résultats d'essai ne remplacent jamais le cache d'export. Voir [ADR-0023](docs/adr/0023-assisted-radial-search.md), V-1157 à V-1159. **288 tests workspace, seize bureau et trente-quatre UI** passent avec les contrôles globaux ; le build de développement est reconstruit. Parcours graphique du nouveau bouton restant à qualifier.

**Lisibilité des faces STEP.** La vue Résultat STEP affiche une palette de huit teintes par face, adaptée aux thèmes clair/sombre, et des contours contrastés dont le contrôle reste disponible dans cette vue. La préparation est effectuée dans le worker ; la sélection bleue est renforcée. Les couleurs ne codent pas la famille ou la qualité. **36 tests UI**, Svelte et build passent ; `artifacts/FabSTEP.app` est reconstruit. Vérification visuelle automatisée bloquée ; voir V-1160.

Le paquet 1.0.0 existant n’a pas été reconstruit ; les extensions A/B/C1a/I1/I1b/I1c/I2a/I2b, la consolidation des cylindres ouverts les petits raccords contextuels et la recherche assistée appartiennent au build de développement. Les validations v1 Mac/Fusion ne qualifient pas automatiquement ces extensions.


**DXF2a — plan manipulable en 3D.** Déplacer selon la normale ou tourner avec deux anneaux autour d’un pivot près de la pièce ; décalage numérique et retour XY conservés. Calcul au relâchement, export immédiatement invalidé et annulation du geste protégée. Repère personnalisé validé côté bureau, sans changement de l’intersection native. **347 tests workspace, 25 bureau, 141 UI, 12 Python et 4 SDK** passent avec les contrôles globaux ; revue indépendante acceptée après correction d’un conflit de pointeur avec l’orbite. Application de développement reconstruite, gestes natifs restant à qualifier. Voir V-1174.


**DXF2a — ellipses exactes et faux refus corrigés (12 novembre).** Le faible basculement du sock-clip était bloqué par les ellipses. Extraction native complète/partielle, aperçu analytique et DXF ELLIPSE ajoutés ; coutures périodiques prouvées et supports hors coupe mieux distingués des vraies tangences. Reconversion du STL privé avec tolérance radiale explicite 0,000012 mm : 14/14 coupes de 0° à 89° sur X/Y exportées et relues par ezdxf sans erreur ni réparation. 355 tests workspace, 25 bureau, 151 UI, 12 Python et 4 SDK passent, avec formatage, Clippy strict et builds ; un défaut de style dans un test a été corrigé avant reprise des contrôles restants. Revue indépendante acceptée, application de développement reconstruite. Parcours graphique/Fusion restant à qualifier ; certaines courbes et configurations ambiguës restent refusées. Voir V-1175 et ADR-0025.


**DXF2b — coupe du maillage source (12 novembre).** Dans Source, la coupe polygonale fonctionne indépendamment d’un STEP réussi. Préparation native de tous les corps, contrôle de séparation, unités/placements et diagnostics conservés ; même outil de plan, aperçu worker et export distinct `-mesh-section.dxf`. Aucun ajustement de courbe ou fermeture artificielle. Dix rapports d’analyse avant/après restent identiques ; 14 DXF de corpus source, dont le sock-clip, sont relus indépendamment sans erreur/réparation. 375 tests workspace, 30 bureau, 170 UI, 12 Python et 4 SDK passent avec les contrôles de formatage, Clippy strict et builds, après correction de deux remarques de style. Revue indépendante acceptée après durcissement d’un cas numérique extrême. Bundle de développement reconstruit ; parcours natif et Fusion restent à qualifier. Voir ADR-0026 et V-1176. La suite fonctionnelle revient aux familles de reconstruction guidée I2.


**I2c — choix de famille guidée (12 novembre).** L’inspecteur propose Cylindre/Cône puis Essayer. Les troncs de cône complets entre deux contours plans source sont prouvés localement ; les autres parois restent facettisées. Provenance native des cônes, indications de familles différentes sur un même corps, retrait individuel et récupération après refus vérifiés. Pointes et raccords directs courbes restent hors périmètre. Revue indépendante acceptée ; `scripts/check.sh` réussit intégralement : **391 tests workspace, 32 bureau, 175 UI, 12 Python et 4 SDK**, formatage, Clippy strict, Svelte et builds. Application de développement reconstruite ; parcours graphique et Fusion à qualifier. Voir [ADR-0027](docs/adr/0027-guided-surface-families.md) et V-1177.


## 2026-11-13 — mise d’équerre avec fab-roadmap

L’utilisateur autorise la poursuite en préservant ses modifications, valide le découpage et les distinctions de migration, choisit la bride percée (facettes vers courbes) uniquement pour la carte FabBoard, puis valide le commit des seuls documents préparés, y compris l’historique documentaire préexistant conservé. Branche dédiée : `fabboard/mise-d-equerre` ; aucun push ni fusion.

Migration : 55 éléments (19 livrés, 18 à tester, 18 idées), textes des lots conservés, 28 actions détaillées pour les essais ouverts. Archive de l’ancienne feuille de route vérifiée exactement, hors liens relatifs adaptés ; l’ancien STATUS est intégralement présent dans l’entrée précédente. Les dates de livraison v1 au 9 novembre reflètent l’acceptation globale documentée, pas les dates précises de chaque implémentation. Tableau de correspondance complet dans A-029. Le sous-titre, l’agent et les deux exécutables de bundle locaux sont reconnus par FabBoard ; aucun navigateur seul n’est proposé comme moteur CAO.

Skills locales de la famille installées, Hairline ajouté pour la figure, sans dépendance de production ni intégration de la figure dans FabSTEP. Documentation : CLAUDE créé, AGENTS relié aux skills locales et au suivi, STATUS ramené à l’état courant. README, ARCHITECTURE, rapports et ADR laissés dans leur état préexistant. Aucun code métier modifié dans cette session.

Contrôles exécutés : `sh scripts/check.sh`, code 0 — 391 tests workspace, 32 bureau, 175 UI, 12 Python et 4 SDK ; formatage workspace/bureau, Clippy strict, Svelte sans erreur/avertissement, build UI et contrôle bureau réussis. Onze tests Rust opt-in sont ignorés par la suite ordinaire, non présentés comme passés. Avertissement Vite de taille du bundle conservé. [Journal local](artifacts/mise-d-equerre/check.log).

Figure : validateur Hairline et look Chrome, code 0 ; huit vues cadrées, lecture correcte, console propre, stabilisation en 1,8 s au plus, réponse visible à 240 px (47 % de l’encre). Planches clair/sombre, miniature et mouvement regardées. Le moteur et la page d’essai n’ont pas été modifiés. FabBoard affiche le suivi et les actions ; le rendu intégré de la figure n’a pas pu être confirmé, la fenêtre devenant indisponible lors du défilement. A-031 conserve cet essai et le jugement utilisateur sur le parcours. Aucun nouvel essai natif FabSTEP ni Fusion réalisé. Les validations de migration et de mise d’équerre restent A-029/A-032 ; les tests des extensions ne sont pas clos par ces validations documentaires.


## 15 novembre 2026 — reprise UX4 sur retour utilisateur

Capture de l’inspecteur : diagnostics trop présents, section Faces STEP repliée et aucune entrée explicite au guidage depuis un résultat analytique. Inspiration Cutaway fournie ensuite par l’utilisateur : choix de famille près du modèle. R-042 repris dans son périmètre existant, sans nouveau reconnaisseur.

Barre contextuelle Cylindre/Cône/Essayer indépendante du repli de l’inspecteur ; préparation explicite par le changement de mode existant, avertissant du remplacement du résultat automatique et de la nouvelle sélection. Diagnostics de routine consultables dans Détails techniques ; alertes géométriques et codes inconnus conservés. Revue indépendante sans défaut bloquant, ajustement de rôle accessible appliqué. 181 tests UI, Svelte et builds réussis ; détails des contrôles et impossibilités visuelles en V-1178. Aucun commit ni push ; état local antérieur conservé.


## 2026-11-15 — Diagnostic de planéité sur body 26mm

Échec à l’ouverture reproduit sur le STL privé. Un réglage explicite « Conserver les facettes » avec planéité 1e-7 mm produit un STEP validé nativement : un solide, 13 397 faces planes, dimensions 90 × 50 × 11 mm. Les essais analytiques restent refusés ; aucun changement de code, assouplissement du noyau ou ajustement silencieux. Matrice, rapport et limites dans [V-1179](docs/VALIDATION.md#v-1179--diagnostic-de-body-26mmstl-15-novembre-2026).


## 2026-11-15 — I2d : fusion plane guidée

Ajout de Plan dans la barre contextuelle et la session guidée. La fusion porte sur des faces polygonales contiguës entièrement sélectionnées, conserve les coordonnées source et borne le résidu ; les contours courbes sont refusés. Parcours entièrement plan construit via l’API native planaire suivie ; coexistence Cylindre/Cône, provenance, retrait et recalcul conservés. ADR-0028 décrit les limites, dont la simplification possible d’un faible relief. Revue indépendante acceptée ; 400 tests workspace, 32 bureau, 184 UI, 12 Python et 4 SDK passent avec formatage, Clippy, Svelte et builds. V-1180 conserve les preuves et les limites graphiques.

Tore est préparé dans R-050 : le raccord actuel exige une portée cylindrique voisine reconstruite. Arbitrage produit A-034 entre sélection explicite des deux surfaces et extension proposée à confirmer ; pas de changement voisin implicite. Aucun commit/push.


## 2026-11-16 — UX5 : reconstruction automatique et parois

Parcours global approuvé par l’utilisateur et développé dans R-057 : Auto initial, stratégies dans Avancé, sélection géométrique par double-clic et correction locale sur résultat suivi. Le moteur retient les candidats locaux prouvés, conserve les zones incertaines en facettes et subdivise les regroupements plans non constructibles sans déplacer les sommets. Les limites de travail, source invalide et contrôles natifs restent bloquants. Provenance additive Rounded pour cylindres partiels, sphères et tores ; guidage Tore avec portée associée, refus des voisins coniques hors périmètre. Migration des préférences conserve unité/tolérances et ne force Auto qu’une fois.

Revue indépendante acceptée, puis 415 tests workspace, 32 bureau, 197 UI, 12 Python et 4 SDK passent avec Clippy strict, formatage, Svelte et builds. Premier essai body 26mm : 1 solide, 6 cylindres, 3 cônes, tolérances initiales ; résultats finaux et limites dans V-1181. Sock_clip absent du chemin historique. Aucun parcours visuel/Fusion revendiqué : le pilote est indisponible. A-032 valide l’organisation du projet, sans accepter implicitement les fonctionnalités à tester. Aucun commit/push.


## 2026-11-16 — Sélection manuelle par balayage

Après le retour sur le double-clic limité à une face, ajout du clic gauche maintenu pour sélectionner les faces STEP visibles sans dépendre de la reconnaissance. Déduplication par balayage, interpolation bornée, file worker avec un actif et une union en attente ; rotation déplacée au bouton droit en mode sélection. Les commandes de clic et Maj-clic sont préservées, message neutre lorsque la proposition géométrique n’étend pas la sélection. 204 tests UI, Svelte et build réussis ; revue de l’intégration par l’orchestrateur. Les tests ne remplacent pas un essai graphique natif, impossible avec le pilote actuel. V-1182 et R-057 conservent les preuves et la qualification restante. Aucun code géométrique changé, aucun commit/push.


## 16–17 novembre 2026 — Raccords de rainures et refus guidé

Le refus `guidance.family_unproven` sur une extrémité de rainure de body 26mm est reproduit. La sélection correspond bien à une bande cylindrique, mais son raccord conique partiel n’était pas représenté. Après accord explicite de l’utilisateur sur l’extension du modèle interne, R-058 ajoute une chaîne demi-circulaire cylindrique/conique, avec arcs partagés et preuve exhaustive, sans changement des tolérances. L’app conserve la provenance native et étend la correction aux bandes dépendantes. Le titre « Correction non appliquée » et le recalcul visible remplacent la présentation trompeuse d’un échec de conversion général.

La revue indépendante a conduit au contrôle de tous les contours des supports plans et au rejet avant mutation des chaînes dont les supports seraient consommés par une autre chaîne. Les groupes sont parcourus par incidences d’arcs. Contrôles ciblés core/CAD/app et UI réussis ; les résultats finaux de la suite globale et du fichier privé sont consignés dans V-1183. Aucun commit/push ni modification du paquet autonome 1.0.0. Qualification utilisateur dans A-040/A-041.


## 17 novembre 2026 — Acceptation des raccords ouverts

L’utilisateur valide le correctif R-058 et demande de poursuivre. A-040 est terminée ; A-041 reste ouverte, sans déduire un essai Fusion de cette acceptation générale. L’utilisateur choisit ensuite l’élargissement géométrique, puis demande explicitement de reprendre la simplification de l’interface. Séquence retenue : C1b (R-032), puis UX4 (R-042) ; I3 reste différé.

### 17 novembre — C1b1 puis simplification contextuelle

Sur demande de poursuivre la géométrie avant l’interface, R-032 admet deux à huit cônes de signes libres séparés par des portées prouvées ; témoins intérieurs obligatoires et compatibilité historique préservée. Corpus indépendant STL/3MF, volumes et surfaces attendus, refus et budgets couverts ; revue indépendante acceptée. Le volet sans portée reste identifié dans R-059. ADR-0031 documente ce choix sans changement de format ni de dépendance.

R-042 allège l’inspecteur, replie familles/provenance et métadonnées, met Essayer en évidence, conserve les actions et alertes. Validation finale V-1184 : 433 tests workspace, 32 bureau, 208 UI, 12 Python, 4 SDK ; formatage, Clippy strict, Svelte et build macOS passent. Premier Clippy corrigé dans un test (`div_ceil`), sans changement du moteur après la suite globale. Paquet `artifacts/FabSTEP.app` reconstruit et vérifié. Qualification graphique impossible ici, Fusion non exécuté ; actions A-042 à A-044. Modifications locales antérieures conservées, aucun commit ni push.

### 17 novembre — qualification native de l’inspecteur

Accès natif rétabli : ouverture/conversion de two_increasing.stl, sélection plane, aperçu DXF et aller-retour source/STEP observés, français sombre et anglais clair. Retrait d’une consigne STEP répétée dans l’inspecteur, avec maintien de l’aide dans le viewport. Svelte et build macOS passent ; portée et limites V-1185. Statut Git documentaire corrigé après les trois commits poussés. Aucun nouveau jalon accepté à la place de l’utilisateur.


### 17 novembre — géométries composées et pièces creuses

R-060 ajoute une rupture source unique entre deux cônes adjacents, avec témoins et marges de pente ; les lois affines historiques gardent la priorité. R-061 ajoute deux quarts de tore extérieurs autour d’une portée commune, dont le rayon est fixé une seule fois puis vérifié contre la source. Revues indépendantes acceptées, ADR-0032/0033. R-062 qualifie les arêtes locales arrondies et le bac ouvert avec fond, par corpus indépendant et sondes matière/vide ; aucun changement de production nécessaire pour ce lot.

Validation V-1186 : 459 tests workspace, 32 bureau, 208 UI, 12 Python et 4 SDK ; contrôle séquentiel complet, analyse statique et build macOS réussis. 48 fichiers des corpus inchangés après régénération. Essais utilisateur et Fusion restent distincts. L’accord structurel A-045 autorise les perçages sécants ; le laboratoire R-063 reste séparé de la conversion pendant la qualification continue des courbes réimportées.

F0 : prototype et tests élémentaires livrés, revue indépendante PARTIAL et rejeu orchestrateur conformes (sortie 2). Certificat global non obtenu : budget et périodique restent ouverts ; aucune activation dans l’application. Preuves V-1187.


### 17 novembre — lancement manuel du STEP

À la demande de l’utilisateur, R-064 sépare aperçu initial et reconstruction, avec bouton primaire dans le panneau de droite. Consentement au calcul remis à zéro à chaque source, réglages préparatoires sans conversion, recalcul ultérieur conservé. Tests de concurrence et de refus adaptés ; 215 tests UI et build passent. Parcours natif principal vérifié, second dialogue de fichiers bloqué dans le pilote ; V-1188 précise les limites.


### 17 novembre — suivi réel et publication des travaux locaux

R-065 observe les checkpoints du moteur par instantané borné et interrogation séquentielle, sans modifier les algorithmes. L’interface expose trois étapes, leur sous-opération et le corps courant ; précontrôle guidé distingué de la validation finale. 219 tests UI, 38 tests bureau et build passent (V-1189). Pilotage graphique indisponible. L’utilisateur demande explicitement de commiter et pousser tous les travaux locaux sur la branche actuelle.


### 17 novembre — diagnostic des STL trop denses

Ajout du nombre exact dans le refus STL (R-066), à la demande de l’utilisateur. Son fichier comporte 540 170 triangles, confirmé par longueur binaire et CLI ; le plafond de 100 000 reste en place. Comptage ASCII complet à stockage borné, format d’erreur conservé et message FR/EN. Tests et limites dans V-1190.


## 17 novembre 2026 — R-067, essai dense isolé et annulation réelle

Accord A-055 enregistré. Capacité source `large-stl` compilée exclusivement dans un helper distinct ; interface d’essai avec compte, avertissement, accord par fichier et Annuler. Supervision versionnée/bornée, publication après validation, arrêt/récupération et nettoyage, seuil RSS surveillé de 4 Gio et délai de 30 minutes. Aperçu dense préparé hors fil UI ; provenance guidée non transférée, fonctions source associées indisponibles pour ce nouveau mode. Revues helper/desktop acceptées et deux courses de nettoyage/progression corrigées avant validation.

Contrôles et essais réels consignés dans V-1191 : cube 101 568 triangles validé après relance, arrêts en analyse et validation source vérifiés ; fichier utilisateur 540 170 triangles admis mais refusé au budget des contours. Ce dernier constat devient l’idée R-068, sans assouplissement du contrôle. Paquet de développement reconstruit avec helper, empreintes vérifiées. Le pilotage graphique reste bloqué au dialogue de fichier ; actions de qualification utilisateur R-067 conservées.


## 17 novembre 2026 — R-068, contours denses bornés

G2 conserve les prédicats historiques et remplace la recherche exhaustive des couples d’arêtes par un index spatial déterministe au-delà de 64 segments, dans un domaine numérique démontré. Construction et parcours débitent le même budget de cinq millions ; aucune tolérance ni limite G1 n’est augmentée. Revue indépendante favorable après correction des réservations et copies ; tests différentiels, cas adverses et prisme dense ajoutés.

Le fichier privé de 540 170 triangles franchit désormais les contours, puis atteint `recognition.too_complex` : aucun STEP n’est produit. L’idée R-069 suit ce second coût, sans élargissement opportuniste de G2. Les mesures locales et la validation complète sont centralisées dans V-1192. Le benchmark de contours isolé ne constitue pas une accélération mesurée de la conversion entière.


## 17 novembre 2026 — R-069, préparation et reconnaissance denses

G3 réutilise le soudage et la topologie du mono-corps identité, et limite le scan des arêtes horizontales à celles du patch lorsque son coût borné est inférieur au scan global. Le multi-corps, les prédicats, les incidences complètes et les budgets restent inchangés. Revue indépendante favorable ; huit tests de régression ajoutés et deux benchmarks opt-in exécutés.

Sur Flansch (540 170 triangles), l’analyse historique et réutilisée est identique bit à bit, pour 99 s contre 51 s dans le benchmark dédié. L’essai complet passe de 111 à 66 s, mais reste refusé : le budget partagé est désormais épuisé dans la recherche des arcs ouverts, sans STEP produit. Cette suite devient l’idée R-070/G4 ; sa recherche non monotone ne doit pas être raccourcie sans preuve. Mesures, tests et limites : V-1193 dans VALIDATION.


## 17 novembre 2026 — R-070, cercles denses et preuves Auto

G4 mesure d’abord le budget partagé, puis corrige un contour circulaire complet pris pour des arcs ouverts après échec de l’ajustement contraint par des extrémités adjacentes. Le secours réutilise la preuve circulaire existante sur tous les points, à tolérance identique et uniquement sur cet échec ; les décisions historiques D/sparse et les fenêtres non monotones restent inchangées. Les preuves Auto réussies sont conservées avec leur géométrie immuable pour éviter leur deuxième calcul à l’assemblage.

La revue numérique et la revue indépendante de production sont favorables. Sept nouveaux tests couvrent les cercles quantifiés, les gardes, budgets, modèles/rapports/provenance et refus ; les résultats ciblés passent. Les mesures et la validation globale sont centralisées dans V-1194. Flansch franchit le premier contour mais reste refusé sans STEP à la face 7 278 ; aucun gain global de durée annoncé. R-071 conserve l’idée d’une réduction démontrée des fenêtres générales, sans hausse de budget ni développement implicite d’une nouvelle preuve numérique.

Validation globale et build macOS réussis, exécutables du paquet vérifiés. R-070 passe à tester avec A-060 ; les résultats et limites restent dans V-1194.


## 17 novembre 2026 — R-071, maximalité exacte des arcs complexes

Le profilage montre que les comparaisons de couverture consomment plus de budget que les fenêtres ajustées avant le refus G4. R-071 remplace uniquement ces comparaisons par une inclusion d’intervalles cycliques après contrôle débité de l’unicité des sommets. Fenêtres, flottants, fusion, départage et preuves géométriques inchangés ; repli historique pour les indices répétés, jamais après épuisement. Revue indépendante favorable et oracle exhaustif passant. Le benchmark synthétique compare les mêmes candidats et leurs bits ; résultats détaillés dans V-1195.

Flansch termine désormais la reconnaissance (4 cylindres, 8 cônes) mais conserve 501 900 plans. Son résultat dépasse le plafond de suivi de 100 000 faces, avant construction du STEP. Le message générique de provenance est précisé avec compte et limite, sans modifier son code ni les protections. Ce nouvel obstacle est suivi dans l’idée R-072, sans relèvement implicite du plafond. L’instrumentation privée de profilage est retirée du code livré.

Validation complète et build macOS réussis, exécutables du paquet vérifiés. Bilan détaillé et limitations dans V-1195.

Le helper embarqué confirme le diagnostic final sur Flansch ; R-071 passe à tester avec A-061. Aucun STEP produit pour ce fichier, suite R-072 conservée au stade idée.


## 2026-11-17 — R-072 : qualifier avant de relever les limites

Deux explorations en lecture seule et des sondes locales écartent une réduction suffisante par simple regroupement plan. Quatre zones courbes refusées sont identifiées ; leur famille exacte reste à qualifier. L’essai natif source isolé atteint le contrôle OCCT, puis est arrêté manuellement après observation d’une empreinte physique supérieure au RSS surveillé. Mesures, conditions et incertitudes dans V-1196. Aucun changement de production retenu ; instrumentation retirée, aucun nouveau paquet construit. R-072 passe à tester avec l’action de validation A-062. Suite proposée : qualifier la surveillance mémoire, puis les profils courbes et le coût de validation avant tout changement structurel.


## 2026-11-17 — R-073 : surveiller aussi l’empreinte physique

Après acceptation du bilan R-072, qualification de `proc_pid_rusage` contre le SDK macOS, puis remplacement de la lecture RSS seule par une lecture commune RSS/empreinte physique. Le maximum déclenche le plafond inchangé de 4 Gio. Aucun changement géométrique, aucun nouveau contrat ou dépendance. Tests natifs sur soi/enfant, seuils injectés, erreurs et nettoyage ; revue indépendante sans défaut. La sonde de 64 Mio confirme la mesure sans saturer la machine.

Contrôles globaux réussis : 496 workspace, 50 bureau, 8 helper, 68 ciblés large-stl, 1 transfert réel, 229 UI, 12 Python, 4 SDK. Paquet local reconstruit et binaires vérifiés, sans release. V-1197 détaille les preuves et la limite de l’échantillonnage. R-073 à tester via A-063 ; Flansch reste refusé en amont sur le nombre de faces.


## 2026-11-17 — R-074 : profils courbes mesurés et prototype natif

Qualification demandée des quatre grandes zones de Flansch. Une sonde core bornée exporte localement les profils et attributions ; l’analyse des sommets identifie des chaînes de lignes et arcs, sans témoins axiaux intérieurs sur les parties droites. Les résidus sont sous la tolérance mais ne remplacent pas les preuves continues. Compteurs et limites détaillés dans V-1198 ; aucun fichier privé versionné.

La revue des contrats confirme que MixedTorus suffit en représentation, mais que segmentation, orientation et raccords nécessitent une extension de preuve. Un modèle natif synthétique tore–cône–cylindre et son reflet réussissent le réimport, les familles, dimensions, volume indépendant et provenance. Le test est conservé ; 85 tests CAO, formatage et Clippy passent, rejeu ciblé de l’orchestrateur réussi. Aucun changement de production ni nouveau build. R-074 à valider via A-064 ; R-075 décrit la première tranche de reconnaissance. La conversion entière reste bloquée indépendamment par les 231 584 plans hors des quatre zones et le coût de validation source.

## 2026-11-17 — R-075 : reconnaissance tore–cône–cylindre

Portée approuvée via A-064. Détecteur borné après les chemins historiques, mesures source restaurées après un essai refusé, calcul conjoint de tangence et preuve continue sur les bandes à deux anneaux. Orientation des anneaux adaptée à la branche du tore et aux reflets ; contrat et ABI inchangés. Corpus synthétique indépendant et revue distincte, puis vérification de l’intégration par l’orchestrateur. Les contrôles natifs couvrent aussi une cavité traversante ; la rotation oblique f32 non coplanaire reste explicitement refusée.

Le premier essai Flansch épuisait le budget partagé après reconnaissance du raccord. Retrait des auto-visites inutiles dans le parcours des composantes, avec équivalence des membres testée et budget inchangé. Le helper propre termine la reconnaissance en 66,561 s : 439 738 faces contre 501 912, puis refus au plafond de 100 000. La sonde d’attribution confirme exactement les 64 080 triangles vers trois surfaces, sans débordement ; aucun STEP complet ni validation native source de Flansch. Mesures, bornes et limites dans V-1199 ; ADR-0035 conserve la décision.

Suite globale réussie : 510 workspace, 50 bureau, 8 helper, 74 ciblés large-stl, un transfert réel, 229 UI, 12 Python, quatre SDK ; formatage, Clippy strict, Svelte et builds passent. Paquet local reconstruit et deux binaires vérifiés. R-075 à tester via A-065/A-066, sans nouvel essai graphique/Fusion ni modification de la release. Les chaînes à deux cônes et arcs de 100° restent des suites distinctes.


## 2026-11-17 — R-063 : preuves natives des perçages sécants

Poursuite de l’ensemble C1b2/C2/D/E/F/G autorisée. Le laboratoire F0 franchit ses blocages de bornes continues et de périodicité par Hermite d’ordre quatre, déroulement exact des données spline et couverture orientée par intervalles. Les budgets et seuils restent inchangés ; volume idéal encadré par série à reste majoré. Revue distincte puis rejeu de l’orchestrateur PASS, rapport identique. V-1200 conserve les chiffres et les limites : aucun STL reconnu ni contrat de production intégré, volume adaptatif OCCT seulement mesuré. R-063 à tester pour son bilan ; les travaux C2 et E2 continuent indépendamment.


## 2026-11-17 — R-079 : domaines locaux D/G approuvés

Laboratoire indépendant : un coin triédrique arrondi et deux congés à rayon linéaire se réimportent comme solides, avec types, volumes, dimensions, limites partagées et sondes matière/vide contrôlés. Revue mathématique distincte et rejeu natif identique réussis (V-1202). Les références sont fixes et ne reconnaissent aucun fichier STL. L’utilisateur approuve explicitement les domaines additifs dans ADR-0036 ; A-068 fait et R-079 livré pour cette décision, R-081/R-082 spécifiés pour la production. La suite globale C1/C2/E continue séparément, après correction du repli Auto des chaînes hors domaine de huit cônes.


## 2026-11-17 — R-076/R-077/R-078 : chaînes composées et bac percé

Reconnaissances C1/C2 intégrées sans nouveau modèle natif ; corpus indépendants, frontières source et preuves continues. Le bac percé qualifie les capacités mixtes existantes. La revue supprime un raccourci de pente flottant non sûr ; la suite globale découvre puis vérifie la correction du repli Auto des profils hors domaine de huit cônes. Index exact des anneaux et comptage du travail sphérique réellement visité, budgets inchangés. Les quatre grandes zones de Flansch sont attribuées à douze surfaces ; 231 608 faces globales empêchent toujours l’export. V-1201 conserve les mesures et les limites.

530 tests workspace et tous les contrôles bureau/worker/UI/statique passent. Paquet local reconstruit, deux binaires vérifiés ; essais graphiques/Fusion ouverts dans A-069 à A-074. Les contrats D/G approuvés restent dans ADR-0036 ; la préparation de D2 est isolée pendant cette validation et ne modifie pas le build qualifié.


## 2026-11-18 — R-081 : coin local à trois arrondis

Contrat approuvé A-068 intégré de façon additive : octant sphérique, trois cylindres et six plans à partir des frontières source. Preuves de disques orientés et bornes continues, volume par flux ; arêtes/pcurves réellement partagées en CAO, contrôles spécialisés au réimport simple et multi-corps. Corpus indépendant transformé, repli intérieur fermé refusé, paramètres malformés et contact de corps refusés. Attribution UI adaptée au nouveau propriétaire, libellés FR/EN existants conservés. Revue indépendante core et CAO puis vérification de l’orchestrateur.

Le premier test négatif attendait à tort un refus Auto sous tolérance stricte ; il vérifie désormais le solide facettisé validé et sa fidélité, tandis que Surfaces conserve son refus. Aucun garde géométrique assoupli. 540 tests workspace, 50 bureau, huit worker, 93 ciblés large-stl, un transfert réel, 230 UI, douze Python et quatre SDK passent ; formatage, Clippy, Svelte et builds passent. Paquet local reconstruit et deux binaires vérifiés, essais graphiques/Fusion ouverts A-075/A-076. V-1203 conserve les preuves, empreintes et limites. STATUS est recentré sur l’état courant ; cet historique reste ici et dans VALIDATION. Les brouillons G restent isolés pendant la qualification D.


## 2026-11-18 — G1 : enveloppe conique à rayon linéaire (R-082)

Second domaine d’ADR-0036/A-068 intégré après D2 : six plans et une portion conique inclinée, deux ellipses partagées et deux génératrices. Le core retrouve les supports et prouve la couverture et l’écart ; quatre extrémités projetées sont comptées sans modifier l’analyse source. Modèle et provenance additifs, anciens JSON conservés, volume indépendant, inspection et profils DXF raccordés. L’allocation séparée du nouveau champ évite de grossir l’enum des reconnaissances.

Trois périmètres disjoints (core, CAO, tests multi-corps) et revues croisées, intégration/validation par l’orchestrateur. La revue a complété le budget des copies finales et évité un parcours inutile. La suite globale a révélé une interception des anciens arcs cylindriques constants par G : ajout d’une preuve préalable de variation sur les deux tangences, régression à quatre variantes, puis rejeu complet réussi. La CAO conserve les ellipses 3D ; son interpolation paramétrique bornée est contrôlée numériquement, sans la présenter comme un certificat continu. Détails et limites dans V-1204.

Validation finale : 553 tests workspace, 50 bureau, huit worker, 101 large-stl, un transfert réel, 231 UI, douze Python et quatre SDK. Format, Clippy, Svelte et builds passent. Application de développement reconstruite, exécutables vérifiés par empreintes. R-082 à tester via A-077/A-078 ; aucun essai Fusion ou graphique revendiqué, aucun nouveau résultat Flansch. La prochaine tranche F a été précisée dans R-036 sous l’accord A-045 existant ; elle n’est pas encore activée en production.


## 2026-11-18 — F1, perçages sécants et préparation de la reprise

R-083 met en œuvre l’accord A-045 : reconnaissance de deux perçages traversants centrés orthogonaux, partition en neuf faces, domaines scalaires additifs, construction native et certificat des intersections réellement réimportées. Le corpus indépendant couvre placements, miroir, échelle, retessellation et refus. ADR-0038 décrit le contrat ; V-1205 porte les résultats de validation et leurs limites. Les branches restent un cas certifié spécifique, sans autorisation générale des splines ni hausse des plafonds.

Les revues croisées ont fait corriger la priorité sur quatre trous borgnes à pointes coniques, ajouter leur corpus de non-régression, aligner les limites de témoins core/CAO et plafonner le résidu public également par le petit rayon. La réserve native composée est comptée dans la borne source. L’interface conserve le libellé Cylindre et sait attribuer les trois propriétaires supplémentaires ; aucun nouveau panneau ou choix n’est ajouté.

Les parents C2/D/E/F/G affichaient à tort « idée » malgré leurs tranches développées : ils passent « en cours », avec état consolidé et périmètres restants. Les sous-jalons ne deviennent pas livrés sans acceptation utilisateur. Les réordonnancements effectués par FabBoard pendant la session sont conservés sans réécriture d’historique.

À la demande de l’utilisateur, aucun nouveau chantier n’est lancé après F1 : préparation d’une reprise autonome dans STATUS et la roadmap, puis commit et push du travail validé sur la branche existante. R-080 reste une piste de diagnostic pour Flansch ; aucun nouvel essai ni conversion de ce fichier n’est revendiqué. La release 1.0.0 demeure inchangée.

Clôture validée : suite complète code 0 (565 workspace, 50 bureau, huit worker, 107 large-stl, un transfert réel, 232 UI, douze Python, quatre SDK), deux tests CAO rejoués après le dernier garde et formatage réussi. Paquet de développement reconstruit ; les deux exécutables sont vérifiés par SHA-256, valeurs dans V-1205. Aucun essai graphique/Fusion nouveau ; les actions F1 restent à faire.


## 2026-11-18 — Reprise autorisée et C2b

Après clôture et push F1, l’utilisateur demande de poursuivre puis approuve le nouveau type de surface pour terminer le jalon pendant son absence. R-084/ADR-0039 bornent la portée : une révolution complète à profil cubique positif, deux disques source, aucune spline multisegment. Un laboratoire natif préalable valide cinq références ; le corpus STL indépendant comprend sept positifs et six négatifs.

La reconnaissance conserve la priorité historique et certifie les cartes de la paroi et des disques. La revue fait séparer tolérance de planéité et tolérance radiale, refuser les profils circulaires non distinguables et composer une réserve native de 1e−6 mm. Le résultat expose une famille Révolution distincte, une provenance complète et un avertissement d’ajustement. Les anciens documents restent lisibles.

L’audit a effectivement bloqué la variante miroir avec le maillage OCCT générique : sommets corrects mais triangles trop éloignés. Une grille paramétrique évaluée sur les faces natives réimportées corrige ce défaut, sous le même plafond de ressources. Les contrôles de séparation restent inchangés, notamment le refus par budget lorsque des sources denses se touchent. Preuves, mesures et limites dans V-1206. Aucun nouveau chantier au-delà de ce jalon n’est lancé à la clôture.


Clôture C2b : suite complète réussie (579 workspace, 50 bureau, huit worker, 115 large-stl, un transfert réel, 234 UI, douze Python et quatre SDK), formatage et analyses statiques sans échec. Application locale reconstruite, deux exécutables comparés par SHA-256 (V-1206). R-084 passe à tester avec A-081/A-082 ; le parent C2 conserve les domaines restants et le statut en cours. Documentation de reprise actualisée, puis commit et push sur la branche existante ; pas de publication release ni de nouvel essai Flansch.

## 2026-11-18 — R-080, localisation des faces restantes de Flansch

Le build V-1206 reproduit le refus à 231 608 faces. Le diagnostic du graphe et des trois bandes de la bride identifie onze supports toriques candidats sur les deux bandes les plus prometteuses, soit 150 441 faces planes. Une troisième bande présente 1 230 écarts excessifs au barycentre malgré la précision aux sommets. Les deux sondes hors production sont versionnées et rejouées. V-1207 porte les mesures, les limites et les commandes. R-085 porte la qualification des limites des secteurs et de leur reconstruction.

## 2026-11-18 — R-085, secteurs toriques et ceinture de Flansch

L’utilisateur approuve l’extension du contrat aux secteurs à quatre arcs partagés. La première configuration fermée entre dans l’application avec sept faces. La qualification sur Flansch retrouve trois propriétaires d’un triangle qui manquaient à deux cartes et vérifie les onze secteurs des deux bandes sans chevauchement de propriétaires. Les tangentes communes permettent de reconstruire la ceinture inférieure réelle de douze faces dans le laboratoire natif. La fermeture du corps dépend des autres contours, détaillés dans V-1208. Le helper de développement refuse encore l’export. La première tranche est à essayer via A-084 et A-085 et R-085 reste en cours.

La suite complète V-1208 passe. Les deux binaires du paquet de développement sont vérifiés par SHA-256. Le rejeu indépendant reproduit les paramètres de la ceinture et les contrôles natifs après STEP. Les réordonnancements FabBoard intervenus pendant la session sont conservés.

## 2026-11-18 — R-085, raccordement du dessous et des flancs de Flansch

La qualification passe d’une ceinture ouverte de douze faces à une coque de 34 faces réimportée avec les cinq entrées de perçage et les dix contours du marquage latéral. Le contrôle sur l’attribution Auto réelle des 540 170 triangles donne 79 879 faces après substitution hypothétique. Le relevé des interfaces distingue les sorties de cône, le marquage et le contour principal. La branche applicative de reconnaissance utilise toujours le résultat à 231 608 faces sur cette pièce. V-1209 contient le rejeu et le travail de fermeture restant.

## 2026-11-18 — R-085, raccordement des perçages et des marquages de Flansch

Le laboratoire relie au pourtour les trois flancs cylindriques longs, les cinq chaînes de perçage, le col central, deux épaulements, les dix marquages et les deux plans d’extrémité des rails. Sur le bord des cylindres longs, 267 triangles voisins ont été reconstruits et cousus en contrôlant leurs 801 incidences. Après transfert STEP, la coque de **2 083 faces** garde deux boucles libres. Ses **460 044 triangles source** ont chacun une attribution. Les **80 126 triangles** encore à raccorder forment trois nappes et trois triangles isolés. Le comptage sur le corps entier donne **79 479 faces** si ses 77 396 propriétaires restants sont conservés.

La revue a reproduit deux faux positifs dans les premières sondes des plans supérieurs. Une correspondance de rail tronquée laissait passer un arc incomplet et un triangle intérieur en double conservait un contour extérieur valide. Des contrôles de couverture des rails et d’incidence orientée des triangles rejettent désormais ces deux cas. Le helper de l’application refuse toujours le document à 231 608 faces. Mesures, périmètre et commande de reprise dans [V-1210](docs/VALIDATION.md#v-1210--r-085-perçages-épaulements-et-marquages-de-flansch).

## 2026-11-18 — R-086, congé de fond de poche entre deux parois

La poche nominale était déjà reconnue. Avec 16 segments à une extrémité et 23 à l’autre, Auto conservait 50 faces planes. La preuve de couverture des triangles en coordonnées cylindriques permet de rétablir un seul congé, avec ses deux arrêts. Le même changement s’applique à une arête extérieure. Le corpus public, la poche indépendante de FreeCAD, le contrôle sur Flansch et la limite rencontrée sur une rotation en STL binaire sont détaillés dans V-1211.

## 2026-11-18 — R-087, qualification d’un congé arrêté en biais

Les STL témoins montrent deux difficultés distinctes : l’arrêt elliptique du cylindre et la fragmentation de son plan sous l’effet de l’encodage binaire. La référence CAO indépendante confirme les quatre bords partagés et le volume. Les profils de ses deux butées s’exportent en DXF exact. La proposition de représentation et les conditions de passage dans Auto sont consignées dans ADR-0041, les essais reproductibles dans V-1212.

## 2026-11-18 — R-087, reconnaissance de la butée oblique dans Auto

Après accord A-087, la reconnaissance Rust et la construction OCCT utilisent le même cercle, la même ellipse et les mêmes génératrices que les quatre voisins du congé. L’essai porte sur les cinq STL publics, une seconde poche maillée par FreeCAD, les profils DXF natifs et un document à deux corps. La comparaison avec le résultat facettisé de V-1212 et les limites mesurées sont consignées dans V-1213.

## 2026-11-18 — R-088, deux congés parallèles au fond d’une poche

Le parcours existant d’Auto et de Surfaces retrouve les deux bandes cylindriques même lorsque les rayons et les nombres de triangles diffèrent. Les essais de régression couvrent l’identité des faces sélectionnables, les arcs de la butée en DXF et la conservation du côté intact sur une pièce déformée. Une seconde poche dessinée dans FreeCAD vérifie la géométrie et la matière sur deux maillages. Voir V-1214.

## 2026-11-18 — R-089, référence du coin à deux congés adjacents

Sur deux maillages indépendants d’une même poche, Auto donne 71 et 159 plans. La pièce CAO a onze plans et deux cylindres reliés par un quart d’ellipse. Les mesures initiales et la commande de contrôle de cette arête sur les deux faces figurent dans V-1215. L’extension correspondante du contrat core/CAO est acceptée dans A-089 et décrite dans ADR-0042.

## 2026-11-18 — R-089, reconstruction du coin dans Auto et Surfaces

Les deux cylindres partagent désormais une arête elliptique après réimport. Sur deux STL dessinés dans FreeCAD, les 71 et 159 faces planes de V-1215 deviennent onze plans et deux cylindres. La revue indépendante a identifié un refus excessif sur les poches courtes. Le contrôle porte désormais sur le quart de cylindre que contient la pièce. Un essai de régression à 3,75 mm de longueur pour un rayon de 2 mm protège ce cas. Voir V-1216.

## 2026-11-18 — R-090, qualification des congés adjacents de rayons différents

Deux maillages FreeCAD donnent encore 69 et 155 plans. La pièce de référence porte une arête non plane entre les deux cylindres et un arc circulaire supplémentaire sur la paroi latérale. Le laboratoire construit une quartique rationnelle pour la jonction et borne l’interpolation des tracés sur les deux supports. Les témoins, le repli actuel et les contrôles de transfert sont décrits dans V-1217. A-092 porte sur l’extension du contrat.

## 2026-11-18 — R-090 intégré après l’accord A-092

Le parcours STL → STEP reconnaît les deux congés de rayons différents au coin d’une poche rectangulaire. La couverture des bandes et des onze plans est vérifiée avant construction. Une représentation paramétrique dédiée fixe la courbe commune et ses deux tracés. La CAO contrôle les cinq pôles de la quartique et les deux domaines après réimport. La sélection Auto conserve la provenance de chaque face et le DXF de la paroi comporte un arc circulaire.

Un essai supplémentaire sur un STL incliné et réécrit en f32 a révélé une fragmentation des plans. Une seconde recherche bornée et l’ajustement des supports sur leurs triangles ont permis de reconstruire la pièce aux tolérances initiales. Le fond volontairement déformé est couvert par un test de refus. Résultats et reproduction dans V-1218, vérification manuelle dans A-093.

## 2026-11-18 — R-091, perçages sécants de rayons plus proches

Le témoin historique de rayons 3 et 5,5 mm passe de 306 plans à six plans et trois faces cylindriques. La vérification du rapport de rayons tient compte des deux incertitudes. Une seconde pièce construite par soustractions dans FreeCAD a révélé un cas fréquent de maillage où tous les sommets d’une paroi se trouvent sur ses bords. Les arêtes internes permettent de retrouver la paroi lorsque deux témoins suffisamment espacés confirment son rayon. Quatre maillages de cette pièce et trois témoins à rapports différents ont été convertis et relus dans FreeCAD. Voir V-1219.

## 2026-11-18 — R-092, perçages dans un bloc incliné avant maillage binaire

La normale d’un triangle mince pouvait diviser la face plane autour d’un trou. Les deux moitiés formaient alors un contour que le moteur refusait. L’ajustement sur toute la face et la mesure des directions entre les centres des bouches permettent de reconstruire le solide aux réglages initiaux. La revue a identifié un contre-exemple important pour le mode Plans, une plaque dont les deux pans peuvent chacun être construits exactement. La partition initiale garde la priorité, et cette plaque conserve ses onze faces. Les résultats des deux poses, des deux densités de maillage et du test de réimport sont consignés dans V-1220.

## 2026-11-18 — R-093, cylindre d’une plaque inclinée en Auto

Sur le témoin de V-1220, les deux ouvertures du trou étaient présentes. L’erreur d’arrondi du STL binaire suffisait à rendre les coins des faces voisines non coplanaires aux exigences du noyau. Auto gardait alors 252 plans. La nouvelle preuve utilise conjointement les deux ouvertures et les quatre arêtes latérales de la plaque, puis vérifie et comptabilise les déplacements par rapport aux triangles. Deux poses et deux densités donnent chacune sept faces en réimport. Le maillage aux génératrices vrillées conserve ses facettes. Le protocole de reproduction et les limites figurent dans V-1221.

## 2026-11-18 — Nettoyage local approuvé A+B

Sur la branche `fabboard/nettoyage`, issue de `fabboard/mise-d-equerre` à `82342b40`, inventaire avec `fabboard status .`, `fabboard clean scan --json` et inspection des répertoires Cargo supplémentaires sous `artifacts/`. Les deux essais à blanc ont réussi. L’utilisateur a approuvé les catégories A et B par « A+B » ; la liste exacte et son accord sont conservés dans [A-097](roadmap/actions/A-097-choisir-ce-qui-se-nettoie-dans-fabstep.md).

`fabboard clean trash` a déplacé les 31 chemins reconstructibles et les quatre fichiers `.DS_Store`, sans refus. Volume retiré du projet : 43 784 871 936 octets, environ 40,8 Gio. Le scan passe de 46 526 246 912 à 2 741 391 360 octets, soit 43,3 à 2,6 Gio ; le scan final ne signale plus de cache récupérable. Les fichiers restent dans la Corbeille : leur déplacement ne libère pas cet espace sur le disque tant qu’ils y sont conservés.

Les deux applications à tester, le helper, les fichiers CAO, corpus, rapports, journaux, illustrations, brainstorming, licences, roadmap et skills sont conservés. Les journaux `clippy.log` et `oblique.log` de `artifacts/r089-core-target/` sont toujours présents. Les 35 chemins approuvés sont absents, aucun fichier suivi n’est supprimé, et les empreintes SHA-256 des deux exécutables FabSTEP et du helper sont identiques avant/après. `cargo fmt --all -- --check` et `git diff --check` ont réussi. Les modifications préexistantes du manifeste et de l’installation de la skill `fab-cleanup` restent hors du commit de nettoyage.

Tests, Clippy, vérifications UI et builds après nettoyage non exécutés : la skill demande de consulter l’utilisateur avant cette recompilation complète. [A-098](roadmap/actions/A-098-choisir-quand-recompiler-fabstep-apres-nettoyage.md) conserve ce choix, le coût et les commandes de reprise (`npm --prefix ui ci`, `scripts/check.sh`, puis `scripts/build-macos-dev.sh` pour le paquet de développement). Aucun merge ni push.

L’utilisateur a ensuite choisi de reporter explicitement les tests et le build au prochain développement. A-098 est close avec cette réponse ; aucun succès de tests après nettoyage n’est revendiqué.

**19 novembre 2026 — R-111 et helper pour les surfaces toriques.** Quatre solides à retour axial sont qualifiés dans V-1248. L’extension au mode Auto pour gros STL, les essais indépendants et les mesures de coût figurent dans V-1249 à V-1251. La rampe publique dense passe la conversion complète. L’essai du fichier Flansch suit dans R-085.


## 2026-11-19 — Second nettoyage local A+B

Sur `fabboard/nettoyage-2026-11-19`, issue de `main` à `5f901e88`, l’inventaire est parti d’un dépôt propre. Les sorties Cargo supplémentaires sous `artifacts/` ont été identifiées par leurs marqueurs et inspectées ; l’essai à blanc des 36 chemins a réussi. L’utilisateur a ensuite confirmé la liste [A-122](roadmap/actions/A-122-choisir-le-second-nettoyage-de-fabstep.md) par « oui vas y ».

`fabboard clean trash` a déplacé les 35 chemins reconstructibles et `.DS_Store`, sans refus, soit 18 156 703 744 octets (16,9 Gio). Le projet passe de 23 940 857 856 à 5784166400 octets, soit 22,3 à 5,4 Gio. Les éléments restent récupérables dans la Corbeille, où ils occupent encore le disque. Les caches du formateur Python sont conservés comme prévu (6914048 octets détectés par le scan).

Les 36 chemins approuvés sont absents et aucun fichier suivi n’est supprimé. Les empreintes de contenu SHA-256 des deux applications FabSTEP et du dossier entier de la candidate NEON sont identiques avant/après. Les corpus, CAO, rapports, journaux, sources de laboratoire, illustrations et historiques restent en place. `cargo fmt --all -- --check` et `git diff --check` ont réussi ; aucun test, Clippy, contrôle UI ou build après nettoyage n’est revendiqué. Le report au prochain développement conserve la préférence exprimée dans A-098 et reprise dans la proposition A-122 approuvée. Réinstaller les dépendances UI via `npm --prefix ui ci` avant `scripts/check.sh` ; conserver la compilation du worker séparée du bureau. Aucun merge ni push.

## 2026-11-19 — R-085, raccords entre bords toriques décalés

Le constructeur natif peut remplacer le sommet pincé d’un raccord peu profond par un domaine triangulaire régulier. Le petit secteur public reproduit le refus d’origine. Sa variante dense passe dans le helper de FabSTEP, avec deux tores et 962 identités de faces dans la qualification native. Le générateur et les deux commandes de qualification sont consignés en V-1261. Le périmètre privé porte sur les 43 tores de Flansch et les faces voisines. La vérification des 7 385 couples autour des nouveaux raccords est réussie.
