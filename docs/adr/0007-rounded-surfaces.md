# ADR-0007 — Surfaces sphériques et toriques explicites

Statut : accepté ; implémenté en 0.7.0-alpha.1
Date : 6 novembre 2026

## Contexte

La deuxième pièce utilisateur contient une portion sphérique et un raccord torique entre un cylindre et un plan. Elle combine ces surfaces avec des plans, un cône et une encoche cylindrique ouverte. Les anneaux des différentes surfaces n'ont pas tous le même nombre de sommets et la triangulation de la sphère ne suit pas une grille de révolution régulière. La preuve par bandes appariées de M4a ne suffit donc pas.

## Décision

Ajouter le mode explicite `surfaces`, présenté comme « Surfaces arrondies ». Les modes précédents gardent leurs contrats et leurs seuils. Le nouveau mode interprète les surfaces courbes depuis le seul maillage ; aucun STEP de référence n'intervient dans la reconnaissance. Les références CAO servent uniquement aux contrôles indépendants.

`fabstep-core` expose `RoundedRecognition` et `RoundedModel`. Ce dernier contient un `RevolvedModel`, des `MixedSphere` et des `MixedTorus`. Les anneaux partagés restent des données géométriques sérialisables. Les faces de provenance couvrent tous les triangles source et distinguent plans, cylindres complets ou partiels, cônes, sphères et tores. Un pôle de rayon nul exige une preuve issue du maillage.

La reconnaissance doit vérifier la surface entière, son orientation, ses limites et sa couverture topologique. Ajuster les seuls contours ou quelques sommets ne suffit pas. Les écarts dus à l'ajustement et à la substitution des facettes sont bornés et comparés au budget explicite. Aucun seuil initial n'est augmenté automatiquement.

Le premier sous-ensemble couvre les hémisphères avec équateur et pôle source, et les congés toriques en quart de cercle entre cylindre et plan. Il accepte les triangulations irrégulières et combine ces patchs avec les arcs cylindriques ouverts de M2c. Les contours exigent au moins douze échantillons et les pas angulaires restent limités à 45°. La recherche est bornée à 128 contours candidats et par le budget de travail existant. Une chaîne conserve au plus une plage conique et une plage torique.

Pour la sphère, la projection radiale et la somme des angles solides prouvent une couverture orientée d'un hémisphère. Les sommets d'équateur et de pôle sont contraints à leurs limites idéales ; leur déplacement vectoriel entre dans le résidu. Le minimum de norme de chaque facette idéale fournit sa flèche. Pour le tore, l'image paramétrique orientée des triangles couvre le rectangle périodique attendu ; les dérivées secondes bornent l'écart entre la surface et les facettes. Le volume analytique provient du flux de toutes les surfaces. L'audit bidirectionnel natif reste échantillonné et n'est pas annoncé comme une distance de Hausdorff globale.

Le compte des tores reste distinct du compte des congés toriques démontrés : une surface torique n'est pas, par sa seule nature, un raccord tangent. Les raccords non toriques, les congés de rayon variable et les mélanges de plusieurs surfaces libres ne sont pas couverts par ce jalon.

`fabstep-cad` construit de véritables surfaces sphériques et toriques, vérifie les anneaux et les singularités, puis réimporte le STEP. Les comptes des surfaces sont conservés par le parcours multi-corps. `fabstep-app` confronte les mesures réimportées à la reconnaissance et conserve les contrôles de la source exacte, du volume, des dimensions, des distances échantillonnées et de la séparation des corps. Un candidat arrondi non prouvé bloque la conversion ; seule l'absence de candidat autorise le parcours des modes précédents.

## Alternatives examinées

- Assembler de petits cônes pour approcher un congé : cela attribuerait une géométrie différente aux facettes et produirait des arêtes artificielles.
- Généraliser immédiatement aux surfaces libres : cela ajoute un problème distinct de paramétrisation et de raccordement, sans preuve suffisante pour le jalon demandé.
- Utiliser le STEP original pour reconstruire la pièce : cela ne validerait pas un convertisseur STL/3MF et rendrait le résultat dépendant d'une référence indisponible dans l'usage normal.

## Conséquences et validation

L'interface expose un choix supplémentaire, les comptes sphériques et toriques, les preuves de congés et les limites de l'audit de fidélité. L'aperçu conserve le maillage source avec ses frontières reconnues ; il ne constitue pas un aperçu de la BRep exportée.

Le corpus synthétique est indépendant du convertisseur et des fichiers utilisateur. Les surfaces, rayons, volumes, dimensions et placements sont contrôlés après réimport FreeCAD. Les cas peu échantillonnés, déformés ou hors budget sont refusés sans altérer une sortie existante. La deuxième pièce utilisateur est reconstruite depuis STL et 3MF aux seuils initiaux, puis comparée indépendamment au STEP original. Les résultats exécutés figurent dans [VALIDATION](../VALIDATION.md).
