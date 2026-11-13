# Installer, créer, adopter, intégrer

Deux skills du dépôt [lucasmarkes/hairline](https://github.com/lucasmarkes/hairline) dessinent les figures qui ne sont pas au catalogue. **Les employer toujours**, sans en refaire le travail à la main :

- **`hairline-create`** : d'une idée à une figure. Elle propose des concepts, écrit la figure, la valide et la regarde dans un navigateur.
- **`hairline-adopt`** : d'un SVG existant (logo, pictogramme) à une figure. Elle lit le dessin puis passe la main à `hairline-create`.

Cette page dit ce que la famille Fab ajoute autour : où ranger les fichiers, quelles contraintes donner au concept, comment faire entrer la figure dans l'application.

## Ce qu'il faut dans le projet

| Élément | Emplacement | Installation | Sert à |
| --- | --- | --- | --- |
| Paquet `@lucasmarkes/hairline` | `ui/package.json` (dépendance) | `cd ui && npm install @lucasmarkes/hairline` | Figures du catalogue, types du composant |
| Skills `hairline-create` et `hairline-adopt` | `.agents/skills/`, liens dans `.claude/skills/`, `skills-lock.json` | `npx skills add lucasmarkes/hairline -s hairline-create hairline-adopt -a claude-code codex -y` (racine du projet) | Dessiner et contrôler une figure |
| `Figure.svelte` | `ui/src/components/` | Copier depuis `assets/` de cette skill | Poser une figure dans un écran |
| `figures.ts` | `ui/src/lib/` | Copier depuis `assets/` de cette skill | Monter une figure maison |
| Jetons `--hairline-*` | `ui/src/app.css` | Blocs de [style.md](style.md) s'ils manquent | Couleurs |
| Node 18 ou plus, Google Chrome | Poste de travail | Déjà là en général ; sinon le dire à l'utilisateur | Contrôles de `hairline-create` |

Remarques :

- La commande `npx skills add` ci-dessus a été vérifiée telle quelle : elle n'interroge personne (`-y`), installe les deux skills pour Claude Code et Codex, et écrit `skills-lock.json`. Versionner `.agents/skills/hairline-*`, les liens et le verrou, comme les autres skills du projet.
- Une skill installée pendant une session n'y est pas encore listée. Lire alors `.agents/skills/hairline-create/SKILL.md` (ou `hairline-adopt/SKILL.md`) et suivre ses étapes à la lettre : le résultat est le même que par `/hairline-create`.
- `look.mjs` installe `playwright-core` à son premier lancement, dans un cache de l'utilisateur (`~/Library/Caches/hairline-look` sur macOS), jamais dans le projet. Il pilote le Chrome installé. Sans navigateur, la skill prévoit un contrôle par le code : le dire à la remise, dans ses mots.
- Rien d'autre à installer : ni React, ni shadcn, ni CDN. L'application reste hors ligne.
- Si le projet tient un inventaire de licences, y ajouter Hairline (MIT) : la notice du paquet, la dépendance npm, et `kernel.js`, qui est du code tiers copié dans le dépôt. Les figures maison sont une œuvre du projet.

## Où vivent les fichiers

```text
<projet>/
├─ .agents/skills/hairline-create/      skill : moteur (kernel.js), page d'essai, contrôles
├─ .agents/skills/hairline-adopt/
├─ skills-lock.json
├─ branding/
│  ├─ <app>-logo.svg                    source du logo (fab-design-system)
│  └─ illustrations/                    dossier de travail des deux skills
│     ├─ <name>.js                      la figure : la source, 200 lignes au plus
│     ├─ hairline-<name>.html           sa page d'essai, autonome (témoin, site)
│     └─ README.md                      concept retenu, date, emplacement dans l'application
└─ ui/
   ├─ public/illustrations/
   │  ├─ kernel.js                      copie exacte de .agents/skills/hairline-create/kernel.js
   │  └─ <name>.js                      copie exacte de branding/illustrations/<name>.js
   └─ src/
      ├─ components/Figure.svelte
      └─ lib/figures.ts
```

- `<name>` est le nom que la figure déclare (`hairline({ name: '…' })`) : en minuscules, en anglais comme les identifiants du code, un mot ou deux avec un tiret (`endmill`, `perforated-plate`).
- Les planches de contrôle (`hairline-<name>-look.png`, `-blind.png`, `-motion.png`, `-source.png`, `-source.txt`) ne se versionnent pas : ajouter `branding/illustrations/*.png` et `branding/illustrations/*-source.txt` à `.gitignore`.

## Créer une figure : `hairline-create`

Se placer dans `branding/illustrations/` : la skill écrit la figure, la page et les planches dans le dossier courant, et lance ses scripts par leur chemin (`node ../../.agents/skills/hairline-create/look.mjs …`).

```text
/hairline-create une fraise deux dents au-dessus d'un brut ; le pointeur règle la profondeur de passe
```

La skill déroule cinq étapes (concept, construction, contrôle, remise, retouche). Ce que la famille Fab y ajoute :

**Au concept**

- L'objet vient de **l'atelier** et dit le métier de l'application en une image, comme le symbole du logo : une fraise, une plaque perforée, une buse, un portique. Pas un écran, pas un schéma, pas un personnage.
- Le pointeur reçoit **la grandeur que l'application règle** (profondeur de passe, taille des perforations, couche) : la figure montre en petit ce que l'application fait en grand.
- La **pose de repos** porte tout : dans l'application, la plupart des utilisateurs ne toucheront pas la figure. Pour un état vide, suivre la section « An empty state » de `concepts.md`.
- Ni lettre, ni chiffre, ni logo, ni flèche (règle 10) ; pas de couleur à prévoir (règle 04) : la figure sera rendue dans les gris de l'application.
- Présenter les **deux ou trois concepts en français** à l'utilisateur et attendre son choix. Ne pas choisir à sa place, même si la skill le permet « quand il n'y a personne à qui demander ».

**Au contrôle**

- Faire tourner `look.mjs` jusqu'à la sortie 0, puis lire la planche et répondre aux questions de `look.md`, sans en sauter. La planche montre la palette d'origine de Hairline, en clair et en sombre.
- Regarder ensuite la figure **dans l'application**, sur sa surface réelle et dans les deux thèmes (étape « Intégrer ») : c'est là que se juge l'accord avec les jetons Fab.

**À la remise**

- La page `hairline-<name>.html` reste dans le dépôt, à côté de la figure. Ne pas la publier ailleurs sans que l'utilisateur le demande.
- Dire, comme la skill le prévoit : la métaphore, les règles sur lesquelles la figure s'appuie, le parcours (`tour`), et ce qui n'a pas pu être vérifié. Demander à l'utilisateur d'ouvrir la page avec `?play=1` et de dire si le parcours raconte bien la figure.
- Noter dans `branding/illustrations/README.md` le concept retenu, la date et l'emplacement prévu.

**À la retouche**

- Ne modifier que `<name>.js`, relancer `look.mjs`, puis **recopier** la figure dans `ui/public/illustrations/`.

## Adopter un SVG : `hairline-adopt`

Pour le **symbole du logo** (À propos, premier lancement, site) ou un pictogramme maison qui mérite une figure.

```text
/hairline-adopt branding/fabspeed-logo.svg le symbole seul, sans la tuile ; il tourne quand le pointeur passe
```

- La skill lit le dessin (`read.mjs` écrit une image et une fiche de mesures), dit ce qu'elle y voit et quelles parties elle garde, puis reconstruit l'objet **en solides, avec ses proportions** : elle ne trace jamais le SVG et ne l'anime pas tel quel. Si sa lecture est fausse, la corriger avant tout concept.
- Dans un logo Fab, l'objet est le **symbole** (`<g id="symbol">`) ; la tuile graphite est un fond : le préciser dans la demande. La couleur d'identité ne passe pas dans la figure, qui reste neutre.
- Le geste reprend ce que le symbole raconte (la rotation pour FabSpeed, les perforations qui grandissent pour FabVents). S'il n'est pas donné, la skill en propose plusieurs : l'utilisateur choisit.
- Un seul fichier à la fois. Pour le logo d'un tiers (fabricant de machine, bibliothèque), ne rien adopter sans l'accord de l'utilisateur : la marque est à son propriétaire, et la skill le rappelle à la remise.

La suite (construction, contrôle, remise, intégration) est celle de `hairline-create`.

## Intégrer une figure maison dans l'application

Une figure maison est un fichier écrit pour le moteur de Hairline (`kernel.js`), que le paquet npm n'expose pas. `lib/figures.ts` charge donc le moteur une fois, puis la figure, et la monte comme le paquet monte les siennes : même boîte, mêmes options, mêmes jetons.

1. Copier le moteur, une fois : `mkdir -p ui/public/illustrations && cp .agents/skills/hairline-create/kernel.js ui/public/illustrations/`.
2. Copier la figure : `cp branding/illustrations/<name>.js ui/public/illustrations/`.
3. La poser : `<Figure figure={made('<name>')} label={t('figure.<name>')} … />`.
4. Ajouter `figure.<name>` à `fr.ts` et `en.ts`.
5. `npm run check`, puis regarder dans l'application : sombre et clair, surface réelle, pointeur dessus, pointeur parti.

À respecter :

- Les deux fichiers sont des **copies exactes** (`diff` sans sortie). Ne pas les retoucher dans `public/`, ne pas les importer depuis `src/`, ne pas les minifier : le moteur est vérifié par son empreinte, et la figure par le validateur de la skill.
- Les fichiers sont servis par l'application elle-même : une politique `script-src 'self'` suffit (Tauri).
- Une figure absente ou cassée laisse sa place vide et écrit une erreur dans la console ; le reste de l'écran n'est pas touché.

## Figure paramétrique

Une figure maison peut suivre les champs d'une fiche (emplacement « aperçu paramétrique d'une fiche »). Elle reste une figure `hairline-create` ordinaire, validée et regardée comme les autres, avec une méthode de plus sur ce que rend `mount` :

