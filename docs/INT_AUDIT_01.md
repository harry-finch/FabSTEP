# INT-AUDIT-01 — intégration raisonnée

Date : 10 novembre 2026. Source évaluée : [AUDIT_TECHNIQUE.md](AUDIT_TECHNIQUE.md), conservé sans réécriture. Ce registre est la source de vérité de l’intégration ; les affirmations de l’audit ne sont pas des preuves acquises.

## Qualification et priorités

Qualification initiale : 22 constats, dont 2 ACCEPTED, 6 ADAPTED, 11 DEFERRED, 1 REJECTED, 1 ALREADY_FIXED et 1 NEEDS_EVIDENCE. Après reprise UX4 et intégration PERF1 : 2 ACCEPTED, 10 ADAPTED, 7 DEFERRED, 1 REJECTED, 1 ALREADY_FIXED et 1 NEEDS_EVIDENCE. Aucun P0 établi. Pas de décision structurelle requise pour les travaux retenus : pas de changement de stack, dépendance structurante, format de données ou suppression de fonction. Les API ordinaires et la priorité des refus restent conservées.

| ID audit | Décision | Priorité | Jalon INT-AUDIT- | Statut | Justification / validation |
| --- | --- | --- | --- | --- | --- |
| AUD-001 | ADAPTED | P1 | 01.3 | DONE | Verrou natif par appel, non par conversion ; ajouter des points d’annulation sans interrompre OCCT ni supprimer les contrôles de publication. PASS : contrôles ciblés et suite globale finale ; V-1161/V-1162, journaux ci-dessous. |
| AUD-002 | ALREADY_FIXED | P2 | — | CLOSED | Viewport sépare reconstruction, contours et cadrage ; shouldFit dépend de la source et du cadrage explicite. Lecture ciblée, aucun correctif revendiqué. |
| AUD-003 | ADAPTED | P1 | 01.1 | DONE | Le script filtre déjà les suffixes et liens ; un fichier non suivi autorisé reste inclus. Sélection Git et refus des instantanés sales/incomplets. PASS : contrôles ciblés et suite globale finale ; V-1161/V-1162, journaux ci-dessous. |
| AUD-004 | DEFERRED | P2 | — | DEFERRED | Répétitions réelles, mais ressoudage depuis les coordonnées originales et segmentation max(plane,fit) non interchangeables. V-1167 : Analysis complète pèse 6,39–10,65 % sur les cas mesurés ; coût de segmentation seul non isolé. Pas de partage aveugle ni cache public Analysis ; validation source répétée traitée dans PERF1/AUD-018 ; segmentation non modifiée ici. |
| AUD-005 | DEFERRED | P2 | — | DEFERRED | I/O temporaire déjà éliminée ; lectures CPU restantes réelles. V-1167 : AuditTessellation complète pèse 0,93–1,44 % ; lecture seule non isolée. Fusion FFI différée, bénéfice non établi face au risque ; réimport obligatoire conservé. |
| AUD-006 | DEFERRED | P3 | — | DEFERRED | Cache d’instances non justifié : affines, arrondis des placements, soudage et provenance imposent une clé et un contrat plus forts que ceux proposés. Lecture ciblée, aucun correctif revendiqué. |
| AUD-007 | ADAPTED | P2 | UX4 | DONE | Contrôleur de production extrait et testé par 23 scénarios asynchrones, sans nouvelle dépendance ; tests des workers conservés. Pas de tests DOM montés revendiqués ; qualification graphique partielle suivie en V-1163/V-1164. |
| AUD-008 | ADAPTED | P2 | 01.1 | DONE | CI déjà suivie dans .github/workflows/check.yml ; pas de duplication. Contrôler la famille OCCT 7.9 documentée sans imposer un correctif exact. PASS : contrôles ciblés et suite globale finale ; V-1161/V-1162, journaux ci-dessous. |
| AUD-009 | ADAPTED | P3 | R-022 | DONE | Démarrage en WebView mesuré (V-1270). Le chargement à la demande du viewport réduit le délai médian de montage des commandes de 71 à 34 ms. |
| AUD-010 | REJECTED | P3 | — | CLOSED | Un chemin via parent symbolique désignant une entrée existante existe aussi et est canonicalisé. Aucun cas de corruption reproduit ; la correction proposée ne répond pas au scénario décrit. Lecture ciblée, aucun correctif revendiqué. |
| AUD-011 | ADAPTED | P3 | R-024 | DONE | Troncature par graphème, cube 3MF relu par OCCT. La bibliothèque Unicode déjà embarquée dans le bureau est partagée avec le service de conversion. |
| AUD-012 | ADAPTED | P3 | UX4 | DONE | Douze clés mortes retirées avec la présentation ; clés des composants partagés conservées (AppBar, StatusBar, Alerts). Contrairement au rapport, guidance.complete_cylinder_verified est réellement émis par mixed.rs : alias conservé. Parité FR/EN et Svelte contrôlés ; V-1163. |
| AUD-013 | ADAPTED | P2 | UX4 | DONE | Template/CSS App rendus lisibles ; import/conversion/guidage/export extraits dans workflow.ts et exercés par les tests réels. Workers, historique et recherche restent locaux pour limiter le risque ; pas de refonte globale. V-1163. |
| AUD-014 | ADAPTED | P3 | R-023 | DONE | Permission applicative effective limitée aux 30 commandes recensées de la fenêtre locale. Résolution des autorisations testée et exports natifs vérifiés. |
| AUD-015 | ADAPTED | P3 | 01.5 | DONE | Ajouter des repères et liens vers la roadmap/ADR existants plutôt que dupliquer un second tableau historique complet. PASS : contrôles ciblés et suite globale finale ; V-1161/V-1162, journaux ci-dessous. |
| AUD-016 | ADAPTED | P1 | 01.2 | DONE | Vérifier les quatre coins analytiques dans le repère réellement construit en CAD. Conserver 1e-7 mm : la marge arbitraire 0,9 rejetterait aussi des cas acceptables. PASS : contrôles ciblés et suite globale finale ; V-1161/V-1162, journaux ci-dessous. |
| AUD-017 | ADAPTED | R-070 / R-026 | DONE | V-1194 reproduit le défaut sur un anneau dense de 362 sommets. Le secours vérifie tous les sommets et le tour complet après échec du garde initial. Les tests conservent le profil en D et les anneaux clairsemés. |
| AUD-018 | ADAPTED | P2 | PERF1 | DONE | Cache privé, borné et paresseux des seules preuves source réussies sur géométrie exacte ; analyse/segmentation et contrôles des résultats conservés. PASS : rapports complets, refus et traces équivalents sur sept candidats STL/3MF/sock-clip ; compteurs natifs réduits ; recherche réelle −36,5 %, −48,9 %, −22,5 % en développement. Suite globale 304 workspace / 16 bureau / 61 UI ; V-1168/V-1169. |
| AUD-019 | ACCEPTED | P2 | 01.4 | DONE | Cache privé de la seule correspondance vérifiée dans DocumentStep immuable ; aucun objet OCCT ni maillage conservé ; mesure avant/après et test explicite du cache. PASS : contrôles ciblés et suite globale finale ; V-1161/V-1162, journaux ci-dessous. |
| AUD-020 | ACCEPTED | P2 | 01.5 | DONE | Choisir l’avertissement contextuel proposé, sans rejouer les indications avec des options nouvelles ni modifier le contrat d’invalidation. PASS : contrôles ciblés et suite globale finale ; V-1161/V-1162, journaux ci-dessous. |
| AUD-021 | ADAPTED | P3 | R-025 | DONE | À l’occasion de R-049 et R-024, extraction de l’historique de session et de la normalisation des noms avec tests ciblés. |
| AUD-022 | ADAPTED | P2 | 01.5 | DONE | README et architecture décrivent déjà zones, chaînes, arcs courts et recherche. Compléter seulement cartographie des fichiers/commandes et aide CLI succincte. PASS : contrôles ciblés et suite globale finale ; V-1161/V-1162, journaux ci-dessous. |

