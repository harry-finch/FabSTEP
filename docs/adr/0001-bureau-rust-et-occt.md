# ADR-0001 : Bureau local Rust, interface FabApps et noyau OCCT isolé

Status: Accepted
Date: 2026-11-06

## Context

Le projet doit convertir des STL puis des 3MF en solides STEP modifiables dans Fusion ou FreeCAD, en commençant par les pièces simples. L'utilisateur souhaite l'architecture modulaire et la licence de FabWeave. La reconstruction géométrique et l'historique de conception sont deux objectifs distincts.

## Decision

L'utilisateur a confirmé le 6 novembre 2026 : priorité à la qualité du STEP, dépendance C++ OpenCASCADE autorisée ; bureau macOS en premier, Windows/Linux ensuite ; solide BRep sans historique paramétrique au premier jalon.

- Pipeline et modèle de données en Rust, interface Svelte 5/TypeScript du kit Fab, bureau Tauri 2.
- Licence du projet GPL-3.0-or-later comme FabWeave.
- OCCT derrière une ABI C étroite pour construction, contrôle et échange. Aucun type OCCT dans core.
- Premier incrément limité aux plans et contours polygonaux ; pas de reconstruction automatique de trou rond sans preuve géométrique.
- Un export réussi est un solide réimporté et validé ; une coque ouverte ne peut pas être un résultat de substitution silencieux.

## Alternatives considered

Truck/monstertruck permettent une chaîne Rust, notamment pour le navigateur. Leur couverture des surfaces, l'export analytique et la robustesse devraient être évalués sur le même corpus avant tout remplacement. Le bureau est prioritaire et le SDK OCCT est déjà disponible sur le poste ; commencer par ce moteur évite de refaire ses fonctions de construction et de validation.

Une application web seule impose d'autres contraintes de compilation et de distribution. Elle n'est pas un critère du premier jalon. Un export STEP écrit à la main ajouterait un second travail de validation du format sans répondre au problème de reconstruction.

## Consequences

Le build de développement dépend d'un SDK OCCT local. Le paquet autonome et les licences tierces devront être traités avant diffusion. Le navigateur peut présenter l'interface, mais la conversion exige le bureau. Les prochains jalons peuvent étendre le modèle de géométrie sans exposer les classes du noyau dans toute l'application.

## Validation

Tests core indépendants du noyau, cas CAD idéaux, conversion bout en bout et réimport STEP. Contrôler fermeture, validité, nombre de corps, volume, dimensions et conservation des trous. Les essais Fusion/FreeCAD sont consignés séparément des tests automatiques OCCT.
