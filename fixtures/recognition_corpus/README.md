# Corpus de reconnaissance R-122

Les douze pièces sont des STEP publics indépendants du moteur. Le manifeste fixe la révision amont, l’empreinte SHA-256, la licence, les contributeurs de l’historique et la répartition entre mise au point et vérification. Les STEP sont téléchargés dans le répertoire de travail. Les dérivés STL sont calculés en millimètres avec FreeCAD 1.1.1 et OCCT 7.8.1. Chaque triangulation est vérifiée contre `meshes.json`. Un autre mailleur peut nécessiter une référence de comparaison distincte.

Depuis la racine du dépôt sur le Mac de développement, exécuter :

```sh
cargo build --release --offline --locked -p fabstep-app --example recognition_corpus
PYTHONPATH=/Applications/FreeCAD.app/Contents/Resources/lib \
  /Applications/FreeCAD.app/Contents/Resources/bin/python scripts/qualify-recognition-corpus.py \
  artifacts/recognition-corpus --probe target/release/examples/recognition_corpus
```

Sans `--probe`, la commande reproduit seulement les entrées. `--part flange --part plate` prépare les deux premières pièces. Le premier passage nécessite curl et une connexion pour les références absentes. Le cache vérifié permet ensuite un rejeu hors ligne. Deux conversions indépendantes de chaque STL doivent donner les mêmes mesures géométriques. Chaque conversion dispose de 120 secondes par défaut. Le banc tourne sur macOS ou Linux et garde les limites ordinaires du moteur.

## Lire report.json

- La source de vérité est la face du STEP amont qui a produit chaque triangle. La prédiction est la famille de la face du STEP reconstruit attribuée à ce triangle par la correspondance exacte de FabSTEP. La matrice comptabilise les aires du maillage source en mm². Une paroi cylindrique restée facettisée apparaît à la ligne cylindre et à la colonne plan. La fragmentation à famille identique se lit dans le nombre de faces.
- La couverture est la fraction d’aire correctement attribuée pour une famille présente. Les aires manquées et attribuées à tort sont enregistrées séparément. Une provenance indisponible est signalée. Le nombre de colliers compte les faces de raccord classées `other` sur le chemin atlas.
- La fidélité indique les maxima échantillonnés dans les deux sens, le nombre d’échantillons et la déflexion. Les mesures couvrent sommets, milieux d’arête et barycentres. Le temps et le pic RSS incluent la conversion, le réimport et la lecture de la provenance dans un processus neuf.
- Un refus du moteur conserve son code. Une absence de source, une empreinte différente, une panne de la sonde ou deux sorties géométriques divergentes font échouer le banc. Les deux témoins négatifs FabSTEP vérifient aussi le refus d’une coque ouverte et le repli d’un tore déformé.

Les droits des pièces et les liens d’attribution figurent dans [le manifeste](manifest.json). Les deux témoins négatifs proviennent de [la série M5d](../m5d_manifest.json).
