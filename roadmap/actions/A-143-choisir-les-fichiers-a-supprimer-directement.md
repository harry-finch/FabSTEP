---
id: A-143
type: decider
titre: "Choisir les fichiers à supprimer directement dans FabSTEP"
statut: fait
cree: 2026-11-20
par: agent
fait: 2026-11-20
---

Inventaire du 20 novembre 2026. Branche locale fabboard/nettoyage-2026-11-20 issue de atelier/carte-ecarts (a0741ec6). Dépôt propre avant inventaire. Taille initiale : 43491020800 octets, soit 40.50 Gio. Aucun retrait n’avait encore été effectué lors de l’inventaire.

L’utilisateur a demandé explicitement « cette fois-ci supprime directement les fichiers. pas besoin de juste les mettre à la corbeille ». Cette instruction remplace le passage par la Corbeille prévu par fab-cleanup pour ce nettoyage. La suppression directe et définitive est donc le mode autorisé, sans toucher à la Corbeille existante. L’utilisateur a ensuite approuvé cette nouvelle liste précise par « je valide ! ». Les précédents accords A+B portaient sur d’autres listes.

Les protections de fabboard clean trash ont été vérifiées en mode --dry-run uniquement : tous les chemins ci-dessous sont acceptés, aucun fichier déplacé. Ce contrôle ne modifie pas le mode demandé de suppression directe. Ne contourner aucun refus lors d’une nouvelle validation avant exécution.

Décision reçue : A+B approuvés et exécutés intégralement en suppression directe.

## A — Reconstructible

21 chemins ; total 33.53 Gio (36001894400 octets). Les sorties Cargo sous artifacts sont identifiées par leurs marqueurs .rustc_info.json et debug/release, et leur racine a été inspectée pour éviter sources ou journaux autonomes.

