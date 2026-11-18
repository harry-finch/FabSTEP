---
id: A-097
type: decider
titre: "Choisir ce qui se nettoie dans FabSTEP"
statut: fait
cree: 2026-11-18
par: agent
fait: 2026-11-18
---

Inventaire du 18 novembre 2026, sur la branche locale fabboard/nettoyage, issue de fabboard/mise-d-equerre (82342b40). Lors de cet inventaire, rien n’avait encore été retiré. Deux essais à blanc fabboard clean trash réussis, aucune protection contournée. Taille de départ fabboard status : 43,3 Gio (46 526 246 912 octets). Les tailles sont des estimations de blocs occupés ; les liens et clones APFS peuvent modifier le gain disque réel. Le déplacement à la Corbeille diminue la taille du projet mais les fichiers occupent encore le disque tant que la Corbeille est conservée.

Décision reçue : l’utilisateur a approuvé A+B. Les deux catégories ont été exécutées intégralement ; le choix initial était indépendant par catégorie.

## A — Reconstructible

Total : 40.78 Gio ; 31 chemins précis. Les 14 emplacements Cargo sous artifacts ont été identifiés par .rustc_info.json et les répertoires debug/release. Les journaux clippy.log et oblique.log de artifacts/r089-core-target restent en place : seuls debug/ et tmp/ sont proposés.

| Chemin | Taille | Motif et coût du retrait |
| --- | ---: | --- |
| `target` | 18.19 Gio | Sorties Cargo identifiées ; recompilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `crates/fabstep-desktop/target` | 11.86 Gio | Sorties Cargo identifiées ; recompilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `crates/fabstep-worker/target` | 3.66 Gio | Sorties Cargo identifiées ; recompilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `ui/node_modules` | 158.1 Mio | Dépendances verrouillées ; npm --prefix ui ci avant les prochains contrôles UI. |
| `ui/dist` | 1.2 Mio | Frontend compilé ; npm --prefix ui run build. |
| `crates/fabstep-desktop/gen` | 320 Kio | Schémas Tauri générés au prochain build du bureau. |
| `.ruff_cache` | 20 Kio | Cache Ruff ; prochaine analyse sans cache. |
| `artifacts/dxf2b/analysis-baseline/after/target` | 160.5 Mio | Sorties Cargo identifiées ; recompilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/dxf2b/analysis-baseline/before/target` | 130.9 Mio | Sorties Cargo identifiées ; recompilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/r080/target` | 93.1 Mio | Sorties Cargo identifiées ; recompilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/r092-frame-lab/target` | 240.7 Mio | Sorties Cargo identifiées ; recompilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/r092-planar/target` | 839.4 Mio | Sorties Cargo identifiées ; recompilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/sock-clip/diag/target` | 199.4 Mio | Sorties Cargo identifiées ; recompilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/r091-cad-target` | 1.20 Gio | Sorties Cargo identifiées ; recompilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/r091/prior-build` | 394.6 Mio | Sorties Cargo identifiées ; recompilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/r091/workspace-final` | 534.6 Mio | Sorties Cargo identifiées ; recompilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/macos-build` | 1.07 Gio | Sorties Cargo identifiées ; recompilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/r089-native/cargo` | 240.7 Mio | Sorties Cargo identifiées ; recompilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/r092-frame-target` | 443.9 Mio | Sorties Cargo identifiées ; recompilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/r089-bridge-target` | 726.3 Mio | Sorties Cargo identifiées ; recompilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/r089-core-target/debug` | 751.8 Mio | Sorties Cargo identifiées ; recompilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/r089-core-target/tmp` | 0 Kio | Sorties Cargo identifiées ; recompilation complète du périmètre au prochain usage, potentiellement plusieurs minutes. |
| `artifacts/m5/__pycache__` | 8 Kio | Cache Python, recréé au prochain lancement. |
| `artifacts/m5b1/__pycache__` | 8 Kio | Cache Python, recréé au prochain lancement. |
| `artifacts/m5b2/__pycache__` | 8 Kio | Cache Python, recréé au prochain lancement. |
| `artifacts/r074/__pycache__` | 8 Kio | Cache Python, recréé au prochain lancement. |
| `artifacts/r093/review/__pycache__` | 8 Kio | Cache Python, recréé au prochain lancement. |
| `fixtures/__pycache__` | 304 Kio | Cache Python, recréé au prochain lancement. |
| `fixtures/complex_v1/__pycache__` | 52 Kio | Cache Python, recréé au prochain lancement. |
| `scripts/__pycache__` | 540 Kio | Cache Python, recréé au prochain lancement. |
| `scripts/tests/__pycache__` | 104 Kio | Cache Python, recréé au prochain lancement. |

