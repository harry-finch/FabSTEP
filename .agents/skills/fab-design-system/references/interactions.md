# Interactions et comportements

## Clavier

Raccourcis de la famille (à reprendre tels quels quand la fonction existe) :

| Touche | Action |
| --- | --- |
| `?` | Aide |
| `Échap` | Fermer le menu / la fenêtre ; sinon revenir à l'outil Sélection ; sinon désélectionner |
| `⌘Z` / `⇧⌘Z` (`Ctrl+Y`) | Annuler / rétablir |
| `⌘O` / `⌘S` | Ouvrir / enregistrer le projet |
| `V` `H` | Outil Sélection / Main |
| Lettre initiale | Autres outils (FabVents : `P` point, `L` ligne, `C` courbe, `R` règle, `I` image) |
| `1` – `9` | Onglets de l'espace de travail, dans l'ordre affiché |
| `F` | Cadrer le travail |
| `Espace` maintenu | Déplacer la vue |
| `⌫` / `Suppr` | Supprimer la sélection |
| `← ↑ → ↓` | Déplacer la sélection de 1 mm (Maj : 10 mm) |

Règles :

- Les raccourcis d'une touche sont ignorés quand le focus est dans un champ (`INPUT`, `TEXTAREA`, `SELECT`, `contenteditable`) et quand une fenêtre est ouverte.
- `⌘` sur macOS, `Ctrl` ailleurs (`e.metaKey || e.ctrlKey`) ; l'aide affiche `⌘`.
- Chaque raccourci figure dans l'infobulle de son bouton (« Cadrer (F) ») et dans le tableau de l'aide.
- Tous les contrôles sont atteignables à la tabulation ; focus visible : contour 2 px `--accent`, décalage 1 px (`:focus-visible`), jamais supprimé.
- Groupes radio (segmentés, outils) : une seule tabulation pour le groupe, flèches pour changer.

## Champs numériques

Voir `NumField` dans [composants.md](composants.md) : langue, Entrée / Échap, flèches (Maj × 10, Alt ÷ 10), défilement par le libellé, sélection au focus. La même grammaire s'applique partout où l'on modifie un nombre (cellule de résultat, cote sur le canevas).

## Annuler / rétablir

- Toute modification du document passe par une seule fonction (`store.edit(fn, mergeKey?)`) qui empile un instantané.
- Les modifications successives d'**un même champ** (même `mergeKey`, ex. `plate.margin`) fusionnent en une seule étape : taper « 125 » ne crée pas trois étapes.
- Un geste continu (glisser une poignée, défiler un libellé) est encadré par `begin()` / `end()` : une étape pour tout le geste.
- Les réglages d'affichage (zoom, thème, langue, unités, onglet) ne sont **pas** dans l'historique.

## Calcul en direct

- Le document est envoyé au moteur à chaque changement (effet Svelte sur `JSON.stringify(document)`).
- Moteur dans un **Web Worker** (WebAssembly) ou un service local : l'interface ne se fige jamais.
- **Un seul calcul en cours** ; s'il arrive des changements entre-temps, seul le dernier est envoyé ensuite.
- Le moteur garde en cache les étages dont l'entrée n'a pas changé.
- Calcul lent pendant un geste (premier calcul du geste > 60 ms) : les suivants attendent le relâchement ; la barre d'état affiche « calcul au relâchement » en orange.
- Erreur bloquante : pas de résultat, l'ancien résultat n'est pas montré comme valide, l'export est désactivé avec la raison en tête du menu.

## Persistance

| Quoi | Où | Quand |
| --- | --- | --- |
| Document en cours | `localStorage` `<app>.project.v<n>` | 400 ms après le dernier changement ; repris au lancement |
| Thème | `<app>.theme.v1` | au choix ; appliqué **avant le premier affichage** par un script dans `index.html` (pas de flash) |
| Langue | `<app>.locale.v1` | au choix ; sinon langue du navigateur (`en*` → anglais, sinon français) ; `lang` de `<html>` mis à jour |
| Unités, onglet, options d'affichage | `<app>.units.v1` (kit) ou un objet `<app>.prefs.v1` (FabVents) | au choix |
| Sections, divulgations | `<app>.section.<id>`, `<app>.disclosure.<id>` | à l'ouverture / fermeture |

