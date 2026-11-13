# Emplacements, tailles et comportement

Dans une application Fab, le centre de l'écran appartient au travail de l'utilisateur et rien n'y est décoratif. Une illustration n'a donc sa place que dans trois situations : **il n'y a pas encore de travail à montrer**, **l'utilisateur décrit un objet qu'il ne voit pas** (aperçu paramétrique), ou **on est hors de l'application**.

## Les six emplacements

| Emplacement | Largeur | `surface` | Mouvement | Figure |
| --- | --- | --- | --- | --- |
| **État vide de l'espace de travail** : aucun document ouvert, bibliothèque vide, recherche sans résultat qui occupe tout l'espace | 240 px | `workspace` | Repos, répond au pointeur | Catalogue, par état |
| **Interruption de l'espace de travail** : moteur indisponible, fichier illisible | 240 px | `workspace` | Repos, répond au pointeur | Catalogue (`plug`…) |
| **Premier lancement** : étape d'accueil de l'assistant (`Dialog`) | 280 px | `elevated` | `play` permis | Figure du métier (maison) |
| **À propos** : en tête de la fenêtre | 200 à 240 px | `elevated` | Repos, répond au pointeur | Symbole du logo en figure (`hairline-adopt`) |
| **Aperçu paramétrique d'une fiche** : en tête de l'inspecteur, pour un objet physique décrit par ses cotes (une fraise) | 240 px | `panel` | Suit les champs ; répond au pointeur | Figure maison **paramétrique** |
| **Hors application** : site, page de téléchargement, README, documentation | jusqu'à 640 px | fond de la page | `play` permis, `intensity` libre | Toutes |

L'espace de travail d'un canevas peut avoir son propre fond (`--canvas-bg`) : il est égal à `--bg-workspace` dans la famille, donc `surface="workspace"` convient. Si une application s'en écarte, régler `--hairline-plate` sur ce jeton à l'endroit de la figure.

## Jamais

- Dans un **panneau latéral** (hors aperçu paramétrique d'une fiche), la barre d'application, la barre d'état, la barre d'outils, un menu, une infobulle, un toast, un message du moteur, une ligne de liste ou de tableau. Un état vide de panneau reste une phrase en muted, avec une icône Lucide de 24 px au plus.
- **À côté du travail** : dès qu'un document, un résultat ou une ligne existe, la figure disparaît. Jamais en filigrane, en fond ou en coin.
- **Deux figures visibles à la fois**, ni une figure par étape d'un assistant : une seule, à l'accueil.
- En **indicateur d'attente** ou de progression : une tâche longue montre son état réel en texte (`TaskList`, barre d'état).
- Pour **dire un état que le texte ne dit pas** : la gravité d'une erreur passe par l'icône, le mot et la couleur de sens du message, pas par le dessin.
- Dans une **fenêtre de saisie ou de confirmation** : la figure y retarde la décision. L'aperçu paramétrique est l'exception, parce qu'il montre la saisie elle-même.

## Composition d'un état vide

De haut en bas, centré dans l'espace de travail : la figure, une phrase, au plus un bouton secondaire. La figure **remplace** l'icône de 24 à 32 px de `fab-design-system` ; jamais les deux.

```svelte
<div class="empty">
  <Figure figure={drawer} label={t('figure.drawer')} />
  <p>{t('tools.empty')}</p>
  <button class="btn" type="button" onclick={add}><Plus size={14} />{t('tools.add')}</button>
</div>

<style>
  .empty {
    display: grid;
    place-content: center;
    justify-items: center;
    gap: var(--s3);
    height: 100%;
    color: var(--text-muted);
    text-align: center;
  }

  .empty p {
    max-width: 320px;
    margin: 0;
  }
</style>
```

- La **phrase** dit ce qui manque et quoi faire, en une ou deux lignes (`--fs-md` hérité, muted, 320 px au plus) : « Aucune fraise dans cette bibliothèque. Ajoutez la première, ou importez un catalogue. » Elle distingue l'absence de données de l'absence de résultat après filtre.
- Le **bouton** est un `.btn` secondaire : l'action principale reste celle de la barre d'application.
- Écart de 12 px (`--s3`) entre les trois ; aucun cadre, aucune carte, aucun titre au-dessus.
- Sous 860 px de large, ou si l'espace de travail fait moins de 360 px de haut, retirer la figure et garder la phrase.

## Premier lancement et À propos

Dans le corps d'un `Dialog` (fond `--bg-elevated`, corps de 300 px de haut au moins) : figure centrée, puis la phrase d'introduction en `--fs-lg`, 12 px d'écart.

```svelte
<div class="welcome">
  <Figure figure={made('endmill')} label={t('figure.endmill')} surface="elevated" width={280} play />
  <p class="lead">{t('welcome.lead')}</p>
</div>
```

