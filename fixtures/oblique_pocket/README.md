# Poche à un congé et une butée oblique

Le bloc mesure 30 × 20 × 12 mm. Une poche de largeur 10 mm descend au niveau Z = 3 mm. Un seul quart de cylindre concave de rayon 2 mm, parallèle à X et centré à Y = −3 mm et Z = 5 mm, raccorde son fond à la paroi de gauche. L’extrémité à X = −8 mm forme un arc de cercle. L’autre se termine sur le plan X = 8 + aY + b(Z − 5) et forme un arc d’ellipse. Le solide de référence possède onze plans et une face cylindrique.

| STL | Pente (a, b) | Particularité |
| --- | --- | --- |
| `nominal.stl` | (0,5 · 0) | Seize intervalles sur chaque arc et deux bandes le long de X. |
| `reverse.stl` | (−0,5 · 0) | Pente opposée, vingt intervalles, trois bandes et diagonales alternées. |
| `staggered.stl` | (0,5 · 0) | Seize intervalles sur le cercle, vingt-trois sur l’ellipse. |
| `rotated.stl` | (0,5 · 0) | Même géométrie nominale après deux rotations et une translation, en ASCII à 17 chiffres significatifs. |
| `compound.stl` | (0,5 · −0,25) | Le plan terminal est incliné selon Y et Z, avec vingt-trois intervalles et trois bandes. |
| `warped_stop.stl` | (0,5 · 0) | Un sommet au milieu de l’arc terminal du témoin décalé s’écarte de 0,16 mm du cylindre. La butée reste plane et le maillage fermé. |

Le générateur donne la longueur locale de la poche à chaque couple (Y, Z). Son intégrale sur la section vide donne le volume de matière. La section elliptique du cylindre a pour demi-axes `2 × √(1 + a² + b²)` et 2 mm. Dans la poche nominale, le centre de l’ellipse est (6,5 · −3 · 5) mm et le volume idéal de la pièce vaut 5 771,780239856 mm³. Le manifeste fournit les rayons, les directions principales, les deux extrémités de l’arc et son point milieu. Les coordonnées des témoins y sont exprimées après placement. Deux sondes à 0,15 mm de part et d’autre de la butée contrôlent le passage du vide à la matière.

Depuis la racine, `python3 scripts/generate-oblique-pocket-fixtures.py` régénère les six STL et le manifeste avec la bibliothèque standard. Un répertoire de sortie facultatif permet de vérifier la reproductibilité. Le générateur contrôle les incidences orientées des triangles et l’écart de volume dû aux cordes.

Sur la version R-086, Auto convertit le fichier nominal en 46 plans, dont 20 fragments sur la butée oblique. Les sommets de cette butée binaire se situent à moins de 2,14 × 10⁻⁷ mm de son plan théorique. Encodés en ASCII avec 17 chiffres significatifs depuis les coordonnées de construction, les mêmes triangles donnent 27 plans, dont une seule face de butée. Les deux STEP sont facettisés sur le congé.

Une seconde poche de rayon 2,5 mm est dessinée dans FreeCAD. Son [STEP de référence](freecad_reference.step) et le [script de contrôle indépendant](../../scripts/validate-oblique-pocket-freecad.py) permettent de vérifier les courbes sur une autre géométrie.

Pour essayer R-087, ouvrir `nominal.stl` en millimètres, choisir la reconstruction automatique et cliquer sur **Reconstruire le STEP**. Le congé se sélectionne en une seule face. Sur la butée inclinée, le profil DXF contient un arc d’ellipse de demi-axes 2,236068 et 2 mm. Les cinq STL positifs donnent onze plans et un cylindre après réimport. Le témoin déformé conserve ses facettes. [Résultats de validation](../../docs/VALIDATION.md#v-1213--r-087-intégré-step-de-la-poche-à-butée-oblique)
