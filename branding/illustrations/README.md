# Figure de FabSTEP

Concept choisi par l’utilisateur le 13 novembre 2026 : **une bride percée dont le pointeur révèle les courbes derrière les contours facettisés**. Identifiant : `mesh-flange`.

Emplacement : carte du projet dans FabBoard seulement, conformément au choix utilisateur. Aucune intégration dans l’application ni dépendance npm de production n’est ajoutée.

- Source : [mesh-flange.js](mesh-flange.js), construite avec `hairline-create`.
- Page autonome : [hairline-mesh-flange.html](hairline-mesh-flange.html). Ouvrir avec `?play=1` pour le parcours, `?theme=light` ou `?theme=dark` pour le thème.
- Geste : l’alésage et les perçages deviennent plus circulaires autour du pointeur, sur leurs positions de repos fixes ; l’effet décroît avec la distance. Il s’agit d’une métaphore graphique, pas d’une simulation du reconnaisseur CAO.
- Parcours : moyeu, perçage latéral, perçage avant, puis retour au repos. L’utilisateur doit encore juger ce parcours.
- Règles appuyées : zones fixes (01), portée bornée (03), repos composé (05), occultation (06), boucle au repos (07), ressort continu (08), solides arrondis (09). Un seul trait actif, aucune couleur ou légende dans l’objet.

## Contrôles du 13 novembre 2026

`look.mjs` termine avec code 0 : fichier valide, huit vues dans le cadre, lectures cohérentes, console sans erreur et mouvement stabilisé en 1,8 s au plus. La réponse déplace 47 % de l’encre à 240 px. Les planches clair/sombre, miniature sans légendes et mouvement ont été regardées : bride, épaisseur, moyeu, alésage et perçages lisibles, une marque active, arêtes arrière masquées, transition sans croisement. Le quatrième perçage est occulté par le moyeu depuis cette caméra. Contrôles 13/14 de `look.md` sans objet : ni état de collection vide ni adaptation d’un SVG.

FabBoard reconnaît `mesh-flange` comme figure du projet ; son suivi et ses actions ont été observés dans l’application. Le dessin intégré n’a pas été confirmé visuellement : la fenêtre est devenue indisponible lors du défilement vers la carte. Ce contrôle reste dans l’action A-031.

Les fichiers du moteur et de la page d’essai sont inchangés. La page autonome incorpore le moteur Hairline, sous licence MIT © Lucas Marques ; voir la mention dans son code et [le dépôt source](https://github.com/lucasmarkes/hairline). Les dessins propres au projet suivent GPL-3.0-or-later. Les planches PNG sont générées et exclues de Git.

Reproduire depuis ce dossier :

```sh
node ../../.agents/skills/hairline-create/look.mjs mesh-flange.js --answer 0,0,30 --edge 40,30,8
```
