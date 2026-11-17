# État — 17 novembre 2026

**V1 mécanique 1.0.0 acceptée ; extensions post-v1 dans le build de développement.** Rust/OCCT, Svelte 5/Tauri, GPL-3.0-or-later. Cible qualifiée : Apple Silicon, macOS 26 ou ultérieur. La v1 correspond au tag `v1.0.0` ; la mise d’équerre est préparée sur `fabboard/mise-d-equerre`, issue de `codex/v1-mechanical`. Les évolutions post-v1, la documentation et les skills ont été commitées et poussées le 17 novembre sur `origin/fabboard/mise-d-equerre` (9f7341c, 702a259, 597863c). La branche par défaut et le paquet release restent inchangés.

## Vérifié et accepté

- La v1 importe STL/3MF Core et exporte les solides STEP des familles mécaniques prouvées. Limites : 32 corps, 64 Mio, 100 000 triangles après placement ; unité STL explicite, refus des contacts et de l’export partiel. Voir [README](README.md) et [jalons livrés](ROADMAP.md).
- Le paquet autonome `artifacts/release/1.0.0/FabSTEP.app` est signé ad hoc et embarque OCCT. L’utilisateur a confirmé le 9 novembre son installation sur un autre Mac et l’utilisation des STEP dans Fusion ([V-1115](docs/VALIDATION.md)). La CI macOS du 7 novembre a réussi. Aucune signature Developer ID ni notarisation acquise.
- L’intégration initiale de l’audit ([R-020](roadmap/items/R-020-int-audit-01-integration-initiale.md)) et PERF1 ([R-051](roadmap/items/R-051-perf1-preuves-source-reutilisees.md)) sont consignés comme validés dans la feuille de route d’origine ; mesures et limites restent dans [VALIDATION](docs/VALIDATION.md) et [INT-AUDIT-01](docs/INT_AUDIT_01.md).

## Développé, qualification encore ouverte

Le build `artifacts/FabSTEP.app` contient les extensions géométriques A/B/C1a, cylindres ouverts décalés et petits raccords ; l’atelier UX4, la sélection source/STEP et la provenance exacte, les indications Plan/Cylindre/Cône, les profils de faces et les sections STEP/source DXF. Les courbes DXF prises en charge incluent les ellipses exactes ; la coupe source reste polygonale. Ces fonctions ne font pas partie du paquet autonome 1.0.0.

La dernière validation globale **historique**, V-1177 du 12 novembre, rapporte 391 tests workspace, 32 bureau, 175 UI, 12 Python et 4 SDK, formatage, Clippy strict, Svelte et builds réussis. Ce bilan ne remplace pas les essais natifs et Fusion encore ouverts. La sélection/ouverture manuelle a été confirmée ; le blocage du pilotage automatisé n’établit pas un défaut produit de l’import.

Les lots concernés restent `a-tester` dans [ROADMAP](ROADMAP.md) ; les actions détaillent séparément les parcours graphiques et réimports attendus. Leurs réponses doivent être examinées avant d’accepter chaque lot.

## Reprise UX4 du 15 novembre

Le retour utilisateur sur l’inspecteur trop chargé et le guidage introuvable est traité dans [R-042](roadmap/items/R-042-ux4-simplifier-latelier.md) : barre contextuelle Cylindre/Cône/Essayer près du modèle, préparation explicite depuis un résultat automatique, détails techniques repliés et ouverture du panneau à une nouvelle sélection. **181 tests UI**, Svelte, build macOS et contrôle bureau réussis ; revue sans défaut bloquant. Rendu interactif restant à qualifier : serveur local refusé et navigateur de pilotage indisponible. Preuves et limites : V-1178 dans [VALIDATION](docs/VALIDATION.md).

## Plan guidé — 15 novembre

