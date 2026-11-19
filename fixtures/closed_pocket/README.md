# Circuit de quatre congés de poche

Le fond d’une poche rectangulaire rencontre ses quatre côtés par des congés cylindriques de même rayon. Chaque cylindre partage un arc elliptique avec chacun de ses deux voisins. Le fond est une face plane de dimensions (largeur − 2r) × (longueur − 2r).

| Dessin | Ouverture (mm) | Rayon (mm) | Aire du fond (mm²) | Volume du solide (mm³) |
| --- | --- | --- | --- | --- |
| loop | 22 × 18 | 2,5 | 221 | 17 921,309022 |
| wide | 29 × 15 | 1,75 | 293,25 | 17 446,779975 |
| narrow | 7 × 18 | 2,5 | 26 | 20 851,071178 |

Les six STL nommés coarse et fine proviennent des trois dessins STEP FreeCAD, tessellés avec des déflexions angulaires de 0,2 et 0,105 rad. Le témoin notched-coarse comprend une encoche traversant la première jonction. Le témoin through-coarse conserve les quatre cylindres et les quatre ellipses, mais son fond a été retiré. Une sonde au centre de l’ouverture vérifie que la conversion conserve le passage. Le script contrôle par intégration le volume compris entre les quatre murs et le fond, puis vérifie la présence ou l’absence de matière aux coins. Il retrouve après réimport les huit courbes paramétriques décrivant les quatre ellipses sur les cylindres.

```sh
cargo build --offline -p fabstep-cli
PYTHONPATH=/Applications/FreeCAD.app/Contents/Resources/lib \
  /Applications/FreeCAD.app/Contents/Resources/bin/python \
  scripts/qualify-u-pocket-freecad.py /tmp/pocket-cycle \
  --closed --cli target/debug/fabstep --require-analytic
```

Le maillage sections utilise un deuxième constructeur indépendant de FreeCAD. Il relie directement 28 sections rectangulaires à des hauteurs non uniformes, en alternant les diagonales des quadrilatères. Il décrit le dessin loop. Pour le régénérer, exécuter `python3 scripts/generate-closed-pocket-fixture.py /tmp/sections.stl`.
