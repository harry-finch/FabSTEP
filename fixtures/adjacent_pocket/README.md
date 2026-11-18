# Poche à deux congés de fond adjacents

Deux arêtes perpendiculaires d’une poche borgne portent des quarts de cylindre concaves de même rayon. Ils se rencontrent le long d’un quart d’ellipse. Cette courbe appartient aux deux cylindres. À son autre extrémité, chaque cylindre se termine par un quart de cercle sur sa propre butée plane. Le solide de référence comporte onze faces planes et deux cylindres.

| STL | Construction | Référence |
| --- | --- | --- |
| `nominal.stl` | Bloc de 30 × 20 × 12 mm, poche de 16 × 10 mm, fond à 3 mm, rayons de 2 mm. Seize intervalles sur l’ellipse et deux bandes par paroi. | Demi-axes 2√2 et 2 mm, volume 5 781,551628288 mm³. |
| `rotated.stl` | Poche de 18 × 12 mm, rayons de 3 mm. La pièce est tournée et translatée. Vingt-trois intervalles sur la courbe commune et trois bandes par paroi. | Demi-axes 3√2 et 3 mm, volume 5 311,353996706 mm³. |
| `short.stl` | Poche longue de 3,75 mm pour un rayon de 2 mm. Le rail le plus court mesure encore 1,75 mm. | Deux congés partagent le quart d’ellipse malgré la faible distance à la butée. |
| `warped_seam.stl` | Même pièce que le nominal. Un sommet intérieur de l’ellipse est déplacé de 0,14 mm selon X et Y et quitte les deux cylindres. Toutes les facettes incidentes utilisent ce même sommet. | Maillage fermé servant de témoin de refus du raccord analytique. |

Le générateur construit les sections horizontales du vide en coordonnées cartésiennes. Si `L` et `W` sont les dimensions de la bouche, à chaque hauteur sous les tangences la section est un rectangle de dimensions `L − q` et `W − q`. La distance commune `q` découle du cercle de rayon `r`. En intégrant ces sections, la matière conservée par rapport à une poche à angles vifs vaut `(L + W) × r² × (1 − π/4) − r³ × (5/3 − π/2)`. Le dernier terme tient compte de la rencontre des deux arrondis.

Depuis la racine, `python3 scripts/generate-adjacent-pocket-fixtures.py` régénère les quatre STL et leur manifeste. Un répertoire facultatif en argument permet de les reproduire ailleurs. Le générateur vérifie la fermeture orientée des triangles après encodage binaire, le nombre d’arêtes partagées entre les deux congés et l’écart de volume dû aux cordes pour les témoins positifs. Le manifeste donne les intervalles de triangles par surface, les bornes, les centres et points témoins des arcs dans le repère final.

Pour essayer la reconstruction, ouvrir `nominal.stl` en millimètres dans FabSTEP et cliquer sur **Reconstruire le STEP** en mode Automatique. En sélectionnant les arrondis adjacents du fond, on obtient deux faces cylindriques de rayon 2 mm. Dans le STEP réimporté, l’arête de leur jonction est un quart d’ellipse de demi-axes 2,828427 et 2 mm.
