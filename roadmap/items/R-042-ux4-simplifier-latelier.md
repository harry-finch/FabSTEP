---
id: "R-042"
titre: "UX4 — simplifier l’atelier"
resume: "Donner la priorité à la pièce et aux actions courantes ; retirer les exemples de l’écran principal ; options et données techniques à la demande"
statut: "a-tester"
jalon: "atelier"
cree: "2026-11-13"
maj: "2026-11-13"
---

| UX4 — simplifier l’atelier | Donner la priorité à la pièce et aux actions courantes ; retirer les exemples de l’écran principal ; options et données techniques à la demande | Parcours lisible sans ouvrir Avancé, outils contextuels, alertes essentielles visibles, contrôles clavier et qualification clair/sombre FR/EN | Implémenté ; tests de parcours du contrôleur validés, accueil natif FR sombre et EN clair observé ; qualification graphique complète restante |

## UX4 — simplification de l’interface

L’utilisateur juge l’interface trop chargée en options et données et demande de s’inspirer de Cutaway pour rendre le travail plus agréable. Ce lot prolonge UX1 au-delà des libellés de reconstruction. **Implémenté le 10 novembre 2026 ; qualification graphique partielle.** Vue 3D avec inspecteur unique, exemples retirés, réglages avancés et rapports repliés, outils de sélection contextuels. L’orchestration asynchrone est extraite et testée ; les capacités géométriques et les contrats natifs sont conservés. Voir V-1163/V-1164 dans [VALIDATION](../../docs/VALIDATION.md).

- **Pièce au premier plan** : agrandir l’espace de visualisation, réduire les panneaux permanents et regrouper les commandes dans une barre compacte. Organiser le parcours autour d’ouvrir un fichier, reconstruire, inspecter/corriger et exporter, avec une action principale adaptée à l’état courant.
- **Retirer les modèles d’exemple de l’écran principal** : supprimer les boutons de chargement du cube, cylindre, pièce mixte, etc. du parcours normal. Les fixtures restent disponibles pour les tests ; un éventuel accès de démonstration dans l’aide reste secondaire.
- **Réglages à la demande** : présenter un choix de reconstruction compréhensible ; regrouper les modes spécialisés, tolérances et autres réglages techniques sous **Avancé**, replié par défaut. La recherche de réglage apparaît comme aide contextuelle, sans modifier silencieusement les préférences.
- **Résumé utile plutôt qu’un rapport permanent** : afficher l’état du résultat et la prochaine action ; déplacer les listes de mesures, comptes détaillés, identifiants, réglages appliqués et diagnostics techniques dans **Détails du résultat**. Garder visibles les erreurs bloquantes, les avertissements qui influent sur l’utilisation du résultat et l’unité source lorsqu’elle doit être choisie.
- **Correction contextuelle** : après sélection d’une face ou d’une zone, afficher seulement ses informations pertinentes et les outils applicables. Conserver un accès simple à Source / Résultat STEP, aux contours et au cadrage ; éviter les actions répétées et les longues explications permanentes.
- **Intégrer les futurs DXF sans surcharger l’écran** : proposer l’export du profil au contexte d’une face plane et les réglages de section seulement lorsque l’outil de coupe est actif. Ne pas afficher de commandes pour des fonctions non livrées.

L’inspiration porte sur la clarté du parcours et la place donnée au modèle, avec maintien de l’identité FabApps. La simplification conserve les capacités avancées, les contrôles de validation et l’interdiction d’exporter un résultat périmé ; elle ne masque pas un refus de conversion.

UX4 fournit le cadre de l’intégration visuelle future de DXF1/DXF2. L’accueil natif français sombre a été observé à 1440 × 900 points, avec Avancé et Détails repliés et les exemples absents. Les tests du contrôleur couvrent les transitions de traitement, succès et refus ; ils ne constituent pas des essais graphiques. Le dialogue d’import a maintenu Ouvrir désactivé sur la fixture sélectionnée, puis le pilotage natif a expiré. Restent donc à qualifier le parcours ouvrir → reconstruire → inspecter → exporter, la sélection, le clavier, les petits écrans et les états avec modèle dans les deux langues/thèmes ; l’accueil anglais clair est observé en V-1166. L’essai de filtre par extensions seules ne résout pas le blocage et a été retiré ; aucune cause du blocage graphique n’est attribuée sans preuve.

**Validation utilisateur du 10 novembre :** sélection et ouverture manuelles confirmées fonctionnelles. Le blocage du sélecteur concerne la qualification automatisée ; aucun correctif produit de l’import n’est retenu. La bascule, les corrections et l’export restent à qualifier graphiquement.

### Bascule automatique vers le STEP

Demande utilisateur du 10 novembre 2026, **implémentée ; qualification graphique restante** : dès qu’une conversion courante produit un STEP validé, son aperçu est chargé et la vue passe automatiquement à **Résultat STEP**, y compris après un recalcul ou une correction guidée réussie. Le chargement et la préparation worker doivent réussir avant d’afficher le STEP. Les 61 tests UI passent, dont 25 tests du contrôleur ; voir V-1165 dans [VALIDATION](../../docs/VALIDATION.md). Garder le retour manuel à **Source** disponible. Une réponse périmée ne doit jamais changer la vue ; un refus de conversion ou un échec de chargement de l’aperçu conserve la source et affiche le diagnostic approprié. Les essais de recherche de paramètres ne déclenchent pas cette bascule puisqu’ils ne publient aucun résultat exportable. Les tests de parcours couvrent les déclenchements ; l’essai dans l’application native reste non concluant après blocage du sélecteur de fichier.

Source : [ancienne feuille de route](../../docs/ROADMAP-avant-fabboard.md). Les validations historiques sont conservées ; aucun essai nouveau n’est déduit de la migration.

La livraison de ce lot attend toutes ses vérifications utilisateur rattachées ; une réponse positive à une seule action ne clôt pas les autres.
