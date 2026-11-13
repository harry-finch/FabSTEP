# Motifs inspirés de Beautiful UI

Source : [Beautiful UI](https://www.beautifului.dev/), catalogue de composants pour interfaces avec assistance. Les motifs ci-dessous sont des adaptations pour Fab, pas une copie du site. Le starter fournit `TaskList`, `ContextCard`, `ProposalCard`, `RecordsTable` et `AssistantPanel` ; les autres motifs restent à implémenter selon le besoin. Réutiliser les jetons de `jetons.md`, les composants et les conventions clavier ; choisir les motifs selon le besoin réel.

## Langage commun

Privilégier des lignes structurées, séparateurs fins, libellés courts et métadonnées secondaires. Les regroupements servent à distinguer un objet, une source ou une décision : surface existante, bord `--border` si nécessaire, rayon 4 px, espacement 4 / 8 / 12 px. Garder les contrôles courants à 28 px et la hiérarchie typographique Fab. Prévoir des textes lisibles dans les deux thèmes, au clavier et sans dépendance à la couleur. Un encart n'est pas un nouveau panneau décoratif.

## Progression et tâches

- Pour une opération longue, montrer son nom, son état textuel (en attente, en cours, terminé, échec, annulé) et, si disponible, une progression réelle ou le temps écoulé. Ne jamais fabriquer un pourcentage ; si la durée est inconnue, afficher une activité indéterminée.
- Présenter les étapes sous forme de lignes compactes extensibles : titre, état, détail utile et éventuellement nombre d'éléments. Laisser visibles les échecs et leur cause ; ne pas les masquer dans un indicateur animé. Une action Annuler apparaît seulement si elle fonctionne réellement.
- Une animation discrète peut accompagner l'attente sans remplacer le texte ; immobiliser ou supprimer le mouvement avec `prefers-reduced-motion`. Annoncer les changements significatifs avec une région accessible, sans annoncer chaque mise à jour de compteur.

## Tableaux, recherche et navigation

- Utiliser un tableau pour les collections comparables : en-têtes explicites, colonnes alignées, nombres tabulaires, unités visibles, sélection et tri accessibles au clavier. Pour les différences, présenter ancienne et nouvelle valeur, ajout ou suppression en texte et symbole, pas seulement en couleur.
- Les filtres d'état peuvent prendre la forme de boutons compacts avec libellé et nombre réel ; l'état actif reste identifiable sans couleur. Afficher le nombre de résultats, une recherche libellée et un état vide qui distingue absence de données et absence de résultat après filtre.
- Une recherche de commandes ou une navigation latérale ne sont utiles que si le volume le justifie. Les résultats présentent nom et contexte ; focus visible, flèches et Échap lorsque pertinent, retour du focus à l'ouverture précédente.

## Sources, propositions et validation

- Un encart de contexte indique un titre, un extrait pertinent et la source identifiable ; prévoir un accès à la source quand disponible. Distinguer source absente, obsolète ou inaccessible d'une source vérifiée. Ne pas inventer de provenance.
- Une proposition montre ce qui changerait, ses hypothèses et conséquences connues, ses sources et des actions explicites adaptées : examiner, appliquer, rejeter ou modifier. Utiliser une comparaison avant/après lorsque cela clarifie l'effet. Ne pas présenter une suggestion comme un résultat validé.
- Une question d'approbation indique l'action précise et sa portée ; permettre l'annulation et, si nécessaire, la saisie d'une autre réponse. Réserver la confirmation aux décisions qui en ont besoin ; une action annulable ordinaire reste directement réversible.
- Un niveau de confiance n'est affiché que s'il est réellement fourni et compréhensible : préciser sa signification et ses limites ; il ne remplace ni preuve ni validation métier. Aucun indicateur visuel ne certifie la sécurité d'une machine ou d'une opération.

## Assistance conversationnelle facultative

Lorsque le produit comporte déjà une assistance, un panneau compact peut regrouper saisie, réponse et sources. Signaler une réponse en cours, permettre l'arrêt si disponible et conserver les erreurs visibles. Les détails d'étapes ou d'outils peuvent être repliés derrière un résumé compréhensible ; ne pas exposer de raisonnement interne supposé ni prétendre à des opérations non exécutées. Les propositions ne modifient jamais silencieusement le document ou la machine. Aucune interface de chat n'est nécessaire aux fonctions classiques de l'atelier.
