# Deux arrondis toriques et une portée — R-061

Corpus en millimètres généré indépendamment par `python3 fixtures/generate_double_torus.py`. Le manifeste fixe les paramètres analytiques et les empreintes avant reconnaissance.

`nominal.stl` : rayon extérieur 8 mm, congés inférieur 1,5 mm et supérieur 2 mm, portée cylindrique de hauteur 5 mm ; hauteur totale 8,5 mm. Résultat attendu : deux plans, un cylindre et deux tores. L’intégrale de volume est la somme du cylindre et des deux quarts de méridienne ; formule dans le générateur. Les variantes couvrent rayons égaux, rotation, échantillonnages non appariés, densité, binaire f32 et un faible écart des cercles terminaux compté dans les résidus. `instances.3mf` contient deux corps séparés, dont un en miroir.

Seules les parois extérieures sont admises dans cette tranche. Portée absente ou insuffisamment prouvée, méridienne déformée, cavité concave et échantillonnage insuffisant restent des négatifs. `touching.3mf` est refusé au budget de séparation dans le parcours testé, sans prétendre avoir qualifié une détection directe du contact.

Utiliser Automatique ou Surfaces arrondies. Contrôles exécutés et limites dans V-1186 de `docs/VALIDATION.md` ; les essais utilisateur et Fusion restent distincts.
