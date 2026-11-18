# Rebord arrondi des deux côtés d’un récipient

Le STL est la tessellation FreeCAD d’un récipient ouvert de 20 mm de diamètre, avec un fond de 2 mm et une hauteur de 14 mm. Sur le rebord, le rayon extérieur vaut 0,45 mm et le rayon intérieur 0,65 mm. Les deux parties du tore rencontrent la même face annulaire et leurs cylindres respectifs.

Le script construit deux dessins, les maille à deux densités et contrôle le STEP réimporté, ses sept faces, son volume et la matière de chaque côté du rebord.

```sh
cargo build -p fabstep-cli --offline
PYTHONPATH=/Applications/FreeCAD.app/Contents/Resources/lib \
  /Applications/FreeCAD.app/Contents/Resources/bin/python \
  scripts/qualify-rounded-lip-freecad.py artifacts/qualification/rounded-lip \
  --cli "$PWD/target/debug/fabstep"
```
