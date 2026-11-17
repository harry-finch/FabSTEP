# ADR-0034 — Essai des gros STL dans un processus isolé

- Date : 17 novembre 2026
- Statut : acceptée par l’utilisateur (A-055), implémentée, qualification utilisateur ouverte (R-067)

## Contexte

Le fichier utilisateur FlanschTurboLuftfilter_REV02.STL contient 540 170 triangles. La limite historique de 100 000 triangles protège notamment la validation native de la source, qui construit une face par triangle avant de valider le résultat. Une reconstruction compacte ne supprime donc pas le coût du maillage source. L’annulation coopérative aux checkpoints ne peut interrompre OCCT engagé dans un appel.

## Décision

Un helper compilé séparément exécute les essais explicitement acceptés de STL binaires denses. Le bureau ordinaire garde sa limite de 100 000 triangles ; la capacité `large-stl` du helper distingue la source (600 000 triangles / 1 800 000 sommets) des faces et tessellations résultat (100 000). Les budgets de reconnaissance et de fidélité restent inchangés, comme le parcours 3MF. Le nouveau plafond reste une limite d’admission, jamais une promesse de conversion ni de durée.

L’utilisateur voit le nombre de triangles et l’avertissement avant « Essayer quand même ». L’accord concerne le fichier courant et disparaît à son remplacement. Aucune décimation automatique ni suppression des validations n’est introduite.

Le parent échange des fichiers privés bornés avec le helper : source, options et identité du travail en entrée ; progression, rapport JSON, STEP validé et marqueur final en sortie. Le marqueur final ne suffit pas : le parent attend aussi la sortie réussie du processus et revérifie l’identité courante avant publication. Le résultat n’existe dans le cache exportable qu’après ces contrôles. Aucun objet OCCT ne traverse la frontière.

Annuler demande l’arrêt du processus et attend sa récupération. Une nouvelle source/révision invalide aussi le travail ; un résultat tardif ne peut devenir exportable. Les fichiers temporaires sont supprimés après récupération du processus. Une borne de 30 minutes limite également les calculs sans fin ; elle n’est pas une estimation de progression. Sur macOS, une surveillance périodique arrête le processus au-delà de 4 Gio de mémoire résidente ; une indisponibilité du moniteur refuse l’essai. Cette borne vient de la qualification : le cube de 101 568 triangles atteint environ 2,54 Gio pendant la validation source. Ce plafond surveillé ne borne pas exactement le pic entre deux observations et ne mesure pas toute la mémoire de l’application.

## Limites de la première tranche

Le STEP récupéré est `DocumentStep::untracked` : export et inspection restent possibles, mais aucune provenance source n’est inventée. Pour ces nouveaux fichiers denses, la correction guidée, la recherche de paramètres et la coupe du maillage source ne sont pas proposées. Les fichiers déjà pris en charge conservent ces fonctions. Le maillage STL brut reste affichable sans l’aperçu natif limité à 100 000 triangles.

Le helper et le bureau doivent être construits séparément pour éviter l’unification des features Cargo. Le build de développement embarque le helper adjacent au binaire principal ; le paquet autonome v1 existant reste inchangé. Sa future qualification devra inclure le helper et ses dépendances/signatures.

## Validation et retour arrière

Les frontières de tailles, le protocole incomplet, l’annulation réelle et les réponses obsolètes sont testés. La qualification du fichier privé et du cube dense, avec durées/mémoire et issues exactes, est consignée dans V-1191 de VALIDATION ; le parcours graphique reste à qualifier par l’utilisateur. Ne pas déduire une conversion réussie de la seule admission du fichier.

Le parcours est additif et ne migre aucune donnée. Son retrait retire l’essai dense et le helper, sans modifier les contrats ordinaires ni les projets existants.
