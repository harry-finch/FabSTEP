# ADR-0040 — Secteurs toriques à quatre arcs partagés

Accord du 18 novembre 2026 pour ajouter au contrat core/CAO un secteur de tore dont les quatre limites sont prouvées. Première configuration disponible dans R-085.

## Domaine représenté

Sur une bride, un congé peut suivre une fraction de cercle et finir sur deux autres surfaces. `MixedTorus` couvre les révolutions complètes. Le nouveau champ optionnel `RoundedModel.torus_sectors` représente les secteurs et ses éléments référencent quatre arcs orientés de `MixedModel.arcs`. Le type de face STEP et le libellé de sélection sont **Tore**. Les anciens modèles se lisent avec une liste vide. Le douzième groupe de provenance conserve l’ordre des onze groupes antérieurs.

Le premier constructeur prend en charge un secteur de couronne fermé. Le congé extérieur relie le dessus plan à un cylindre coaxial. Ses deux autres bords sont partagés avec les plans radiaux de début et de fin. Le solide comprend aussi un fond et une paroi cylindrique intérieure. L’axe, la direction de départ, l’ouverture, la hauteur et le rayon intérieur définissent ce contexte fermé. La somme des volumes de ses sections annulaires fournit une référence indépendante du noyau CAO.

## Source et transfert

Les quatre grands contours plans de la source déterminent le repère. La reconnaissance vérifie les sept régions, leur incidence et leurs quinze limites communes. Chaque région a une frontière unique. Les triangles orientés et les bords monotones doivent couvrir leur domaine une seule fois. La borne dans les deux sens comptabilise la projection des sommets, la flèche entre échantillons, le terme mixte angle–méridienne, la soudure, l’arrondi et une réserve de transfert de 0,00001 mm. L’écart des disques radiaux à leur plan est contrôlé séparément. Les ajustements du congé et des coins ont leurs diagnostics.

Le constructeur natif attribue chaque face suivant ses bords physiques. Au réimport il contrôle les quinze arêtes, les deux incidences de chaque arête, les courbes dans l’espace et dans les repères des faces, les quatre plans, les deux cylindres, le tore et le volume du solide. L’origine, les axes, les rayons et l’ouverture réinférés sont comparés au domaine attendu avec des tolérances en millimètres. Ces contrôles s’appliquent aussi au second transfert des documents multicorps. Le prétraitement STEP de la première écriture de cette configuration est désactivé pour conserver l’identité des sept faces déjà validées.

## Application à Flansch

Sur Flansch, deux des côtés d’un secteur rejoignent souvent des congés cylindriques tangents. Le constructeur actuel exige des plans radiaux à ces positions. Une intégration locale requiert la preuve des bandes voisines, puis leur raccordement aux autres faces du solide. Les rectangles UV de la paroi torique, les bords matériels et les essais de transfert sont suivis séparément dans V-1208.
