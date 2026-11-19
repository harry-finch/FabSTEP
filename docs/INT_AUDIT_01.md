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
| AUD-009 | DEFERRED | P3 | — | DEFERRED | Bundle volumineux confirmé, mais un découpage statique ne prouve pas un démarrage plus rapide. Mesurer avant chargement différé. Lecture ciblée, aucun correctif revendiqué. |
| AUD-010 | REJECTED | P3 | — | CLOSED | Un chemin via parent symbolique désignant une entrée existante existe aussi et est canonicalisé. Aucun cas de corruption reproduit ; la correction proposée ne répond pas au scénario décrit. Lecture ciblée, aucun correctif revendiqué. |
| AUD-011 | DEFERRED | P3 | — | DEFERRED | Troncature UTF-8 sûre mais non par graphème ; effet limité au nom de produit, nom source conservé. Pas de nouvelle dépendance Unicode pour ce seul confort. Lecture ciblée, aucun correctif revendiqué. |
| AUD-012 | ADAPTED | P3 | UX4 | DONE | Douze clés mortes retirées avec la présentation ; clés des composants partagés conservées (AppBar, StatusBar, Alerts). Contrairement au rapport, guidance.complete_cylinder_verified est réellement émis par mixed.rs : alias conservé. Parité FR/EN et Svelte contrôlés ; V-1163. |
| AUD-013 | ADAPTED | P2 | UX4 | DONE | Template/CSS App rendus lisibles ; import/conversion/guidage/export extraits dans workflow.ts et exercés par les tests réels. Workers, historique et recherche restent locaux pour limiter le risque ; pas de refonte globale. V-1163. |
| AUD-014 | DEFERRED | P3 | — | DEFERRED | Durcissement local envisageable, pas de faille distante établie. main.json seul ne définit pas les permissions des commandes ; manifeste applicatif et tests IPC nécessaires. Lecture ciblée, aucun correctif revendiqué. |
| AUD-015 | ADAPTED | P3 | 01.5 | DONE | Ajouter des repères et liens vers la roadmap/ADR existants plutôt que dupliquer un second tableau historique complet. PASS : contrôles ciblés et suite globale finale ; V-1161/V-1162, journaux ci-dessous. |
| AUD-016 | ADAPTED | P1 | 01.2 | DONE | Vérifier les quatre coins analytiques dans le repère réellement construit en CAD. Conserver 1e-7 mm : la marge arbitraire 0,9 rejetterait aussi des cas acceptables. PASS : contrôles ciblés et suite globale finale ; V-1161/V-1162, journaux ci-dessous. |
| AUD-017 | ADAPTED | R-070 / R-026 | DONE | V-1194 reproduit le défaut sur un anneau dense de 362 sommets. Le secours vérifie tous les sommets et le tour complet après échec du garde initial. Les tests conservent le profil en D et les anneaux clairsemés. |
| AUD-018 | ADAPTED | P2 | PERF1 | DONE | Cache privé, borné et paresseux des seules preuves source réussies sur géométrie exacte ; analyse/segmentation et contrôles des résultats conservés. PASS : rapports complets, refus et traces équivalents sur sept candidats STL/3MF/sock-clip ; compteurs natifs réduits ; recherche réelle −36,5 %, −48,9 %, −22,5 % en développement. Suite globale 304 workspace / 16 bureau / 61 UI ; V-1168/V-1169. |
| AUD-019 | ACCEPTED | P2 | 01.4 | DONE | Cache privé de la seule correspondance vérifiée dans DocumentStep immuable ; aucun objet OCCT ni maillage conservé ; mesure avant/après et test explicite du cache. PASS : contrôles ciblés et suite globale finale ; V-1161/V-1162, journaux ci-dessous. |
| AUD-020 | ACCEPTED | P2 | 01.5 | DONE | Choisir l’avertissement contextuel proposé, sans rejouer les indications avec des options nouvelles ni modifier le contrat d’invalidation. PASS : contrôles ciblés et suite globale finale ; V-1161/V-1162, journaux ci-dessous. |
| AUD-021 | DEFERRED | P3 | — | DEFERRED | Taille/duplication confirmées, seuil prescrit par la skill introuvable. Découpage massif et mutualisation vectorielle non nécessaires aux défauts retenus ; les tests ne rendent pas ces opérations sans risque. Lecture ciblée, aucun correctif revendiqué. |
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
| 004 | NEEDS_DECISION | P1 | .11 | BLOCKED | Le rapport dense contient mesh/model, et le superviseur charge sa représentation JSON. Le supprimer touche les consommateurs et le protocole. Arbitrage A-119 avant sérialisation compacte. |
| 005 | ADAPTED | P2 | .9 | IN_PROGRESS | Préparation de coupe mono-corps et ressoudage mesurables. La topologie partagée, le remplacement des tables ordonnées, les grilles et le retrait du clone de rollback demandent une preuve distincte de parité et de gain. |
| 006 | NEEDS_DECISION | P1 | .12 | BLOCKED | Le build de développement embarque le helper, le script de release v1 inventorie uniquement le bureau. A-120 porte sur la prochaine archive et la vérification rétrospective. |
| 007 | ADAPTED | P2 | .10 | IN_PROGRESS | Tableaux immuables, conversion directe des positions et rectangle de peinture partagé. Un nouveau BVH demande une mesure de latence WebKit, de mémoire et de fidélité du picking. |
| 008 | ADAPTED | P1 | .8 | IN_PROGRESS | Mesurer le flux STEP emprunté et limiter les adaptateurs non restreints aux usages compatibles. La détection des coutures toriques exige encore les bornes UV de la face. |
| 009 | ADAPTED | P2 | .9 | IN_PROGRESS | Mesurer l’index et les distances avec maxima, échantillons et budgets identiques. Tri, SAH, pile, normales et hachage s’évaluent indépendamment, aucune accélération présumée. |
| 010 | ACCEPTED | P1 | .7 | DONE | Écriture et sync sous mutex. Deux contrôles de fraîcheur encadrent la préparation et le renommage reste sérialisé avec l’invalidation. |
| 011 | ADAPTED | P2 | .7 | DONE | Snapshots de progression transitoires. Les octets STEP, rapports et fins de job nécessitent encore leur sync. Recherche multi-réglages et provenance à mesurer séparément, les réglages peuvent modifier soudage et composants. |
| 012 | DEFERRED | P2 | .9 | DEFERRED | Mesurer les intersections de sections indépendamment de la projection des paires candidates. Une réécriture des preuves partielles exige un gain sur un modèle concerné et la parité des refus. |
| 013 | ADAPTED | P2 | .10 | IN_PROGRESS | Peinture et Échap, décimales pendant l’édition et export frais, focus et identité du nouvel aperçu. Préserver précision et annuler/rétablir. |
| 014 | ADAPTED | P3 | .10 | IN_PROGRESS | Mesurer les imports ciblés d’icônes. Garder la couverture Unicode des noms de fichiers. Chunks séparés à considérer après mesure du démarrage. |
| 015 | ADAPTED | P2 | .6 | DONE | La limite existante n’acceptait pas un STL tronqué comme succès. Diagnostiquer la taille avant parse pour les deux formats et refuser tôt une sortie occupée tout en conservant la publication atomique. |
| 016 | ADAPTED | P2/P3 | .6–.8, .10 | IN_PROGRESS | Fetch du verrou worker, publication de l’archive source, gardes ABI et relecture du type de fichier ouvert. Un lien de helper local n’authentifie pas son éditeur ; conserver le contrat développeur. Découpage C++/App, MSVC, NDEBUG et retrait de composants à reprendre selon un coût mesuré ou une cible Windows active. Corriger les repères documentaires vérifiés. |

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