## Jalons retenus

### INT-AUDIT-01.1 — archive source et SDK (P1/P2, complexité S)

- Objectif / constats : AUD-003, AUD-008 ; maîtriser les fichiers distribués et refuser un SDK hors de la famille documentée.
- Périmètre : `scripts/package-source.py`, tests Python isolés, `scripts/check.sh`, contrôle SDK dans `crates/fabstep-cad/build.rs` ou en-tête dédié ; documentation de construction/livraison.
- Modifications : énumération des sources suivies Git, refus explicite d’un arbre sale ou incomplet avant écriture ; conservation des protections de liens et d’écrasement ; contrôle majeure/mineure 7.9, CI existante conservée.
- Dépendances : aucune. Risques : archive de développement désormais refusée sur cet arbre volontairement sale ; ne pas produire une archive incomplète sous couvert d’un override.
- Tests / acceptation : dépôt Git temporaire propre/sale/non suivi/liens, archive déterministe et non-écrasement ; SDK actuel accepté, version non prise en charge refusée clairement. Aucune publication.
- Rollback : restaurer seulement les fichiers de ce jalon depuis la sauvegarde initiale, sans toucher aux modifications préexistantes.

### INT-AUDIT-01.2 — cohérence des coins cylindriques (P1, complexité M)

- Objectif / constat : AUD-016 ; vérifier les quatre distances 3D entre sources et coins analytiques après ajustement final.
- Périmètre : `crates/fabstep-core/src/partial.rs` et tests ciblés core/app.
- Modifications : reproduire le repère natif, garder le seuil existant et les refus bloquants. Aucun changement de l’ABI ou du constructeur CAD.
- Dépendances : aucune. Risques : refus conservateurs aux frontières numériques, différences d’arrondi suivant orientation/translation.
- Tests / acceptation : témoin montrant la divergence ancienne, cas acceptés/refusés autour du seuil, axes obliques et régressions arcs décalés/courts ; contrôle natif et revue indépendante. Ne pas annoncer un bug d’export invalide.
- Rollback : correctif et tests isolés, retour au fichier pré-jalon conservé localement.

### INT-AUDIT-01.3 — annulation coopérative (P1, complexité M)

- Objectif / constat : AUD-001 ; abandonner le travail périmé entre phases et corps.
- Périmètre : `fabstep-app` pipeline/document/recherche, commandes `fabstep-desktop` et tests associés.
- Modifications : variantes internes/complémentaires contrôlées, wrappers ordinaires inchangés ; callback de fraîcheur aux frontières, y compris essais et guidage ; contrôles de publication finaux conservés.
- Dépendances : aucune, mais écritures app/bureau sérialisées avec 01.4. Risques : priorité des erreurs, propagation de l’annulation, verrous imbriqués.
- Tests / acceptation : arrêt déterministe avant étape/corps suivant, aucune proposition partielle, aucun résultat périmé exportable, callbacks sans mutex retenu pendant OCCT ; revue indépendante. Pas de promesse d’interruption d’un appel natif déjà engagé.
- Rollback : restaurer les variantes contrôlées et les appels bureau à partir de la sauvegarde du jalon ; aucun format persistant modifié.

### INT-AUDIT-01.4 — correspondance guidée réutilisée (P2, complexité M)

- Objectif / constat : AUD-019 ; éviter la seconde tessellation lorsque l’inspection a déjà établi la provenance.
- Périmètre : `fabstep-app/src/document.rs`, tests guidage/provenance et mesure locale.
- Modifications : cache privé attaché à l’objet immuable, uniquement après inspection réussie ; repli historique sans inspection préalable ; aucun cache des erreurs natives transitoires ou des objets OCCT.
- Dépendances : 01.3 pour éviter les conflits dans les mêmes fichiers. Risques : identité de la correspondance, mémoire retenue, cache transféré à tort vers un nouveau résultat.
- Tests / acceptation : usage effectif du cache, inspection préalable ou non, résultat renouvelé, multi-corps/provenance ; mesures comparables avant/après, mêmes résultats ; revue indépendante.
- Rollback : supprimer le cache et revenir à l’inspection normale ; le fichier STEP reste inchangé.

