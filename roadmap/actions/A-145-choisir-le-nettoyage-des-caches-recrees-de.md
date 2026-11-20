---
id: A-145
type: decider
titre: "Choisir le nettoyage des caches recréés de FabSTEP"
statut: fait
cree: 2026-11-20
par: agent
fait: 2026-11-20
---

Inventaire du 20 novembre 2026 au soir, sur fabboard/nettoyage-2026-11-20-soir, issue de main (2e3c62b8). Dépôt propre avant inventaire. Taille initiale : 20141666304 octets (18.76 Gio). Rien n’avait encore été supprimé lors de l’inventaire. Tous les chemins ci-dessous passent fabboard clean trash --dry-run, sans refus et sans déplacement.

La préférence utilisateur pour la suppression directe, sans Corbeille, exprimée avant A-143 et confirmée lors de son exécution, est conservée. L’utilisateur a ensuite approuvé cette nouvelle liste par « oui je confirme. et tu peux supprimer directement ». Les fichiers seront supprimés définitivement par liste explicite après approbation de A et/ou B ; la Corbeille existante restera intacte.

Décision reçue : A+B approuvés et exécutés intégralement en suppression directe.

## A — Reconstructible

9 chemins ; total 11.74 Gio (12607561728 octets).

| Chemin | Taille | Motif et coût du retrait |
| --- | ---: | --- |
| `artifacts/auto-intent/probe-target` | 36.4 Mio | Sorties Cargo identifiées ; recompilation complète de ce périmètre au prochain usage, potentiellement plusieurs minutes. |
| `crates/fabstep-desktop/gen` | 312 Kio | Schémas Tauri ; régénérés au prochain build du bureau. |
| `crates/fabstep-desktop/target` | 3.03 Gio | Sorties Cargo identifiées ; recompilation complète de ce périmètre au prochain usage, potentiellement plusieurs minutes. |
| `crates/fabstep-worker/target` | 2.18 Gio | Sorties Cargo identifiées ; recompilation complète de ce périmètre au prochain usage, potentiellement plusieurs minutes. |
| `scripts/__pycache__` | 64 Kio | Cache Python, recréé au prochain lancement. |
| `scripts/tests/__pycache__` | 72 Kio | Cache Python, recréé au prochain lancement. |
| `target` | 6.35 Gio | Sorties Cargo identifiées ; recompilation complète de ce périmètre au prochain usage, potentiellement plusieurs minutes. |
| `ui/dist` | 1.3 Mio | Frontend généré ; npm --prefix ui run build. |
| `ui/node_modules` | 151.6 Mio | Dépendances verrouillées ; réinstaller via npm --prefix ui ci. |

## B — Résidus non suivis

1 fichier .DS_Store ; total 20 Kio (20480 octets). Métadonnées de présentation Finder déjà ignorées par Git et recréables ; la présentation locale sera réinitialisée.

| Chemin | Taille |
| --- | ---: |
| `.DS_Store` | 20 Kio |

## C et D

Aucun fichier suivi proposé au retrait, faute de preuve suffisante d’inutilité. Les données incertaines sont conservées ; aucun arbitrage supplémentaire.

## Conservation

Conserver artifacts/FabSTEP.app, artifacts/release/1.0.0/FabSTEP.app et artifacts/audit-neon/candidate/, leurs ressources et helpers. Conserver fichiers CAO, données utilisateur, corpus, rapports, journaux, sources de laboratoire, illustrations, historiques, licences, roadmap et skills. Conserver les dépôts imbriqués artifacts/audit-neon/source-checkout et artifacts/audit-neon/final-source-checkout. L’environnement artifacts/geometry-run/formatter/ et ses caches restent hors du périmètre. roadmap/.DS_Store est protégé et reste en place. Aucune modification des droits ni des autorisations macOS.

## Travail concurrent à préserver

Pendant l’inventaire, des clôtures groupées FabBoard ont modifié les actions A-126 à A-144 et plusieurs éléments, ainsi que ordre.txt et ROADMAP.md. Ces modifications ne viennent pas du nettoyage : les conserver et ne pas les incorporer à son commit. Une suite `cargo test --release --offline --locked -p fabstep-app --test automatic_tolerances --test cut_sphere --test rolling_bore --test drilled_shaft` travaille dans FabSTEP. Attendre sa fin avant de supprimer les caches concernés ; ne pas interrompre ce travail. Les tailles restent des estimations à revalider après la suite.

## Exécution après accord

Revalider ces seuls chemins avec fabboard clean trash --dry-run ; ne contourner aucun refus. Après accord, supprimer directement les chemins approuvés, sans joker et sans suivre de lien symbolique ni sortir du projet ; aucun fichier suivi, secret, dépôt imbriqué ou build protégé ne doit être supprimé. Vérifier les empreintes des applications et de la candidate avant/après, la conservation des fichiers suivis et l’absence des chemins approuvés. Mesurer fabboard status avant/après. Le gain disque réel dépend des liens, clones et instantanés APFS. Ne pas toucher à la Corbeille des précédents nettoyages.

Aucun test ni build exécuté pendant cet inventaire. Conserver le report au prochain développement explicitement choisi dans A-098 et repris dans les propositions approuvées suivantes, pour éviter de recréer immédiatement les caches. Reprise : npm --prefix ui ci, scripts/check.sh, scripts/build-macos-dev.sh pour le paquet de développement ; toujours compiler le worker séparément du bureau, large-stl réservé au worker. Distinguer les contrôles automatiques des essais natifs et graphiques. Aucun merge ni push.

## Bilan de l’exécution

Le 20 novembre 2026 au soir, aucun processus de build/test n’était encore actif dans FabSTEP lors du contrôle précédant la suppression. Les protections de fabboard ont été revalidées avec --dry-run sur les dix chemins, sans refus ni déplacement. Après contrôle du confinement au projet, de l’absence de liens symboliques dans les chemins et de fichiers suivis dans les candidats, la suppression directe a utilisé shutil.rmtree pour les dossiers et unlink pour le fichier. La Corbeille existante et les permissions sont restées intactes.

Mesure juste avant exécution : 20 141 948 928 octets (18,8 Gio). A = 12 607 561 728 octets ; B = 20 480 octets ; total retiré estimé = 12 607 582 208 octets (11,74 Gio). Scan après suppression : 7534366720 octets (7,0 Gio). Les dix chemins sont absents. Aucun fichier suivi n’a été supprimé. Les empreintes de contenu SHA-256 des deux applications et du dossier entier de la candidate NEON sont identiques avant/après. Le reliquat de caches (6914048 octets) appartient au formateur Python conservé hors de la liste approuvée.

`cargo fmt --all -- --check`, `git diff --check` et `fabboard roadmap lint` ont réussi. Aucun test, Clippy, contrôle UI ou build supplémentaire n’a été relancé dans ce nettoyage, conformément au report conservé dans la proposition approuvée. Les tests Rust concurrents étaient terminés ; leur résultat n’est pas revendiqué ici. Les modifications concurrentes de STATUS.md, DEVLOG.md, des actions, éléments, ordre.txt, intro.md et ROADMAP.md sont conservées hors du commit de nettoyage. Pour éviter d’entremêler ces travaux, le présent bilan est consigné dans cette action ; les deux documents d’état/journal en cours de modification par ailleurs restent intacts. Aucun merge ni push.

## Réponse

L’utilisateur confirme A+B et la suppression directe : « oui je confirme. et tu peux supprimer directement ». Les dix chemins ont été supprimés définitivement après la fin des tests Rust.
