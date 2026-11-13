# Revue : anti-modèles et liste de contrôle

## Anti-modèles (refuser)

| Anti-modèle | À la place |
| --- | --- |
| Couleur hexadécimale dans un composant ou dans le code de dessin | Un jeton existant, ou un nouveau jeton du domaine défini dans les deux thèmes |
| Couleur d'identité sur un bouton, une sélection, un graphique | Accent bleu pour l'action ; identité réservée au logo et au liseré |
| Grandes cartes décoratives ou ombres sur les panneaux | Sections au trait `--divider` ; encart compact si nécessaire à une décision ou une source |
| Bouton principal multiple | Un seul `btn primary` par contexte |
| Segment actif en bleu | Segment en relief (`--bg-elevated` + liseré intérieur) |
| Onglet actif en aplat | Texte primary + soulignement 2 px accent |
| Bandeau d'avertissement pour une adaptation normale | Remarque `info` repliée avec compteur |
| Couleur seule pour un état | Icône + texte |
| Valeur bridée en silence | Message du moteur à code stable, champ souligné |
| Fenêtre modale pour confirmer une action courante | Toast, ou annuler / rétablir |
| « Êtes-vous sûr ? » avant une suppression annulable | Supprimer, et laisser `⌘Z` |
| Libellé avec unité entre parenthèses | Unité dans la colonne d'unité |
| Texte en dur, phrase fabriquée par le moteur | `t('…')`, codes traduits par l'interface |
| Émoji comme icône, deux bibliothèques d'icônes | Lucide, ou pictogramme maison au même trait |
| Icône seule sans infobulle | `aria-label` + `use:tip` avec le raccourci |
| Contrôles de 36–44 px, rayons de 8–12 px, grandes marges « aérées » | 28 px, rayon 4 px, rythme 4 / 8 / 12 |
| Défilement horizontal d'un panneau, texte qui passe sous un bord | Libellé plus court, sélecteur en pleine largeur, troncature avec infobulle |
| Dégradés, images d'ambiance, animations décoratives | Rien : le travail de l'utilisateur est l'image |
| Illustration libre (image de banque, personnage, pictogramme agrandi), ou figure hors de ses emplacements | Figure au trait de `fab-illustration-system`, neutre, là où il n'y a pas encore de travail à montrer |
| Paramètres tous visibles en même temps | Inspecteur contextuel, sections, « ▸ Avancé » |
| Noir pur / blanc pur | Échelle de surfaces et de textes |
| Stockage local sans `try/catch` | Toujours protégé, l'application marche sans |
| Raccourci d'une lettre actif pendant la saisie dans un champ | Ignorer les raccourcis quand le focus est dans un champ |

## Liste de contrôle avant de livrer un écran

**Jetons et thèmes**

- [ ] Aucune couleur en dur (`grep -nE '#[0-9a-fA-F]{3,8}\b|rgb\(' src/components`) hors `app.css`.
- [ ] Vérifié en thème sombre **et** clair ; les jetons du domaine sont définis dans les deux.
- [ ] Identité limitée au logo et au liseré.

**Mise en page et densité**

- [ ] Contrôles de 28 px, rayon 4 px, espacements par jetons.
- [ ] Une propriété par ligne ; libellés de 100 px ; unités alignées.
- [ ] Aucun débordement à 1100 px de large (panneaux de 264 px) ni défilement horizontal.
- [ ] Le travail de l'utilisateur occupe le centre.

**Comportement**

- [ ] Une seule action principale visible.
- [ ] Champs numériques : langue, flèches, défilement du libellé, Entrée / Échap.
- [ ] Toute modification du document est annulable ; un geste = une étape.
- [ ] Le résultat se met à jour sans bouton « Calculer ».
- [ ] Préférences et document mémorisés et repris.

**Messages**

- [ ] Toute valeur adaptée ou retirée est signalée, avec sa gravité juste.
- [ ] Erreur : champ souligné, export désactivé avec la raison.
- [ ] Chaque état a une icône ou un texte en plus de la couleur.

**Textes**

- [ ] Toutes les clés dans `fr.ts` et `en.ts` (`npm run check` sans erreur).
- [ ] Typographie française (espaces avant `:` et `%`, « », ’, virgule décimale).
- [ ] Chaque réglage technique a une infobulle courte et concrète.
- [ ] Les trois segments d'un sélecteur tiennent dans 264 px.

**Accessibilité**

- [ ] Tout est atteignable au clavier ; focus visible.
- [ ] Boutons icônes avec `aria-label` ; groupes avec `role` et `aria-label`.
- [ ] Contraste du texte ≥ 4,5:1 sur les panneaux dans les deux thèmes.
- [ ] Messages dynamiques dans une région `aria-live="polite"`.

**Vérification**

- [ ] Parcouru dans le navigateur (ou l'application) : capture en sombre et en clair, à 1440 px et 1100 px de large.

**Tâches et assistance, si présentes**

- [ ] Progression, échec et annulation reflètent l’état réel ; le résultat précédent n’est pas présenté comme actuel.
- [ ] Sources et modifications proposées sont consultables ; les actions conséquentes demandent une décision explicite.
- [ ] Les encarts servent le contexte, restent compacts et fonctionnent aux deux thèmes et au clavier.
- [ ] Les animations respectent `prefers-reduced-motion` ; l’information reste disponible sans animation.

**Illustrations, si présentes**

- [ ] Figure au trait neutre (jetons `--hairline-*`), à l'un des emplacements prévus, accompagnée d'une phrase qui dit quoi faire ; liste de contrôle de `fab-illustration-system` passée.