```js
return {
  set: (v) => { turn = v; },          // le curseur de la page d'essai, comme toujours
  shape: (tool) => { /* … */ },       // la description : un objet simple, cotes en mm
  destroy: bag.dispose,
};
```

- `Figure.svelte` transmet `params` à `shape()` chaque fois qu'ils changent (comparés par leur JSON : n'y mettre que des nombres, des chaînes et `null`).
- **Sans `shape()`, la figure est complète** : elle se dessine avec des valeurs par défaut, qui sont sa pose de repos sur la page d'essai, au premier lancement et sur le site. Le pointeur garde son geste propre (faire rouler la fraise).
- **Chaque cote passe par un `tween`** (700 ms) : `tset(T.d, tool.diameter, performance.now(), 0)`, puis le dessin lit `tval` dans la boucle. Un type ou un nombre de dents change d'un coup.
- **Toujours dans le cadre** (règle 03) : la figure calcule son échelle à partir des cotes pour remplir la même boîte, et se recentre quand un bout est plus large que l'autre. Borner chaque cote à un domaine dessinable.
- **Ce qui manque se voit** : une cote `null` prend une valeur vraisemblable, et la partie concernée passe en pointillé (classe `dash`).
- **Un seul fichier, 200 lignes au plus** : un corps de révolution se dessine comme l'enveloppe (`hull`) de ses anneaux, ce qui couvre plusieurs profils avec le même code.
- `look.mjs` ne voit que la figure par défaut. Regarder les autres formes **dans l'application**, en saisissant les cas extrêmes (le plus fin, le plus large, chaque type), dans les deux thèmes.
- Exemple : `endmill` de FabSpeed (`FabSpeed/branding/illustrations/endmill.js`).

