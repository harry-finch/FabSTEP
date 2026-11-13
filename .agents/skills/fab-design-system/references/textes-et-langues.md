# Textes et langues

## Langues

- **Français = langue de référence**, anglais = traduction complète. Pas de texte en dur dans un composant : tout passe par `t('section.cle', { param })`.
- `locales/fr.ts` définit les clés ; `locales/en.ts` est typé `Shape<typeof fr>` : une clé manquante ou en trop est une erreur TypeScript.
- `Key` (type des clés) est dérivé du dictionnaire : `t('typo.cle')` ne compile pas.
- Paramètres entre accolades : `'{count} perforations retirées.'`.
- Repli : langue courante → français → la clé elle-même (visible, donc repérable).
- Langue choisie mémorisée ; par défaut, celle du navigateur (`en*` → anglais, sinon français). Bascule FR / EN dans la barre d'application, sans rechargement.
- `Intl` : `fr-FR` (16 850,5 ; 23/10/2026) ou `en-GB` (16,850.5 ; 23/10/2026 aussi : dates jj/mm/aaaa dans les deux langues).

## Organisation des clés

Par zone de l'interface puis par objet, une feuille = un texte :

```ts
plate: {
  width: 'Largeur',
  margin: 'Marge',
  marginHint: 'Bande sans perforation le long du bord.',
},
issues: {                    // messages du moteur, par code stable
  plate: { margin_too_large: 'Marge trop grande : il ne reste aucune zone à perforer.' },
},
fields: {                    // nom d'un champ dans un message (« {field} : … »)
  plate: { margin: 'Marge' },
},
errors: { project: { not_fabvents: 'ce n’est pas un projet FabVents.' } },
```

- `xxxHint` : texte de l'infobulle du réglage `xxx`.
- **Messages du moteur** : le moteur renvoie `{ code, severity, field?, value?, count? }` ; l'interface les rend par `issues.<code>`, nom du champ par `fields.<chemin>` (sans les indices), valeur formatée dans l'unité affichée. Le moteur ne produit jamais de phrase.

## Typographie française

| Règle | Exemple |
| --- | --- |
| Espace (insécable) avant `:` `;` `!` `?` et `%` | `Marge : 4 mm`, `35,3 %` |
| Pas d'espace en anglais | `Margin: 4 mm`, `35.3%` |
| Guillemets français avec espaces | `« Mettre le motif à l'échelle »` |
| Apostrophe typographique dans les textes affichés | `l’aide`, `n’est` |
| Virgule décimale | `0,5 mm` |
| Espace entre nombre et unité | `120 mm`, `18 000 tr/min`, `45°` (pas d'espace avant `°` d'angle) |
| Points de suspension (caractère `…`) pour une action qui ouvre une fenêtre ou un sélecteur | `Ouvrir…`, `Importer un contour…` |
| Majuscule au premier mot seulement | `Trou de fixation`, pas `Trou De Fixation` |
| Tiret cadratin pour une valeur absente | `—` |
| Signe moins et multiplication | `−`, `×` (`3 × 2 mm`) |

Helpers fournis : `labelled(label, value)` (espace avant les deux-points en français seulement), `pct(v)`, `num(v, d)`, `fmtLen(mm, units)`, `fmtArea(mm², units)`.

## Unités

- Le moteur et les fichiers travaillent en **millimètres et degrés**, toujours. Les pouces ne sont qu'un affichage (bascule mm / pouces dans la barre).
- Unités courantes : `mm`, `in`, `°`, `%`, `cm²` / `in²`, `mm/min` / `in/min`, `tr/min` / `rpm`, `W` / `kW`.
- Une unité s'affiche à côté de la valeur, jamais dans le libellé (`Largeur [120] mm`, pas `Largeur (mm)`).

## Style d'écriture

- **Libellés** : un nom, court, sans article (« Marge », « Pas », « Garde »).
- **Boutons** : un verbe à l'infinitif ou un nom d'objet (« Exporter », « Utiliser comme contour », « + Trou de fixation »).
- **Infobulles** : une à trois phrases qui disent *à quoi sert* le réglage, avec une image d'atelier si possible (« Rainure : creuser un sillon de la largeur de la fraise sans traverser (logement d'un panneau, d'une charnière). »).
- **Messages** : constat + chiffre, puis ce que fait le moteur ou ce qu'on peut faire (« Trop de perforations (52 000) : augmentez le pas ou l’espacement. »).
- **États vides** : ce qui manque + la première action (« Disposition Chemin : aucun tracé. Tracez-en un dans l’onglet Chemin. »).
- Pas de point d'exclamation, pas d'émoji, pas de « Oups ». Vouvoiement.
- Vocabulaire du métier, expliqué une fois dans l'aide ou l'infobulle : nervure, entraxe, martyr, copeau, passe, pas latéral.
- Traduction anglaise : même sens, même brièveté, termes d'atelier anglais (*stepover*, *chipload*, *bridge*, *pitch*), orthographe britannique ou américaine cohérente dans une application.
