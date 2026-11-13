# Typographie

Police unique : **Inter Variable** (`@fontsource-variable/inter`, importée en tête de `main.ts`), variantes `cv11` et `ss01` activées sur le `body` (chiffres et « a » plus lisibles à petite taille). Corps par défaut 12,5 px, interligne 1,4, lissage `antialiased`.

## Échelle

| Niveau | Taille | Graisse | Couleur | Exemples |
| --- | --- | --- | --- | --- |
| Nom de l'application | `--fs-lg` 14 px | 700, `letter-spacing: -0.01em` | primary | « FabVents » dans la barre |
| Titre de fenêtre | `--fs-lg` 14 px | 600 (h2 par défaut) | primary | « Aide », « Noter un essai » |
| Chiffre principal | `--fs-lg` 14 px dans un inspecteur ; 28 px (`letter-spacing: -0.01em`, interligne 36 px) dans une cellule de résultat | 600 | primary | Taux d'ouverture (FabVents), vitesse de rotation (FabSpeed) |
| Unité d'un chiffre principal | `--fs-md` | 400 | secondary | « tr/min » à côté de 18 000 |
| Nom d'objet / valeur | `--fs-md` 12,5 px | 500–600 | primary | « Trou 5 », valeur d'un champ, nom du projet (600) |
| Texte courant | `--fs-md` 12,5 px | 400 | primary / secondary | Corps de l'aide, lignes de menu |
| Libellé de champ | `--fs-sm` 11,5 px | 400 | secondary | « Largeur », « Marge » |
| Bouton, segment, onglet | `--fs-sm` 11,5 px (onglet du canevas : `--fs-md`) | 500 | secondary → primary | « Exporter », « Centre » |
| Description, unité | `--fs-sm` 11,5 px | 400 | muted | « mm », description d'un format d'export |
| Titre de section | `--fs-xs` 10,5 px | 600, **MAJUSCULES**, `letter-spacing: 0.06em` | `--text-section` | « PLAQUE », « MESURES » |
| Sous-titre de groupe | `--fs-xs` 10,5 px | 600, majuscules, `0.04em` | `--text-section` | « GRILLES DE VENTILATEUR » dans un menu |
| Métadonnées, barre d'état, raccourci | `--fs-xs` 10,5 px | 400 | muted | « ⌘O », « Moteur 0.3.0-dev » |
| Étiquette (`Tag`) | `--fs-xs` 10,5 px | 600 | couleur de sens | « Testé », « Calculé » |

Pas d'autre taille. Pas d'italique (sauf citation). Pas de souligné hors liens survolés.

## Chiffres

- Toute colonne ou valeur qui change en direct prend la classe **`.tabular`** (`font-variant-numeric: tabular-nums`) : les chiffres ne « dansent » pas.
- Champs numériques alignés **à droite**, unité à droite du champ, en muted, dans une colonne de 30 px.
- Formatage selon la langue (`Intl.NumberFormat`) : `16 850,5` en français, `16,850.5` en anglais. Pas de séparateur de milliers dans un champ en cours de saisie.
- Décimales par défaut : longueurs 2 en mm, 3 en pouces ; angles 1 ; comptes 0 ; pourcentages 0 à 1.
- Valeur absente : tiret cadratin `—`.

## Petites capitales de section

Le titre de section est le marqueur le plus reconnaissable de la famille : petit, gris, en capitales espacées, précédé d'un chevron et éventuellement d'un **numéro d'étape** dans un cercle de 16 px (bord `--border-strong`, chiffre 9,5 px 600). Les étapes numérotées (1 Plaque, 2 Disposition…) disent l'ordre de travail dans le panneau de gauche ; le panneau de droite n'est pas numéroté.

## Longueur des textes

- Libellé de champ : 1 à 2 mots, tronqué avec `…` si besoin (largeur 100 px), le texte complet dans l'infobulle ⓘ.
- Libellé de propriété quand ⓘ et ↺ partagent la colonne de 100 px (`NumField` avec retour à la valeur proposée) : **14 caractères au plus**, sans compter sur la troncature (« Rayon de base », « Pas vertical », « Surplus départ » dans FabWeave).
- Segment : 1 mot court. Trois segments doivent tenir dans un panneau de 264 px (« Centre / Entiers / Découper », pas « Par le centre / Trous entiers »).
- Infobulle : 1 à 3 phrases, 280 px de large au plus.
- Message du moteur : 1 phrase, avec le nombre et la valeur (« 3 nervures plus fines que le minimum (la plus fine : 0,8 mm). »).