## B — Résidus non suivis

Total : 48 Kio. Métadonnées de présentation du Finder, sans contenu du projet ; leur disposition locale sera oubliée et macOS pourra les recréer. Déjà ignorées par .gitignore.

| Chemin | Taille |
| --- | ---: |
| `.DS_Store` | 16 Kio |
| `crates/.DS_Store` | 12 Kio |
| `crates/fabstep-desktop/.DS_Store` | 12 Kio |
| `docs/.DS_Store` | 8 Kio |

## C — Suivis devenus inutiles

Aucun retrait proposé : aucune preuve suffisante de fichier suivi inutile n'a été établie. Aucune modification de .gitignore proposée : les candidats sont déjà ignorés (Ruff ignore son propre cache).

## Conservation

Garder les deux applications artifacts/FabSTEP.app et artifacts/release/1.0.0/FabSTEP.app, ainsi que leurs ressources et helper ; les rapports, journaux, exports CAO et autres preuves sous artifacts ; les fixtures, sources utilisateur, branding, brainstorming, historique, licences, roadmap et skills. Les changements préexistants .agents/skills-fabboard.json, .agents/skills/fab-cleanup/ et .claude/skills/fab-cleanup restent intacts et ne doivent pas être incorporés à un commit de nettoyage. Aucun fichier incertain supplémentaire soumis au retrait.

## Exécution et validation après accord

Utiliser exclusivement fabboard clean trash sur les chemins approuvés ; en cas de refus, retirer le chemin de la liste, signaler le refus et ne pas le contourner. Refaire fabboard status avant/après. Les fichiers restent récupérables dans la Corbeille ; ne pas la vider.

Lors de l’inventaire, aucun test ni build n’avait été relancé, car rien n’avait encore été retiré. Après retrait de A, demander si la recompilation complète doit être lancée maintenant : elle recréera une part importante des caches. Commandes de référence : scripts/check.sh (tests Python, fmt/test/clippy Rust workspace, tests/check/build UI, validations bureau et worker séparés ; large-stl uniquement sur le target worker) et scripts/build-macos-dev.sh pour reconstruire le paquet de développement. Réinstaller d'abord les dépendances via npm --prefix ui ci si node_modules est retiré. Les essais graphiques et natifs restent distincts des contrôles automatiques. Aucun merge ni push implicite.

## Bilan de l’exécution

Le 18 novembre 2026, `fabboard clean trash` a déplacé les 35 chemins approuvés à la Corbeille, sans refus : A = 43 784 822 784 octets et B = 49 152 octets. Le projet passe de 43,3 à 2,6 Gio (scan final : 2 741 391 360 octets avant écriture du bilan). Les deux exécutables FabSTEP et le helper ont des empreintes SHA-256 identiques avant/après. Les journaux clippy.log et oblique.log conservés sont présents ; les 35 chemins approuvés sont absents ; aucun fichier suivi n’a été retiré. `cargo fmt --all -- --check` et `git diff --check` réussis. Tests et builds non relancés après nettoyage : choix suivi en A-098. Les fichiers de la Corbeille restent récupérables et occupent encore le disque. Aucun merge ni push.

## Réponse

Accord explicite de l’utilisateur dans la conversation : « A+B ». Les 31 chemins A et 4 chemins B ont ensuite été déplacés à la Corbeille sans refus.
