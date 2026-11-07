# ADR-0005 : Documents 3MF et corps distincts

Statut : accepté et implémenté
Date : 2026-11-06

## Contexte

Les jalons M1/M2 traitent un seul composant STL. Le 3MF contient des unités, des objets réutilisables et des instances placées ; convertir simplement tous les triangles des ressources inclurait des objets absents du build et perdrait les transformations. La priorité reste la fidélité géométrique et un refus explicite lorsque le résultat n'est pas démontré.

## Décision

Ajouter `fabstep-import`, indépendant du noyau CAO, pour lire un sous-ensemble explicite de 3MF Core en mémoire. Il résout la relation OPC StartPart, les ressources et les occurrences du build, compose les transformations dans l'ordre prescrit et convertit les coordonnées en millimètres une seule fois. Une transformation miroir inverse les indices des triangles pour préserver le signe du volume. L'unité du fichier est prioritaire sur les préférences STL ; la CLI refuse `--unit` sur un 3MF. [Spécification Core](https://github.com/3MFConsortium/spec_core/blob/master/3MF%20Core%20Specification.md).

Les extensions requises inconnues et les références Production vers d'autres parties sont refusées, même lorsque l'extension n'est pas déclarée requise. Cette première version ne prétend pas importer tous les projets 3MF de trancheurs. Les propriétés non géométriques ignorées font l'objet d'un diagnostic. [Extension Production](https://github.com/3MFConsortium/spec_production/blob/master/3MF%20Production%20Extension.md).

Chaque occurrence reste distincte. Le core ajoute des API d'analyse de maillages indexés en mm et de séparation des composantes connexes par arêtes. Les API monocomposants existantes gardent leur contrat. La séparation topologique n'est pas une preuve de séparation spatiale : des coques imbriquées pourraient représenter une cavité, et les orienter individuellement vers un volume positif ne doit jamais remplir cette cavité.

Le noyau natif vérifie donc la séparation de tous les corps avant reconstruction, sur les triangles source, puis sur les solides reconstruits et après réimport du document STEP. Des boîtes englobantes disjointes suffisent ; sinon une distance native strictement positive au-delà de la précision OCCT est exigée. Tout contact, recouvrement, imbrication, coïncidence ou résultat indéterminé bloque le document. Le contrôle a un budget d'admission fondé sur les nombres de faces ; ce n'est pas une garantie de temps d'exécution natif.

Cette politique est conservative. Le Core demande une union géométrique des items qui se chevauchent pour la fabrication ; FabSTEP ne prétend pas respecter cette union en conservant artificiellement des volumes superposés. La fusion booléenne reste un travail distinct.

Le STEP contient une racine par solide en coordonnées monde. Aucun historique ni arbre d'assemblage XCAF n'est créé. Chaque racine est relue et comparée : type de solide, surfaces, volume, position et dimensions. Le volume global est la somme des volumes des corps dont la séparation a été vérifiée. Les noms de produit peuvent être raccourcis ou normalisés pour OCCT, avec un diagnostic ; le nom source et sa provenance restent dans le rapport.

Les limites portent sur le document, avant et après expansion : 32 corps, 100 000 triangles, 300 000 sommets. ZIP32 Stored/Deflate seulement, 64 Mio compressés et par partie décompressée, 128 Mio cumulés et 4 096 entrées. Le préflight XML borne profondeur, attributs et traitement des espaces de noms avant le parseur DOM. DTD, chemins ambigus, ressources externes, doublons et descripteurs ZIP incohérents sont refusés. Les valeurs détaillées sont centralisées dans le [README import](../../crates/fabstep-import/README.md).

## Interface et contrats

Le bureau exécute une lecture bornée pour l'aperçu puis la conversion complète sous la même révision. Un aperçu ne certifie aucune validité CAO. Si un corps échoue, l'aperçu source reste disponible et aucun STEP partiel n'est mis en cache. Les réponses obsolètes sont écartées aux deux étapes et après le dialogue d'export.

Le rapport document contient les corps, leurs identifiants issus du chemin d'instance et de la composante, les unités et les mesures cumulées. L'inspecteur peut afficher le document ou un corps. `check` indique explicitement que la validité CAO et la séparation spatiale n'ont pas été vérifiées. Les champs JSON historiques mono-corps restent disponibles en CLI ; les diagnostics du document sont conservés.

## Alternatives et conséquences

Étendre core avec ZIP/XML aurait mélangé la lecture d'un conteneur et la géométrie pure. Fusionner toutes les instances aurait perdu leur provenance et aurait exigé une sémantique booléenne. Un arbre d'assemblage XCAF ajouterait une représentation native dont ce jalon n'a pas besoin : les placements et chemins source sont conservés pour permettre cette évolution.

Les fichiers volumineux, les extensions Production, les corps en contact et les cavités à coques disjointes restent des refus assumés. La reconnaissance géométrique M2 conserve ses limites et ses budgets. Les essais exécutés sont consignés dans [VALIDATION](../VALIDATION.md).