### INT-AUDIT-01.5 — information utilisateur et documentation (P2/P3, complexité S)

- Objectif / constats : AUD-020, AUD-015, AUD-022.
- Périmètre : avertissement Avancé dans `ui/src/App.svelte`, traductions FR/EN, aide CLI et documentation existante.
- Modifications : avertissement près des réglages quand une session guidée risque d’être réinitialisée ; aucune reprise implicite. Repères de code/ADR et familles post-v1 dans l’aide CLI.
- Dépendances : aucune ; conclusion documentaire après 01.1–01.4. Risque : encombrement UI, contradiction avec UX4 ; message strictement contextuel.
- Tests / acceptation : tests UI, Svelte/build, aide CLI ; inspection du diff et liens documentaires. Qualification graphique distinguée des contrôles automatisés.
- Rollback : changements locaux de texte et affichage, sans migration.

## État initial, preuves et reprise

L’arbre comporte de nombreux changements post-v1 préexistants, suivis et non suivis. Une sauvegarde des 436 fichiers source visibles par Git (suivis + non suivis non ignorés), leurs SHA-256 et l’état Git sont conservés dans `artifacts/int-audit-01/baseline*`. `.git` est en lecture seule dans cet environnement : aucun commit, branche ni push n’est effectué. Les changements propres à l’intégration seront inventoriés par comparaison avec ces empreintes, et non avec HEAD qui précède les lots post-v1.

Base précédente : V-1159 (288 workspace, 16 bureau, 34 UI) et V-1160 (36 UI). La nouvelle suite complète `scripts/check.sh` exécutée avant les modifications termine avec code 0 : 288 tests workspace, 16 bureau, 36 UI ; formatage, Clippy, Svelte et builds passent. Journal : `artifacts/int-audit-01/baseline-check.log`. Les builds/essais natifs sont sérialisés pour éviter les conflits de caches. Les limites graphiques et FreeCAD connues restent ouvertes.

État de reprise : les cinq jalons retenus sont DONE/PASS. Aucune tâche d’intégration active. Reprendre les points différés seulement avec leurs critères de preuve, sans recommencer la qualification des constats clos.

## Comptes rendus des jalons

### INT-AUDIT-01.1 — DONE / PASS

AUD-003/AUD-008 : sélection des sources suivies et refus d’arbre sale/non suivi avant écriture ; garde des liens feuilles et ancêtres, dont remplacement après contrôle initial. Douze tests Python passent dans des dépôts temporaires. Quatre tests Rust autonomes passent pour la politique SDK 7.9.x, intégrés à `scripts/check.sh`. La compilation native courante accepte le SDK installé. La version des en-têtes ne certifie pas à elle seule celle des bibliothèques binaires. Aucun workflow supplémentaire, commit, archive de livraison ou push. Régression détectée pendant revue : garde d’ancêtre symbolique initialement omise, restaurée avant intégration avec test ciblé.

### INT-AUDIT-01.2 — DONE / PASS

AUD-016 : avant correction, le prisme D indépendant à erreurs combinées 0,8e-7 mm aboutit à `cad.invalid_model`, le témoin à 0,6e-7 réussit. Le test de régression échoue avant correction puis passe. Le contrôle des quatre coins reproduit le repère `gp_Ax3` et conserve `NATIVE_TOL=1e-7`. Treize tests unitaires, deux nouvelles régressions et dix-huit tests historiques ciblés passent (33). Revue indépendante ACCEPT ; source, ABI et preuves de paroi inchangées. Aucun correctif AUD-017 appliqué. Après correction, la CLI refuse le cas 0,8e-7 avec `recognition.partial_unproven` sans créer de STEP ; le témoin 0,6e-7 reste un solide réimporté à quatre faces, volume 274,8893571891189 mm³. Le sock-clip conserve 123 faces, dont 25 cylindres, volume 2950,4648808945294 mm³ au seuil radial explicite 0,000012 mm. Preuves : `artifacts/int-audit-01/native-regressions.json`.

### INT-AUDIT-01.3 — DONE / PASS

AUD-001 : callbacks aux frontières nommées et corps, versions contrôlées additives et wrappers ordinaires conservés. Les erreurs de callbacks sont restituées sans devenir des essais rejetés ; les contrôles de publication bureau restent actifs. Quatre tests ciblés d’annulation, un test de recherche et les seize tests bureau passent, revue indépendante ACCEPT. Un appel natif déjà commencé ou déjà en attente dans le mutex natif n’est pas interrompu au milieu de son exécution.

### INT-AUDIT-01.5 — DONE / PASS

AUD-020/015/022 : avertissement FR/EN dans Avancé quand des indications ou une tentative guidée existent ; contrat d’invalidation inchangé. Aide CLI précisée, repères des fichiers et renvoi aux jalons/ADR ajoutés à l’architecture, documentation d’archive et de SDK mise à jour. Trente-six tests UI, Svelte sans erreur/avertissement et build UI passent. Le binaire CLI reconstruit affiche bien les familles complémentaires dans `--help` (`artifacts/int-audit-01/cli-help.txt`). Aucun test de composant monté ni qualification graphique revendiqué. Aucun commit.

### INT-AUDIT-01.4 — DONE / PASS

AUD-019 : cache privé `OnceLock` de la correspondance vérifiée, lié au résultat immuable, initialisé uniquement après inspection native réussie. Deux nouveaux tests avec compteur d’inspections démontrent l’utilisation du cache, le repli sans inspection préalable, l’absence de cache hérité après reconstruction et la possibilité de réessayer une erreur native. Trois tests document et quatorze tests d’intégration (annulation, guidage, recherche, provenance) passent. Revue indépendante ACCEPT. L’inspection publique continue à produire sa tessellation ; aucun objet OCCT ni maillage n’est conservé dans ce cache.

