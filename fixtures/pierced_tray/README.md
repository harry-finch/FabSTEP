
## Bac à deux cavités et rebord extérieur

`rim_and_divider.stl` est un STL binaire en millimètres produit par FreeCAD. Le rebord déborde de 2 mm sur les quatre côtés. Les deux cavités sont séparées par une cloison de 5 mm, avec des fonds à 2 et 4 mm. Un perçage de rayon 1,6 mm traverse uniquement le fond gauche. Les seize coins verticaux du corps, du rebord et des cavités ont chacun un quart de cylindre. Les arêtes horizontales du rebord sont vives.

Les deux maillages générés comportent 1 256 et 2 052 triangles. La référence a 21 plans et 17 cylindres, pour un volume de 13 755,66058 mm³. Le script contrôle les deux STEP Auto réimportés, la profondeur des cavités, le perçage, la matière sous le rebord et les deux arcs de chaque coin. Le test Rust vérifie la provenance des seize coins et du perçage, et le refus d’une coque ouverte.

```sh
cargo build --offline -p fabstep-cli
PYTHONPATH=/Applications/FreeCAD.app/Contents/Resources/lib \
  /Applications/FreeCAD.app/Contents/Resources/bin/python \
  scripts/qualify-rimmed-tray-freecad.py artifacts/qualification/tray-rim --cli target/debug/fabstep
cargo test --offline -p fabstep-app --test rimmed_tray
```
