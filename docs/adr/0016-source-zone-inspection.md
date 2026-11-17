# ADR-0016 — Inspection de zones source

Date : 9 novembre 2026. Lot I1.

## Contexte

Le maillage affiché conserve l’ordre des triangles importés. La reconstruction sépare ses composantes et soude ses sommets ; ses identifiants de corps et ses indices locaux ne peuvent donc pas servir directement au clic dans l’aperçu. Une sélection doit aussi rester disponible lorsque la reconstruction échoue.

## Décision

Le noyau partage la partition topologique entre l’analyse et une nouvelle fonction compagnon. Les indices source triés de chaque composante constituent la provenance exacte. Le soudage et une inversion d’orientation ne changent pas cet ordre ; aucune correspondance n’est déduite des coordonnées.

L’application associe chaque composante au corps placé de l’aperçu et à son identifiant d’analyse. La commande bureau `selection_topology` calcule cette table et les voisins source hors fil UI, sans construire de contours analytiques ni appeler OCCT. Elle exige une source et une requête courantes, vérifie à nouveau leur époque après calcul et ne modifie jamais le cache STEP. Les plafonds existants de 32 corps et 100 000 triangles restent applicables. Une topologie invalide est refusée dans son ensemble.

Un mode explicite permet de choisir le triangle visible le plus proche, en tenant compte de tous les corps affichés. Les contours et le surlignage ne sont pas des cibles. Un déplacement, un geste multipoint ou un geste annulé ne devient pas un clic de sélection. Le surlignage utilise les positions source sans recadrer la caméra.

Un worker UI prépare le graphe et calcule les anneaux d’adjacence. Agrandir une zone suit la topologie, sans prétendre reconnaître une surface. Le classement utilise uniquement une table `triangle_faces` exhaustive du résultat courant et sa provenance exacte. Une attribution absente reste explicitement indisponible ; un diagnostic de corps n’est pas présenté comme un diagnostic local.

Une nouvelle source ou une nouvelle unité STL efface la sélection. Un changement d’options conserve les indices source, mais invalide topologie et classement. Les réponses portent des gardes de source, de requête, de sélection et de travail : une réponse tardive ne peut restaurer une zone effacée. Un refus de reconstruction conserve l’aperçu et la sélection ; sans topologie exploitable, le triangle cliqué reste inspectable mais la croissance est indisponible.

## Conséquences

I1 n’ajoute aucun paramètre au moteur de reconstruction et ne change ni la géométrie ni l’autorisation d’export. Les indications guidant la reconnaissance relèvent de I2 et devront être vérifiées par les preuves géométriques existantes. La sélection n’est pas persistée entre sessions.

Les preuves automatisées et les essais graphiques effectivement exécutés sont consignés séparément dans [VALIDATION](../VALIDATION.md).