Mesure explicite avant/après, même harness, profil `test unoptimized + debuginfo`, OCCT 7.9.3, trois répétitions par fixture après conversion et inspection initiales :

| Fixture | Médiane guidage avant | Médiane après | Étendue avant / après | Réduction observée |
| --- | ---: | ---: | --- | ---: |
| `m2b_boss_blind.stl` | 2344,082 ms | 2304,690 ms | 2338,428–2347,629 / 2298,968–2316,098 ms | 1,68 % |
| `guided_cylinder_instances.3mf` | 7253,398 ms | 7140,568 ms | 7192,900–7283,973 / 7118,110–7145,618 ms | 1,56 % |

Les six paires conservent exactement réglages, rapports CAD (comptes, volumes et dimensions), fidélité et nombres de faces inspectées. Le gain total reste faible et ne constitue pas une garantie de performance générale ; la suppression de la seconde inspection est prouvée séparément par les tests. Le point « avant » contient déjà l’annulation 01.3 mais pas le cache. Preuves : `artifacts/int-audit-01/runtime/benchmark-comparison.json`, `before-cache.log`, `after-cache.log`. Le benchmark est ignoré par défaut dans la suite normale et a été exécuté explicitement avant puis après.

## Bilan final et suite utile

Huit constats sont traités dans les cinq jalons validés ; un était déjà corrigé, un est rejeté, onze restent différés et un nécessite une reproduction avant décision. La suite finale `scripts/check.sh` termine avec code 0 : **297 tests workspace, 16 bureau, 36 UI, 12 Python et 4 tests SDK** réussissent ; formatage, Clippy strict, Svelte et builds passent. Le benchmark ignoré par défaut a été exécuté explicitement avant/après. Le dernier ajustement du nettoyage des dépôts Git de test a passé à nouveau les douze tests Python après son intégration. Aucune régression finale détectée ; avertissement Vite habituel de taille de module conservé.

`artifacts/FabSTEP.app` est reconstruit avec succès ; son exécutable est identique au binaire bureau compilé, SHA-256 `0022c36794f957e775c9e99f0e41399b3c74f15a60f6d4641d6493381aab0207`. Quitter puis relancer l’application charge ces changements. Le paquet autonome 1.0.0 n’est pas remplacé. Journaux : `artifacts/int-audit-01/final-check.log`, `build.log`, `package-tests-final.log` et sous-dossiers de mesures. Voir V-1161/V-1162 dans [VALIDATION](VALIDATION.md).

Le rapport source est inchangé, les modifications préexistantes sont préservées. Aucun commit ni push : le dossier Git réel est en lecture seule. L’inventaire et le patch relatifs à la sauvegarde initiale sont conservés dans `artifacts/int-audit-01/changes.json` et `integration.patch` ; ils distinguent ces corrections des développements post-v1 déjà présents. Le refus de l’archive source sur cet arbre sale est intentionnel, pas une archive de livraison manquante à contourner.

Limites : pas de nouvel essai graphique Tauri, Fusion ou réimport FreeCAD indépendant ; pas d’interruption au milieu d’un appel OCCT ; pas de mesure générale de performance, de version binaire OCCT certifiée par le seul en-tête, ni de tests de composants Svelte montés. Aucun problème d’export périmé n’a été découvert par cette qualification.

Prochaines actions utiles après UX4/PERF1 : achever la qualification graphique avec modèle (accueil EN clair observé en V-1166). PERF1 est validé en V-1168/V-1169 ; segmentation partagée et fusion FFI restent différées. Les conditions de preuve et de compatibilité sont dans la roadmap. AUD-017 a ensuite été repris par R-070 et qualifié dans V-1194.

## NEON — audit du 19 novembre 2026 (R-112)

Rapport local `docs/audit-neon.md`, SHA-256 `3b6819c6683f2d29b9d8c902f50988fbcf19a508fc8f7095b25ffa4d5fc2cc23`. Les identifiants ci-dessous désignent ce rapport. Les lots 1 à 5 ci-dessus concernent le rapport antérieur. Base Git `2e51866`, branche d’intégration `audit/neon`. Le rapport fourni, la skill locale fab-cleanup et son inventaire préexistaient. Référence V-1258 et STATUS avant travaux : 707 tests workspace, 51 bureau, 238 UI. Au démarrage, CLI verte et 12 tests Python réussis, 11 conditionnels ignorés.

### Qualification et registre