[R-056](roadmap/items/R-056-i2d-fusion-plane-guidee.md) ajoute Plan à la correction contextuelle : fusion de faces polygonales contiguës, sans déplacement des coordonnées source, avec refus des contours courbes. Un faible relief peut être simplifié dans la tolérance ; son résidu reste signalé. Revue indépendante acceptée et contrôles globaux réussis : **400 tests workspace, 32 bureau, 184 UI, 12 Python et 4 SDK**. Qualification graphique/Fusion encore ouverte ; preuves V-1180 dans [VALIDATION](docs/VALIDATION.md). Le guidage Tore est désormais intégré à UX5 ci-dessous.

## UX5 — automatique et sélection des parois

[R-057](roadmap/items/R-057-ux5-reconstruction-automatique-et-selection-des.md) ajoute Auto par défaut, sélection géométrique par double-clic et correction locale sans perte des régions indépendantes, avec Tore et sa portée cylindrique. Stratégies historiques dans Avancé, paramètres conservés. **415 tests workspace, 32 bureau, 197 UI, 12 Python et 4 SDK**, Clippy, formatage et builds passent ; revue indépendante acceptée. Essai final body 26mm validé avec 6 cylindres et 3 cônes, tolérances initiales ; paquet `artifacts/FabSTEP.app` reconstruit et empreinte vérifiée. V-1181 dans [VALIDATION](docs/VALIDATION.md) détaille preuves et limites : boîtier arrondi spécialisé en mode avancé, contours couplés ambigus conservateurs, graphique/Fusion non qualifiés et sock_clip absent de son ancien chemin.

## Ajustement de la sélection

Le retour utilisateur sur le double-clic est traité par un balayage manuel indépendant de la reconnaissance : clic gauche maintenu pour ajouter les faces visibles, Maj-clic pour retirer une face, bouton droit pour tourner. Un double-clic sans extension affiche désormais une aide explicite. **204 tests UI**, Svelte et build passent ; qualification graphique impossible ici, restant à faire sur le bureau. Preuves V-1182 dans [VALIDATION](docs/VALIDATION.md), suivi R-057.

## Raccords ouverts cylindre–chanfrein

[R-058](roadmap/items/R-058-raccords-ouverts-cylindre-chanfrein.md), approuvé par l’utilisateur, traite le refus sur la paroi cylindrique de body 26mm en reconstruisant aussi le chanfrein conique partiel et la bande inférieure. Auto et correction guidée sont validés nativement : 1 solide, 14 faces cylindriques et 7 coniques, 144 triangles sélectionnés étendus aux 432 triangles du raccord. Tolérances initiales conservées. Revue indépendante acceptée ; **423 tests workspace, 32 bureau, 208 UI, 12 Python et 4 SDK** réussis, compléments ciblés finaux et analyse statique passent. L’interruption initiale des doctests par conflit d’artefacts a été résolue par un rejeu séquentiel. Build `artifacts/FabSTEP.app` reconstruit, empreinte vérifiée. V-1183 détaille les preuves et la portée demi-circulaire ; correctif accepté par l’utilisateur le 17 novembre (A-040), réimport Fusion encore non confirmé (A-041). Le refus guidé affiche désormais « Correction non appliquée » et le recalcul est directement accessible.

## Chaînes coniques et interface — 17 novembre

R-032 ajoute les chaînes de deux à huit cônes de signes libres avec portées cylindriques résolues et témoins source intérieurs ; revue indépendante acceptée. Les cônes sans portée restent dans R-059. R-042 simplifie ensuite l’inspecteur : détails techniques repliés, Essayer primaire, actions utiles conservées et répétitions retirées. **433 tests workspace, 32 bureau, 208 UI, 12 Python et 4 SDK** réussis ; formatage, Clippy strict, Svelte et builds passent. `artifacts/FabSTEP.app` reconstruit, empreinte vérifiée. Preuves V-1184 ; essais graphiques et Fusion non exécutés, suivis par A-042 à A-044. Lots à tester, sans nouvelle publication distante.

