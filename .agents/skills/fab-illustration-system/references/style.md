# Style : jetons et règles du trait

## Jetons

Une figure Hairline se colore par cinq propriétés. Dans la famille Fab, elles sont réglées une fois dans `app.css`, à partir des jetons communs ; aucune figure, aucun composant n'y touche.

| Propriété | Rôle | Valeur Fab |
| --- | --- | --- |
| `--hairline-plate` | Remplissage de chaque plaque : le fond sur lequel la figure est posée | `var(--bg-workspace)`, réglée par `Figure.svelte` selon `surface` |
| `--hairline-hi` | Le trait vif : la seule arête mise en avant | `var(--text-primary)` |
| `--hairline-edge` | Les silhouettes | trait vif fondu à 40 % dans la plaque |
| `--hairline-mid` | Tous les autres traits | trait vif fondu à 26 % dans la plaque |
| `--hairline-lo` | Ce qui est en retrait | trait vif fondu à 15 % dans la plaque |

Les trois gris sont **calculés** (40, 26 et 15 % : les rapports de la palette d'origine de Hairline), et non pris dans les jetons de bord : la figure garde ainsi la même hiérarchie sur l'espace de travail, dans une fenêtre ou sur une page, en sombre comme en clair.

Le starter de `fab-design-system` contient déjà ces deux blocs. Pour une application plus ancienne, les ajouter à `ui/src/app.css` :

```css
:root {
  /* … après les rails, avant les jetons du domaine … */

  /* Illustrations au trait (figures Hairline, skill fab-illustration-system) :
     un seul trait, neutre. La plaque vaut le fond sur lequel la figure est
     posée (`Figure.svelte` la règle par surface) ; les trois gris du trait
     en sont tirés plus bas (`[data-hairline]`). Valent pour les deux thèmes. */
  --hairline-plate: var(--bg-workspace);
  --hairline-hi: var(--text-primary);
}

/* … avec les styles de base … */

/* Illustrations au trait : silhouette, trait courant et trait en retrait sont
   le trait vif fondu dans la plaque (40 %, 26 %, 15 % : les poids de
   Hairline). La figure garde ainsi sa hiérarchie sur toute surface, dans les
   deux thèmes. Sans `color-mix`, elle prend les gris d'origine de Hairline.
   `.linework` donne les mêmes gris à un schéma de l'application. */
@supports (color: color-mix(in srgb, red, blue)) {
  [data-hairline],
  .linework {
    --hairline-edge: color-mix(in srgb, var(--hairline-hi) 40%, var(--hairline-plate));
    --hairline-mid: color-mix(in srgb, var(--hairline-hi) 26%, var(--hairline-plate));
    --hairline-lo: color-mix(in srgb, var(--hairline-hi) 15%, var(--hairline-plate));
  }
}
```

Rien à redéfinir dans `:root[data-theme='light']` : ces propriétés ne font que renvoyer à des jetons qui, eux, changent avec le thème.

## La plaque

Les plaques d'une figure sont **remplies**, pas transparentes : une plaque cache ce qui est dessiné derrière elle. Si `--hairline-plate` n'est pas exactement le fond réel, chaque plaque se voit comme une tache.

| Où est la figure | `surface` | Fond |
| --- | --- | --- |
| Espace de travail | `workspace` (défaut) | `--bg-workspace` |
| Fenêtre (`Dialog`), menu | `elevated` | `--bg-elevated` |
| Panneau (non prévu : voir les emplacements) | `panel` | `--bg-panel` |
| Barre (non prévu) | `chrome` | `--bg-chrome` |

Hors application (site, documentation), poser `--hairline-plate` sur le fond de la page, et `--hairline-hi` sur sa couleur de texte.

## Ce qu'on ne règle pas

- **L'option `theme`** du paquet (`auto`, `light`, `dark`) : les jetons suivent déjà le thème de l'application. La laisser à sa valeur par défaut.
- **`--hairline-stroke`** : 0,9 px, l'épaisseur d'origine, proche des bords de 1 px de l'interface.
- **Une couleur par figure**. Pas d'accent pour « la figure active », pas de vert pour « tout va bien », pas de rouge pour une erreur, pas de `--identity`, même pour le logo en figure : l'identité reste au symbole du logo et au liseré (`fab-design-system`). Une figure est toujours neutre ; c'est le texte à côté qui porte le sens.
- **Un cadre, une carte, une ombre, un fond teinté** autour de la figure.

## Les dix règles de Hairline

Toute figure, du catalogue ou maison, les respecte ; `hairline-create` les fait tenir (son `rules.md` en donne le détail et les motifs de refus).

| Règle | Ce qu'elle dit |
| --- | --- |
| 01 · *hit* | Le pointeur est testé contre la pose de repos, jamais contre ce qui bouge : rien ne clignote |
| 02 · *order* | Le mouvement se propage à partir du pointeur, décalé selon la distance |
| 03 · *reach* | La réponse a une borne ; la figure reste composée et dans son cadre à l'extrême |
| 04 · *accent* | Le trait vif est la seule mise en avant : ni aplat, ni halo, ni ombre ; un seul endroit vif à la fois |
| 05 · *rest* | Le repos est une composition, jamais un vide ni une grille plate |
| 06 · *honesty* | Les plaques sont opaques, peintes de l'arrière vers l'avant ; aucune ligne ne traverse ce qu'elle ne devrait pas |
| 07 · *cost* | Une seule boucle d'animation, qui dort hors écran et au repos |
| 08 · *clock* | Un choix discret suit une courbe de 700 ms ; une position continue suit un ressort |
| 09 · *radius* | Tout angle est arrondi ; un solide se réduit à une silhouette et un pli |
| 10 · *quiet* | Aucun mot dans la figure ; l'identité passe par la géométrie |

Quatre d'entre elles sont déjà des règles de la famille, ce qui explique que ces figures s'y fondent : **04** est « couleur = sens » (pas de dégradé, pas d'ombre, une mise en avant qui veut dire quelque chose), **06** est l'honnêteté du dessin, **07** est le rendu à la demande des canevas, **10** laisse tous les textes aux dictionnaires.

## Parenté avec les autres dessins de l'interface

- **Icônes** : Lucide et les pictogrammes maison (24 × 24, trait de 2) restent le langage des contrôles. Une figure n'est pas une grande icône : elle ne s'emploie jamais à moins de 160 px, une icône jamais à plus de 32 px.
- **Canevas et schémas** : ils montrent le travail, à l'échelle, en jetons du domaine. Un schéma peut se dessiner au trait avec les mêmes gris (classe `.linework`, voir `canevas.md` de `fab-design-system`) : il garde ses cotes, ses textes et ses couleurs de sens, ce qu'une figure n'a jamais. Une figure ne représente jamais le travail de l'utilisateur à l'échelle ni une valeur calculée. L'aperçu paramétrique d'une fiche montre des proportions, pas des cotes.
- **Logo** : tuile et symbole plein (`identite-et-logo.md` de `fab-design-system`). Sa figure, tirée du SVG par `hairline-adopt`, en garde les proportions mais pas la couleur.