- `play` promène un pointeur invisible sur la figure jusqu'à ce qu'une main arrive : c'est le seul endroit de l'application où une figure bouge seule, parce que l'utilisateur ne sait pas encore qu'elle répond.
- La figure n'est présente qu'à l'étape d'accueil ; les étapes suivantes montrent l'interface, pas des dessins.
- À propos : le symbole du logo en figure, 200 à 240 px, au-dessus du nom, de la version et des licences ; sans `play`.

## Aperçu paramétrique

Quand l'utilisateur crée la fiche d'un **objet physique défini par quelques cotes** (une fraise : type, diamètre, longueur utile, queue, nombre de dents), une figure maison peut en montrer la forme et se redessiner à chaque champ modifié. C'est le seul cas où une figure vit dans un panneau et suit des valeurs.

```svelte
<script lang="ts">
  const endmill = made('endmill');
  // Clés de la figure, cotes en mm ; une cote absente reste `null`.
  const shape = $derived({ family: d.family, flutes: d.flutes, diameter: d.diameter_mm, fluteLength: d.flute_length_mm, shank: d.shank_mm, overall: d.overall_length_mm });
</script>

<div class="preview">
  <Figure figure={endmill} label={t('figure.toolPreview')} surface="panel" params={shape} />
  <p class="sub">{t('toolEditor.previewNote')}</p>
</div>
```

- **Sur la fiche, en création comme en consultation** : la figure confirme une saisie, et fait reconnaître une fiche existante d'un coup d'œil. Une seule par inspecteur, en tête.
- **En tête de la fiche, collant** (`position: sticky; top: 0`, fond `--bg-panel`, trait `--divider` dessous) : l'aperçu reste en vue pendant qu'on fait défiler les champs.
- **Proportions, pas cotes.** La figure garde les rapports de la fiche et se cadre toujours dans la même boîte : elle ne remplace ni un schéma à l'échelle (`canevas.md` de `fab-design-system`), ni une valeur calculée. Une ligne en muted le dit (« Aperçu aux proportions de la fiche. »).
- **Honnête** : une cote que la fiche laisse vide est dessinée **en pointillé** avec une valeur vraisemblable, et la ligne en muted dit ce que le pointillé signifie. Une valeur hors domaine ne déforme pas la figure : elle la borne, et c'est le champ qui signale l'erreur.
- **Le changement est animé** par la figure elle-même (700 ms, règle 08) : rien à ajouter. Elle répond toujours au pointeur.
- Une seule figure visible : si la liste voisine affiche un état vide avec figure, celle-ci s'efface pendant la création.
- Écrire la figure : [creer-et-adopter.md](creer-et-adopter.md), « Figure paramétrique ».

## Tailles

- La figure occupe la largeur donnée par `width`, au rapport **5:4** (240 × 192 px par défaut). Ne jamais l'étirer ni la rogner.
- **160 px** au moins (en dessous, la silhouette ne se lit plus), **320 px** au plus dans l'application. Les figures sont jugées à 240 px par leurs auteurs : c'est la taille de référence.
- Le trait garde son épaisseur à toute taille (0,9 px) : ne pas régler `--hairline-stroke`.

## Mouvement

- **Au repos par défaut.** La pose de repos est une composition complète ; la plupart des utilisateurs n'y porteront jamais le pointeur.
- **Répond au pointeur**, puis revient au repos : c'est le seul mouvement dans l'application, hors premier lancement.
- `intensity` reste à **0,5** (valeur par défaut) dans l'application ; plus fort, la réponse sort du registre calme de l'atelier.
- Les deux figures **ambiantes** du catalogue (`phosphor`, `slow`) bougent sans qu'on les touche : hors application seulement.
- **Mouvement réduit** (`prefers-reduced-motion`) : le moteur s'en charge. Les figures ambiantes et `play` s'arrêtent, les ressorts atterrissent d'un coup, la figure répond toujours. Rien à ajouter.
- **Coût** : toutes les figures partagent une seule boucle d'animation, qui s'arrête quand tout est au repos ou hors écran ; cohérent avec le rendu à la demande des canevas.

## Textes et accessibilité

- **Aucun mot dans la figure** : rien à traduire dans le dessin.
- `label` est la description lue par les lecteurs d'écran (la figure est une image, `role="img"`). Toujours une clé `figure.<nom>` dans `fr.ts` et `en.ts`, qui décrit l'objet en une phrase courte : « Un meuble à trois tiroirs vides. » / “A cabinet of three empty drawers.” Les descriptions par défaut du paquet sont en anglais : ne jamais les laisser.
- La **légende** que la figure émet (`rest`, `03`, `gap 28.0`) est un code en anglais : on ne l'affiche pas dans l'application.
- La figure n'est **jamais le seul porteur** d'une information : la phrase dit tout. Elle n'a donc pas besoin d'être atteinte au clavier. Exception : `riffle` est un groupe focalisable (flèches) ; le contour de focus est celui de l'application (`--accent`).
- Le trait vif a un contraste supérieur à 11:1 sur toutes les surfaces, dans les deux thèmes ; les autres traits sont volontairement en retrait.