## Mise d’équerre acceptée

Ancienne feuille de route conservée dans [l’archive](docs/ROADMAP-avant-fabboard.md) ; suivi détaillé dans `roadmap/`. L’ancien état complet est conservé dans [DEVLOG](DEVLOG.md), entrée du 13 novembre. Skills de la famille installées localement et consignes partagées par `CLAUDE.md`.

Contrôles **relancés le 13 novembre** : `scripts/check.sh` termine avec code 0 — 391 tests workspace, 32 bureau, 175 UI, 12 Python et 4 SDK ; formatage, Clippy strict, Svelte (0 erreur/avertissement), build UI et contrôle bureau réussis. Onze tests Rust opt-in restent ignorés par la suite ordinaire ; avertissement Vite connu sur la taille du bundle. [Journal local](artifacts/mise-d-equerre/check.log). Le format FabBoard, les liens et la conservation exacte des archives ont aussi été contrôlés. La figure Hairline passe son validateur et ses huit vues Chrome, regardées en clair/sombre et en mouvement. Aucun nouvel essai graphique natif ni Fusion n’est revendiqué. Les idées futures et sujets conditionnels restent dans la feuille de route sans nouvelle autorisation de développement.

Validation documentaire reçue : A-030. Réserves de mise d’équerre : [A-029](roadmap/actions/A-029-valider-la-migration-de-la-feuille-de-route-de.md) pour le détail de migration, [A-031](roadmap/actions/A-031-examiner-la-figure-de-la-bride-percee-de-fabstep.md) pour le parcours de la figure et son rendu intégré non confirmé. La validation d’organisation A-032 a été reçue le 16 novembre ; elle ne vaut pas acceptation des extensions fonctionnelles. La branche dédiée suit désormais `origin/fabboard/mise-d-equerre` ; aucune fusion dans la branche par défaut.

## Contrôle natif complémentaire

Le 17 novembre, le pilotage natif a permis d’ouvrir `two_increasing.stl`, de constater le STEP validé, de sélectionner une face plane et de vérifier son profil DXF (32 × 32 mm, un trou), puis l’aller-retour STEP/source. Barre Essayer observée en anglais clair ; retour aux préférences françaises sombres effectué. Une consigne STEP répétée dans l’inspecteur est supprimée ; elle reste dans la vue 3D. Portée exacte et limites : V-1185. Les actions utilisateur restent ouvertes, sans acceptation déduite de ces contrôles.

## Chantier géométrique en cours

Demande du 17 novembre : poursuivre chaînes coniques, révolutions courbes, arrondis généraux et pièces complexes. Trois tranches bornées sont lancées : R-060 (deux cônes adjacents sans portée, rupture unique), R-061 (deux quarts de tore sur une portée cylindrique commune), R-062 (qualification des arêtes droites arrondies et du bac ouvert à coins verticaux). Corpus/test indépendants préparés avant modifications ; écritures des classificateurs et exécutions Cargo sérialisées. Les profils libres, coins triédriques, perçages sécants et rayons variables restent des suites distinctes, avec contrats supplémentaires à établir. Les trois lots passent la validation automatisée V-1186 : 459 tests workspace, 32 bureau, 208 UI, 12 Python et 4 SDK ; analyse statique, formatage et builds réussis. `artifacts/FabSTEP.app` reconstruit, empreinte vérifiée. Essais graphiques/Fusion ouverts ; R-062 qualifie des capacités existantes sans nouveau code de production.

L’utilisateur a approuvé l’extension additive des perçages sécants (A-045). R-063 prépare un laboratoire isolé avant R-036 : le solide de référence se réimporte, mais les courbes d’intersection ne sont pas encore toutes certifiées dans le budget. Aucune nouvelle reconstruction de perçages sécants n’est activée dans l’application.


## Lancement manuel du STEP — UX6

