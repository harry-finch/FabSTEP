# Poche à deux congés adjacents de même rayon

Les deux STL binaires en millimètres proviennent de dessins FreeCAD. Chaque poche a deux congés de fond, de rayon 2,5 mm, qui se rencontrent entre des parois verticales à 65° ou à 115°. L’arrêt extérieur de chaque congé est perpendiculaire à son axe. La référence comporte onze plans, deux cylindres concaves et une seule arête elliptique commune.

La section par le plan bissecteur donne un quart d’ellipse de demi-axes r et r/sin(α/2), où α est l’angle intérieur des parois. Les demi-grands axes attendus sont 4,65289749 mm pour 65° et 2,96422262 mm pour 115°. Le volume est vérifié à partir de l’aire de la poche et des deux bandes de congé, en déduisant leur recouvrement dans le coin. Le script compare les deux p-courbes à la même arête 3D, teste la matière au raccord et relit les STEP à deux densités. Il génère également un angle de 40°, hors du domaine retenu, pour lequel Auto conserve les facettes.

```sh
cargo build --offline -p fabstep-cli
PYTHONPATH=/Applications/FreeCAD.app/Contents/Resources/lib \
  /Applications/FreeCAD.app/Contents/Resources/bin/python \
  scripts/qualify-angled-pocket-freecad.py artifacts/qualification/angled-pocket \
    --cli target/debug/fabstep --require-analytic
cargo test --offline -p fabstep-app --test angled_pocket
```

Le domaine de reconnaissance exige un angle strictement compris entre 45° et 135°, des parois perpendiculaires au fond, deux rayons égaux et la couverture des deux cylindres jusqu’à leurs rails et leurs arrêts. Les contrôles de planéité et le budget d’écart saisis restent applicables. Le test Rust vérifie également le repli lorsqu’un sommet de l’arête commune est déplacé.
