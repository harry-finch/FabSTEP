---
id: R-118
titre: "Exporter un rapport Markdown en CLI et exposer une passerelle MCP locale"
resume: "Ajouter une sortie Markdown lisible à fabstep-cli (--report) et un wrapper MCP optionnel appelant la CLI locale, sans modifier le moteur ni l'atelier bureau."
statut: idee
jalon: post-v1
cree: 2026-11-19
maj: 2026-11-19
livree:
---


## But

`fabstep-cli` produit aujourd’hui un rapport JSON complet, idéal pour les scripts, mais peu pratique à lire directement ou à partager dans une revue d’atelier. Par ailleurs, des utilisateurs qui travaillent avec des agents IA locaux (Claude Desktop, Cursor, etc.) souhaitent pouvoir analyser et convertir un STL/3MF via le protocole MCP en profitant des garanties strictes de FabSTEP. Ajouter un rapport Markdown optionnel en CLI et une passerelle MCP locale légère répond à ces deux usages sans alourdir le moteur ni modifier l’atelier bureau.

## Inclus

- Option `--report <chemin.md>` sur `fabstep-cli` (`check` et `convert`) générant un rapport Markdown lisible en français : source, unité, réglages appliqués, tableau par corps (faces par famille, volume, dimensions, bornes et fidélité échantillonnée), diagnostics et statut final (réussite ou refus avec code stable).
- Écriture atomique du rapport Markdown (fichier temporaire + renommage, respect de `--force`, refus d’écraser la source ou la sortie STEP).
- Passerelle MCP locale optionnelle (binaire ou crate séparé hors workspace bureau, ou sous-commande dédiée) exposant en lecture/conversion locale les opérations existantes de `fabstep-cli` (`check` et `convert`) avec les mêmes limites strictes (64 Mio, 32 corps, 100 000 triangles, tolérances explicites, aucun accès réseau).
- Documentation d’usage succincte dans `README.md` (exemple `--report` et configuration client MCP).

## Hors périmètre

- Génération de scripts paramétriques (CadQuery, OpenSCAD, macros FreeCAD) ou promesse d’historique d’esquisses.
- Exposition du mode gros STL (`large-stl`) via MCP dans cette première tranche, afin de conserver strictement l’isolement du helper (ADR-0034).
- Ouverture réseau, serveur distant ou téléchargement automatique de fichiers par la passerelle MCP.
- Modification de l’interface graphique Tauri ou du schéma JSON existant de `fabstep-cli`.

## Dépendances

- Décision utilisateur à confirmer avant implémentation : garder `--report` et la passerelle MCP dans le même lot (`R-118`) ou scinder la passerelle MCP dans un élément distinct si seule la sortie Markdown est souhaitée à court terme.

## Critères d'acceptation

- `fabstep convert fixtures/m2b_boss_blind.stl -o out.step --mode auto --report out.md` écrit un fichier Markdown valide contenant les mêmes nombres de faces, volume, dimensions, réglages et écarts de fidélité que la sortie `--json`.
- En cas d’échec de conversion, `--report` consigne le code d’erreur stable et aucun fichier STEP n’est écrit ; sans `--force`, un fichier rapport existant n’est pas écrasé.
- La passerelle MCP démarre en local sur stdio, expose les schémas d’outils `check` et `convert`, rejette les chemins/extensions invalides et les entrées > 64 Mio avec les codes d’erreur FabSTEP, et produit le même STEP validé que la CLI.
- Aucun appel réseau et aucune modification du paquet macOS bureau v1/post-v1 ne sont introduits.
- `sh scripts/check.sh` passe sans erreur ni avertissement.

## Vérifications de l'utilisateur

- Relire un rapport Markdown généré par `--report` pour valider sa clarté et les informations affichées.
- Essayer la passerelle MCP locale depuis un client compatible (ex. Claude Desktop ou Cursor) sur un STL d’exemple.
