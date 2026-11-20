# ADR-0048 — Parois coupées, profils mixtes et raccords mesurés

Le 20 novembre 2026, les extensions de contrat jusqu’à R-132 ont été approuvées. Les domaines suivants complètent Auto. Ils utilisent les tolérances et les budgets du fichier ouvert.

| Domaine | Preuve exigée à la construction et à la relecture |
|---|---|
| Régions planes voisines d’un tore | Un propriétaire par triangle et une même arête matérielle entre les deux régions. Une plage plane fusionnée peut partager plusieurs arcs distincts avec le tore. |
| Lèvre torique, cône ou zone sphérique coupés en biais | La carte déployée vérifie les deux branches de coupe, les faces voisines et la couverture des triangles. La relecture contrôle les arcs de cercle sur la sphère et les branches hyperboliques du cône. |
| Révolution mixte | Les anneaux séparent les portées cylindriques et la gorge torique ou la portion cubique à retour axial. Les cotes et le sens de parcours des deux extrémités sont contrôlés. |
| Extrusion à flanc cubique | Les sections terminales déterminent la même courbe. Les flancs, la direction d’extrusion, la hauteur et l’éventuel alésage sont vérifiés sur le corps entier. Le STEP conserve une surface d’extrusion identifiable. |
| Coin à trois rayons | Sur un bloc aligné, les trois cylindres déterminent le coin candidat. Le contrôle porte sur le réseau rationnel, ses quatre bords et l’intérieur de chaque triangle associé. |
| Congé sur le débouché d’un perçage | Le premier domaine porte sur un fût d’axe Z percé suivant X, avec une bouche arrondie et l’autre vive. Deux surfaces cylindriques, deux fonds et un raccord annulaire ont chacun leur propre groupe de triangles. La certification du raccord découpe les triangles aux nœuds de la spline périodique. Elle vérifie les deux sens de parcours des rives et l’écart de la surface entière sur chaque cellule de Bernstein. |

Le noyau Rust propose les supports depuis les coordonnées du maillage. Le module CAO construit les surfaces, vérifie le solide, écrit le STEP puis examine les paramètres, les arêtes et les identités des faces relues. La mesure bidirectionnelle sur tessellation complète ces contrôles. Un témoin à bords identiques et à intérieur déplacé vérifie le refus d’un faux raccord. Les cas non démontrés reprennent le parcours Auto disponible.

Les bancs R-130 et R-131 ont comparé le contrôle d’interférence courant à deux groupes simultanés, au découpage par composantes et au filtrage des colliers. Les résultats publics conduisent à conserver le contrôle de production. Le helper conserve ses budgets de 30 minutes et de 4 Gio. Voir [les mesures](../VALIDATION.md#v-1283--ordonnancement-et-interférences-sur-atlas-publics-r-130--r-131).