| Chemin | Taille | Motif et coût du retrait |
| --- | ---: | --- |
| `.ruff_cache` | 16 Kio | Cache Ruff ; prochaine analyse sans cache. |
| `artifacts/r085-reprise/source/target` | 63.9 Mio | Sorties Cargo identifiées ; recompilation complète de ce périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/r085-suite/capture/repo/crates/fabstep-worker/target` | 112.9 Mio | Sorties Cargo identifiées ; recompilation complète de ce périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/r085-suite/downstream/target` | 66.0 Mio | Sorties Cargo identifiées ; recompilation complète de ce périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/r085-suite/full/__pycache__` | 20 Kio | Cache Python ; recréé au prochain lancement. |
| `artifacts/r085-suite/rotated-source/target` | 49.6 Mio | Sorties Cargo identifiées ; recompilation complète de ce périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/r085-suite/transfer-proof/target` | 112.9 Mio | Sorties Cargo identifiées ; recompilation complète de ce périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/r085-suite/transfer-proof/target-control` | 112.8 Mio | Sorties Cargo identifiées ; recompilation complète de ce périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/r085-suite/transfer-proof/target-corrected` | 112.8 Mio | Sorties Cargo identifiées ; recompilation complète de ce périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/r115-perf/after/target` | 55.4 Mio | Sorties Cargo identifiées ; recompilation complète de ce périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/r115-perf/before/target` | 55.1 Mio | Sorties Cargo identifiées ; recompilation complète de ce périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/r115-perf/profile/target` | 43.6 Mio | Sorties Cargo identifiées ; recompilation complète de ce périmètre au prochain usage, potentiellement plusieurs minutes. |
| `crates/fabstep-desktop/gen` | 312 Kio | Schémas Tauri ; régénérés au prochain build du bureau. |
| `crates/fabstep-desktop/target` | 7.36 Gio | Sorties Cargo identifiées ; recompilation complète de ce périmètre au prochain usage, potentiellement plusieurs minutes. |
| `crates/fabstep-worker/target` | 4.72 Gio | Sorties Cargo identifiées ; recompilation complète de ce périmètre au prochain usage, potentiellement plusieurs minutes. |
| `fixtures/__pycache__` | 36 Kio | Cache Python ; recréé au prochain lancement. |
| `scripts/__pycache__` | 464 Kio | Cache Python ; recréé au prochain lancement. |
| `scripts/tests/__pycache__` | 136 Kio | Cache Python ; recréé au prochain lancement. |
| `target` | 20.53 Gio | Sorties Cargo identifiées ; recompilation complète de ce périmètre au prochain usage, potentiellement plusieurs minutes. |
| `ui/dist` | 1.2 Mio | Frontend généré ; npm --prefix ui run build. |
| `ui/node_modules` | 151.6 Mio | Dépendances verrouillées ; réinstaller avec npm --prefix ui ci. |

## B — Résidus non suivis

8 fichiers .DS_Store ; total 96 Kio (98304 octets). Métadonnées de présentation du Finder déjà ignorées par Git, recréables ; la présentation locale correspondante sera réinitialisée.

| Chemin | Taille |
| --- | ---: |
| `.DS_Store` | 20 Kio |
| `crates/.DS_Store` | 12 Kio |
| `crates/fabstep-desktop/.DS_Store` | 12 Kio |
| `crates/fabstep-worker/.DS_Store` | 8 Kio |
| `docs/.DS_Store` | 8 Kio |
| `fixtures/.DS_Store` | 20 Kio |
| `scripts/.DS_Store` | 8 Kio |
| `ui/.DS_Store` | 8 Kio |

## C et D

Aucun fichier suivi proposé au retrait : aucune preuve suffisante d’inutilité établie. Les données incertaines restent conservées ; aucun arbitrage supplémentaire.

## Conservation

Conserver artifacts/FabSTEP.app, artifacts/release/1.0.0/FabSTEP.app et artifacts/audit-neon/candidate/, leurs ressources et helpers, ainsi que les données utilisateur, CAO, corpus, rapports, journaux, sources de laboratoire, branding, brainstorming, historiques, licences, roadmap et skills. Conserver l’environnement artifacts/geometry-run/formatter/ et ses caches, comme lors des nettoyages précédents. Conserver les dépôts imbriqués artifacts/audit-neon/source-checkout et artifacts/audit-neon/final-source-checkout. Le fichier roadmap/.DS_Store est exclu car protégé par FabBoard.

## Exécution après accord

Valider à nouveau tous les chemins en mode fabboard clean trash --dry-run, sans utiliser son mode de déplacement. Après accord sur A et/ou B, supprimer définitivement ces seuls chemins par une liste explicite, sans joker, sans suivre les liens symboliques ni sortir du projet ; ne pas supprimer un fichier suivi, un secret, un dépôt imbriqué ou un build protégé. Une liste validée ne donne aucun accord pour des chemins supplémentaires. Ne pas vider ni modifier la Corbeille des nettoyages précédents.

Vérifier les empreintes des applications conservées avant/après, l’absence des chemins approuvés et la conservation des fichiers suivis. Mesurer fabboard status avant/après et consigner le résultat. Pas de merge ni push. Aucun test ni build exécuté pendant l’inventaire. Conserver la préférence de report des tests et builds au prochain développement, explicitée dans A-098 et dans les précédentes propositions approuvées. Reprise : npm --prefix ui ci, scripts/check.sh, puis scripts/build-macos-dev.sh pour le paquet de développement, avec builds bureau/worker distincts et large-stl réservé au worker. Les essais graphiques et natifs restent distincts des contrôles automatiques.

La suppression directe ne permet pas de récupérer les caches par la Corbeille : ils devront être reconstruits. Le gain disque réel dépend des liens et clones APFS et des instantanés éventuels.

## Bilan de l’exécution

Le 20 novembre 2026, les protections de fabboard ont été revalidées avec --dry-run sur les 29 chemins, sans refus et sans déplacement. Les chemins ont ensuite été supprimés définitivement par une liste explicite (shutil.rmtree pour les dossiers, unlink pour les fichiers), après vérification du confinement au projet, de l’absence de liens symboliques dans les chemins et de fichiers suivis par Git dans les candidats. Aucun fichier n’est passé par la Corbeille et son contenu préexistant n’a pas été touché.

A = 36 001 894 400 octets ; B = 98 304 octets ; total 36 001 992 704 octets (33,5 Gio estimés). Le projet passe de 40,5 à 7,0 Gio ; scan après suppression : 7489036288 octets. Les 29 chemins sont absents. Aucun fichier suivi n’a été supprimé. Les empreintes de contenu SHA-256 des deux applications et du dossier entier de la candidate NEON sont identiques avant/après. Le reliquat de caches (6914048 octets) appartient au formateur Python conservé, hors de la liste approuvée.

`cargo fmt --all -- --check` et `git diff --check` ont réussi. Tests, Clippy, vérifications UI et builds non relancés, conformément au report au prochain développement conservé dans la proposition approuvée. Les commandes de reprise sont indiquées ci-dessus. Aucun merge ni push.

## Réponse

L’utilisateur approuve explicitement les lots A+B de la liste A-143 par « je valide ! », après avoir demandé la suppression directe. Les 21 chemins A et les huit fichiers B ont été supprimés définitivement, sans Corbeille.
