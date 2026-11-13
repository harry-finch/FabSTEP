---
name: fab-illustration-system
description: Illustrations des applications Fab (FabSpeed, FabVents, FabWeave et les suivantes) — figures isométriques au trait qui répondent au pointeur, sur le moteur Hairline (@lucasmarkes/hairline). Utiliser pour ajouter ou revoir une illustration (état vide de l'espace de travail, premier lancement, À propos, site ou README d'une application), choisir une figure du catalogue, en dessiner une nouvelle avec la skill hairline-create, transformer un logo ou un pictogramme SVG en figure avec la skill hairline-adopt, ou installer Hairline dans un projet. Emplacements autorisés, tailles, jetons, composant Svelte 5 prêt à copier, procédure d'installation et liste de contrôle. Complète fab-design-system.
---

# Fab illustration system

Les illustrations de la famille Fab sont des **figures Hairline** : un objet dessiné en isométrie, d'un seul trait neutre, posé au repos, qui répond quand le pointeur s'en approche. Elles viennent de [lucasmarkes/hairline](https://github.com/lucasmarkes/hairline) (MIT) : un paquet de trente-trois figures, et deux skills qui en dessinent de nouvelles sur le même moteur.

Cette skill dit **où** une application Fab a droit à une illustration, **laquelle**, **comment l'installer** et comment la faire entrer dans les jetons de [`fab-design-system`](../fab-design-system/SKILL.md), dont elle est le complément : lire d'abord sa philosophie. Le principe « le travail de l'utilisateur est l'image » reste entier. Une figure n'apparaît que là où il n'y a pas encore de travail à montrer, ou hors de l'application.

Une illustration Fab se reconnaît à :

- un **objet d'atelier ou de bureau**, entier et composé, jamais un schéma, un personnage ni une scène ;
- **un seul trait gris**, sans aplat, sans dégradé, sans ombre ; une seule arête vive (`--text-primary`) dit où regarder ;
- **aucune couleur de sens ni d'identité**, aucun mot, aucun chiffre dans le dessin ;
- une figure **au repos** tant qu'on ne la touche pas, qui répond au pointeur et se rendort ;
- une phrase à côté qui dit quoi faire : la figure ne porte jamais l'information seule.

## Trois sources, dans cet ordre

| Besoin | Source | Comment |
| --- | --- | --- |
| Un état courant (rien ici, aucun résultat, moteur hors service…) | Le **catalogue** du paquet `@lucasmarkes/hairline` | [catalogue.md](references/catalogue.md) : la figure de la famille pour cet état |
| Un objet du métier que le catalogue n'a pas (fraise, plaque perforée, buse, portique…) | La skill **`hairline-create`** | `/hairline-create <idée>`, voir [creer-et-adopter.md](references/creer-et-adopter.md) |
| Un dessin qui existe déjà en SVG (symbole du logo, pictogramme maison) | La skill **`hairline-adopt`** | `/hairline-adopt <fichier.svg>`, voir [creer-et-adopter.md](references/creer-et-adopter.md) |

Toujours passer par ces deux skills pour une figure nouvelle : elles tiennent les dix règles de Hairline, valident le fichier et le regardent dans un navigateur. **Ne jamais dessiner une figure à la main**, ni en SVG libre, ni en modifiant le moteur.

## Installer dans le projet

Vérifier chaque point, installer ce qui manque, sans demander (ce sont des ajouts au projet, tous réversibles) :

1. **Le paquet** (figures du catalogue, types) — dans `ui/` : `npm install @lucasmarkes/hairline`. Aucune dépendance, ESM, hors ligne ; React n'est pas requis.
2. **Les deux skills** — à la racine du projet si `.agents/skills/hairline-create` ou `hairline-adopt` manque :

   ```sh
   npx skills add lucasmarkes/hairline -s hairline-create hairline-adopt -a claude-code codex -y
   ```

   Elles arrivent dans `.agents/skills/` (liens dans `.claude/skills/`), avec `skills-lock.json` : versionner les trois. Dans la session en cours elles ne sont pas encore listées : lire `.agents/skills/hairline-create/SKILL.md` et le suivre tel quel.
3. **Le composant et le chargeur** — copier [`assets/Figure.svelte`](assets/Figure.svelte) dans `ui/src/components/` et [`assets/figures.ts`](assets/figures.ts) dans `ui/src/lib/`.
4. **Les jetons** — si `ui/src/app.css` n'a pas `--hairline-plate`, y ajouter les deux blocs de [style.md](references/style.md) (le starter de `fab-design-system` les a déjà).
5. **Les outils de contrôle** des skills : Node 18 ou plus et Google Chrome. Au premier contrôle, `look.mjs` installe `playwright-core` dans un cache de l'utilisateur, hors du projet : rien à ajouter à `package.json`.
6. **Les mentions de licence**, si le projet en tient l'inventaire (`THIRD_PARTY_NOTICES.md`, `ui/public/licenses/`, liste des dépendances) : Hairline est sous licence MIT, et `kernel.js` est du code tiers copié dans le dépôt.
7. `npm run check` (0 erreur, 0 avertissement) et `npm run build`.

Détail, emplacement des fichiers et cas d'une figure maison : [creer-et-adopter.md](references/creer-et-adopter.md).

## Employer une figure

```svelte
<script lang="ts">
  import { drawer } from '@lucasmarkes/hairline';
  import Figure from './components/Figure.svelte';
  import { made } from './lib/figures';
  import { t } from './lib/i18n.svelte';
</script>

<!-- Catalogue : état vide de l'espace de travail. -->
<Figure figure={drawer} label={t('figure.drawer')} />

<!-- Figure maison (hairline-create), dans une fenêtre. -->
<Figure figure={made('endmill')} label={t('figure.endmill')} surface="elevated" width={280} />

<!-- Figure paramétrique : aperçu de la fiche d'une fraise, dans l'inspecteur. -->
<Figure figure={made('endmill')} label={t('figure.toolPreview')} surface="panel" params={shape} />
```

`surface` dit sur quel fond la figure est posée (`workspace` par défaut, `elevated` dans une fenêtre) ; `width` vaut 240 px ; `params` ne sert qu'à une figure maison paramétrique. Mise en page d'un état vide, d'un premier lancement et d'un aperçu : [emplacements.md](references/emplacements.md).

## Règles non négociables

1. **Six emplacements, pas un de plus** : état vide de l'espace de travail, interruption de l'espace de travail, premier lancement, À propos, aperçu paramétrique d'une fiche, et hors application (site, README, documentation). Jamais ailleurs dans un panneau, ni dans une barre, un menu, une ligne de liste, ni derrière un contenu.
2. **Une figure à la fois** à l'écran, et jamais à côté du travail de l'utilisateur : dès qu'il y a quelque chose à montrer, la figure disparaît. Seul l'aperçu paramétrique accompagne une saisie : il *est* ce que l'utilisateur décrit.
3. **Même état, même figure** dans toute la famille ([catalogue.md](references/catalogue.md)).
4. **Aucune couleur propre** : les figures prennent les jetons `--hairline-*` de `app.css`, rien d'autre. Ni accent, ni couleur de sens, ni `--identity`, ni option `theme`, ni couleur en dur.
5. **La plaque vaut le fond** : `surface` nomme la surface réelle. Une figure posée sur un fond qui n'est pas le sien laisse voir ses plaques.
6. **Au repos par défaut** : `play` seulement au premier lancement et hors application ; jamais une figure en guise d'indicateur d'attente ou de progression.
7. **Aucun mot dans la figure** ; sa description (`label`) est une clé `figure.<nom>` présente dans `fr.ts` et `en.ts` ; la phrase qui l'accompagne dit quoi faire.
8. **De 160 à 320 px de large** dans l'application (240 px par défaut), au rapport 5:4, sans cadre, sans carte, sans ombre.
9. **Moteur et page d'essai intacts** : `kernel.js` et `bench.html` ne se modifient jamais ; une figure qui ne marche pas se corrige dans son fichier.
10. **Le concept revient à l'utilisateur** : `hairline-create` propose deux ou trois concepts, l'utilisateur choisit, comme pour un logo.

## Références

| Fichier | Contenu | Quand |
| --- | --- | --- |
| [emplacements.md](references/emplacements.md) | Où une figure est permise, tailles, composition, mouvement, textes et accessibilité | Avant d'ajouter une illustration |
| [style.md](references/style.md) | Jetons `--hairline-*`, surfaces, les dix règles de Hairline et leur parenté avec Fab | Style, thème, nouvelle surface |
| [catalogue.md](references/catalogue.md) | Les figures de la famille par état, les trente-trois figures du paquet | Choisir une figure |
| [creer-et-adopter.md](references/creer-et-adopter.md) | Installation détaillée, `hairline-create`, `hairline-adopt`, intégration d'une figure maison, mise à jour | Figure nouvelle, logo en figure |
| [revue.md](references/revue.md) | Anti-modèles et liste de contrôle | Avant de livrer |

## Façon de travailler

- **Chercher d'abord dans le catalogue.** Une figure nouvelle se justifie par un objet du métier, pas par l'envie d'un dessin de plus.
- **Écrire la phrase avant de choisir la figure** : si l'état vide se comprend sans figure et que l'écran est déjà chargé, une icône de 24 px suffit (règle de `fab-design-system`).
- **Vérifier dans le navigateur**, en sombre et en clair, sur la surface réelle, à 1440 et à 1100 px de large ; passer la [liste de contrôle](references/revue.md), puis celle de `fab-design-system`.
- **Nouvelle décision** (un emplacement, une figure d'état qui manque ici) : la proposer à l'utilisateur, puis l'ajouter à cette skill pour que toute la famille en profite.
