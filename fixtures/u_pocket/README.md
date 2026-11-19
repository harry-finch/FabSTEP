# Congés en U au fond d’une poche

Ces sept STL proviennent de solides dessinés par opérations volumiques dans FreeCAD. Le bloc mesure 44 × 36 × 14 mm. La cavité est profonde de 11 mm. Deux congés sur des flancs opposés rejoignent un troisième congé au fond de la poche sur deux ellipses distinctes. La quatrième paroi latérale arrête les deux flancs sur des quarts de cercle.

| Pièce | Ouverture (mm) | Rayon (mm) | Volume du dessin (mm³) |
| --- | --- | --- | --- |
| u | 22 × 18 | 2,5 | 17 894,797218 |
| wide | 29 × 15 | 1,75 | 17 428,748259 |
| narrow | 7 × 18 | 2,5 | 20 844,678295 |

Les suffixes coarse et fine correspondent à des déflexions angulaires de 0,2 et 0,105 rad. Le témoin notched-coarse contient une encoche qui interrompt une jonction. La commande suivante régénère le corpus dans un dossier temporaire et relit les STEP exportés par FabSTEP.

```sh
cargo build --offline -p fabstep-cli
PYTHONPATH=/Applications/FreeCAD.app/Contents/Resources/lib \
  /Applications/FreeCAD.app/Contents/Resources/bin/python \
  scripts/qualify-u-pocket-freecad.py /tmp/u-pocket \
  --cli target/debug/fabstep --require-analytic
```

Le script contrôle les volumes par intégration de sections, le matériau de part et d’autre des parois, les deux extrémités circulaires et les deux arêtes elliptiques partagées. Il compare la courbe 3D de chaque arête aux deux courbes paramétriques sur les cylindres incidents. Les tests Rust couvrent également la réflexion du réseau, une déformation locale et des modèles CAO incohérents.