| NEON/AUD | Décision | Priorité | Lot | État | Preuve, limite ou condition de reprise |
|---|---|---|---|---|---|
| 001 | DEFERRED | P1 | R-085 | DEFERRED | Travail répété et délai réel sur Flansch. Deux triangles qui se croisent sont acceptés avec le contrôle OCCT bi-forme proposé. Le banc de quatre verdicts et les cas V-1251 à V-1258 restent les témoins. Reprendre sur une accélération équivalente pour les interférences internes **et** croisées, sous les plafonds actuels. |
| 002 | REJECTED | P1 | — | CLOSED | Le solide construit et le solide relu sont distincts. V-1256 montre un défaut avant transfert et V-1252 une perte de découpe après transfert. Supprimer un des deux contrôles enlèverait une preuve indépendante. |
| 003 | ALREADY_FIXED | P2 | — | CLOSED | Le constructeur filtre déjà topologie, coplanarité et injectivité avant FaceFace. Les 174 s proviennent d’une sonde exhaustive. Six éventails sont réparés sur la pièce. |
| 004 | ADAPTED | P1 | .11 | DONE | Le rapport dense contient mesh/model, et le superviseur charge sa représentation JSON. Le supprimer touche les consommateurs et le protocole. A-119 approuvée le 19 novembre. |
| 005 | ADAPTED | P2 | .9 | DONE | Préparation de coupe mono-corps et ressoudage mesurables. La topologie partagée, le remplacement des tables ordonnées, les grilles et le retrait du clone de rollback demandent une preuve distincte de parité et de gain. |
| 006 | ACCEPTED | P1 | .12 | DONE | Le build de développement embarque le helper, le script de release v1 inventorie uniquement le bureau. A-120 approuvée pour la prochaine archive et la vérification rétrospective. |
| 007 | ADAPTED | P2 | .10 | DONE | Tableaux immuables, conversion directe des positions et rectangle de peinture partagé. Un nouveau BVH demande une mesure de latence WebKit, de mémoire et de fidélité du picking. |
| 008 | ADAPTED | P1 | .8 | DONE | Mesurer le flux STEP emprunté et limiter les adaptateurs non restreints aux usages compatibles. La détection des coutures toriques exige encore les bornes UV de la face. |
| 009 | DEFERRED | P2 | .9 | DEFERRED | Mesurer l’index et les distances avec maxima, échantillons et budgets identiques. Tri, SAH, pile, normales et hachage s’évaluent indépendamment, aucune accélération présumée. |
| 010 | ACCEPTED | P1 | .7 | DONE | Écriture et sync sous mutex. Deux contrôles de fraîcheur encadrent la préparation et le renommage reste sérialisé avec l’invalidation. |
| 011 | ADAPTED | P2 | .7 | DONE | Snapshots de progression transitoires. Les octets STEP, rapports et fins de job nécessitent encore leur sync. Recherche multi-réglages et provenance à mesurer séparément, les réglages peuvent modifier soudage et composants. |
| 012 | ADAPTED | P2 | .9 | DONE | Mesurer les intersections de sections indépendamment de la projection des paires candidates. Une réécriture des preuves partielles exige un gain sur un modèle concerné et la parité des refus. |
| 013 | ADAPTED | P2 | .10 | DONE | Peinture et Échap, décimales pendant l’édition et export frais, focus et identité du nouvel aperçu. Préserver précision et annuler/rétablir. |
| 014 | ADAPTED | P3 | .10 | DONE | Mesurer les imports ciblés d’icônes. Garder la couverture Unicode des noms de fichiers. Chunks séparés à considérer après mesure du démarrage. |
| 015 | ADAPTED | P2 | .6 | DONE | La limite existante n’acceptait pas un STL tronqué comme succès. Diagnostiquer la taille avant parse pour les deux formats et refuser tôt une sortie occupée tout en conservant la publication atomique. |
| 016 | ADAPTED | P2/P3 | .6–.8, .10 | DONE | Fetch du verrou worker, publication de l’archive source, gardes ABI et relecture du type de fichier ouvert. Un lien de helper local n’authentifie pas son éditeur ; conserver le contrat développeur. Découpage C++/App, MSVC, NDEBUG et retrait de composants à reprendre selon un coût mesuré ou une cible Windows active. Corriger les repères documentaires vérifiés. |

### Plan, acceptation et réversibilité

Les lots .6 à .10 sont indépendants. Risques principaux : export d’une ancienne révision, perte d’une couture ou d’un contrôle de solide, changement des distances de fidélité, perte de sélection ou de précision pendant la saisie. Chaque commit de lot peut être annulé isolément. Les fichiers d’entrée, paquets déjà livrés et formats utilisateurs restent utilisables. Aucun P0 établi.

| INT-AUDIT-01 | Objectif et composants | Complexité et risque | Critère d’acceptation et test |
|---|---|---|---|
| .6 | CLI, workflow CI et scripts d’archive source | Faible. Destination préexistante et archive interrompue | Tests CLI des refus précoces, ancien fichier intact, suite Python, création depuis un checkout propre incluant les sources natives. |
| .7 | Publication STEP/DXF dans desktop/{main,section}, lectures supervisor et progressions worker | Moyenne. Course entre écriture et invalidation | Export périmé refusé jusque dans l’intervalle de sync, remplacement courant réussi, nettoyage temporaire. Tests bureau, helper et revue indépendante. |
| .8 | Tessellation et gardes C++ dans fabstep-cad | Moyenne. Limites paramétriques et mémoire empruntée | Mêmes verdicts natifs, même empreinte des coordonnées et indices, suite CAO, pic RSS et durée sur fichiers identiques. |
| .9 | Préparation des sections dans core ; expérimentation distincte dans verify | Moyenne. Indices après soudage, budgets et déterminisme | Cas mono et multi, limites/cancellation, parité du profil et des audits ; ne retenir une optimisation que si le banc l’étaye. |
| .10 | App, Viewport, Dialog, NumField, CSS, imports Svelte | Moyenne. Fraîcheur, précision, focus et picking | Tests UI, check, build, mesures avant/après ; essais clavier, export et rafraîchissement sur une vue montée. |
| .11 | Rapport allégé versionné dans worker, superviseur, consommateurs UI | Élevée. Dépend de A-119. Versions et informations d’inspection | Cube et rampe dense, état exportable strict, refus d’un résultat incomplet, taille/RSS et compatibilité prouvées. Retour à la lecture et à l’écriture historiques. |
| .12 | Helper dans le prochain paquet, scripts macOS, manifeste et notices | Élevée. Dépend de A-120. Fermeture des bibliothèques et anciennes archives | Vérifier les deux exécutables sans Homebrew, leurs signatures et inventaires, conversion par le helper, contrôle d’une ancienne archive. Conserver la release 1.0.0. |

### .6 — CLI, CI et paquet source · DONE / PASS

AUD-015 et partie d’AUD-016. Le CLI refuse la taille excessive et la destination occupée avant analyse. L’archive source est publiée en une opération sans écrasement, inclut les en-têtes .hxx et les trois STEP de référence explicitement versionnés. La CI récupère aussi le lockfile du helper. Treize tests CLI réussis. Python système : 14 réussis et 11 conditionnels ignorés. Une copie Git propre du projet avec le correctif produit une archive lisible de 1 061 entrées, manifeste compris. Contrôles détaillés sous `artifacts/audit-neon/`.

