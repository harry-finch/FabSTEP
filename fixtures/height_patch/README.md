# Reliefs bicubiques à bord horizontal

Ces deux STL binaires en millimètres proviennent de la tessellation indépendante par FreeCAD de deux solides de 20 × 16 mm. La base se trouve à Z = −2 mm et la rive à Z = 4 mm. Le premier modèle porte une bosse, le second une selle. Les quatre pôles intérieurs diffèrent et les douze pôles de rive sont coplanaires. Le volume analytique vaut respectivement 2 090 et 1 920 mm³.

Le script génère les dessins STEP, deux densités de maillage par dessin et vérifie la relecture des solides ajustés. Les deux STL de densité 1.0 sont conservés ici pour les tests Rust. Depuis la racine du projet, avec FreeCAD installé sur macOS :

```sh
PYTHONPATH=/Applications/FreeCAD.app/Contents/Resources/lib \
  /Applications/FreeCAD.app/Contents/Resources/bin/python \
  scripts/qualify-height-patch-freecad.py artifacts/geometry-run/height --cli "$PWD/target/debug/fabstep"
```
