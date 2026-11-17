# Zones sphériques post-v1

Corpus analytique indépendant du reconnaisseur, généré avec Python standard par
`python3 fixtures/generate_post_v1_zones.py`. Les dimensions nominales,
volumes intégrés, latitudes et empreintes sont fixés dans `manifest.json`.

Les positifs sont des zones de sphère R10 entre deux vrais plans : z=-4 à 6,
z=2 à 8 et z=-8 à -2. Les variantes couvrent une rotation/translation, des rangées
intérieures irrégulières, une tessellation dense et un STL binaire f32. La variante
concave est une cavité traversante dans un bloc 30×30 entre z=-4 et 6 : les six
supports plans et la paroi sphérique sont présents, sans fond ni pôle ajouté.
Le 3MF contient deux instances séparées, dont une avec miroir Z et translation.

Les négatifs représentent un sphéroïde, un repli local, une tessellation hors
budget d'écart, un triangle source manquant et deux corps tangents. L'intention
nominale ne dispense jamais de la preuve et de la validation native.

Les tests `spherical_zones.rs` core et app contrôlent le nouveau contrat et le
réimport OCCT. Les contrôles FreeCAD indépendants et les essais visuels du paquet
ne sont pas attestés par ce générateur ni par son manifeste. Voir
[VALIDATION](../../docs/VALIDATION.md) pour les essais effectivement exécutés.