### .7 — Publication STEP/DXF et progression · DONE / PASS

Le bureau écrit et synchronise le temporaire hors mutex. Le contrôle final porte sur l’identité du résultat, la révision et, pour les DXF, le contexte de face ou de coupe. Le renommage s’effectue encore sous verrou. La lecture du helper revérifie le type du descripteur ouvert. Les snapshots de progression sont remplacés atomiquement, sans synchronisation persistante de chaque mise à jour. Les sorties finales restent synchronisées. Les 52 tests bureau et les 8 tests worker passent. Revue indépendante acceptée, trois invalidations concurrentes sur 64 Mio ont conservé la destination et éliminé le temporaire. La factorisation des sept essais de paramètres reste différée jusqu’à une mesure isolant le coût et démontrant la parité des réglages.

### .8 — Mémoire et accès aux surfaces OCCT · DONE / PASS

La tessellation lit les octets STEP empruntés. Les chemins qui inspectent le support d’une face évitent les bornes UV inutiles ; la couture d’un tore complet vérifie les bornes de la face découpée. Deux sorties ABI sont initialisées dès l’entrée du constructeur et le message d’erreur accepte un tampon absent. 116 tests CAO réussis, un optionnel ignoré, Clippy strict réussi. Quatre verdicts concordants au banc d’interférence. Empreinte des coordonnées et indices identique dans chaque paire.

| STEP, déflexion 0,005 mm | Tessellation avant → après | Pic RSS avant → après |
|---|---:|---:|
| Public, 1 784 117 octets, médiane de cinq processus par version | 0,232 → 0,222 s | 89 686 016 → 87 818 240 octets |
| Même géométrie avec remplissage à 64 Mio, même protocole | 0,421 → 0,403 s | 289 341 440 → 154 796 032 octets |
| Flansch expérimental déjà construit, 130 701 976 octets, un passage par version | 18,86 → 17,99 s | 3 162 832 896 → 2 899 460 096 octets |

Pour Flansch, échantillonnage 100 ms, 175 415 sommets, 98 576 triangles. Le délai total de reconstruction depuis le STL reste ouvert dans R-085. Preuves locales dans `artifacts/neon-cad/`.

### .10 — Sélection, tolérances et construction de l’interface · DONE / PASS

AUD-007, 013 et 014. Les maillages reçus sont immuables dans Svelte. La peinture partage le rectangle de la vue entre les échantillons, Échap annule le geste et préserve la sélection. Un nouvel aperçu remplace ses géométries à révision source identique. Les tolérances conservent la précision décimale en FR/EN et s’appliquent sur Entrée ou à la sortie du champ. Le dialogue d’aide restitue le focus. Les imports d’icônes sont ciblés. La liste des polices reste complète.

Sur une grille de 100 000 triangles (médiane de 25 passages, Node sur M4), préparation des positions 16,50 → 1,22 ms et des contours 31,84 → 13,58 ms, coordonnées identiques. Ces mesures isolent la préparation des données. Build Vite local 4 262 → 226 modules, 1,18 s → 0,465 s sur le passage comparatif. Module principal 876,08 ko, 236,75 ko gzip, avertissement de taille toujours présent. 240 tests UI réussis et Svelte 0 erreur, 0 avertissement. Un parcours Chrome/WebGL avec pont Tauri simulé couvre peinture/Échap, tolérances FR/EN, focus, invalidation à l’export, nouvel aperçu et historique.

### .9 — Préparation des coupes et fidélité · DONE / PASS sur le domaine retenu

La coupe mono-corps réutilise le soudage et l’adjacence lorsque l’ordre source est identique. La découverte emprunte les triangles bruts. Deux incidences sont stockées directement dans la table ordonnée de chaque arête. Les diagnostics non-manifold, arête ouverte puis orientation gardent leur priorité. Le calcul d’une intersection de triangle sans sommet sur le plan conserve son ordre cyclique et évite une allocation. La préparation par composante est conservée pour les sources multi-corps.

Sur la rampe publique à 49 222 triangles (`qualify-flange-public.py ramp`, u=54, v=32, sparse-floor, subdivisions=2), sept passages release. Préparation médiane **170,248 → 91,321 ms**, pic RSS de processus **81 313 792 → 69 632 000 octets**. Section médiane **22,570 → 21,902 ms**. Préparations et sections sont identiques octet pour octet en JSON. Core + verify : 348 tests réussis et 8 ignorés en mode ordinaire, 355 réussis et 10 ignorés en mode gros STL. Tests de parité mono/multi-corps, inversion et soudage inclus. Clippy strict réussi.

Deux décisions ont évolué après mesure. AUD-012 devient ADAPTED avec la seule optimisation locale de section. Les projections par paire, les fenêtres et le partage d’une topologie entre modules restent différés. AUD-009 devient DEFERRED. La variante combinant cache et hachage a ralenti la fidélité sur la rampe (163,692 → 175,778 ms) et sur le maillage expérimental de Flansch (868,524 → 910,016 ms). Une autre disposition a donné des écarts inférieurs à la variabilité entre passages. Les deux directions et les comptes d’échantillons sont identiques. Le code de fidélité initial est conservé. Mesures dans `artifacts/neon-core/`.

Le banc public de bout en bout `scripts/qualify-large-torus-worker.py` mesure désormais aussi le temps du helper et les tailles du STEP et du rapport. Le lancer avec le binaire release et Python équipé de NumPy. Sur macOS, `/usr/bin/time -l` autour du banc fournit les métriques du processus pour une comparaison à environnement identique. Le compteur Work reste un budget interne.


### .11 — Rapport compact du gros STL · DONE / PASS

Arbitrage A-119 accepté. Le helper et le superviseur partagent un rapport typé v2 réservé aux mesures et à l’inspection, plafonné à 4 Mio. Le bureau transmet à la vue un marqueur de version et obtient les faces affichées depuis le STEP validé. L’ancien rapport reste disponible pour les requêtes v1. Un refus initial de version par un ancien helper autorise un seul nouvel essai v1. L’identité, les limites, l’échéance commune et la fraîcheur restent vérifiées. La revue croisée a conduit à refuser les enveloppes mélangées, à comparer les quatre tolérances en tenant compte de l’arrondi JSON et à contrôler le marqueur de fin du STEP.

