---
id: A-122
type: decider
titre: "Choisir le second nettoyage de FabSTEP"
statut: fait
cree: 2026-11-19
par: agent
fait: 2026-11-19
---

Inventaire du 19 novembre 2026. Branche locale fabboard/nettoyage-2026-11-19 issue de main (5f901e88), dépôt propre avant inventaire. Le précédent accord A+B de A-097 portait sur la liste du 18 novembre : cette nouvelle liste a ensuite été approuvée par « oui vas y ». Rien n’avait été retiré au moment de l’inventaire. Essai à blanc fabboard clean trash réussi pour tous les chemins ci-dessous, sans refus.

État de départ : 23940857856 octets, soit 22.30 Gio. Estimations de blocs occupés ; liens et clones APFS peuvent modifier le gain disque réel. Les éléments déplacés à la Corbeille continuent d’occuper le disque tant qu’ils y sont conservés.

Décision reçue : A+B approuvés et exécutés intégralement.

## A — Reconstructible

35 chemins, total 16.91 Gio (18156687360 octets). Les sorties Cargo supplémentaires sous artifacts ont été identifiées par .rustc_info.json et debug/release ; aucune source ou journal autonome ne figure à leur racine. Les deux applications à tester et la candidate autonome NEON sont hors de la liste.

| Chemin | Taille | Motif et coût du retrait |
| --- | ---: | --- |
| `.ruff_cache` | 20 Kio | Cache Ruff ; prochaine analyse sans cache. |
| `artifacts/argon/core/before-probe/target` | 52.7 Mio | Sorties Cargo ; compilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/argon/core/probe/target` | 52.7 Mio | Sorties Cargo ; compilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/argon/core/side/target` | 61.4 Mio | Sorties Cargo ; compilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/argon/core/test-target` | 44.8 Mio | Sorties Cargo ; compilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/argon/core/three-phase/target` | 77.8 Mio | Sorties Cargo ; compilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/argon/parallel-worker/target` | 116.3 Mio | Sorties Cargo ; compilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/argon/recognition/__pycache__` | 16 Kio | Cache Python ; recréé lors du prochain lancement. |
| `artifacts/argon/recognition/target` | 119.8 Mio | Sorties Cargo ; compilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/argon/verify/probe/target` | 69.5 Mio | Sorties Cargo ; compilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/audit-neon/legacy-cli/target` | 99.7 Mio | Sorties Cargo ; compilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/geometry-run/__pycache__` | 4 Kio | Cache Python ; recréé lors du prochain lancement. |
| `artifacts/geometry-run/atlas-app-target` | 208.3 Mio | Sorties Cargo ; compilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/geometry-run/distance-lab/target` | 42.0 Mio | Sorties Cargo ; compilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/geometry-run/drills/target` | 672.0 Mio | Sorties Cargo ; compilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/geometry-run/f-oblique-core` | 334.0 Mio | Sorties Cargo ; compilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/geometry-run/flange/rust-target` | 236.5 Mio | Sorties Cargo ; compilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/geometry-run/flange/writer-target` | 362.9 Mio | Sorties Cargo ; compilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/geometry-run/guided-target` | 106.9 Mio | Sorties Cargo ; compilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/geometry-run/height-target` | 606.4 Mio | Sorties Cargo ; compilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/geometry-run/int-target` | 268.7 Mio | Sorties Cargo ; compilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/geometry-run/largetarget` | 70.7 Mio | Sorties Cargo ; compilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/geometry-run/source-target` | 125.9 Mio | Sorties Cargo ; compilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/geometry-run/workspace` | 664.6 Mio | Sorties Cargo ; compilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/macos-build` | 1.05 Gio | Sorties Cargo ; compilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/neon-cad/bench/target` | 54.5 Mio | Sorties Cargo ; compilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/neon-cad/target` | 525.9 Mio | Sorties Cargo ; compilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `crates/fabstep-desktop/gen` | 312 Kio | Schémas Tauri ; régénérés à la compilation du bureau. |
| `crates/fabstep-desktop/target` | 3.75 Gio | Sorties Cargo ; compilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `crates/fabstep-worker/target` | 1.66 Gio | Sorties Cargo ; compilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `scripts/__pycache__` | 196 Kio | Cache Python ; recréé lors du prochain lancement. |
| `scripts/tests/__pycache__` | 136 Kio | Cache Python ; recréé lors du prochain lancement. |
| `target` | 5.44 Gio | Sorties Cargo ; compilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `ui/dist` | 1.2 Mio | Frontend généré ; npm --prefix ui run build. |
| `ui/node_modules` | 151.6 Mio | Dépendances verrouillées ; réinstaller via npm --prefix ui ci. |

## B — Résidus non suivis

1 chemin, total 16 Kio. Métadonnées de présentation du Finder, déjà ignorées par Git ; présentation locale réinitialisée et fichier recréable par macOS.

| Chemin | Taille |
| --- | ---: |
| `.DS_Store` | 16 Kio |

## C et D

Aucun fichier suivi proposé au retrait : aucune preuve suffisante de fichier devenu inutile établie. Aucun arbitrage supplémentaire sur des données incertaines ; elles sont conservées.

## Conservation

Conserver artifacts/FabSTEP.app, artifacts/release/1.0.0/FabSTEP.app, la candidate autonome artifacts/audit-neon/candidate/, leurs ressources et helpers. Conserver corpus, données utilisateur, CAO, rapports, journaux, scripts et sources de laboratoire, branding, brainstorming, historique, licences, roadmap et skills. L’environnement de formatage artifacts/geometry-run/formatter/ est conservé avec ses dépendances et caches : il reste un outil utilisable, hors du périmètre proposé.

## Exécution après accord

Déplacer uniquement les chemins approuvés avec fabboard clean trash ; en cas de refus, ne pas le contourner. Vérifier l’absence des chemins, la conservation des exécutables et des fichiers suivis, puis mesurer fabboard status avant/après. Pas de merge ni push. Aucun test ni build exécuté pendant cet inventaire. Le précédent choix de reporter les tests au prochain développement (A-098) reste la préférence exprimée ; ne pas relancer automatiquement une reconstruction complète qui recréerait les caches. Reprise : npm --prefix ui ci, scripts/check.sh, puis scripts/build-macos-dev.sh pour le paquet de développement ; conserver les builds bureau/worker distincts et large-stl réservé au worker. Les essais natifs et graphiques restent distincts des contrôles automatiques.

## Bilan de l’exécution

Le 19 novembre 2026, `fabboard clean trash` a déplacé les 36 chemins à la Corbeille sans refus : A = 18 156 687 360 octets ; B = 16 384 octets. Le projet passe de 22,3 à 5,4 Gio (scan après déplacement : 5784166400 octets). Les 36 chemins sont absents, aucun fichier suivi n’a été supprimé. Les empreintes de contenu SHA-256 des deux applications et de tout le dossier de la candidate NEON sont identiques avant/après. Le reliquat de caches détecté (6914048 octets) correspond aux caches de l’environnement de formatage conservé, hors de la liste approuvée.

`cargo fmt --all -- --check` et `git diff --check` ont réussi. Tests, Clippy, contrôles UI et builds non relancés : la préférence de report au prochain développement est conservée conformément à la proposition approuvée. Les commandes de reprise restent ci-dessus. Les fichiers à la Corbeille sont récupérables et continuent d’occuper le disque ; ne pas la vider. Aucun merge ni push.

## Réponse

L’utilisateur confirme les lots A+B de la liste A-122 par « oui vas y ». Les 35 chemins A et le fichier B ont été déplacés à la Corbeille sans refus.