R-064 : chaque ouverture affiche seulement le maillage ; « Reconstruire le STEP » dans l’inspecteur lance le premier calcul. 215 tests UI, Svelte et build macOS passent ; parcours natif ouverture → clic → STEP validé observé. Détails et limites du pilotage dans V-1188. Après le premier clic, les réglages gardent leur recalcul habituel.


## Progression réelle — UX7

R-065 affiche les étapes natives, la sous-opération et le corps courant, avec durée et activité indéterminée pendant les appels longs. 219 tests UI, 38 bureau, Svelte, Clippy et build macOS réussis. Vérification visuelle empêchée par le pilote natif ; V-1189 distingue les preuves et limites. Aucun compteur de zones ni pourcentage de temps estimé.


## Gros STL — G1

R-066 conserve le compte exact dans les refus ordinaires. R-067 ajoute l’essai explicite des STL binaires jusqu’à 600 000 triangles dans un processus isolé, Annuler, surveillance RSS à 4 Gio et délai de 30 minutes. Aperçu dense hors fil UI ; correction guidée, recherche de paramètres et coupe source non proposées pour ce nouveau domaine. Le bureau/3MF ordinaires et les plafonds résultat restent inchangés.

**V-1191 : 470 tests workspace, 49 bureau, 1 transfert réel, 8 helper, 36 ciblés avec capacité élargie, 229 UI, 12 Python et 4 SDK réussis ; formatage, Clippy, Svelte et builds passent.** `artifacts/FabSTEP.app` embarque les deux binaires vérifiés. Cube dense à 101 568 triangles reconstruit, annulation/récupération prouvées. Lors de V-1191, FlanschTurboLuftfilter_REV02.STL était refusé au budget de contours (`planar.too_complex`, 108 s), sans STEP ; G2 ci-dessous remplace ce constat courant. Essai graphique complet bloqué par le pilote de dialogue ; qualification utilisateur R-067 ouverte. Détails, mesures et limites : [V-1191](docs/VALIDATION.md).


## Contours denses — G2

R-068 ajoute un filtrage spatial borné, sans changer les prédicats ni les seuils. **V-1192 : 476 tests workspace, 49 bureau, 1 transfert réel, 8 helper, 46 ciblés `large-stl`, 229 UI, 12 Python et 4 SDK réussis** ; benchmark opt-in, formatage, Clippy, Svelte et builds passent. Les deux exécutables de `artifacts/FabSTEP.app` correspondent aux builds vérifiés.

Le fichier Flansch de 540 170 triangles franchit les contours, puis refuse sur `recognition.too_complex` après 108,8 s : **aucun STEP produit**. Le gain mesuré sur les contours isolés ne vaut pas gain global de conversion. R-069 (G3 ci-dessous) prolonge cette optimisation ; R-068 attend sa qualification utilisateur. Mesures, domaine numérique, limites et contrôles : [V-1192](docs/VALIDATION.md). Aucun nouvel essai graphique ou Fusion revendiqué.


## Préparation et reconnaissance denses — G3

R-069 réutilise la préparation mono-corps strictement identique et visite les arêtes locales des patchs lorsque cela évite un scan global. Prédicats, provenance et budgets conservés. Sur Flansch (540 170 triangles), le benchmark d’analyse passe de **99 à 51 s avec égalité bit à bit**. L’essai complet passe de 111 à 66 s, mais **aucun STEP n’est produit** : le budget partagé est désormais épuisé dans la recherche d’arcs ouverts (`partial::candidates`). R-070/G4 traite ce contour ci-dessous.

**V-1193 : 484 tests workspace, 49 bureau, 8 helper, 54 ciblés `large-stl`, 1 transfert réel, 229 UI, 12 Python et 4 SDK réussis**, avec deux benchmarks opt-in, formatage, Clippy, Svelte et builds réussis. `artifacts/FabSTEP.app` contient les deux exécutables vérifiés ; R-069 attend sa qualification utilisateur. Mesures et limites dans [V-1193](docs/VALIDATION.md). Aucun nouvel essai graphique ni Fusion.