| Rampe publique, 194 694 triangles | v1 | v2 |
|---|---:|---:|
| Rapport privé | 33 402 531 octets | 2 261 octets |
| Lecture, désérialisation et libération, médiane de cinq processus release | 115,71 ms | 0,046 ms |
| Pic RSS du processus de mesure, mêmes passages | 315 654 144 octets | 18 104 320 octets |

Les deux rapports concordent sur toutes les mesures conservées, dont 659 faces et un volume de 93,536793 mm³. La section DATA des STEP de 1 784 117 octets est identique. Les maxima de fidélité sont 0,004717 et 0,004702 mm. La mesure de lecture utilise le DOM historique et le type compact dans un processus isolé ; elle exclut la reconstruction et la WebView. Les durées globales ont été mesurées sous des charges différentes. Reproduction de la géométrie, taille et durée du helper via `scripts/qualify-large-torus-worker.py <helper> --protocol 1` puis `--protocol 2`. Journaux locaux dans `artifacts/audit-neon/worker-report/`.

55 tests bureau, dont le passage de relais avec les deux générations de helpers, 11 tests worker, 241 tests UI. Le parcours Chrome/WebGL avec un STL binaire de 100 001 triangles et un pont Tauri simulé couvre consentement, source brute, rapport compact, diagnostics traduits, sélection de face STEP et commande d’export. En WKWebView macOS séparée, l’édition d’une tolérance, la restauration du focus, l’invalidation à l’export et la nouvelle géométrie ont été observées. Le clic de sélection synthétique en fenêtre masquée n’a pas abouti au test de peinture WebKit ; le parcours peinture a réussi dans Chrome.

### .12 — Paquet autonome avec helper · DONE / PASS

A-120 approuvée. Le manifeste v2 décrit les deux exécutables et leurs 30 bibliothèques. Les notices réunissent les dépendances des deux graphes Cargo, le profil gros STL est réservé au helper. La publication du répertoire utilise un renommage exclusif sur macOS, y compris lorsqu’un autre processus crée une destination vide.

La candidate de 22 952 788 octets a pour SHA-256 `fcb1b13c4537d50e5e36fcbcdd07cdfae1cbc10ecb6af1e0651f2d4484a0891d`. Après extraction dans un chemin contenant des espaces, signature ad hoc vérifiée, quatre conversions CLI (cinq solides) et un cube de 1 mm³ par chaque protocole du helper. Une deuxième lecture indépendante reproduit le rapport de validation. Le véritable bureau extrait a ouvert une fenêtre de 1 440 × 900 avec les lectures Homebrew bloquées. Les 30 bibliothèques embarquées ont été observées au chargement, puis le seul processus de sonde a été arrêté. L’archive v1 historique garde son empreinte `9bac6a8d…`, ses 29 bibliothèques et passe encore la validation stricte avec son CLI. Six tests Python ciblent les chemins négatifs et la compatibilité. Preuves locales dans `artifacts/audit-neon/candidate/`, `release-candidate-strict.json` et `release-candidate-startup.json`.

### Pistes transversales et reprise

Les suggestions sans identifiant du §5 sont rattachées aux décisions ci-dessus. Les repères de projection locaux et les tolérances ont des domaines différents selon les familles ; une extraction commune justifierait un lot séparé avec preuves de parité. Déplacer les tests inline qui utilisent les internes FFI ou découper la traduction C++ suppose de mesurer le temps de recompilation touché. L’extraction des cartes de messages et le nettoyage des composants du design system restent secondaires. Le détail localisé de planéité repose encore sur le texte du noyau ; un sous-code stable devra conserver les consommateurs de `cad.invalid_model`. Le tri spatial, la parallélisation et une autre comptabilisation de Work requièrent des échantillons, des refus et un comportement déterministes. Les actions Fusion existantes gardent leur qualification manuelle.

Les deux propositions de gains « 5×–20× » et « 2× » pour l’interférence restent sans mesure équivalente acceptable. Pour R-085, reproduire les quatre verdicts du banc et les cas de défaut avant/après transfert, puis mesurer sur la rampe et seulement ensuite sur Flansch avec le superviseur de production, 30 minutes et 4 Gio. La dernière conversion complète mesurée sur le STL de 540 170 triangles reste V-1258 (délai dépassé). Une tessellation réussie du STEP expérimental n’établit pas la réussite de cette conversion.

Les sept lots ont leurs propres commits identifiés par `INT-AUDIT-01.X`. La publication locale du paquet n’installe rien sur un autre Mac. FabBoard A-121 porte sur le geste de peinture et l’invalidation d’export à confirmer dans le bureau. Le dépôt contient le code et les tests nécessaires à la reprise ; les fichiers privés et les sorties lourdes sont dans les répertoires locaux ignorés.


### Bilan de validation NEON

Deux recommandations ACCEPTED, dix ADAPTED, deux DEFERRED, une REJECTED et une ALREADY_FIXED. Sept lots autonomes validés. Les reports d’optimisations à l’intérieur des recommandations adaptées figurent dans le registre. Les arbitrages A-119 et A-120 sont clos.