## Dans une application servie par un binaire

Quand l'interface est embarquée dans un serveur ou une application de bureau (rust-embed, Tauri), vérifier que `illustrations/kernel.js` et `illustrations/<name>.js` sont bien servis : reconstruire l'interface, puis le binaire s'il intègre les fichiers à la compilation. Un 404 sur ces fichiers laisse la figure vide, avec une erreur dans la console.

## Hors application

- `hairline-<name>.html` est une page autonome, sans dépendance : elle s'ouvre depuis le disque et peut être servie par le site de l'application. `?play=1` lance le parcours, `?theme=light` ou `?theme=dark` fixe le thème.
- Pour un README ou une documentation, tirer une image de la pose de repos : `look.mjs <name>.js --zoom rest` écrit `hairline-<name>-rest.png` à trois fois la taille.
- Sur une page web, les figures du catalogue se montent avec le paquet (`drawer(element, { play: true })`), en posant `--hairline-plate` et `--hairline-hi` sur les couleurs de la page.

## Mettre à jour

- Paquet : `npm update @lucasmarkes/hairline`, puis revoir les écrans qui ont une figure.
- Skills : `npx skills update`, puis recopier `kernel.js` dans `ui/public/illustrations/`, reconstruire chaque page (`build.mjs`) et repasser `look.mjs` sur chaque figure maison.
- `figures.ts` suit le montage du paquet à la version 0.5.1 (options `intensity`, `label`, `onRead`, `play`). Si une version ultérieure ajoute une option, l'y ajouter.

## Licence

Hairline est sous licence MIT (© Lucas Marques), compatible avec la GPL-3.0-or-later des applications Fab. Citer Hairline dans les mentions de l'application (À propos).