## Cercles denses et preuves réutilisées — G4

R-070 corrige le classement des cercles complets quantifiés avant la recherche d’arcs et réutilise les preuves Auto immuables à l’assemblage. Les budgets, tolérances et gardes des profils D restent inchangés. **491 tests workspace, 49 bureau, 8 helper, 63 ciblés `large-stl`, 1 transfert réel, 229 UI, 12 Python et 4 SDK réussis** ; revue indépendante, formatage, Clippy, Svelte et builds passent. Les deux exécutables de `artifacts/FabSTEP.app` sont vérifiés. R-070 attend la qualification utilisateur A-060 ; aucun nouvel essai graphique ni Fusion. Preuves détaillées dans [V-1194](docs/VALIDATION.md).

**Flansch reste refusé sans STEP** : la recherche franchit le contour circulaire initial et atteint la face 7 278, puis épuise le budget sur un contour de 300 points. Aucun gain global de durée n’est revendiqué (73 s lors de cet essai contre 66 s après G3, avec une analyse également plus lente). R-071 ci-dessous traite ce dépassement ; aucun raccourci numérique non prouvé n’est introduit.


## Recherche d’arcs complexes — R-071

Les comparaisons de maximalité utilisent des intervalles cycliques après contrôle d’unicité, avec repli historique et budget inchangé. **496 tests workspace, 49 bureau, 8 helper, 68 ciblés `large-stl`, 1 transfert réel, 229 UI, 12 Python et 4 SDK réussis** ; revue indépendante, benchmark, formatage, Clippy, Svelte et builds passent. Paquet reconstruit, exécutables vérifiés ; R-071 attend l’essai utilisateur A-061. Les preuves et mesures sont dans [V-1195](docs/VALIDATION.md).

**Flansch franchit la reconnaissance** (4 cylindres, 8 cônes), mais conserve 501 900 plans : ses 501 912 faces dépassent le plafond de suivi du résultat de 100 000. Aucun STEP produit. Le refus conserve son code stable et affiche désormais compte et limite ; la réduction ou qualification de ces résultats très facettisés est qualifiée dans R-072. Aucun gain global de durée ni essai graphique/Fusion revendiqué.


## Qualification des gros résultats — R-072

Qualification documentée dans V-1196 ; code de production et application R-071 inchangés. Le regroupement des plans ne suffit pas : 480 024 faces planes précèdent déjà leur sécurisation. Quatre zones courbes refusées couvrent 278 642 triangles, sans famille exacte encore prouvée. La sonde native source a été arrêtée à 241,622 s : RSS maximal observé 2,35 Gio, mais empreinte physique `sample` de 6.9G. Validation native non terminée, aucun STEP. Le suivi mémoire mérite une qualification dédiée ; ni hausse de plafond ni simplification intégrée. R-072 accepté par l’utilisateur (A-062) ; suite globale V-1195 non rejouée, aucun nouveau test graphique/Fusion.


## Suivi mémoire macOS — R-073

Le helper est arrêté dès que le RSS ou l’empreinte physique observée atteint 4 Gio. Lecture libproc vérifiée contre le SDK, sans nouvelle dépendance ; plafond et cadence inchangés. Les seuils, erreurs de lecture et récolte d’un vrai enfant sont testés. **496 tests workspace, 50 bureau, 8 helper, 68 ciblés large-stl, 1 transfert réel, 229 UI, 12 Python et 4 SDK réussis** ; formatage, Clippy, Svelte et builds passent. Revue indépendante acceptée. `artifacts/FabSTEP.app` reconstruit, empreintes des exécutables vérifiées. Détails V-1197 ; essai manuel d’annulation A-063 ouvert, aucun test graphique/Fusion ni nouvelle conversion Flansch revendiqués.