| Contrôle exécuté | Résultat |
|---|---|
| Workspace, 147 suites en profil test optimisé hors ligne avec verrou | 710 réussis, 18 optionnels ignorés. Exécution lancée sur les lots .6 à .10 ; recompilation du workspace et vérifications ciblées sur le protocole final. |
| Core et fidélité en profil gros STL ; intégrations app/CAO en profil gros STL | 355 réussis, 10 optionnels ignorés ; 42 contrôles app/CAO supplémentaires réussis. |
| Bureau, helpers réel v1 et v2 ; worker | 55 réussis au bureau et 11 au worker. Vérification supplémentaire avec le binaire du paquet autonome. |
| Interface, Svelte et Vite | 241 réussis, analyse Svelte sans erreur ni avertissement, build réussi. Avertissement Vite de taille du module principal. |
| Python avec NumPy ; détection du SDK OCCT | 31 tests et 4 tests réussis. |
| Formatage, Clippy strict workspace/bureau/worker, compilation bureau, lint FabBoard et contrôle du diff | Réussis. |
| Rampe publique avec le helper de la candidate autonome | 659 faces, 93,536793 mm³, rapport 2 261 octets en 34,105 s lors de ce passage. |
| Grande archive source sur checkout propre | 1 065 entrées, contenu revérifié avec le manifeste. |

Les essais manuels sur un second Mac ou dans Fusion pour cette candidate et la reconstruction de bout en bout du STL privé ne faisaient pas partie des relectures exécutées. Les logs et la candidate sont locaux. Les fichiers du rapport source et du nettoyage local préexistant sont restés dans leur état Git initial.

### Contre-expertise Argon du 19 novembre

Cette passe reprend les propositions faites sur `0f29e00`. Les résultats détaillés sont dans V-1259 et V-1260 de `docs/VALIDATION.md`. Les étapes .13, .14, .15 et .16 sont indépendantes. Chaque commit peut être rétabli séparément. Les risques principaux étaient un changement des décisions de distance, une provenance incomplète et une acceptation erronée d’une intersection.

| Proposition | Priorité et décision | Jalon et état | Motif et critère de validation |
|---|---|---|---|
| Métadonnées du BVH de fidélité | P2 · ACCEPTED | .13 · DONE | Tri stable sur les métadonnées temporaires, triangles compacts pour les requêtes. Comparer rapports, compteur de travail, RSS et temps sur les deux corpus. |
| Distance précalculée sur la pile | P3 · DEFERRED | .13 · DEFERRED | Le bénéfice additionnel change de signe entre deux séries sur Flansch. |
| Provenance inutilisée du helper | P2 · ADAPTED | .14 · DONE | Sur un STL à un corps, les conversions Auto destinées à l’export vérifient la partition et produisent les jetons natifs sans allouer les listes de sélection. Les rapports v1/v2 conservent leur format. L’analyse dense reste nécessaire au calcul. |
| OCCT-1, suppression des tests individuels répétés | P1 · ADAPTED | .15 · DEFERRED | La variante littérale accepte une face ouverte et refuse un raccord valide. La variante mono-forme corrigée conserve le contrôle topologique et la table filtrée d’interférences. Les mesures sur les groupes observés ne justifient pas une dépendance aux détails internes d’OCCT 7.9.3. Témoins ajoutés au banc public. |
| OCCT-2, deux arguments | P1 · REJECTED | .15 · CLOSED | Le contrôle bi-forme de haut niveau accepte deux triangles sécants. Le PaveFiller à deux arguments avec filtrage refuse une jonction légitime dans le groupe 11 de la rampe. Une simple distinction entre nouveaux indices et indices d’origine ne résout pas cette différence. |
| OCCT-3, parallélisme public | P1 · ADAPTED | .15 / R-085 · DEFERRED | L’API publique parallèle accélère le diagnostic. Le STEP de laboratoire est refusé à la diagonale 3 094, également refusée en séquentiel. Le helper expérimental de V-1259 refusait le STL original en 17 min 36 s et sous 4 Gio. Les défauts de raccord et de transfert ont depuis été corrigés dans R-085. Le paquet atteint toujours trente minutes en séquentiel. La qualification de coût et de mémoire du parallélisme en production reste ouverte (V-1262). |
| Calcul UV des sommets | P2 · ACCEPTED | .16 · DONE | Calculer les UV uniquement pour les sommets admissibles. Comparer l’atlas, le modèle et le rapport complets. |
| Régions planes d’un triangle | P2 · ADAPTED | .16 · DONE | Réutiliser la file de croissance et former directement le contour canonique. Le contrôle projeté des arêtes consécutives peut refuser une facette d’aire positive. Tester ce refus et les deux sens de parcours. |
| Bords non isoparamétriques des tores | P2 · ADAPTED | R-113 · IN_PROGRESS | V-1264 vérifie les deux branches plan–tore, les deux traces du STEP et la borne continue sur le carreau torique nominal. Une corde droite en UV quitte le plan. La découverte du carreau et la construction de son bord dans Auto restent ouvertes. |
| Atlas cylindrique et conique interrompu | P2 · ADAPTED | R-113 · IN_PROGRESS | Sur trois petits STL, le prototype V-1264 retrouve un arbre avec perçage transversal et réimporte quatre faces valides. Le domaine natif, les autres coupes cylindriques et les sections de cône restent ouverts. |
| Composition locale dans Auto | P2 · ADAPTED | R-114 · TODO | Choisir deux familles déjà certifiées et un contre-exemple qui revendique la même zone. La composition doit conserver leurs plans porteurs, les arêtes partagées et une attribution exhaustive et déterministe. |
| Graines planaires par aire | P2 · ADAPTED | R-115 · DONE | La seconde segmentation conserve uniquement une réunion vérifiée de régions entières. Les deux solides f32 de 96 triangles donnent six et sept faces dans FabSTEP et FreeCAD, avec maintien du pli et de la provenance. Le refus hors tolérance et le coût près du plafond ordinaire sont documentés en V-1263. La recette utilisateur reste ouverte dans A-124. |
| Estimateur de cercle pour arcs courts | P3 · DEFERRED | R-113 · DEFERRED | Trois pas de Gauss–Newton n’ont gagné aucune candidature sur les témoins bruités décrits en V-1260. L’aptitude d’un autre estimateur, dont Taubin, reste à mesurer sur un cas rejeté à tort. |

Les extensions de contrat envisagées pour les contours libres et la composition seront précisées à partir des témoins avant arbitrage. Les preuves de fermeture, d’interférence et d’écart restent les critères d’export.