Toujours dans un `try { … } catch {}` : le stockage peut être indisponible (navigation privée) et l'application doit fonctionner sans. Les clés sont versionnées (`.v1`) : changer de format = changer de clé ou migrer.

## Fichiers

- **Ouvrir** : `⌘O`, menu Fichier, ou **glisser-déposer n'importe où** dans la fenêtre (voile pointillé bleu). Le type est reconnu par le contenu, pas seulement par l'extension ; un fichier étranger utilisable (SVG ou DXF quelconque, image) est importé dans le bon rôle plutôt que refusé.
- **Exporter** : menu de l'action principale, une ligne par format avec une description d'une ligne (« DXF R12, calques de découpe intérieure et extérieure, en mm »). Le nom du fichier vient du nom du projet (minuscules, tirets).
- Le projet est embarquable dans l'export quand c'est possible (FabVents : projet dans les métadonnées du SVG, rouvrable).
- Format de projet `{ format: "<app>.project", version: n, … }`, lisible par les versions suivantes.
- Une erreur de lecture donne un message clair en toast, par code stable (« forme.json : ce n’est pas un projet FabVents. », « … : projet créé par une version plus récente de FabVents. »), jamais une exception brute.

## Sélection et manipulation (espaces de travail graphiques)

- Clic : sélectionner ; clic dans le vide : désélectionner ; l'inspecteur suit.
- Survol d'un objet : contour plus visible, curseur adapté ; objet sélectionné : couleur d'accent et poignées.
- Glisser : déplacer (une étape d'annulation) ; poignées pour redimensionner ; double-clic ou Alt + clic pour ajouter / retirer un nœud.
- Clic droit : menu contextuel (dupliquer, convertir, supprimer…).
- Molette : zoom autour du curseur ; Espace + glisser ou outil Main : déplacer la vue.
- Les cotes saisies dans l'inspecteur sont exactes ; le canevas sert à placer, l'inspecteur à préciser.

## Premier lancement et aide

- Poste neuf : un assistant court (FabSpeed : langue et thème, mot de passe, machine, premier calcul commenté sur les vraies valeurs) ; revisitable depuis l'aide. Une application plus simple peut se contenter d'un **projet exemple** chargé au premier lancement.
- Aide (`?`) : quatre étapes numérotées du flux de travail, un lexique des termes métier, la légende des étiquettes, le tableau des raccourcis. Une page, pas un manuel.
- Les infobulles sont la documentation principale : chaque libellé technique a son ⓘ.

## Retours visuels

| Situation | Retour |
| --- | --- |
| Action réussie sans effet visible (copie, sauvegarde) | Toast 2,6 s |
| Changement visible (valeur, géométrie) | Aucun message : le résultat parle |
| Valeur adaptée par le moteur | Remarque repliée, compteur |
| Risque pour la pièce ou la machine | Avertissement orange visible, icône + phrase |
| Calcul impossible | Erreur rouge, champ souligné, export désactivé avec la raison |
| Chargement | Pastille orange dans la barre d'état ; écran d'accueil sobre (texte muted centré) au démarrage |

Animations : 120 ms (chevrons, fondu d'infobulle), 160 ms (apparition d'un toast). Une progression peut employer un mouvement discret et utile ; respecter `prefers-reduced-motion` et conserver un état textuel.

## Tâches longues et propositions

Afficher étapes, état et erreurs réels, avec annulation seulement si elle est prise en charge. Une proposition reste distincte du document jusqu’à son application explicite ; voir [beautiful-ui.md](beautiful-ui.md).
