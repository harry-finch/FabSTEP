# fabstep-import

Import géométrique 3MF Core en mémoire, déterministe, sans accès au système de
fichiers ni au réseau. `import_3mf(&[u8])` renvoie les occurrences de maillage du
`build`, les unités source et leur origine (`declared` ou `default`), et une
provenance par occurrence. L'absence de `unit` signifie **millimeter** selon le
format ; ce n'est pas une estimation géométrique.

Les six unités Core sont converties en millimètres après composition des
transformations. Les matrices suivent la convention 3MF ligne-vecteur ; une
réflexion inverse l'ordre des triangles pour préserver l'orientation du volume.
Chaque occurrence de composant conserve un maillage séparé, même quand elle
réutilise la même ressource. Les ressources hors `build` ne sont pas exportées.
Les références doivent suivre leurs définitions ; cycles et références avant
définition sont refusés. Les transformations affines finies, non singulières
sont admises, y compris échelle non uniforme ; la reconnaissance ultérieure
reste responsable de refuser les formes devenues non cylindriques.

Ce jalon prend en charge les objets `model` du Core, pas les extensions
requises. En particulier, l'extension Production et ses références `p:path`
vers plusieurs fichiers modèle sont refusées, même si un producteur omet
`requiredextensions`. Certains fichiers Bambu/Orca ne sont donc pas encore
compatibles. Les propriétés, couleurs et métadonnées facultatives ignorées
font l'objet du diagnostic `threemf.metadata_ignored`. Les noms sont conservés
jusqu'à 1 024 octets UTF-8 ; l'application choisit séparément leur représentation
compatible avec STEP.

## Limites appliquées

- Archive : 64 Mio, ZIP32 uniquement, une table centrale cohérente avec l'EOCD,
  4 096 entrées, table centrale de 8 Mio au plus. Stored et Deflate uniquement,
  sans chiffrement ni extraction de fichiers. Préflight avant allocation de la
  table par la bibliothèque ZIP ; contrôles des en-têtes local/central et des
  plages qui se chevauchent. Les descripteurs ZIP32 avec ou sans signature sont
  vérifiés (CRC et tailles) et inclus dans ces plages. CRC et taille réelle
  vérifiés pour chaque partie lue.
- Parties : 64 Mio chacune, somme des tailles déclarées de toutes les parties
  de 128 Mio au plus, et somme des octets effectivement lus de 128 Mio au plus.
- Noms de parties : 1 024 octets, chemins canoniques internes uniquement,
  doublons et collisions ASCII de casse refusés. `..`, `.`, chemins hôtes,
  antislashs, échappements `%`, fragments et requêtes URI sont refusés.
- XML UTF-8 : DTD interdit ; préflight de profondeur avant le parseur récursif,
  puis contrôle de 64 niveaux racine/texte compris ; un million de nœuds par
  document XML, 64 attributs par balise, deux millions d'attributs cumulés,
  balise de 16 Kio au plus. Aucune entité externe.
- Espaces de noms : avant création du DOM, au plus 32 déclarations cumulées
  sur un chemin, un million de références de portée estimées et dix millions
  d'opérations de copie/résolution estimées (somme des carrés des portées sur
  les balises déclarantes). Les redéfinitions de préfixes sont comptées à
  nouveau par prudence ; certains XML très riches mais valides sont refusés.
  Cela borne notamment le coût de copie des portées héritées dans roxmltree.
- Les éléments structurels Core et OPC refusent le texte non blanc et les
  attributs non qualifiés non prévus. Les feuilles géométriques et OPC ne
  peuvent contenir d'éléments enfants ; les textes de métadonnées sont permis.
- Ressources : 4 096 identifiants, 4 096 références ; total de 100 000 triangles
  et 300 000 sommets avant expansion. Après expansion : au plus 32 occurrences,
  100 000 triangles, 300 000 sommets et 4 096 visites. Profondeur de composants
  limitée à 64. Ces limites sont des plafonds de travail, pas une promesse de
  performance sur toute géométrie.

L'import ne valide pas la fermeture ni les intersections. `analyze_mesh_bodies`
(core) scinde chaque occurrence en composantes fermées par adjacence d'arêtes.
L'application doit appliquer à nouveau le plafond global de 32 corps et faire
refuser par OCCT les contacts, recouvrements et imbrications, sur la source puis
sur le résultat. Aucune union implicite des composants : FabSTEP exporte des
corps CAO séparés et ne prétend pas être un moteur d'impression 3MF conforme
aux opérations d'union de composants se recouvrant.

## Validation et références

`cargo test -p fabstep-import --offline` utilise le corpus indépendant
`fixtures/generate_m3.py` et des paquets construits pour les cas adversariaux :
unités, ordre de transformations, miroir, occurrences, composants, références,
OPC personnalisé, indices, CRC, DTD, profondeur XML et budgets ZIP.

Sources primaires consultées : [3MF Core 1.4](https://github.com/3MFConsortium/spec_core/blob/master/3MF%20Core%20Specification.md),
[Production](https://github.com/3MFConsortium/spec_production/blob/master/3MF%20Production%20Extension.md),
[zip 9.4.1](https://github.com/zip-rs/zip2),
[roxmltree](https://github.com/RazrFalcon/roxmltree).
