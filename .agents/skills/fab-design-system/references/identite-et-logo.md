# Identité et logo

## Principe

Toutes les applications partagent **la même interface** ; seule l'identité change : un logo (tuile commune + symbole propre) et une couleur d'identité. Cette couleur n'apparaît qu'à deux endroits :

1. le **symbole du logo** (barre d'application, favicon, icônes d'application) ;
2. le **liseré de 2 px** en haut de la barre d'application (`--identity`, opacité 0,85).

Elle n'est jamais utilisée pour un bouton, une sélection, un état ou un graphique. Le bleu d'action, le vert, l'orange et le rouge gardent leur sens dans toutes les applications.

## Couleurs de la famille

| Application | Identité (sombre) | Identité (thème clair) | Symbole | Source |
| --- | --- | --- | --- | --- |
| FabSpeed | `#EE46FC` magenta | `#C03FC4` | Pivot et deux anneaux d'arcs interrompus (rotation) | `FabSpeed/branding/fabspeed-logo.svg` (logo V2, 25/10/2026) |
| FabVents | `#B5D93A` chartreuse | `#6F8C12` | Perforations hexagonales dont la taille croît vers le centre | `FabVents/ui/public/fabvents-logo.svg` |
| FabWeave | `#58B5F5` bleu FabCAM | `#438ABA` (3:1 sur `#e6e6e6`) | Deux cercles ondulés entrelacés, 3 ondes, dessus-dessous alternés | `FabWeave/branding/fabweave-logo.svg` |
| FabCAM | `#58B5F5` (logo) | — | — | `FabCAM/logo/FabCAM_A_optical_precision/A0_baseline_app-icon.svg` |
| KiPartManager | `#E8B954` (logo) | — | — | `KiPartManager/src/kipartmanager/ui/branding/source/app_icon.svg` |

Attention : le bleu de FabCAM est proche de `--accent` et l'ambre de KiPartManager proche de `--warning`. C'est acceptable tant que la règle d'usage ci-dessus est tenue (logo et liseré seulement), mais **pour une nouvelle application, choisir une teinte qui ne se confond avec aucune couleur de sens** : ni bleu (accent), ni vert (succès), ni orange / ambre (avertissement), ni rouge (erreur), ni cyan (information). Restent libres : magenta, chartreuse (déjà pris), violet, rose corail, turquoise profond, or pâle… Vérifier aussi l'écart avec les identités existantes.

Exception assumée : **FabWeave reprend volontairement le bleu de FabCAM** (choix de l'utilisateur, 24/10/2026), parce que les deux applications génèrent du G-code. La règle « une teinte distincte pour une nouvelle application » est levée dans ce seul cas, ce qui reste admissible parce que l'identité ne sert qu'au logo et au liseré : les symboles et les noms suffisent à distinguer les deux applications. Une autre reprise de teinte demande le même accord explicite de l'utilisateur.

Choisir l'identité :

1. Une teinte vive, saturée, lumineuse sur la tuile graphite (contraste ≥ 3:1 avec `#181f2a`).
2. Une variante assombrie pour le thème clair (≈ −25 % de luminosité, lisible sur `#e6e6e6`), utilisée seulement pour le liseré.
3. Les deux dans `app.css` : `--identity` dans `:root` et dans `:root[data-theme='light']`.

## La tuile

Commune à toute la famille, dessinée dans un `viewBox` de 128 × 128 :

```svg
<path d="M31 4 L97 4 C111.912 4 124 16.088 124 31 L124 97 C124 111.912 111.912 124 97 124 L31 124 C16.088 124 4 111.912 4 97 L4 31 C4 16.088 16.088 4 31 4 Z"
      fill="#181f2a" stroke="#354759" stroke-width="3.6"/>
```

Carré à coins arrondis de 120 unités, rayon ≈ 27, fond graphite bleuté `#181f2a`, bord `#354759` de 3,6 unités. (Les tuiles plus anciennes de FabCAM et KiPartManager utilisent `#171E28` / `#354250` : très proches, à aligner sur celle-ci pour toute nouvelle application.) Gabarit : [`assets/starter/public/logo.svg`](../assets/starter/public/logo.svg).

## Le symbole

- **Une seule couleur pleine** (l'identité), sans dégradé, sans filtre, sans texte, sans ressource externe ; groupe `<g id="symbol" fill="#…">`.
- **Géométrique et explicite** : cercles, arcs SVG exacts (`A`), segments ; coordonnées arrondies ; formes nommées (`id`) pour pouvoir les retoucher.
- Centré sur (64, 64), contenu dans un disque de rayon ≈ 50 (marge intérieure ≥ 14 unités).
- **Raconte le métier** en une image : rotation de la broche (FabSpeed), perforations à taille variable (FabVents).
- **Lisible à 16 px** : épaisseurs ≥ 6 unités, vides ≥ 4–5 unités (5 unités = 0,6 px à 16 px : c'est la limite). Tester les rendus natifs à 128, 64, 48, 32, 24 et 16 px (rasterisés directement à chaque taille, pas réduits d'une grande image) et une version monochrome (symbole et bord en blanc, tuile en noir).
- Procéder comme pour FabSpeed (`Logo_FabSpeed/FabSpeed_01C_refinement_notes.md`) : plusieurs variantes, planche de comparaison, **choix laissé à l'utilisateur**.

## Fichiers et usages

| Fichier | Usage |
| --- | --- |
| `branding/<app>-logo.svg` (+ `branding/README.md` qui documente la variante retenue, la date, les couleurs) | Source |
| `ui/public/<app>-logo.svg` | Barre d'application, 24 × 24 px, `alt=""` (le nom est écrit à côté) |
| `ui/public/favicon.svg` | Onglet du navigateur |
| `favicon.ico` (16, 32, 48), `favicon-32.png`, `apple-touch-icon.png` (180) | Navigateurs sans SVG ; liens versionnés (`?v=…`) pour vider les caches |
| Icônes macOS / Windows / Linux | Générées depuis le SVG à l'étape de packaging (Tauri) |
| `branding/illustrations/` | Figures au trait de l'application, dont le symbole en figure pour À propos et le premier lancement. Elles restent neutres : la couleur d'identité n'y passe pas (skill `fab-illustration-system`) |

## Dans la barre d'application

Logo 24 px, puis le **nom** (`--fs-lg`, 700, `letter-spacing: -0.01em`, primary), puis un **sous-titre** court en muted (« Motifs de perforation », « Feeds & Speeds »), masqué sous 1100 px. Écart de 8 px entre ces trois éléments. Les noms suivent le schéma **Fab + mot du métier** en un seul mot à capitales internes (FabSpeed, FabVents, FabCAM, FabSBP).
