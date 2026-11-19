# Révolutions avec retour axial

Ces quatre STL ont été maillés depuis deux solides FreeCAD fermés, chacun reconstruit à partir d’un profil de Bézier cubique et de deux disques. La coordonnée radiale varie linéairement. La coordonnée axiale remonte, redescend, puis remonte. La paroi occupe donc plusieurs rayons à la même hauteur. La reconnaissance utilise le rayon comme paramètre sur toute la paroi et exige des témoins de la descente axiale dans huit secteurs.

| Dessin | Rayon initial / final | Pôles axiaux en mm | Retour axial mesuré | Triangles par maillage |
| --- | --- | --- | ---: | ---: |
| flared | 3 / 10,5 mm | 0, 13, −5, 8 | 2,008 mm | 12 634 / 13 740 |
| inverted | 11 / 4 mm | 0, 15, −3, 10 | 1,431 mm | 15 676 / 15 490 |

Le second dessin tourne de 90° et change d’origine. Avec le Python et le PYTHONPATH de FreeCAD, lancer `scripts/qualify-returning-meridians-freecad.py artifacts/c2-return --mesh-dir fixtures/returning_meridian`. Pour contrôler les exports, exécuter depuis la racine du projet :

```sh
for source in fixtures/returning_meridian/*.stl; do
  name=$(basename "$source" .stl)
  cargo run --offline -p fabstep-cli -- convert "$source" --mode auto --unit mm -o "artifacts/c2-return/${name}.step"
done
```

Avec le même interpréteur FreeCAD, lancer le générateur avec `artifacts/c2-return --check`. Il compare les 245 points du support et le volume du dessin indépendant et place 30 sondes de part et d’autre de la paroi pour chaque fichier.
