# Deux perçages sécants à 60° et 75°

STL binaires en millimètres, issus de deux dessins FreeCAD indépendants. Le grand perçage a un rayon de 5 mm et une longueur de 32 mm, le petit un rayon de 2 mm et une longueur de 30 mm. Leurs axes passent par le centre. Deux paires de plans du bloc sont normales aux axes correspondants et la troisième paire est normale aux deux axes. Les ouvertures sont donc quatre cercles. Les deux branches de l’intersection débouchent dans la même cavité.

Les deux densités de chaque dessin sont conservées ici. À 60°, les maillages comportent 1 088 et 1 562 triangles. À 75°, ils en comportent 1 068 et 1 556. Les quatre STEP Auto réimportés ont six plans et trois faces cylindriques. Les deux tronçons du petit trou traversant partagent le grand perçage.

```sh
PYTHONPATH=/Applications/FreeCAD.app/Contents/Resources/lib \
  /Applications/FreeCAD.app/Contents/Resources/bin/python \
  scripts/qualify-oblique-cross-drills-freecad.py artifacts/qualification/oblique-drills \
    --cli target/debug/fabstep --require-analytic
```

Construire la CLI avec `cargo build --offline -p fabstep-cli` avant la comparaison. Sans `--cli`, le script génère uniquement le corpus et vérifie les dessins. Le script compare le volume à une intégration indépendante des cordes communes, contrôle les lèvres, les cycles de jonction et la matière autour de chacune des quatre bouches. Le test d’échantillonnage de ce script complète le certificat natif continu.

Le laboratoire natif mesure les arêtes communes du STEP de référence, avant et après leur reparamétrage conjoint avec les deux courbes UV. Les six arêtes des deux dessins nécessitent le reparamétrage pour démontrer un écart continu sous 10⁻⁷ mm. Cette expérience vérifie les arêtes du dessin. Elle ne reconstruit pas le maillage STL.

```sh
sdk="$(brew --prefix opencascade)"
out=artifacts/qualification/oblique-drills
c++ -std=c++17 -O2 -fno-fast-math -ffp-contract=off \
  -isystem "$sdk/include/opencascade" scripts/qualify-oblique-cross-drills.cpp crates/fabstep-cad/src/cross_drill_verify.cpp \
  -L "$sdk/lib" -Wl,-rpath,"$sdk/lib" -lTKDESTEP -lTKXSBase -lTKTopAlgo \
  -lTKBRep -lTKG3d -lTKG2d -lTKGeomBase -lTKGeomAlgo -lTKMath -lTKernel -o "$out/cert"
"$out/cert" "$out/angle-60-design.step" "$out/angle-60-certified.step"
"$out/cert" "$out/angle-60-certified.step"
```


Les volumes analytiques valent 23 856,17587 et 21 089,93782 mm³. Pour les quatre maillages, la borne composée mesurée reste comprise entre 0,00528 et 0,01056 mm. La relecture du STEP vérifie les cycles de jonction, les deux courbes UV et leur écart dans le repère physique. Le témoin à trois bouches conserve la matière à l’emplacement de la quatrième bouche absente. Le test multi-corps associe un perçage oblique et un perçage orthogonal, avec provenance exacte pour chaque solide.

Le modèle garde une base orthonormée (U, Y, Z), où Z est le petit axe. Pour un cosinus c et un sinus s, le grand axe est sU + cZ. Le domaine incliné exige ici |c| ≤ 0,51, des axes centrés, des rayons distincts respectant la marge historique et des bouches entièrement contenues dans leurs propres plans. Le champ `large_axis_cosine` vaut zéro dans les anciens documents.

Ajouter `--displaced` à la commande pour décaler le petit perçage de +0,65 ou −0,90 mm le long de la normale commune. Les quatre STL de ce sous-ensemble exercent les deux signes et les deux densités. Le dessin vérifie la position des quatre bouches, les deux branches et la matière près de chaque sortie. Les volumes attendus sont 23 854,860545 et 21 087,665160 mm³. Un décalage dans le plan des deux axes reste hors de ce domaine oblique.
