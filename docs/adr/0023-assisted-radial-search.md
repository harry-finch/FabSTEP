# ADR-0023 — recherche assistée de la tolérance radiale

Date : 9 novembre 2026. Statut : accepté.

## Contexte

Un seuil radial plus large ne garantit pas une meilleure reconstruction. Sur la pièce privée de V-1155, 0,000012 mm produit 123 faces, contre 124 à 0,00005 mm ; un petit raccord devient insuffisamment déterminé. L'utilisateur souhaite automatiser les essais de paramètres sans changer le budget d'écart choisi.

## Décision

Une action explicite recherche un réglage parmi au plus sept tolérances radiales : le seuil courant multiplié par 0,5 ; 1 ; 1,2 ; 1,5 ; 2 ; 5 et 10. Les valeurs supplémentaires sont limitées au minimum de 0,01 mm et du budget d'écart ; le seuil courant valide reste présent. Les doublons sont supprimés et les essais ordonnés par tolérance croissante. Il ne s'agit pas d'une optimisation continue ou exhaustive.

Chaque essai passe par la conversion complète du document, avec validation du STEP réimporté et audit de fidélité. Un échec est consigné avec son code stable ; aucun résultat partiel n'est proposé. L'unité, le mode de reconstruction, le soudage, la planéité et le budget d'écart restent inchangés. Les modes analytiques seuls sont concernés. La recherche ne s'applique pas à une session d'indications manuelles.

Parmi les essais validés, le classement privilégie le plus petit nombre total de faces, puis la tolérance radiale la plus stricte à égalité. Les comptes de surfaces courbes et les mesures d'écart accompagnent cette proposition. Ce classement décrit une simplification validée parmi les essais, pas une preuve d'intention de conception ni un optimum géométrique universel.

La recherche native est en lecture seule : les STEP intermédiaires sont libérés, et aucun essai n'est installé dans le cache d'export. L'utilisateur applique explicitement la tolérance proposée ; cela emprunte le parcours ordinaire de changement d'options, avec historique, invalidation et nouvelle conversion complète.

## Concurrence et ressources

Les essais sont séquentiels, hors du fil UI, sous les limites ordinaires d'import et de conversion. L'identité source, la révision et l'époque native sont contrôlées avant et après chaque essai. Une modification invalide la proposition ; elle interrompt la suite à la prochaine frontière d'essai, sans promettre d'interrompre un appel OCCT déjà en cours. L'interface sérialise ce travail avec les conversions et rejette les réponses périmées.

## Validation

Les contrôles exécutés et les limites de qualification sont consignés dans `docs/VALIDATION.md`, V-1157 à V-1159.
