# État — 18 novembre 2026

**V1 mécanique 1.0.0 acceptée ; extensions post-v1 dans le build de développement.** Rust/OCCT, Svelte 5/Tauri, GPL-3.0-or-later. Branche de travail : `fabboard/mise-d-equerre` ; aucune modification de la branche par défaut ni du paquet release. Historique et mesures détaillés dans [DEVLOG](DEVLOG.md) et [VALIDATION](docs/VALIDATION.md), suivi utilisateur dans [ROADMAP](ROADMAP.md).

## Version et validation courantes

- `artifacts/release/1.0.0/FabSTEP.app` reste le paquet autonome signé ad hoc, accepté sur un autre Mac et dans Fusion le 9 novembre (V-1115). Pas de signature Developer ID ni notarisation revendiquée.
- `artifacts/FabSTEP.app` est le build local de développement, dépendant du SDK installé. Dernière qualification **V-1203** : **540 tests workspace, 50 bureau, huit worker, 93 ciblés large-stl, un transfert réel, 230 UI, douze Python et quatre SDK réussis**. Formatage, Clippy strict, Svelte et builds passent ; deux exécutables du paquet vérifiés par SHA-256.
- Seize tests opt-in workspace restent ignorés par la suite ordinaire. Les essais graphiques/Fusion des extensions restent ouverts dans FabBoard ; aucun succès automatique ne vaut acceptation utilisateur. Le pilotage du dialogue d’ouverture avait été bloqué, mais l’utilisateur a confirmé l’ouverture manuelle.

## Capacités développées

L’import STL/3MF Core conserve les corps et placements ; les STEP sont réimportés et contrôlés avant export. Le STEP est un BRep sans historique d’esquisses. L’atelier propose lancement manuel, progression native, sélection source/STEP et provenance, corrections Plan/Cylindre/Cône/Tore dans leurs domaines prouvés, profils et sections DXF exactes (ellipses comprises), coupe source polygonale. Les erreurs ou changements de requête invalident l’export.

Les familles mécaniques qualifiées comprennent extrusions, cylindres et cônes locaux, chaînes coniques, sphères/calottes/zones, tores et raccords toriques, boîtiers arrondis et bacs ouverts. Dernières tranches :

- **R-076 / C2** : profils toriques composés et arcs obtus, anneaux et raccords démontrés.
- **R-078 / C1b2** : deux à huit cônes adjacents ou mélangés à des portées, témoins intérieurs et découpage unique ; repli Auto historique préservé hors domaine.
- **R-077 / E2** : bac ouvert à coins arrondis avec perçage traversant du fond, sans nouveau moteur de production.
- **R-081 / D2** : coin de bloc orthogonal à trois cylindres partiels et un octant sphérique. Champs/ABI additifs approuvés (ADR-0036/A-068), sources transformées, fidélité, provenance et documents à deux corps contrôlés. Repli Auto facettisé conservé si la preuve analytique échoue ; correction locale couplée refusée. Domaine limité à 32 000 triangles, 8 192 régions planes et six supports dominants.

Ces lots sont à tester (A-069 à A-076). Les détails de portée et de preuve sont dans V-1201/V-1203. Les compteurs de la v1 ne sont pas confondus avec ces extensions.

## Limites et fichier Flansch

Import ordinaire : 32 corps, 64 Mio et 100 000 triangles après placement. Le mode gros STL accepté utilise un helper séparé `large-stl`, source jusqu’à 600 000 triangles et 1 800 000 sommets, annulation, surveillance RSS/empreinte physique à 4 Gio et délai de 30 minutes. Les plafonds résultat/tessellation et 3MF restent inchangés ; le bureau n’active jamais cette feature (ADR-0034).

Dernier essai complet de Flansch (V-1201, avant D2) : quatre zones de 278 642 triangles deviennent douze surfaces, attribution contrôlée. Le résultat conserve **231 608 faces**, dont 231 584 plans, et reste refusé au plafond résultat de 100 000. **Aucun STEP produit, validation native source non atteinte.** La qualification des plans/coûts restants est R-080 ; aucun relèvement de plafond ni gain global de latence promis.

## Chantier en cours

**R-082 / G1** : contrat de cône incliné à deux extrémités elliptiques approuvé avec D par A-068. Corpus indépendant et brouillons core/CAO/tests en préparation, séparés du build D2 qualifié.

**R-036 / F** : le laboratoire R-063 certifie les trois courbes et leurs cycles dans les budgets (V-1200), avec volume idéal encadré et volume OCCT adaptatif mesuré. L’extension des perçages sécants est autorisée par A-045 ; la reconnaissance STL et l’intégration de production restent à réaliser.

Les méridiennes libres C2, les congés du fond/rebord E et les combinaisons dépassant les domaines qualifiés restent dans leurs entrées de roadmap. L’utilisateur a autorisé la poursuite globale ; tout nouveau contrat structurant encore indéfini doit être présenté concrètement avant intégration.
