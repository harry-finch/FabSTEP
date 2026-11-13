# FabSTEP — consignes de développement

Lire `README.md`, `STATUS.md`, `ROADMAP.md` et `ARCHITECTURE.md` avant de modifier.

FabSTEP appartient à FabApps. Rust pour la géométrie, Svelte 5/TypeScript pour l'interface, Tauri pour le bureau ; licence `GPL-3.0-or-later`. La cible initiale est macOS, puis Windows/Linux. Le navigateur seul n'exécute pas le noyau CAO natif.

## Invariants

- `fabstep-core` est pur et déterministe : aucune IO, interface ni dépendance OCCT. Millimètres et `f64` en interne ; l'unité STL est explicite.
- `fabstep-cad` contient seul l'ABI C++ et le noyau OCCT. Les objets OCCT ne traversent pas cette frontière.
- Une coque ouverte n'est jamais une conversion réussie. Réimporter le STEP, vérifier le solide, son volume et ses dimensions avant de le proposer à l'export.
- Ne jamais inventer une fermeture, un fond de trou, une unité ou une primitive. Toute modification géométrique est comptée et signalée par un code stable.
- Les erreurs ont un code stable. Aucun résultat ancien ne devient exportable après changement du fichier ou des options.
- Un STEP BRep n'est pas un historique d'esquisses. Ne pas présenter les surfaces facettisées comme des cylindres reconstruits.
- Les références et sorties attendues du brainstorming sont des hypothèses à vérifier, pas des tests déjà passés.
- Limiter explicitement tailles, allocations et travail sur les fichiers importés. 3MF Core utilise l’unité déclarée ou celle par défaut et conserve les placements ; les extensions requises non prises en charge sont refusées. Les limites globales comprennent 32 corps et 100 000 triangles après placement.
- Préserver chaque corps distinct ; refuser contacts, chevauchements et imbrications à la source et au résultat. Ne jamais proposer un export partiel. Le STEP multi-racines conserve les placements sans prétendre constituer un assemblage XCAF. Conserver l’aperçu importé si la conversion échoue.
- UI : jetons CSS uniquement, thèmes sombre/clair, contrôles 28 px, français de référence et anglais complet, dernière requête gagnante, calcul hors fil UI.
- Mettre la documentation à jour avec le comportement. Ne pas modifier ou effacer le brainstorming d'origine.

## Références de la famille

Les skills de la famille sont installées dans `.agents/skills/`. Les règles spécifiques à ivaCAM (fork, remotes, FAO) ne s’appliquent pas à FabSTEP. Aucune publication distante implicite.

## Skills à utiliser

- `fab-roadmap` : spécification, suivi, actions de l’utilisateur et mise d’équerre.
- `project-documentation` : README, consignes, état courant et journal.
- `fab-design-system` et `professional-workshop-ui` : interface compacte, jetons, thèmes et langues ; conserver les décisions UX4 propres à FabSTEP.
- `fab-illustration-system`, puis `hairline-create` ou `hairline-adopt` : figures au trait, avec choix du concept par l’utilisateur.
- `github-upstream-workflow` : opérations GitHub explicitement demandées, sous réserve des règles FabSTEP ; aucun fork, remote, merge ou push implicite.
- `skills/chef-orchestre/SKILL.md` : orchestration des chantiers non triviaux.

## Feuille de route

Le suivi vit dans `roadmap/`, lu par FabBoard ; `ROADMAP.md` est généré et ne se modifie jamais à la main. Utiliser `.agents/skills/fab-roadmap/SKILL.md` pour spécifier, tenir les statuts et créer une action quand on attend l’utilisateur. Lire ses réponses au début de chaque session ; `fabboard roadmap lint` avant chaque commit. L’ancienne feuille de route est conservée dans `docs/ROADMAP-avant-fabboard.md`.

## Validation

`cargo fmt --all -- --check`, `cargo test --workspace --offline`, `cargo clippy --workspace --all-targets --offline -- -D warnings`, `npm --prefix ui run check`, `npm --prefix ui run build`. Bureau hors workspace : `cargo check --manifest-path crates/fabstep-desktop/Cargo.toml --offline`. Relire `docs/VALIDATION.md` pour les contrôles natifs et visuels. Distinguer les contrôles automatisés réellement exécutés des essais manuels restant à faire.

## Orchestration

Pour les chantiers de développement non triviaux, utiliser la skill locale
`skills/chef-orchestre/SKILL.md`. L’agent principal reste responsable de
l’intégration et des preuves de validation ; les écritures déléguées doivent
porter sur des périmètres disjoints.
