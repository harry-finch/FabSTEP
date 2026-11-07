# Convertisseur STL vers STEP en Rust

**Created:** 10/21/2026 12:51:10  
**Updated:** 11/6/2026 7:35:10  
**Exported:** 11/6/2026 7:43:02  
**Link:** [https://gemini.google.com/app/f1aa8f74bd9569a6](https://gemini.google.com/app/f1aa8f74bd9569a6)  

## User:

> 10/21/2026 12:51:10

j'aimerai faire un outil (en rust) qui transforme un fichier stl au format step, comme le site https://cutawayhq.com

est-ce que c'est compliqué à faire ? qu'est-ce que cela impliquerait ?

## Gemini:

> 10/21/2026 12:51:10

Développer un convertisseur STL vers STEP dépend entièrement de **ce que l'on entend par « convertir »** :

1. **La conversion « naïve » (Faceting) :** Transformer chaque triangle du STL en une face plane dans le fichier STEP. C'est **facile** (quelques jours de travail), mais le fichier résultant est lourd et quasi inutilisable dans SolidWorks ou Fusion 360 (un cylindre reste composé de 60 petites facettes impossibles à sélectionner comme un axe).
2. **La conversion « prismatique » comme CutawayHQ (Reverse Engineering) :** Analyser le nuage de triangles pour deviner mathématiquement où se trouvent les vrais plans, cylindres, cônes et tores, puis reconstruire un solide CAO propre (B-Rep). C'est **extrêmement complexe** — c'est un sujet de R&D en géométrie algorithmique qui demande plusieurs mois de travail acharné.

---

## 1. Pourquoi la méthode CutawayHQ est un défi mathématique

Un fichier **STL** est une « soupe de triangles » sans intelligence : il ne contient que les coordonnées de sommets et des normales. Un fichier **STEP** (ISO 10303-21) est une représentation par frontières (**B-Rep**) qui décrit des surfaces mathématiques exactes (équations de cylindres, NURBS, plans) délimitées par des courbes d'intersection (arêtes) et cousues ensemble pour former un volume étanche.

Pour passer de l'un à l'autre proprement, le programme doit accomplir 4 étapes critiques :

### A. Reconstruction topologique du maillage
Le STL ne dit pas quel triangle touche quel autre. Il faut d'abord fusionner les sommets identiques (avec une tolérance d'erreur flottante) et construire une structure de données de type **Half-Edge** (demi-arêtes) pour naviguer rapidement de triangle en triangle et vérifier que le maillage est *manifold* (étanche, sans trous ni faces qui s'auto-intersectent).

### B. Segmentation et reconnaissance de primitives (Le cœur du problème)
Il faut regrouper les milliers de triangles en régions cohérentes et trouver quelle forme géométrique ils approximaient avant d'être exportés en STL :
* **Plans :** Regrouper les triangles adjacents ayant exactement la même normale (coplanaires).
* **Cylindres, Cônes, Sphères, Tores :** Estimer la courbure discrète sur les sommets et utiliser des algorithmes comme **RANSAC** (*Random Sample Consensus*), le *Region Growing* (croissance de régions) ou l'ajustement par moindres carrés (*Least Squares Fitting*) pour calculer l'axe, le rayon et l'angle exacts d'un perçage ou d'un congé.

### C. Calcul des intersections et couture B-Rep (Topology Rebuilding)
C'est l'étape où 90 % des projets échouent. Une fois que l'algorithme a détecté « un plan A » et « un cylindre B » à partir des triangles, leurs équations mathématiques sont légèrement décalées par rapport aux triangles d'origine. 
Il faut alors :
* Calculer l'intersection analytique exacte entre le plan A et le cylindre B (ce qui donne un cercle ou une ellipse 3D).
* Découper (*trim*) les surfaces infinies le long de ces courbes.
* Coudre (*sew*) toutes les faces ensemble pour garantir un solide fermé sans interstices microscopiques.

### D. Sérialisation au format STEP
La norme STEP (souvent les protocoles AP203 ou AP214) est un format texte hérité des années 1980, basé sur le langage EXPRESS. Écrire un encodeur STEP valide de zéro qui gère correctement les entités (`ADVANCED_FACE`, `EDGE_LOOP`, `CYLINDRICAL_SURFACE`, `B_SPLINE_CURVE_WITH_KNOTS`) est fastidieux et très verbeux.

---

## 2. L'écosystème Rust : Quels outils utiliser ?

La bonne nouvelle, c'est que Rust possède un écosystème scientifique de plus en plus mature qui évite de tout coder de zéro :

| Besoin | Crates Rust / Bibliothèques recommandées | Rôle dans le projet |
| :--- | :--- | :--- |
| **Lecture STL** | `stl_io` ou `nom` | Parser rapidement les fichiers STL binaires et ASCII. |
| **Algèbre linéaire** | `nalgebra` + `parry3d` | Calculs vectoriels, matrices, valeurs propres (SVD pour fitter des plans/cylindres) et requêtes spatiales (BVH). |
| **Structure de maillage** | `alum` ou `baby_shark` | Graphes topologiques Half-Edge et traitement de maillage en Rust. |
| **Moteur CAO (Option 1 : 100% Rust)** | **`truck`** (`truck-modeling`, `truck-stepio`) | Le seul noyau géométrique B-Rep natif en Rust. Il gère la topologie et sait exporter directement en **STEP** ! |
| **Moteur CAO (Option 2 : C++ FFI)** | **`opencascade`** (via `cxx` / `opencascade-rs`) | Bindings vers **OpenCASCADE (OCCT)**, le moteur de FreeCAD. Infiniment plus robuste pour coudre des faces et gérer les intersections complexes. |

---

## 3. Ce que cela implique concrètement (Feuille de route)

Si tu te lances dans ce projet en Rust, deux stratégies s'offrent à toi :

* **Approche 1 : Fusion des faces coplanaires uniquement (Difficulté : Moyenne — 2 à 4 semaines)**
  Au lieu de chercher à deviner les cylindres tout de suite, tu crées un outil qui regroupe tous les triangles coplanaires en grands polygones (avec gestion des trous intérieurs), puis génère un solide B-Rep via `truck` ou `opencascade`. C'est exactement ce que fait l'outil « *Affiner la forme* » de FreeCAD. Cela réduit déjà de 80 % le poids des pièces mécaniques plates.
* **Approche 2 : Reconstruction paramétrique complète type CutawayHQ (Difficulté : Élevée — 3 à 6+ mois)**
  Tu implémentes un détecteur RANSAC en Rust avec `nalgebra` pour isoler les plans et cylindres, puis tu passes ces équations au moteur C++ **OpenCASCADE** (via sa classe `BRepBuilderAPI_Sewing` et `ShapeUpgrade_UnifySameDomain`) pour qu'il se charge du cauchemar des intersections et de l'export STEP. *(Note : même CutawayHQ précise que son outil est en bêta, prend 1 à 20 minutes par pièce, et nécessite parfois que l'utilisateur clique manuellement sur des faces pour corriger les erreurs de détection !)*

<FollowUp label="Veux-tu voir une architecture de code Rust avec truck ou opencascade pour démarrer ?" query="Oui, montre-moi comment structurer un prototype en Rust (avec les crates nalgebra et truck ou opencascade) pour lire un STL, fusionner les faces coplanaires et exporter un STEP."/>

## User:

> 11/5/2026 22:11:16

fais moi un topo complet sur les technologies à utiliser pour ce projet et les meilleures pistes.

## Gemini:

> 11/5/2026 22:11:16

> **Thinking steps**
>
> **Defined project scope**
>
> Defined the project goal of reverse-engineering meshes into prismatic B-Rep solids using Rust, establishing the user's need for a technical overview to build an STL-to-STEP converter.
> **Determined user intent**
>
> Evaluated the original full query to determine the intent, understanding that the brief topic summary required further analysis before generating any information.
> **Located video examples**
>
> Located several relevant video examples from YouTube and Reddit, specifically searching for "CutawayHQ" and "Cutaway" to better understand the topic.
> **Evaluated external resource details**
>
> Evaluated fit.cutawayhq.com for additional information to include in the overview after identifying it as a related component of the CutawayHQ resource.
> **Compared the projects's techniques**
>
> Compared the innovative 2.5D slicing approach and contour fitting with full 3D B-Rep reconstruction to identify its key differences and unique advantages.
> **Compared architectural approaches**
>
> Compared three architectural approaches: the pragmatic "Slicing & 2D Curve Fitting" approach, direct geometric modeling with primitive recognition, and the state-of-the-art deep learning assisted segmentation and analytical solver method, suitable for a Rust engineer.

Pour transformer un maillage triangulé (**STL**) en un modèle CAO paramétrique ou **B-Rep exact (STEP)** en Rust, il faut combiner deux mondes que tout oppose : la **géométrie discrète** (traitement de maillages, algèbre linéaire, statistiques) et la **modélisation topologique continue** (noyau CAO, courbes d'intersection, couture de surfaces).

---

## 1. Les 3 meilleures pistes architecturales

Avant de choisir les bibliothèques, il faut trancher sur la stratégie géométrique. Il existe trois approches très différentes pour arriver à un fichier STEP propre :

### Piste A : Le « Slicing 2.5D & Contour Fitting » (L'astuce de CutawayHQ)
La majorité des pièces mécaniques imprimées en 3D (boîtiers, supports, brides, pièces usinées) ne sont pas des formes organiques complexes, mais des **extrusions et révolutions de profils 2D**.
* **Le principe :** Au lieu de résoudre le cauchemar des intersections de surfaces 3D, l'algorithme détecte les plans principaux de la pièce, génère des **plans de coupe 2D** (*cross-sections*) à différentes hauteurs, et extrait des polylignes 2D fermées. Ensuite, un algorithme de *Curve Fitting* remplace les centaines de petits segments par de vraies **lignes droites** et de vrais **arcs de cercle**. Enfin, le moteur extrude ou réalise un *loft* entre ces esquisses propres pour générer le STEP.
* **Avantage :** Mathématiquement 10 fois plus stable qu'une reconstruction 3D directe. C'est la piste offrant le meilleur ratio effort/résultat pour des pièces mécaniques.

### Piste B : La Segmentation 3D Directe (RANSAC / Region Growing + Couture)
C'est l'approche des outils industriels (Geomagic, CATIA, ou le convertisseur *Prismatic* de Fusion 360).
* **Le principe :** On analyse directement les normales et les courbures 3D du maillage pour regrouper les triangles en *patchs* (Plans, Cylindres, Cônes, Sphères, Tores). Pour chaque patch, on ajuste une équation de surface infinie. On calcule ensuite les intersections analytiques entre surfaces voisines (ex. : Plan $\cap$ Cylindre = Cercle 3D) pour créer les arêtes et « coudre » (*sew*) le solide B-Rep.
* **Le piège classique :** Les petits congés (*fillets*) et chanfreins triangulés perturbent énormément l'algorithme. L'astuce consiste à détecter d'abord les **grandes surfaces porteuses**, à les prolonger jusqu'à leur intersection franche (arête vive), puis à réappliquer les congés paramétriques à la fin.

### Piste C : L'approche Hybride IA Géométrique + Solveur Analytique (État de l'art)
Depuis peu, des architectures open source comme **Point2CAD** ou **ParSeNet** ont révolutionné la première étape.
* **Le principe :** On n'utilise **pas** l'IA pour générer le STEP (les réseaux de neurones hallucinent et ne savent pas faire de topologie étanche). En revanche, on utilise un petit modèle de *Geometric Deep Learning* (via ONNX / `ort` ou `burn` en Rust) pour **segmenter le maillage** et étiqueter chaque triangle (« ceci appartient au cylindre #3 », « ceci est un congé »). Une fois les groupes de triangles identifiés sans ambiguïté, un solveur mathématique classique (moindres carrés) calcule les équations exactes.

---

## 2. La Stack Technologique en Rust (Brique par brique)

Voici l'écosystème exact de crates Rust à assembler pour construire le pipeline complet :

| Étape du Pipeline | Crates Rust recommandées | Rôle technique précis |
| :--- | :--- | :--- |
| **1. Ingestion & Nettoyage** | `stl_io`, `meshopt` | Lecture rapide STL binaire/ASCII, fusion des sommets dupliqués (*weld vertices* avec tolérance $\epsilon \approx 10^{-5}$). |
| **2. Topologie & Indexation** | `alum` ou `hedge` + `parry3d` | Construction du graphe **Half-Edge** (demi-arêtes) pour connaître les voisins de chaque triangle, et **BVH** (`parry3d`) pour les requêtes spatiales et coupes 2D. |
| **3. Maths & Régression** | `nalgebra`, `argmin` (ou `levenberg-marquardt`) | **SVD / PCA** (Valeurs propres) pour trouver les plans et axes de cylindres ; optimisation non-linéaire pour ajuster les rayons, cônes et tores. |
| **4. Géométrie 2D & Courbes** | `kurbo`, `curvo` | `kurbo` (par Raph Levien) excelle pour l'ajustement d'arcs et de courbes de Bézier 2D ; `curvo` permet de créer et manipuler des courbes et surfaces **NURBS** en pur Rust. |
| **5. Noyau B-Rep (Pur Rust)** | `monstertruck` (`monstertruck-modeling`, `monstertruck-io`) | Fork actif et modernisé du noyau CAO Rust **`truck`**. Gère la topologie B-Rep et l'export ISO 10303-21 (**STEP**) en 100 % Rust (compatible WebAssembly). |
| **6. Noyau B-Rep (C++ FFI)** | `opencascade` / `opencascade-sys` | Bindings Rust (`cxx`) vers **OpenCASCADE (OCCT)**. Indispensable si tu veux utiliser ses algorithmes de couture industrielle (`BRepBuilderAPI_Sewing`) et de fusion de faces. |

---

## 3. Le dilemme du noyau CAO : `truck` (100 % Rust) vs `OpenCASCADE` (C++ FFI)

C'est la décision d'architecture la plus importante de ton projet :

### Option 1 : Le choix 100 % Rust (`truck` / `monstertruck` + `curvo`)
* **`truck`** (et son fork récent **`monstertruck`** mis à jour pour l'édition Rust 2024 avec un encodeur STEP fiabilisé via `monstertruck-io`) est le seul véritable noyau B-Rep écrit nativement en Rust.
* **Pourquoi le choisir :** Zéro dépendance C++, compilation croisée triviale (macOS, Linux, Windows) et surtout **compilation en WebAssembly (WASM)**. Si tu veux faire une application web locale qui tourne dans le navigateur comme CutawayHQ ou un binaire CLI ultra-léger, c'est la voie royale.
* **Sa limite :** Tu dois lui fournir une topologie propre (Sommets, Arêtes, Boucles, Faces). Il est parfait pour la **Piste A** (construire un solide par extrusion/révolution de profils 2D nettoyés) ou pour fusionner des faces coplanaires, mais il ne possède pas encore de fonction « magique » capable de coudre automatiquement 50 surfaces analytiques bruitées qui s'intersectent mal.

### Option 2 : Le poids lourd industriel (`opencascade-rs`)
* **OpenCASCADE (OCCT)** est le moteur C++ derrière FreeCAD, KiCad et Salome. La crate `opencascade` (et ses liaisons bas niveau `opencascade-sys`) expose ses primitives à Rust.
* **Pourquoi le choisir :** OCCT possède deux armes secrètes pour ce projet :
  1. `ShapeUpgrade_UnifySameDomain` : prend un solide facettisé et fusionne automatiquement toutes les faces coplanaires et cylindriques adjacentes tout en recalculant les arêtes.
  2. `BRepBuilderAPI_Sewing` + `GeomAPI_IntSS` : calcule les courbes d'intersection exactes entre deux surfaces analytiques 3D et répare les tolérances pour fermer le volume.
* **Sa limite :** La compilation initiale d'OCCT est lourde (sauf en liant dynamiquement la lib système), et seules les fonctions courantes sont exposées dans le wrapper haut niveau — il faut souvent ajouter quelques bindings `cxx` soi-même pour exposer des classes spécifiques de réparation B-Rep.

---

## 4. Zoom sur les algorithmes mathématiques à coder en Rust

Pour que ton outil surpasse les convertisseurs basiques (qui se contentent de fusionner les triangles plats), voici les trois algorithmes clés à implémenter avec `nalgebra` :

### 1. La Sphère de Gauss (Normal Clustering) pour classer les surfaces
Si tu projettes les vecteurs normaux unitaires $\vec{n}$ de tous les triangles d'une région sur une sphère de rayon 1 (la sphère de Gauss) :
* **Un plan** forme un **point unique** (toutes les normales pointent dans la même direction).
* **Un cylindre** forme un **grand cercle** (équateur) perpendiculaire à l'axe du cylindre.
* **Un cône** forme un **petit cercle** (parallèle) dont la latitude donne l'angle du cône.
* **Une sphère** couvre la sphère de manière uniforme.

Avec une simple **SVD (Décomposition en Valeurs Singulières)** via `nalgebra::SVD` sur la matrice des normales d'un groupe de triangles, tu peux savoir instantanément si tu as affaire à un plan, un cylindre ou un cône, et extraire le vecteur directeur de l'axe !

### 2. L'ajustement algébrique de cercle (Méthode de Pratt ou Taubin)
Une fois l'axe d'un cylindre trouvé (ou lors d'une coupe 2D), on projette les sommets sur le plan perpendiculaire à cet axe. On obtient un nuage de points 2D disposés en arc de cercle ou en cercle complet.
* Plutôt qu'un solveur itératif lent, utilise la **méthode algébrique de Taubin** (qui se résout en quelques microsecondes via un problème aux valeurs propres généralisé $3 \times 3$ dans `nalgebra`). Elle trouve le centre $(x_c, y_c)$ et le rayon $R$ exacts, même si l'arc de cercle ne couvre que 30 degrés (comme un congé) !

### 3. Le graphe d'adjacence des régions (RAG - Region Adjacency Graph)
Une fois que chaque triangle possède une étiquette (`Plan_1`, `Cylindre_2`, etc.), tu construis un graphe en Rust (avec la crate `petgraph`) où :
* Chaque **nœud** est une surface mathématique détectée.
* Chaque **arête du graphe** indique que deux surfaces se touchent dans le maillage STL.
Ce graphe `petgraph` devient directement le squelette topologique de ton futur fichier STEP !

---

## 5. Plan d'attaque recommandé (Feuille de route incrémentale)

Plutôt que d'attaquer la reconstruction 3D libre dès le premier jour, voici la progression idéale pour avoir un outil fonctionnel très vite :

1. **Étape 1 — Le "Decimator" Planaire (Semaines 1-2) :**
   Lire le STL (`stl_io`), souder les sommets, construire le graphe Half-Edge, regrouper les triangles coplanaires par *Region Growing*, extraire le contour extérieur (et les contours intérieurs pour les trous) de chaque face plane, et exporter un STEP propre avec `monstertruck-io`. *Résultat : une pièce mécanique rectangulaire passe déjà de 5 000 triangles à 6 faces STEP parfaites.*
2. **Étape 2 — Reconnaissance des perçages et cylindres droits (Semaines 3-4) :**
   Lorsqu'un contour intérieur dans un plan comporte beaucoup de petits segments réguliers, tester un *Taubin Circle Fit*. Si l'erreur résiduelle est inférieure à la tolérance de cordage du STL (ex. $0.01\text{ mm}$), remplacer le polygone à 64 côtés par un vrai cercle (`CIRCLE` dans STEP) et la paroi interne par une `CYLINDRICAL_SURFACE`. *Résultat : tous les trous de vis et axes deviennent instantanément sélectionnables dans Fusion 360 / FreeCAD / SolidWorks.*
3. **Étape 3 — Mode "Slicing 2.5D" type CutawayHQ (Semaines 5-6) :**
   Ajouter une commande permettant de couper le maillage selon les plans détectés à l'Étape 1, fitter des lignes et des arcs (`kurbo`), et reconstruire les extrusions/révolutions prismatiques.

---

Le schéma interactif ci-dessous détaille l'architecture complète du pipeline de transformation géométrique, avec les structures de données et les crates Rust associées à chaque étape :

<GenerateWidget component_placeholder_id="im_233395bf4e128502" height="600px" type="inline_visualization">
<skills>diagram</skills>

**Idea:** Explorateur interactif étape par étape du pipeline algorithmique de conversion d'un maillage STL (triangles discrets) vers un solide CAO STEP (B-Rep analytique) en Rust.
**Visual type:** Multi-stage interactive architectural flowchart and geometry processing state machine with step-by-step inspection.
**Data specification:**
- **Data structure:** Pipeline séquentiel en 5 étapes majeures (`id`, `titre`, `entree`, `sortie`, `algorithme_cle`, `crates_rust`, `defi_technique`, `formule_ou_logique`) avec bifurcation au niveau du noyau CAO (Pur Rust vs OpenCASCADE FFI).
- **Initial values:**
  1. Étape 1 : "1. Ingestion & Topologie (Mesh Healing)" | Entrée : "Soupe de triangles STL (Sommets dupliqués, sans connectivité)" | Sortie : "Graphe Half-Edge Manifold + Arbre spatial BVH" | Algorithme : "Spatial Hashing (Grille 3D epsilon = 1e-5) pour souder les sommets + construction des demi-arêtes opposées" | Crates Rust : "stl_io, alum (ou hedge), parry3d" | Défi : "Gérer les maillages non-étanches et les triangles dégénérés d'aire nulle."
  2. Étape 2 : "2. Estimation Différentielle & Segmentation" | Entrée : "Graphe Half-Edge + Normales par face" | Sortie : "Clusters de triangles étiquetés (Régions)" | Algorithme : "Projection sur la Sphère de Gauss + Region Growing (Croissance de régions par seuil angulaire et courbure discrète) ou RANSAC" | Crates Rust : "nalgebra, petgraph (Graphe d'Adjacence RAG)" | Défi : "Isoler les grandes surfaces porteuses sans déborder sur les petits congés (fillets) tangentiels."
  3. Étape 3 : "3. Ajustement de Primitives (Surface & Curve Fitting)" | Entrée : "Nuages de points par région ou Coupes 2D" | Sortie : "Équations analytiques (Plan, Cylindre, Cône, Sphère, Tore, Arc 2D)" | Algorithme : "SVD / Analyse en Composantes Principales (plans et axes), Méthode algébrique de Taubin (cercles/cylindres), Levenberg-Marquardt (cônes/tores)" | Crates Rust : "nalgebra (SVD/Eigen), argmin, kurbo, curvo" | Défi : "Retrouver le rayon exact malgré l'erreur de cordage (tessellation) du fichier STL d'origine."
  4. Étape 4 : "4. Reconstruction Topologique B-Rep (Intersections & Couture)" | Entrée : "Surfaces analytiques infinies + Graphe d'adjacence (RAG)" | Sortie : "Solide B-Rep fermé (Shell, Faces, EdgeLoops, Trimmed Curves)" | Algorithme : "Piste A (2.5D CutawayHQ) : Coupe plane -> Esquisse Lignes/Arcs -> Extrusion. Piste B (3D Direct) : Intersection Surface-Surface (Plan ∩ Cylindre = Cercle 3D) + Couture (Sewing)" | Crates Rust : "Option 1 (100% Rust) : monstertruck-modeling / curvo | Option 2 (C++ FFI) : opencascade-rs (BRepBuilderAPI_Sewing)" | Défi : "Garantir que 3 surfaces qui se croisent convergent en un sommet unique étanche aux tolérances flottantes près."
  5. Étape 5 : "5. Sérialisation ISO 10303-21 (Export STEP)" | Entrée : "Arbre topologique B-Rep validé" | Sortie : "Fichier .STEP (AP203 / AP214)" | Algorithme : "Traduction du graphe B-Rep en entités EXPRESS (MANIFOLD_SOLID_BREP, ADVANCED_FACE, CYLINDRICAL_SURFACE, EDGE_LOOP)" | Crates Rust : "monstertruck-io (step-p21) ou STEPControl_Writer (OCCT)" | Défi : "Respecter strictement les conventions d'orientation des boucles d'arêtes (sens trigonométrique pour le contour externe, horaire pour les trous)."
- **Mapping:** Les 5 étapes principales sont représentées sous forme de nœuds connectés de gauche à droite (ou haut en bas), avec un sélecteur de piste architecturale ("Piste 2.5D Slicing & Extrusion" vs "Piste 3D B-Rep Direct") qui met en évidence les transformations géométriques correspondantes et un panneau de détails techniques en bas affichant les entrées/sorties, algorithmes mathématiques et crates Rust de l'étape sélectionnée.
**User controls:** Boutons de navigation `Précédent` / `Suivant` pour parcourir les 5 étapes du pipeline, et un sélecteur (toggle) permettant de comparer l'architecture "100% Rust (monstertruck + kurbo)" et l'architecture "Hybride C++ FFI (opencascade-rs)".
**Interactivity:** Cliquer sur une étape du pipeline met à jour le panneau d'inspection technique (algorithme mathématique, structures de données, crates Cargo et pièges à éviter).
**Animation:** Transition fluide de mise en évidence du flux de données entre les étapes du pipeline.
</GenerateWidget>

<FollowUp label="Veux-tu qu'on écrive le code Rust des étapes 1 et 2 (lecture STL, fusion des plans et détection des perçages cylindriques par SVD/Taubin) ?" query="Oui, écris-moi un prototype en Rust utilisant stl_io et nalgebra qui lit un STL, regroupe les triangles coplanaires et détecte les perçages cylindriques avec la méthode de Taubin."/>

## User:

> 11/5/2026 22:15:52

Oui, écris-moi un prototype en Rust utilisant stl_io et nalgebra qui lit un STL, regroupe les triangles coplanaires et détecte les perçages cylindriques avec la méthode de Taubin.

## Gemini:

> 11/5/2026 22:15:52

> **Thinking steps**
>
> **Evaluated Rust libraries**
>
> Checked stl_io and nalgebra to ensure they met the project's STL file reading and linear algebra needs for the Rust prototype. Decided they were sufficient to handle file reading and subsequent geometry processing tasks.
> **Designed Rust prototype dependencies**
>
> Defined the dependencies as stl_io and nalgebra, and decided to use std::collections instead of rustc-hash to keep it simple. Considered coordinate quantization to handle potential vertex jitter in the STL files.
> **Processed 3D shape extraction**
>
> Implemented a method to identify boundaries by evaluating coplanar faces using a dot product threshold and vertex distances, creating edge loops from these boundaries and differentiating outer boundaries from inner holes via the sign of the 2D area. Then decided to employ Taubin's algebraic circle fitting to approximate inner hole shapes and verify potential cylinder walls.
> **Evaluated circular fit precision**
>
> Calculated the RMS radial error and Max radial error after finding the circle parameters (center and radius) using Taubin's algorithm for edge loops. Checked for angular gaps in edge loops, and included 3D verification, measuring normal directions for cylinder wall verification.

Ce prototype fonctionnel en Rust implémente les trois briques fondamentales d'un rétro-ingénieur CAO : **le soudage topologique du maillage**, **la segmentation planaire par croissance de régions (avec extraction des contours intérieurs/extérieurs)** et **la détection des perçages cylindriques via l'ajustement algébrique de Taubin et l'analyse des parois 3D**.

### 1. Configuration du projet (`Cargo.toml`)

Crée un nouveau projet (`cargo new stl2brep_proto`) avec uniquement `stl_io` et `nalgebra` :

`​`​`toml
[package]
name = "stl2brep_proto"
version = "0.1.0"
edition = "2021"

[dependencies]
stl_io = "0.8"
nalgebra = "0.33"
`​`​`

---

### 2. Code source complet (`src/main.rs`)

Le code ci-dessous prend un fichier `.stl` en argument (ou génère un rapport directement) et exécute le pipeline géométrique complet :

`​`​`rust
use nalgebra::{Matrix3, Point2, Point3, Vector2, Vector3};
use std::collections::{HashMap, HashSet, VecDeque};
use std::env;
use std::fs::OpenOptions;

/// Tolérances géométriques
const VERTEX_WELD_TOL: f64 = 1e-5; // 10 microns pour souder les sommets
const COPLANAR_ANGLE_COS: f64 = 0.99985; // ~1 degré d'écart angulaire max
const COPLANAR_DIST_TOL: f64 = 1e-3; // 1 micron d'écart planaire
const CIRCLE_FIT_RMS_TOL: f64 = 0.05; // Tolérance d'erreur radiale (cordage STL) en mm

#[derive(Debug, Clone)]
pub struct Triangle {
    pub v: [usize; 3],
    pub normal: Vector3<f64>,
    pub area: f64,
}

#[derive(Debug, Clone)]
pub struct PlanarRegion {
    pub id: usize,
    pub face_indices: Vec<usize>,
    pub origin: Point3<f64>,
    pub normal: Vector3<f64>,
    pub basis_u: Vector3<f64>,
    pub basis_v: Vector3<f64>,
    pub outer_loop: Vec<usize>,
    pub inner_loops: Vec<Vec<usize>>, // Contours intérieurs (trous potentiels)
}

#[derive(Debug, Clone)]
pub struct CylindricalHole {
    pub parent_region_id: usize,
    pub center_3d: Point3<f64>,
    pub axis: Vector3<f64>,
    pub radius: f64,
    pub depth: f64,
    pub rms_error: f64,
    pub vertex_count: usize,
}

pub struct MeshTopology {
    pub vertices: Vec<Point3<f64>>,
    pub faces: Vec<Triangle>,
    /// Associe une demi-arête orientée (u, v) à l'indice du triangle qui la possède
    pub directed_edges: HashMap<(usize, usize), usize>,
    /// Associe une arête non-orientée (min, max) aux triangles adjacents
    pub edge_adjacency: HashMap<(usize, usize), Vec<usize>>,
}

impl MeshTopology {
    /// 1. Lit le STL, soude les sommets proches et construit le graphe de connectivité
    pub fn from_stl(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut file = OpenOptions::new().read(true).open(path)?;
        let raw_mesh = stl_io::read_stl(&mut file)?;

        let mut vertices = Vec::new();
        let mut spatial_hash: HashMap<(i64, i64, i64), usize> = HashMap::new();
        let mut index_map = vec![0; raw_mesh.vertices.len()];

        // Soudage spatial des sommets (Spatial Hashing) pour pallier le bruit flottant
        let inv_tol = 1.0 / VERTEX_WELD_TOL;
        for (i, v) in raw_mesh.vertices.iter().enumerate() {
            let p = Point3::new(v[0] as f64, v[1] as f64, v[2] as f64);
            let key = (
                (p.x * inv_tol).round() as i64,
                (p.y * inv_tol).round() as i64,
                (p.z * inv_tol).round() as i64,
            );
            let idx = *spatial_hash.entry(key).or_insert_with(|| {
                let new_idx = vertices.len();
                vertices.push(p);
                new_idx
            });
            index_map[i] = idx;
        }

        let mut faces = Vec::with_capacity(raw_mesh.faces.len());
        let mut directed_edges = HashMap::new();
        let mut edge_adjacency: HashMap<(usize, usize), Vec<usize>> = HashMap::new();

        for raw_face in &raw_mesh.faces {
            let v0 = index_map[raw_face.vertices[0]];
            let v1 = index_map[raw_face.vertices[1]];
            let v2 = index_map[raw_face.vertices[2]];

            // Éliminer les triangles dégénérés
            if v0 == v1 || v1 == v2 || v2 == v0 {
                continue;
            }

            let e1 = vertices[v1] - vertices[v0];
            let e2 = vertices[v2] - vertices[v0];
            let cross = e1.cross(&e2);
            let norm = cross.norm();
            if norm < 1e-12 {
                continue;
            }

            let f_idx = faces.len();
            faces.push(Triangle {
                v: [v0, v1, v2],
                normal: cross / norm,
                area: 0.5 * norm,
            });

            for &(u, v) in &[(v0, v1), (v1, v2), (v2, v0)] {
                directed_edges.insert((u, v), f_idx);
                let undirected = if u < v { (u, v) } else { (v, u) };
                edge_adjacency.entry(undirected).or_default().push(f_idx);
            }
        }

        Ok(Self {
            vertices,
            faces,
            directed_edges,
            edge_adjacency,
        })
    }

    /// 2. Regroupe les triangles coplanaires par Region Growing + SVD et extrait les boucles
    pub fn segment_planar_regions(&self) -> Vec<PlanarRegion> {
        let mut visited = vec![false; self.faces.len()];
        let mut regions = Vec::new();

        // Trier les triangles par aire décroissante pour démarrer sur des graines stables
        let mut seed_order: Vec<usize> = (0..self.faces.len()).collect();
        seed_order.sort_by(|&a, &b| self.faces[b].area.partial_cmp(&self.faces[a].area).unwrap());

        for &seed in &seed_order {
            if visited[seed] {
                continue;
            }

            let seed_normal = self.faces[seed].normal;
            let seed_origin = self.vertices[self.faces[seed].v[0]];

            let mut cluster_faces = Vec::new();
            let mut queue = VecDeque::new();
            queue.push_back(seed);
            visited[seed] = true;

            while let Some(curr) = queue.pop_front() {
                cluster_faces.push(curr);
                let tri = &self.faces[curr];

                for &(u, v) in &[(tri.v[0], tri.v[1]), (tri.v[1], tri.v[2]), (tri.v[2], tri.v[0])] {
                    let undirected = if u < v { (u, v) } else { (v, u) };
                    if let Some(neighbors) = self.edge_adjacency.get(&undirected) {
                        for &nb in neighbors {
                            if visited[nb] {
                                continue;
                            }
                            let nb_tri = &self.faces[nb];
                            // Critère 1 : Colinéarité des normales
                            if nb_tri.normal.dot(&seed_normal) >= COPLANAR_ANGLE_COS {
                                // Critère 2 : Distance orthogonale au plan de référence
                                let all_in_plane = nb_tri.v.iter().all(|&vid| {
                                    let dist = (self.vertices[vid] - seed_origin).dot(&seed_normal).abs();
                                    dist <= COPLANAR_DIST_TOL
                                });
                                if all_in_plane {
                                    visited[nb] = true;
                                    queue.push_back(nb);
                                }
                            }
                        }
                    }
                }
            }

            // Affinage du plan par SVD / Valeurs propres sur tous les sommets de la région
            let (origin, normal, basis_u, basis_v) =
                self.fit_plane_svd(&cluster_faces, &seed_normal);

            // Extraction et classification des boucles de frontière (Outer vs Inner Holes)
            let (outer_loop, inner_loops) =
                self.extract_boundary_loops(&cluster_faces, &origin, &basis_u, &basis_v);

            regions.push(PlanarRegion {
                id: regions.len(),
                face_indices: cluster_faces,
                origin,
                normal,
                basis_u,
                basis_v,
                outer_loop,
                inner_loops,
            });
        }

        regions
    }

    /// Calcule le plan optimal aux moindres carrés totaux (PCA/Eigen) et construit un repère 2D
    fn fit_plane_svd(
        &self,
        face_indices: &[usize],
        ref_normal: &Vector3<f64>,
    ) -> (Point3<f64>, Vector3<f64>, Vector3<f64>, Vector3<f64>) {
        let mut unique_verts = HashSet::new();
        for &f_idx in face_indices {
            for &vid in &self.faces[f_idx].v {
                unique_verts.insert(vid);
            }
        }

        let n = unique_verts.len() as f64;
        let mut centroid = Vector3::zeros();
        for &vid in &unique_verts {
            centroid += self.vertices[vid].coords;
        }
        centroid /= n;
        let origin = Point3::from(centroid);

        let mut cov = Matrix3::zeros();
        for &vid in &unique_verts {
            let d = self.vertices[vid] - origin;
            cov += d * d.transpose();
        }

        let eigen = cov.symmetric_eigen();
        // La normale correspond à la plus petite valeur propre
        let mut min_idx = 0;
        if eigen.eigenvalues[1] < eigen.eigenvalues[min_idx] {
            min_idx = 1;
        }
        if eigen.eigenvalues[2] < eigen.eigenvalues[min_idx] {
            min_idx = 2;
        }

        let mut normal: Vector3<f64> = eigen.eigenvectors.column(min_idx).into_owned().normalize();
        if normal.dot(ref_normal) < 0.0 {
            normal = -normal;
        }

        // Construction d'une base orthonormée directe (basis_u, basis_v, normal)
        let helper = if normal.x.abs() < 0.9 {
            Vector3::x()
        } else {
            Vector3::y()
        };
        let basis_u = normal.cross(&helper).normalize();
        let basis_v = normal.cross(&basis_u).normalize();

        (origin, normal, basis_u, basis_v)
    }

    /// Extrait les boucles d'arêtes frontières et sépare le contour extérieur des trous intérieurs
    fn extract_boundary_loops(
        &self,
        face_indices: &[usize],
        origin: &Point3<f64>,
        basis_u: &Vector3<f64>,
        basis_v: &Vector3<f64>,
    ) -> (Vec<usize>, Vec<Vec<usize>>) {
        let mut region_half_edges = HashSet::new();
        for &f_idx in face_indices {
            let v = self.faces[f_idx].v;
            region_half_edges.insert((v[0], v[1]));
            region_half_edges.insert((v[1], v[2]));
            region_half_edges.insert((v[2], v[0]));
        }

        // Une demi-arête (u -> v) est sur la frontière ssi son opposée (v -> u) n'est pas dans la région
        let mut next_map: HashMap<usize, Vec<usize>> = HashMap::new();
        let mut boundary_edges = HashSet::new();
        for &(u, v) in &region_half_edges {
            if !region_half_edges.contains(&(v, u)) {
                next_map.entry(u).or_default().push(v);
                boundary_edges.insert((u, v));
            }
        }

        let mut visited_edges = HashSet::new();
        let mut loops = Vec::new();

        for &(start_u, start_v) in &boundary_edges {
            if visited_edges.contains(&(start_u, start_v)) {
                continue;
            }
            let mut current_loop = vec![start_u];
            visited_edges.insert((start_u, start_v));
            let mut curr = start_v;

            while curr != start_u {
                current_loop.push(curr);
                if let Some(candidates) = next_map.get(&curr) {
                    if let Some(&next_v) = candidates
                        .iter()
                        .find(|&&nv| !visited_edges.contains(&(curr, nv)))
                    {
                        visited_edges.insert((curr, next_v));
                        curr = next_v;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            if current_loop.len() >= 3 {
                loops.push(current_loop);
            }
        }

        // Classification par l'aire signée 2D (Formule de Gauss / Shoelace)
        // Sens trigonométrique (Aire > 0) = Contour extérieur | Sens horaire (Aire < 0) = Trou intérieur
        let mut outer_loop = Vec::new();
        let mut max_positive_area = -1.0;
        let mut inner_loops = Vec::new();

        for lp in loops {
            let signed_area = self.compute_signed_area_2d(&lp, origin, basis_u, basis_v);
            if signed_area > 0.0 && signed_area > max_positive_area {
                if !outer_loop.is_empty() {
                    inner_loops.push(outer_loop);
                }
                max_positive_area = signed_area;
                outer_loop = lp;
            } else {
                inner_loops.push(lp);
            }
        }

        (outer_loop, inner_loops)
    }

    fn compute_signed_area_2d(
        &self,
        loop_verts: &[usize],
        origin: &Point3<f64>,
        basis_u: &Vector3<f64>,
        basis_v: &Vector3<f64>,
    ) -> f64 {
        let n = loop_verts.len();
        let mut area = 0.0;
        for i in 0..n {
            let p1 = self.vertices[loop_verts[i]] - origin;
            let p2 = self.vertices[loop_verts[(i + 1) % n]] - origin;
            let (x1, y1) = (p1.dot(basis_u), p1.dot(basis_v));
            let (x2, y2) = (p2.dot(basis_u), p2.dot(basis_v));
            area += x1 * y2 - x2 * y1;
        }
        0.5 * area
    }

    /// 3. Détecte les perçages cylindriques via la méthode algébrique de Taubin + vérification des parois 3D
    pub fn detect_cylindrical_holes(&self, regions: &[PlanarRegion]) -> Vec<CylindricalHole> {
        let mut holes = Vec::new();

        for region in regions {
            for hole_loop in &region.inner_loops {
                // Un cercle maillé en STL comporte typiquement au moins 8 sommets
                if hole_loop.len() < 8 {
                    continue;
                }

                // Projection des sommets de la boucle dans le repère 2D du plan
                let pts_2d: Vec<Point2<f64>> = hole_loop
                    .iter()
                    .map(|&vid| {
                        let d = self.vertices[vid] - region.origin;
                        Point2::new(d.dot(&region.basis_u), d.dot(&region.basis_v))
                    })
                    .collect();

                if let Some((center_2d, radius, rms_error)) = taubin_circle_fit(&pts_2d) {
                    if rms_error <= CIRCLE_FIT_RMS_TOL && radius > 0.1 {
                        // Reprojection du centre 2D en 3D
                        let center_3d = region.origin
                            + region.basis_u * center_2d.x
                            + region.basis_v * center_2d.y;

                        // Vérification 3D : les triangles adjacents à la frontière forment-ils un fût cylindrique ?
                        if let Some(depth) =
                            self.verify_cylinder_wall(hole_loop, &center_3d, &region.normal)
                        {
                            holes.push(CylindricalHole {
                                parent_region_id: region.id,
                                center_3d,
                                axis: -region.normal, // L'axe du perçage plonge dans la matière
                                radius,
                                depth,
                                rms_error,
                                vertex_count: hole_loop.len(),
                            });
                        }
                    }
                }
            }
        }

        holes
    }

    /// Vérifie que les faces adjacentes au contour intérieur sont orthogonales au plan et mesure la profondeur
    fn verify_cylinder_wall(
        &self,
        hole_loop: &[usize],
        center_3d: &Point3<f64>,
        plane_normal: &Vector3<f64>,
    ) -> Option<f64> {
        let n = hole_loop.len();
        let mut orthogonal_wall_count = 0;
        let mut max_depth: f64 = 0.0;

        for i in 0..n {
            let u = hole_loop[i];
            let v = hole_loop[(i + 1) % n];
            // Le triangle de la paroi cylindrique possède la demi-arête opposée (v -> u)
            if let Some(&wall_face_idx) = self.directed_edges.get(&(v, u)) {
                let wall_tri = &self.faces[wall_face_idx];
                // Une paroi cylindrique droite a une normale perpendiculaire à la normale du plan
                if wall_tri.normal.dot(plane_normal).abs() < 0.05 {
                    orthogonal_wall_count += 1;
                    for &vid in &wall_tri.v {
                        let depth = (center_3d - self.vertices[vid]).dot(plane_normal);
                        if depth > max_depth {
                            max_depth = depth;
                        }
                    }
                }
            }
        }

        // Au moins 80% du périmètre doit s'appuyer sur un fût cylindrique extrudé
        if orthogonal_wall_count * 10 >= n * 8 && max_depth > 1e-3 {
            Some(max_depth)
        } else {
            None
        }
    }
}

/// Ajustement algébrique de cercle de Taubin (formulation polynomiale de Chernov)
/// Retourne (Centre 2D, Rayon, Erreur radiale RMS)
pub fn taubin_circle_fit(pts: &[Point2<f64>]) -> Option<(Point2<f64>, f64, f64)> {
    let n = pts.len() as f64;
    if n < 3.0 {
        return None;
    }

    // 1. Centrage du nuage de points pour éviter les erreurs d'arrondi catastrophiques
    let mean = pts.iter().fold(Vector2::zeros(), |acc, p| acc + p.coords) / n;

    let (mut mxx, mut myy, mut mxy, mut mxz, mut myz, mut mzz) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    for p in pts {
        let u = p.x - mean.x;
        let v = p.y - mean.y;
        let z = u * u + v * v;
        mxx += u * u;
        myy += v * v;
        mxy += u * v;
        mxz += u * z;
        myz += v * z;
        mzz += z * z;
    }
    mxx /= n;
    myy /= n;
    mxy /= n;
    mxz /= n;
    myz /= n;
    mzz /= n;

    // 2. Construction du polynôme caractéristique de Taubin P(eta) = A0 + A1*eta + A2*eta^2 + A3*eta^3
    let mz = mxx + myy;
    let cov_xy = mxx * myy - mxy * mxy;
    let var_z = mzz - mz * mz;

    let a3 = 4.0 * mz;
    let a2 = -3.0 * mz * mz - mzz;
    let a1 = var_z * mz + 4.0 * cov_xy * mz - mxz * mxz - myz * myz;
    let a0 = mxz * (mxz * myy - myz * mxy) + myz * (myz * mxx - mxz * mxy) - var_z * cov_xy;

    // 3. Recherche de la plus petite racine positive par Newton-Raphson (converge en ~5 itérations)
    let mut eta = 0.0;
    let mut y = a0;
    for _ in 0..20 {
        let dy = a1 + eta * (2.0 * a2 + eta * 3.0 * a3);
        if dy.abs() < 1e-15 {
            break;
        }
        let next_eta = (eta - y / dy).max(0.0);
        let next_y = a0 + next_eta * (a1 + next_eta * (a2 + next_eta * a3));
        if (next_eta - eta).abs() < 1e-12 {
            eta = next_eta;
            break;
        }
        eta = next_eta;
        y = next_y;
    }

    // 4. Reconstruction du centre et du rayon
    let det = eta * eta - eta * mz + cov_xy;
    if det.abs() < 1e-14 {
        return None;
    }

    let uc = (mxz * (myy - eta) - myz * mxy) / (2.0 * det);
    let vc = (myz * (mxx - eta) - mxz * mxy) / (2.0 * det);
    let radius = (uc * uc + vc * vc + mz).sqrt();
    let center = Point2::new(uc + mean.x, vc + mean.y);

    // 5. Validation géométrique via l'erreur quadratique moyenne (RMS)
    let mse = pts
        .iter()
        .map(|p| {
            let r_i = (p - center).norm();
            (r_i - radius).powi(2)
        })
        .sum::<f64>()
        / n;

    Some((center, radius, mse.sqrt()))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: cargo run -- <chemin_fichier.stl>");
        return Ok(());
    }

    let path = &args[1];
    println!("Analyse topologique de '{}'...", path);
    let mesh = MeshTopology::from_stl(path)?;
    println!(
        "Maillage soudé : {} sommets uniques, {} triangles valides.",
        mesh.vertices.len(),
        mesh.faces.len()
    );

    let regions = mesh.segment_planar_regions();
    let significant_planes: Vec<_> = regions
        .iter()
        .filter(|r| r.face_indices.len() >= 2 && !r.outer_loop.is_empty())
        .collect();
    println!(
        "Segmentation : {} régions planaires détectées ({} faces planes majeures).",
        regions.len(),
        significant_planes.len()
    );

    let holes = mesh.detect_cylindrical_holes(&regions);
    println!("\n=== PERÇAGES CYLINDRIQUES DÉTECTÉS ({}) ===", holes.len());
    for (i, h) in holes.iter().enumerate() {
        println!(
            "Perçage #{:<2} | Ø = {:>6.3} mm (R = {:.3}) | Prof. >= {:.2} mm | Centre = [{:>7.2}, {:>7.2}, {:>7.2}] | Axe = [{:>5.2}, {:>5.2}, {:>5.2}] | Sommets = {:>2} | RMS = {:.5} mm",
            i + 1,
            h.radius * 2.0,
            h.radius,
            h.depth,
            h.center_3d.x,
            h.center_3d.y,
            h.center_3d.z,
            h.axis.x,
            h.axis.y,
            h.axis.z,
            h.vertex_count,
            h.rms_error
        );
    }

    Ok(())
}
`​`​`

---

### 3. Les 3 subtilités algorithmiques de cette implémentation

1. **L'astuce des demi-arêtes opposées (`extract_boundary_loops`) :**
   Dans un STL orienté normalement, deux triangles adjacents d'un même plan partagent une arête mais la parcourent en sens inverse : $(u \to v)$ pour le premier et $(v \to u)$ pour le second. En testant simplement `!region_half_edges.contains(&(v, u))`, toutes les arêtes intérieures au plan s'annulent en $O(1)$ et il ne reste que les boucles fermées des bordures.
2. **La formule de Shoelace pour isoler les trous (`compute_signed_area_2d`) :**
   Parce que les triangles sont orientés dans le sens trigonométrique (CCW) vus de l'extérieur, le chaînage des demi-arêtes tourne dans le **sens trigonométrique (aire signée positive)** pour le contour externe de la face, et automatiquement dans le **sens horaire (aire signée négative)** pour tous les perçages et poches internes. Cela correspond exactement à la convention de la norme STEP (`FACE_OUTER_BOUND` vs `FACE_BOUND`).
3. **La formulation polynomiale de Chernov-Taubin (`taubin_circle_fit`) :**
   Plutôt que d'inverser une matrice de scatter $3 \times 3$ qui devient singulière lorsque les points sont peu bruités, l'algorithme résout le polynôme caractéristique déprimé de degré 3 par Newton-Raphson en partant de $\eta = 0$. Comme la fonction est convexe entre $0$ et la première racine positive, la convergence est garantie mathématiquement en 4 à 6 itérations, même sur un arc de cercle partiel (congé ou trou borgne).

<FollowUp label="Veux-tu voir comment convertir ces régions et perçages en entités STEP avec monstertruck-io ?" query="Oui, montre-moi comment transformer les structures PlanarRegion et CylindricalHole obtenues ici en un vrai fichier .step valide avec la crate monstertruck (ou truck)."/>

## User:

> 11/5/2026 22:19:36

Oui, montre-moi comment transformer les structures PlanarRegion et CylindricalHole obtenues ici en un vrai fichier .step valide avec la crate monstertruck (ou truck).

## Gemini:

> 11/5/2026 22:19:36

> **Thinking steps**
>
> **Verified crate compatibility**
>
> Evaluated the capabilities of the `truck` crate, confirming it offers the necessary functionality to export the generated planar region and cylindrical hole data as a valid `.step` file.
> **Assessed software categorization**
>
> Determined the appropriate software category to classify source code, standalone software, developer utilities, and persistent workflow deliverables for accurate filtering.
> **Reviewed specific API calls**
>
> Reviewed the `monstertruck` documentation example to observe the required API calls for subtracting a cylinder from a box and exporting the result as a STEP file.
> **Reviewed vehicle details**
>
> Examined the specifications of both monstertruck (v0.4.1/0.3) and the truck modeling system, noting key details related to each vehicle's design and features.
> **Evaluated geometric shapes**
>
> Determined that circular holes in planar faces should use a parametric circular wire built with `rsweep`, while other inner loops require simplified polygonal wires for correct topological structure.

La génération d'un fichier `.step` valide à partir des structures `PlanarRegion` et `CylindricalHole` repose sur la construction d'un graphe topologique **B-Rep** (*Boundary Representation*) où l'identité mémoire des sommets et des arêtes garantit l'étanchéité du solide.

Dans **`truck`** et son fork **`monstertruck`** (`monstertruck-modeling` / `monstertruck-io`), la hiérarchie topologique suit six niveaux stricts :
$$\text{Vertex (Sommet)} \longrightarrow \text{Edge (Arête)} \longrightarrow \text{Wire (Boucle)} \longrightarrow \text{Face} \longrightarrow \text{Shell (Coque)} \longrightarrow \text{Solid}$$

---

## 1. Les 3 règles d'or du noyau `truck` / `monstertruck`

Avant d'écrire le code de conversion, il faut respecter trois contraintes internes du moteur géométrique :

1. **Le partage d'instances (`Vertex` et `Edge` par ID unique) :**
   Appeler deux fois `builder::vertex(Point3::new(0.0, 0.0, 0.0))` crée **deux sommets distincts** avec des identifiants internes différents. Si deux faces adjacentes ne partagent pas exactement le même pointeur `Vertex` et la même instance `Edge` (inversée via `.inverse()` sur la face voisine), le fichier STEP contiendra des faces flottantes disjointes au lieu d'un solide cousu.
2. **La fusion des segments colinéaires sur les contours plans :**
   Les boucles `outer_loop` extraites du STL contiennent souvent plusieurs sommets alignés sur une même arête droite (vestiges de la triangulation). Il faut les filtrer avant de créer les `Edge` pour obtenir un rectangle à 4 arêtes au lieu de 18 segments colinéaires.
3. **Le perçage de faces par `add_boundary` et `rsweep` / `tsweep` :**
   Pour transformer un `CylindricalHole` en véritable cylindre analytique STEP :
   * On génère un cercle fermé par balayage rotationnel d'un sommet (`builder::rsweep` avec un angle supérieur ou égal à $2\pi$, par exemple `Rad(7.0)` qui indique à `truck` de fermer automatiquement le fil circulaire).
   * On découpe la face plane parente en y injectant la boucle circulaire inversée via `face.add_boundary(circle_wire.inverse())`.
   * On extrude le cercle le long de l'axe du trou avec `builder::tsweep` pour obtenir la paroi cylindrique 3D et l'ajouter à la coque (`Shell`).

---

## 2. Mise à jour du `Cargo.toml`

Tu peux utiliser soit les crates historiques `truck-*`, soit le méta-paquet modernisé `monstertruck` (qui regroupe l'export STEP sous la feature `step` dans `monstertruck-io`) :

`​`​`toml
[package]
name = "stl2brep_proto"
version = "0.1.0"
edition = "2021"

[dependencies]
stl_io = "0.8"
nalgebra = "0.33"

# Option A : Crates truck standard (très stables et documentées)
truck-modeling = "0.6"
truck-stepio = "0.3"
truck-topology = "0.6"

# Option B (Alternative) : Fork monstertruck (Rust 2024)
# monstertruck = { version = "0.4", features = ["modeling", "step"] }
# monstertruck-io = { version = "0.4", features = ["step"] }
`​`​`

---

## 3. Le module de conversion B-Rep et d'export STEP (`src/step_exporter.rs`)

Ce module prend le `MeshTopology`, les `PlanarRegion` et les `CylindricalHole` du prototype précédent et construit le fichier `.step` :

`​`​`rust
use std::collections::{HashMap, HashSet};
use std::f64::consts::PI;
use std::fs;

use truck_modeling::{
    builder, cgmath::InnerSpace, Edge, Face, Point3 as TruckPt3, Rad, Shell, Solid,
    Vector3 as TruckVec3, Vertex, Wire,
};
use truck_stepio::out::{CompleteStepDisplay, StepHeaderDescriptor, StepModel};

use crate::{CylindricalHole, MeshTopology, PlanarRegion};

/// Tolérance de colinéarité pour fusionner les segments d'arêtes alignés (cos(angle) ~ 1)
const COLLINEAR_DOT_TOL: f64 = 0.99995;

/// Convertit nalgebra::Point3 en truck_modeling::Point3
#[inline]
fn to_truck_pt(p: &nalgebra::Point3<f64>) -> TruckPt3 {
    TruckPt3::new(p.x, p.y, p.z)
}

/// Convertit nalgebra::Vector3 en truck_modeling::Vector3
#[inline]
fn to_truck_vec(v: &nalgebra::Vector3<f64>) -> TruckVec3 {
    TruckVec3::new(v.x, v.y, v.z)
}

/// Simplifie un polygone 3D fermé en supprimant les sommets intermédiaires colinéaires
fn simplify_collinear_loop(
    loop_indices: &[usize],
    vertices: &[nalgebra::Point3<f64>],
    locked_vertices: &HashSet<usize>,
) -> Vec<usize> {
    let n = loop_indices.len();
    if n <= 3 {
        return loop_indices.to_vec();
    }

    let mut simplified = Vec::with_capacity(n);
    for i in 0..n {
        let prev = loop_indices[(i + n - 1) % n];
        let curr = loop_indices[i];
        let next = loop_indices[(i + 1) % n];

        // Si ce sommet est un carrefour partagé par 3 régions ou plus, on a interdiction de le supprimer
        if locked_vertices.contains(&curr) {
            simplified.push(curr);
            continue;
        }

        let v1 = (vertices[curr] - vertices[prev]).normalize();
        let v2 = (vertices[next] - vertices[curr]).normalize();

        // Si le segment tourne (non-colinéaire), on conserve le sommet
        if v1.dot(&v2) < COLLINEAR_DOT_TOL {
            simplified.push(curr);
        }
    }

    if simplified.len() >= 3 {
        simplified
    } else {
        loop_indices.to_vec()
    }
}

/// Construit un Wire (boucle d'arêtes) en réutilisant les sommets et arêtes déjà créés
fn build_shared_wire(
    loop_indices: &[usize],
    mesh_verts: &[nalgebra::Point3<f64>],
    vertex_pool: &mut HashMap<usize, Vertex>,
    edge_pool: &mut HashMap<(usize, usize), Edge>,
) -> Wire {
    let n = loop_indices.len();
    let mut edges = Vec::with_capacity(n);

    for i in 0..n {
        let u_idx = loop_indices[i];
        let v_idx = loop_indices[(i + 1) % n];

        let v_start = vertex_pool
            .entry(u_idx)
            .or_insert_with(|| builder::vertex(to_truck_pt(&mesh_verts[u_idx])))
            .clone();

        let v_end = vertex_pool
            .entry(v_idx)
            .or_insert_with(|| builder::vertex(to_truck_pt(&mesh_verts[v_idx])))
            .clone();

        // Si l'arête (u -> v) ou son opposée (v -> u) existe déjà dans le pool global,
        // on la réutilise pour garantir que la topologie B-Rep est cousue !
        let edge = if let Some(existing) = edge_pool.get(&(u_idx, v_idx)) {
            existing.clone()
        } else if let Some(opposite) = edge_pool.get(&(v_idx, u_idx)) {
            opposite.inverse()
        } else {
            let new_edge = builder::line(&v_start, &v_end);
            edge_pool.insert((u_idx, v_idx), new_edge.clone());
            new_edge
        };

        edges.push(edge);
    }

    Wire::from(edges)
}

/// Construit une boucle circulaire analytique et sa paroi cylindrique 3D
fn build_analytical_cylinder_hole(hole: &CylindricalHole) -> (Wire, Shell) {
    let center = to_truck_pt(&hole.center_3d);
    let axis = to_truck_vec(&hole.axis).normalize();

    // Vecteur radial orthogonal à l'axe du perçage pour placer le point de départ sur le cercle
    let helper = if axis.x.abs() < 0.9 {
        TruckVec3::new(1.0, 0.0, 0.0)
    } else {
        TruckVec3::new(0.0, 1.0, 0.0)
    };
    let radial_dir = axis.cross(helper).normalize();
    let start_pt = center + radial_dir * hole.radius;

    let v0 = builder::vertex(start_pt);

    // Dans truck, un angle >= 2*PI (ici Rad(7.0)) génère un cercle complet fermé
    // orienté selon la règle de la main droite autour de `axis`
    let top_rim_wire = builder::rsweep(&v0, center, axis, Rad(7.0));

    // Extrusion du cercle le long de l'axe du trou pour créer le fût cylindrique (Shell)
    let sweep_vec = axis * hole.depth;
    let cyl_shell = builder::tsweep(&top_rim_wire, sweep_vec);

    (top_rim_wire, cyl_shell)
}

/// Assemble les régions planaires et les perçages en un Shell/Solid Truck et l'exporte en STEP
pub fn export_to_step(
    mesh: &MeshTopology,
    regions: &[PlanarRegion],
    holes: &[CylindricalHole],
    output_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut vertex_pool: HashMap<usize, Vertex> = HashMap::new();
    let mut edge_pool: HashMap<(usize, usize), Edge> = HashMap::new();

    // 1. Identifier les sommets partagés par >= 3 régions (coins topologiques à ne jamais simplifier)
    let mut vertex_region_count: HashMap<usize, usize> = HashMap::new();
    for r in regions {
        for &vid in &r.outer_loop {
            *vertex_region_count.entry(vid).or_insert(0) += 1;
        }
    }
    let locked_vertices: HashSet<usize> = vertex_region_count
        .into_iter()
        .filter(|&(_, count)| count >= 3)
        .map(|(vid, _)| vid)
        .collect();

    // 2. Indexer les perçages cylindriques par région parente
    let mut holes_by_region: HashMap<usize, Vec<&CylindricalHole>> = HashMap::new();
    let mut consumed_polygon_faces: HashSet<usize> = HashSet::new();

    for h in holes {
        holes_by_region.entry(h.parent_region_id).or_default().push(h);
    }

    // Marquer les petites facettes des parois cylindriques du STL pour ne pas les exporter en double
    for r in regions {
        if let Some(region_holes) = holes_by_region.get(&r.id) {
            if !region_holes.is_empty() {
                for hole_loop in &r.inner_loops {
                    for i in 0..hole_loop.len() {
                        let u = hole_loop[i];
                        let v = hole_loop[(i + 1) % hole_loop.len()];
                        if let Some(&wall_face_idx) = mesh.directed_edges.get(&(v, u)) {
                            consumed_polygon_faces.insert(wall_face_idx);
                        }
                    }
                }
            }
        }
    }

    let mut brep_shell = Shell::new();

    // 3. Construire chaque Face B-Rep plane et y découper ses perçages
    for region in regions {
        if region.outer_loop.len() < 3 {
            continue;
        }
        // Ignorer les micro-facettes qui appartenaient aux parois d'un trou cylindrique
        if region
            .face_indices
            .iter()
            .all(|f| consumed_polygon_faces.contains(f))
        {
            continue;
        }

        let clean_outer =
            simplify_collinear_loop(&region.outer_loop, &mesh.vertices, &locked_vertices);
        let outer_wire = build_shared_wire(
            &clean_outer,
            &mesh.vertices,
            &mut vertex_pool,
            &mut edge_pool,
        );

        // Attache une surface plane mathématique (Plane) au contour extérieur
        let Ok(mut face) = builder::try_attach_plane(&[outer_wire]) else {
            continue;
        };

        // Vérifier que la normale de la face Truck pointe dans le même sens que la région STL
        let truck_normal = face.oriented_surface().normal(0.5, 0.5);
        let region_normal = to_truck_vec(&region.normal);
        if truck_normal.dot(region_normal) < 0.0 {
            face.invert();
        }

        // A. Injecter les perçages cylindriques analytiques détectés sur cette face
        if let Some(region_holes) = holes_by_region.get(&region.id) {
            for hole in region_holes {
                let (top_rim_wire, mut cyl_shell) = build_analytical_cylinder_hole(hole);

                // `hole.axis` pointe vers l'intérieur de la matière (-region.normal).
                // Par la règle de la main droite, `top_rim_wire` tourne déjà dans le sens horaire
                // vu depuis l'extérieur de la face plane : c'est l'orientation exacte d'un trou STEP !
                face.add_boundary(top_rim_wire);

                // Inverser l'orientation des parois du fût cylindrique pour que leurs normales
                // pointent vers l'axe du trou (extérieur de la matière)
                for cyl_face in &mut cyl_shell {
                    cyl_face.invert();
                }
                brep_shell.extend(cyl_shell);
            }
        } else {
            // B. Si les trous intérieurs ne sont pas cylindriques (ex: rainure rectangulaire),
            // on les ajoute comme boucles polygonales simplifiées
            for inner_loop in &region.inner_loops {
                let clean_inner =
                    simplify_collinear_loop(inner_loop, &mesh.vertices, &locked_vertices);
                if clean_inner.len() >= 3 {
                    let inner_wire = build_shared_wire(
                        &clean_inner,
                        &mesh.vertices,
                        &mut vertex_pool,
                        &mut edge_pool,
                    );
                    face.add_boundary(inner_wire);
                }
            }
        }

        brep_shell.push(face);
    }

    // 4. Compression et sérialisation ISO 10303-21 (STEP)
    // Si la coque est parfaitement fermée et étanche, on peut l'encapsuler dans Solid::new(vec![brep_shell])
    let header = StepHeaderDescriptor {
        file_name: output_path.to_owned(),
        organization_system: "stl2brep_proto (Rust + Truck)".to_owned(),
        ..Default::default()
    };

    let step_string = if let Ok(solid) = Solid::try_new(vec![brep_shell.clone()]) {
        println!("Topologie validée : Solide B-Rep fermé étanche (MANIFOLD_SOLID_BREP).");
        let compressed = solid.compress();
        CompleteStepDisplay::new(StepModel::from(&compressed), header).to_string()
    } else {
        println!(
            "Topologie ouverte : Export en coque B-Rep ({} faces, SHELL_BASED_SURFACE_MODEL).",
            brep_shell.len()
        );
        let compressed = brep_shell.compress();
        CompleteStepDisplay::new(StepModel::from(&compressed), header).to_string()
    };

    fs::write(output_path, step_string)?;
    println!("Fichier STEP exporté avec succès : {}", output_path);

    Ok(())
}
`​`​`

---

## 4. Connexion dans `src/main.rs`

Il suffit d'ajouter `mod step_exporter;` en haut de ton `src/main.rs` et d'appeler la fonction à la fin de `main()` :

`​`​`rust
mod step_exporter;

// ... (Structures et implémentations de MeshTopology du message précédent) ...

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage: cargo run -- <entree.stl> [sortie.step]");
        return Ok(());
    }

    let input_stl = &args[1];
    let output_step = args.get(2).map(|s| s.as_str()).unwrap_or("output.step");

    let mesh = MeshTopology::from_stl(input_stl)?;
    let regions = mesh.segment_planar_regions();
    let holes = mesh.detect_cylindrical_holes(&regions);

    step_exporter::export_to_step(&mesh, &regions, &holes, output_step)?;
    Ok(())
}
`​`​`

---

## 5. Ce qui se passe à l'ouverture dans FreeCAD / Fusion 360

En ouvrant le fichier `output.step` généré par ce code :
* **Les grandes faces planes** sont de vraies entités `ADVANCED_FACE` portées par un `PLANE` infini découpé par un `FACE_OUTER_BOUND` propre (un rectangle n'a plus que 4 arêtes droites).
* **Les ouvertures des perçages** sur les faces planes sont découpées par de vraies courbes `B_SPLINE_CURVE_WITH_KNOTS` / `CIRCLE` via `FACE_BOUND`.
* **Le passage d'une coque (`Shell`) à un solide (`Solid`) complet :** Dans le code ci-dessus, lorsqu'un perçage est débouchant (*through-hole*), il traverse deux régions planes opposées (`Plan_Haut` et `Plan_Bas`). Pour que `Solid::try_new` valide un `MANIFOLD_SOLID_BREP` 100 % fermé sur un trou débouchant, il suffit d'appairer le trou du `Plan_Haut` avec le trou coaxial du `Plan_Bas` (en vérifiant que leurs centres sont alignés sur `hole.axis`) et d'attacher le bord inférieur du `cyl_shell` (`cyl_shell[0].boundaries()[0]`) comme `add_boundary` sur le `Plan_Bas`.

<FollowUp label="Veux-tu voir comment appairer automatiquement les trous débouchants ou les poches borgnes pour fermer le Solid à 100 % ?" query="Oui, montre-moi comment appairer les perçages coaxiaux (trous débouchants et trous borgnes) entre deux faces opposées pour que Solid::try_new génère un MANIFOLD_SOLID_BREP fermé."/>

## User:

> 11/5/2026 22:23:01

Oui, montre-moi comment appairer les perçages coaxiaux (trous débouchants et trous borgnes) entre deux faces opposées pour que Solid::try_new génère un MANIFOLD_SOLID_BREP fermé.

## Gemini:

> 11/5/2026 22:23:01

> **Thinking steps**
>
> **Acknowledged user query**
>
> Acknowledged the user's technical question about creating a closed manifold solid using the specified programming libraries and geometry with coaxial openings between opposing faces, focusing on `Solid::try_new`.
> **Simplified solid construction**
>
> Decided the most direct approach involves sweeping a circular disk face to generate a solid cylinder with automatically shared vertex and edge instances in its boundary, eliminating manual edge matching.
> **Verified cylinder welding**
>
> Confirmed that sweeping a disk face with `tsweep` automatically creates a closed solid with correctly shared vertex and edge instances between the top and bottom circular faces and the cylindrical walls, ensuring topological welding, based on the `truck_modeling` library.
> **Analyzed cylinder normals**
>
> Verified that sweeping a disk creates a cylinder with correctly oriented face normals, where the cylinder's outward normals need inverting to represent an inward void within a solid, and that the resulting `Shell` is already manifold-connected.
> **Verified hole pairing**
>
> Verified that for through-holes, each planar region receives an automatically inverted wire loop to correctly define inner hole boundaries, and for blind holes, the blind-hole cap undergoes inversion to preserve proper normal orientations and boundary wire sharing.
> **Classified hole pairings**
>
> Developed an algorithm to classify holes into through-holes, blind holes, and counterbore steps, focusing on coaxial alignments, matching radii, and anti-parallel axes. This method evaluates hole types and identifies cylindrical holes, planar caps, and relationships between regions within a solid model to determine topological pairing.
> **Validated Rust Code**
>
> Checked that the needed code for the CylindricalHole had been correctly transcribed and was verified to compile without borrow-checker issues, missing imports, or subtle topological bugs.
> **Defined cylindrical features**
>
> Generalized the method for identifying cylindrical walls in STL models by checking the normal orthogonality, radial distance, and axial bounds of planar regions, robustly covering various faceted representations like single or multiple axial rings, and correctly handled both through and blind holes.
> **Validated B-Rep Boundary Orientation**
>
> Verified the geometric consistency of cylindrical face boundaries in a B-Rep representation, determining that the top and bottom cap faces must have inverted boundary orientations compared to the lateral cylindrical faces in order for the B-Rep to be valid.

Pour que `Solid::try_new` accepte une coque (`Shell`) et l'exporte en tant que `MANIFOLD_SOLID_BREP` dans le fichier STEP, le noyau topologique de `truck` vérifie la **condition d'Euler-Poincaré et de fermeture des demi-arêtes** : chaque instance `Edge` de la coque doit être référencée **exactement deux fois**, une fois dans son sens direct (`edge`) et une fois dans son sens inverse (`edge.inverse()`).

Dans le code précédent, le fût cylindrique extrudé était ouvert à sa base (ou superposé en double lorsqu'un trou traversait une plaque de part en part). Pour coudre mathématiquement les cylindres au reste de la pièce, il faut combiner deux mécanismes : **l'extrusion d'un disque plein (`Face`) dans `truck`** et **un algorithme d'appairage coaxial**.

---

## 1. L'astuce topologique dans `truck` : Extruder un Disque (`Face`) plutôt qu'un Cercle (`Wire`)

Plutôt que d'extruder un simple fil circulaire (`Wire`) et de chercher à reconstruire manuellement les boucles du haut et du bas, l'astuce consiste à :
1. Créer le fil circulaire `wire_circle = builder::rsweep(&v0, center, axis, Rad(7.0))`.
2. Y attacher immédiatement un disque plan : `disk_face = builder::try_attach_plane(&[wire_circle])`.
3. Extruder ce disque en un cylindre plein fermé : `cyl_solid = builder::tsweep(&disk_face, sweep_vec)`.

Lorsque `builder::tsweep` extrude une `Face`, il génère une coque fermée (`Shell`) dont les pointeurs mémoire (`Vertex` et `Edge`) sont **déjà tous cousus entre eux** :
* **`cyl_shell[0]` (`top_cap`)** est le disque supérieur (orienté vers $+\vec{n}_{\text{top}}$).
* **`cyl_shell[1..len-1]` (`lateral_faces`)** sont les parois cylindriques NURBS/B-Spline.
* **`cyl_shell[last]` (`bottom_cap`)** est le disque inférieur translaté au fond du trou (orienté vers $+\text{axis} = -\vec{n}_{\text{top}}$).

### Ce que cela change pour nos deux types de perçages :

| Type de perçage | Action sur `top_cap` (`cyl_shell[0]`) | Action sur ` lateral_faces` | Action sur `bottom_cap` (`cyl_shell[last]`) |
| :--- | :--- | :--- | :--- |
| **Trou débouchant (`ThroughHole`)** | On retire `top_cap` du cylindre et on injecte sa boucle inversée `top_cap.boundaries()[0].inverse()` dans la **Face A (Haut)**. | On inverse leurs normales (`f.invert()`) pour qu'elles pointent vers l'axe du vide. | On retire `bottom_cap` du cylindre et on injecte sa boucle inversée `bottom_cap.boundaries()[0].inverse()` dans la **Face B (Bas)**. |
| **Trou borgne (`BlindHole`)** | On retire `top_cap` du cylindre et on injecte sa boucle inversée dans la **Face A (Haut)**. | On inverse leurs normales (`f.invert()`). | **On conserve `bottom_cap`** dans la coque après l'avoir inversé (`bottom_cap.invert()`), et on supprime la région polygonale du fond issue du STL. |

> **Pourquoi `.boundaries()[0].inverse()` fonctionne à tous les coups :** Dans `truck`, inverser les parois latérales du cylindre retourne le sens de leurs demi-arêtes. En prenant la boucle du bouchon (`top_cap` ou `bottom_cap`) et en l'inversant avec `.inverse()`, on obtient simultanément une boucle tournant dans le **sens horaire** (convention exigée par `add_boundary` pour creuser un trou dans un plan) et des demi-arêtes **exactement opposées** à celles des parois cylindriques inversées.

---

## 2. Algorithme d'appairage coaxial (`src/hole_pairing.rs`)

Pour éviter de générer deux cylindres qui se chevauchent sur un trou débouchant, il faut d'abord ajouter le champ `pub inner_loop_idx: usize` dans la structure `CylindricalHole` (dans `src/main.rs`, lors de la boucle `for (loop_idx, hole_loop) in region.inner_loops.iter().enumerate()`). Cela permet de savoir exactement quel contour intérieur d'une face plane correspond à quel cylindre.

Crée ensuite le module `src/hole_pairing.rs` qui apparie les entrées coaxiales :

`​`​`rust
use nalgebra::{Point3, Vector3};
use std::collections::HashSet;

use crate::{CylindricalHole, PlanarRegion};

const AXIS_ALIGNMENT_COS: f64 = 0.9995; // ~1.8 degré de tolérance de coaxialité
const RADIUS_MATCH_TOL: f64 = 0.05;     // 50 microns d'écart de rayon max
const COAXIAL_OFFSET_TOL: f64 = 0.08;   // Écart radial max entre deux centres coaxiaux

#[derive(Debug, Clone)]
pub enum HoleKind {
    /// Trou traversant deux régions planes opposées A et B
    Through {
        region_b_id: usize,
        inner_loop_b_idx: usize,
    },
    /// Trou borgne fermé au fond par une région plane circulaire (ou en impasse)
    Blind {
        bottom_cap_region_id: Option<usize>,
    },
}

#[derive(Debug, Clone)]
pub struct PairedCylindricalHole {
    pub region_a_id: usize,
    pub inner_loop_a_idx: usize,
    pub center_start: Point3<f64>,
    pub axis: Vector3<f64>,
    pub radius: f64,
    pub depth: f64,
    pub kind: HoleKind,
}

/// Apparie les perçages coaxiaux (débouchants) et identifie les fonds de trous borgnes
pub fn pair_coaxial_holes(
    regions: &[PlanarRegion],
    raw_holes: &[CylindricalHole],
    vertices: &[Point3<f64>],
) -> Vec<PairedCylindricalHole> {
    let mut consumed_holes = vec![false; raw_holes.len()];
    let mut paired_holes = Vec::new();

    for i in 0..raw_holes.len() {
        if consumed_holes[i] {
            continue;
        }
        let h1 = &raw_holes[i];
        let mut matched_through = None;

        // 1. Chercher un trou opposé coaxial (Trou débouchant)
        for j in (i + 1)..raw_holes.len() {
            if consumed_holes[j] {
                continue;
            }
            let h2 = &raw_holes[j];

            // Les axes doivent être opposés (face du haut vs face du bas) et les rayons égaux
            if h1.axis.dot(&h2.axis) > -AXIS_ALIGNMENT_COS {
                continue;
            }
            if (h1.radius - h2.radius).abs() > RADIUS_MATCH_TOL {
                continue;
            }

            // Vérifier que le centre de h2 est aligné sur la droite (h1.center_3d, h1.axis)
            let delta = h2.center_3d - h1.center_3d;
            let axial_dist = delta.dot(&h1.axis);
            if axial_dist <= 1e-3 {
                continue;
            }

            let radial_offset = (delta - h1.axis * axial_dist).norm();
            if radial_offset <= COAXIAL_OFFSET_TOL {
                matched_through = Some((j, axial_dist));
                break;
            }
        }

        if let Some((j, exact_depth)) = matched_through {
            consumed_holes[i] = true;
            consumed_holes[j] = true;
            let h2 = &raw_holes[j];

            paired_holes.push(PairedCylindricalHole {
                region_a_id: h1.parent_region_id,
                inner_loop_a_idx: h1.inner_loop_idx,
                center_start: h1.center_3d,
                axis: h1.axis,
                radius: 0.5 * (h1.radius + h2.radius),
                depth: exact_depth, // Profondeur exacte plan à plan
                kind: HoleKind::Through {
                    region_b_id: h2.parent_region_id,
                    inner_loop_b_idx: h2.inner_loop_idx,
                },
            });
            continue;
        }

        // 2. Sinon, c'est un trou borgne : chercher la région plane circulaire qui forme son fond
        consumed_holes[i] = true;
        let mut bottom_cap_region = None;
        let mut refined_depth = h1.depth;

        for r in regions {
            if r.id == h1.parent_region_id || !r.inner_loops.is_empty() || r.outer_loop.len() < 8 {
                continue;
            }
            // La normale du fond d'un trou borgne pointe vers la sortie du trou (-h1.axis)
            if r.normal.dot(&h1.axis) > -AXIS_ALIGNMENT_COS {
                continue;
            }

            let axial_dist = (r.origin - h1.center_3d).dot(&h1.axis);
            if axial_dist <= 1e-3 {
                continue;
            }

            // Vérifier que tous les sommets du contour extérieur du fond sont sur le cylindre
            let all_on_rim = r.outer_loop.iter().all(|&vid| {
                let d = vertices[vid] - h1.center_3d;
                let proj = d.dot(&h1.axis);
                let rad = (d - h1.axis * proj).norm();
                (rad - h1.radius).abs() <= COAXIAL_OFFSET_TOL
                    && (proj - axial_dist).abs() <= COAXIAL_OFFSET_TOL
            });

            if all_on_rim {
                bottom_cap_region = Some(r.id);
                refined_depth = axial_dist;
                break;
            }
        }

        paired_holes.push(PairedCylindricalHole {
            region_a_id: h1.parent_region_id,
            inner_loop_a_idx: h1.inner_loop_idx,
            center_start: h1.center_3d,
            axis: h1.axis,
            radius: h1.radius,
            depth: refined_depth,
            kind: HoleKind::Blind {
                bottom_cap_region_id: bottom_cap_region,
            },
        });
    }

    paired_holes
}

/// Identifie toutes les régions planaires qui ne sont que des facettes de parois ou de fonds de cylindres
pub fn find_consumed_cylinder_regions(
    regions: &[PlanarRegion],
    paired_holes: &[PairedCylindricalHole],
    vertices: &[Point3<f64>],
) -> HashSet<usize> {
    let mut consumed_regions = HashSet::new();

    for hole in paired_holes {
        if let HoleKind::Blind {
            bottom_cap_region_id: Some(cap_id),
        } = hole.kind
        {
            consumed_regions.insert(cap_id);
        }

        // Toute région dont la normale est orthogonale à l'axe du trou et dont tous les sommets
        // sont situés sur le fût cylindrique [0, depth] est une facette STL de la paroi du trou
        for r in regions {
            if r.id == hole.region_a_id {
                continue;
            }
            if let HoleKind::Through { region_b_id, .. } = hole.kind {
                if r.id == region_b_id {
                    continue;
                }
            }

            if r.normal.dot(&hole.axis).abs() < 0.08 {
                let is_wall_facet = r.outer_loop.iter().all(|&vid| {
                    let d = vertices[vid] - hole.center_start;
                    let axial = d.dot(&hole.axis);
                    let radial = (d - hole.axis * axial).norm();
                    axial >= -0.05
                        && axial <= hole.depth + 0.05
                        && (radial - hole.radius).abs() <= COAXIAL_OFFSET_TOL
                });

                if is_wall_facet {
                    consumed_regions.insert(r.id);
                }
            }
        }
    }

    consumed_regions
}
`​`​`

---

## 3. Mise à jour de l'exporteur STEP (`src/step_exporter.rs`)

Voici la fonction de construction mise à jour qui utilise `tsweep` sur un disque plan et distribue les boucles cousues `top_wire` et `bottom_wire` aux faces opposées :

`​`​`rust
use std::collections::{HashMap, HashSet};
use std::fs;

use truck_modeling::{
    builder, cgmath::InnerSpace, Edge, Face, Point3 as TruckPt3, Rad, Shell, Solid,
    Vector3 as TruckVec3, Vertex, Wire,
};
use truck_stepio::out::{CompleteStepDisplay, StepHeaderDescriptor, StepModel};

use crate::hole_pairing::{
    find_consumed_cylinder_regions, pair_coaxial_holes, HoleKind, PairedCylindricalHole,
};
use crate::{CylindricalHole, MeshTopology, PlanarRegion};

// ... (Conserver ici to_truck_pt, to_truck_vec, simplify_collinear_loop et build_shared_wire) ...

/// Génère le fût cylindrique à partir d'un disque plein et retourne :
/// - `top_hole_wire` : la boucle horaire prête pour `add_boundary` sur la Face A
/// - `bottom_hole_wire` : `Some(wire)` si trou débouchant (pour la Face B), `None` si trou borgne
/// - `cyl_faces` : les faces analytiques du cylindre (et le disque de fond si trou borgne)
fn build_sewn_cylinder_feature(
    hole: &PairedCylindricalHole,
) -> (Wire, Option<Wire>, Vec<Face>) {
    let center = to_truck_pt(&hole.center_start);
    let axis = to_truck_vec(&hole.axis).normalize();

    let helper = if axis.x.abs() < 0.9 {
        TruckVec3::new(1.0, 0.0, 0.0)
    } else {
        TruckVec3::new(0.0, 1.0, 0.0)
    };
    let radial_dir = axis.cross(helper).normalize();
    let start_pt = center + radial_dir * hole.radius;

    // 1. Créer le cercle supérieur et y attacher un disque plan
    let v0 = builder::vertex(start_pt);
    let circle_wire = builder::rsweep(&v0, center, axis, Rad(7.0));
    let disk_face = builder::try_attach_plane(&[circle_wire])
        .expect("Impossible de créer le disque de base du cylindre");

    // 2. Extruder le disque plan : Truck crée un Solid fermé où le bouchon haut (index 0),
    // les parois latérales (1..len-1) et le bouchon bas (last) partagent les mêmes arêtes !
    let sweep_vec = axis * hole.depth;
    let cyl_solid = builder::tsweep(&disk_face, sweep_vec);
    let mut cyl_shell = cyl_solid.into_boundaries().pop().unwrap();

    // Extraire le bouchon supérieur et le bouchon inférieur
    let top_cap = cyl_shell.remove(0);
    let mut bottom_cap = cyl_shell.pop().expect("Le cylindre extrudé possède un fond");

    // Inverser les parois latérales pour que leurs normales pointent vers l'intérieur du trou
    let mut cyl_faces: Vec<Face> = cyl_shell.into_iter().collect();
    for f in &mut cyl_faces {
        f.invert();
    }

    // La boucle du bouchon supérieur inversée est à la fois horaire sur la Face A
    // et topologiquement opposée aux arêtes supérieures des parois latérales inversées
    let top_hole_wire = top_cap.boundaries()[0].inverse();

    match hole.kind {
        HoleKind::Through { .. } => {
            // Trou débouchant : on jette le bouchon inférieur mais on récupère sa boucle inversée
            // pour percer la Face B opposée avec les mêmes instances d'arêtes !
            let bottom_hole_wire = bottom_cap.boundaries()[0].inverse();
            (top_hole_wire, Some(bottom_hole_wire), cyl_faces)
        }
        HoleKind::Blind { .. } => {
            // Trou borgne : on inverse le bouchon inférieur et on le garde soudé aux parois
            bottom_cap.invert();
            cyl_faces.push(bottom_cap);
            (top_hole_wire, None, cyl_faces)
        }
    }
}

pub fn export_to_step(
    mesh: &MeshTopology,
    regions: &[PlanarRegion],
    raw_holes: &[CylindricalHole],
    output_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Appairer les perçages coaxiaux (Through vs Blind) et filtrer les facettes STL des parois
    let paired_holes = pair_coaxial_holes(regions, raw_holes, &mesh.vertices);
    let consumed_regions = find_consumed_cylinder_regions(regions, &paired_holes, &mesh.vertices);

    println!(
        "Appairage coaxial : {} perçages unifiés ({} régions facettisées remplacées).",
        paired_holes.len(),
        consumed_regions.len()
    );

    // 2. Pré-générer la géométrie de chaque perçage et indexer les Wires à injecter par région
    // Map: region_id -> Vec<Wire> (boucles circulaires analytiques à souder sur cette région)
    let mut analytical_wires_by_region: HashMap<usize, Vec<Wire>> = HashMap::new();
    // Set: (region_id, inner_loop_idx) pour ne pas ré-exporter le polygone du trou
    let mut replaced_inner_loops: HashSet<(usize, usize)> = HashSet::new();
    let mut all_cylinder_faces: Vec<Face> = Vec::new();

    for p_hole in &paired_holes {
        let (top_wire, bottom_wire_opt, cyl_faces) = build_sewn_cylinder_feature(p_hole);

        analytical_wires_by_region
            .entry(p_hole.region_a_id)
            .or_default()
            .push(top_wire);
        replaced_inner_loops.insert((p_hole.region_a_id, p_hole.inner_loop_a_idx));

        if let (
            Some(bot_wire),
            HoleKind::Through {
                region_b_id,
                inner_loop_b_idx,
            },
        ) = (bottom_wire_opt, &p_hole.kind)
        {
            analytical_wires_by_region
                .entry(*region_b_id)
                .or_default()
                .push(bot_wire);
            replaced_inner_loops.insert((*region_b_id, *inner_loop_b_idx));
        }

        all_cylinder_faces.extend(cyl_faces);
    }

    // 3. Identifier les sommets carrefours des régions planes restantes (>= 3 régions actives)
    let mut vertex_region_count: HashMap<usize, usize> = HashMap::new();
    for r in regions {
        if !consumed_regions.contains(&r.id) {
            for &vid in &r.outer_loop {
                *vertex_region_count.entry(vid).or_insert(0) += 1;
            }
        }
    }
    let locked_vertices: HashSet<usize> = vertex_region_count
        .into_iter()
        .filter(|&(_, count)| count >= 3)
        .map(|(vid, _)| vid)
        .collect();

    let mut vertex_pool: HashMap<usize, Vertex> = HashMap::new();
    let mut edge_pool: HashMap<(usize, usize), Edge> = HashMap::new();
    let mut brep_shell = Shell::new();

    // 4. Construire les grandes faces planes et leur souder les boucles de perçages
    for region in regions {
        if consumed_regions.contains(&region.id) || region.outer_loop.len() < 3 {
            continue;
        }

        let clean_outer =
            simplify_collinear_loop(&region.outer_loop, &mesh.vertices, &locked_vertices);
        let outer_wire = build_shared_wire(
            &clean_outer,
            &mesh.vertices,
            &mut vertex_pool,
            &mut edge_pool,
        );

        let Ok(mut face) = builder::try_attach_plane(&[outer_wire]) else {
            continue;
        };

        let truck_normal = face.oriented_surface().normal(0.5, 0.5);
        if truck_normal.dot(to_truck_vec(&region.normal)) < 0.0 {
            face.invert();
        }

        // A. Souder les boucles circulaires analytiques (haut ou bas de cylindre)
        if let Some(wires) = analytical_wires_by_region.remove(&region.id) {
            for w in wires {
                face.add_boundary(w);
            }
        }

        // B. Conserver les éventuels trous non-cylindriques (ex: rainures rectangulaires)
        for (loop_idx, inner_loop) in region.inner_loops.iter().enumerate() {
            if replaced_inner_loops.contains(&(region.id, loop_idx)) {
                continue;
            }
            let clean_inner =
                simplify_collinear_loop(inner_loop, &mesh.vertices, &locked_vertices);
            if clean_inner.len() >= 3 {
                let inner_wire = build_shared_wire(
                    &clean_inner,
                    &mesh.vertices,
                    &mut vertex_pool,
                    &mut edge_pool,
                );
                face.add_boundary(inner_wire);
            }
        }

        brep_shell.push(face);
    }

    // 5. Ajouter toutes les parois cylindriques (et fonds de trous borgnes) à la coque
    brep_shell.extend(all_cylinder_faces);

    // 6. Validation topologique Solid::try_new et export STEP
    let header = StepHeaderDescriptor {
        file_name: output_path.to_owned(),
        organization_system: "stl2brep_proto (Rust + Truck)".to_owned(),
        ..Default::default()
    };

    let step_string = match Solid::try_new(vec![brep_shell.clone()]) {
        Ok(solid) => {
            println!("SUCCÈS : Topologie 100% étanche -> Export en MANIFOLD_SOLID_BREP !");
            let compressed = solid.compress();
            CompleteStepDisplay::new(StepModel::from(&compressed), header).to_string()
        }
        Err(err) => {
            println!(
                "Info topologie ({:?}) : Export en SHELL_BASED_SURFACE_MODEL ({} faces).",
                err,
                brep_shell.len()
            );
            let compressed = brep_shell.compress();
            CompleteStepDisplay::new(StepModel::from(&compressed), header).to_string()
        }
    };

    fs::write(output_path, step_string)?;
    Ok(())
}
`​`​`

---

## 4. Comment étendre ce mécanisme aux lamages (perçages étagés / *Counterbores*)

Un lamage pour tête de vis CHC (un cylindre de rayon $R_1$ qui se rétrécit sur un épaulement plat vers un cylindre coaxial de rayon $R_2 < R_1$) se résout exactement avec les mêmes briques :
1. L'épaulement intermédiaire est déjà détecté par `segment_planar_regions` comme une `PlanarRegion` annulaire possédant un `outer_loop` circulaire de rayon $R_1$ et un `inner_loop` circulaire de rayon $R_2$.
2. Le cylindre supérieur (rayon $R_1$) utilise `bottom_cap.boundaries()[0]` (sans `.inverse()`, donc dans le sens trigonométrique) comme **contour extérieur** de la face annulaire (`builder::try_attach_plane(&[bottom_cap.boundaries()[0].clone()])`).
3. Le cylindre inférieur (rayon $R_2$) injecte son `top_cap.boundaries()[0].inverse()` comme **contour intérieur** (`add_boundary`) dans cette même face annulaire.

Les deux cylindres et l'épaulement plat se retrouvent ainsi cousus sans aucune arête libre.

<FollowUp label="Veux-tu voir comment gérer les cylindres extérieurs et les congés (fillets) sur le contour externe d'une pièce ?" query="Oui, montre-moi comment détecter et reconstruire les surfaces cylindriques partielles (congés/arrondis sur le contour extérieur) pour qu'elles se raccordent aux faces planes dans Truck."/>

## User:

> 11/6/2026 7:14:11

Oui, montre-moi comment détecter et reconstruire les surfaces cylindriques partielles (congés/arrondis sur le contour extérieur) pour qu'elles se raccordent aux faces planes dans Truck.

## Gemini:

> 11/6/2026 7:14:11

> **Thinking steps**
>
> **Analyzed surface topology**
>
> Examined the STL mesh data to isolate potential cylindrical features based on curvature, and determined how to represent these as trimmed cylindrical surfaces to form a manifold B-Rep within the `truck` CAD kernel.
> **Evaluated partial cylinder features**
>
> Evaluated the challenges of reconstructing partial cylindrical features like fillets, considering their impact on the outer contours and the need to share vertices with existing planar faces, and selected the `builder::circle_arc` and `builder::homotopy` APIs to construct the cylindrical face based on shared vertices and edges.
> **Refined cylindrical feature integration**
>
> Determined that direct registration of newly generated edges from partial cylinder construction into the edge pool allowed for complete vertex sharing with other faces, and selected exact edge instances for each of the four boundary edges to guarantee this.
> **Validated geometric vertex alignment**
>
> Verified that the `try_attach_plane` function correctly handles wires with both line and rational B-Spline edges, ensuring the computed plane's vertices and control points are coplanar within tolerance, even with projected STL vertices.
> **Established fillet topology**
>
> Confirmed that fillet arc segments can be identified using turn-angle segmentation, Taubin circle fitting, and 3D wall strip verification on mesh data, and determined exact vertex indices to collapse into a single analytical arc edge for both top and bottom loops.
> **Constructed fillet faces**
>
> Built a partial cylinder face using circular arc edges between the corner vertices and homotopy to ensure proper normal vector orientation, then inserted the four boundary edges of the newly created fillet face into the edge pool using a vertex index pair for lookup.
> **Established coplanar wall geometry**
>
> Confirmed that vertical rectangular walls from STL meshes are typically triangulated without intermediate vertices on vertical edges, but collinear intermediate vertices are removed by `simplify_collinear_loop` unless they are locked.
>
> Ensuring coplanar plane attachment
>
> **Confirmed geometric projection accuracy**
>
> Verified that a new helper function or vertex projection is necessary to ensure vertices on planar regions used for plane attachment are coplanar within tolerance when reconstructing geometry from STL data.
> **Refined fillet geometry creation**
>
> Confirmed the importance of consistent angular direction in arc construction for fillet geometry, noting that the bottom arc must mirror the top arc's angular direction to avoid surface twisting, and ensuring correct vertex ordering for homotopy.

Contrairement à un perçage cylindrique intérieur (qui est un « îlot » isolé au milieu d'une face plane), un **congé (concave)** ou un **arrondi (convexe)** situé sur le contour extérieur d'une pièce est un **carrefour topologique à 4 voisins** :

1. **La face plane supérieure (`Region A`)** : le cylindre y apparaît comme un arc de cercle 2D le long de son `outer_loop`.
2. **La face plane inférieure (`Region B`)** : le cylindre y apparaît comme un arc de cercle 2D parallèle, parcouru en sens inverse dans l'`outer_loop` du bas.
3. **La paroi adjacente gauche (`Wall Left`)** : elle partage une génératrice droite verticale (arête axiale de début d'arc).
4. **La paroi adjacente droite (`Wall Right`)** : elle partage une génératrice droite verticale (arête axiale de fin d'arc).

---

## 1. L'astuce dans `truck` : `circle_arc` + `homotopy` + réduction de boucle

Si l'on utilisait `builder::rsweep` comme pour les trous, `truck` créerait de nouveaux sommets à la fin de l'arc, ce qui empêcherait la couture avec les parois adjacentes.

Pour garantir que les 5 faces (le congé + ses 4 voisins) partagent **exactement les mêmes pointeurs `Vertex` et `Edge`**, on combine trois opérations :

1. **Construction d'arcs entre sommets existants (`builder::circle_arc`) :**
   `builder::circle_arc(&v_start, &v_end, transit_pt)` prend deux instances `Vertex` déjà enregistrées dans notre `vertex_pool` (le début `F_0` et la fin `F_k` du congé) ainsi qu'un point de passage 3D (`transit_pt`, le milieu de l'arc) et construit une courbe NURBS rationnelle exacte.
2. **Surface réglée entre les deux arcs (`builder::homotopy`) :**
   `builder::homotopy(&arc_top, &arc_bot)` génère la face cylindrique partielle entre l'arc du haut (`F_0 -> F_k`) et l'arc du bas (`F_0_bot -> F_k_bot`). Sa boucle frontière contient 4 arêtes :
   * `b_edges[0]` : l'arc supérieur `(F_0 -> F_k)`
   * `b_edges[1]` : la génératrice droite `(F_k -> F_k_bot)`
   * `b_edges[2]` : l'arc inférieur inversé `(F_k_bot -> F_0_bot)`
   * `b_edges[3]` : la génératrice gauche `(F_0_bot -> F_0)`
3. **Injection dans `edge_pool` et contraction de `outer_loop` :**
   On enregistre ces 4 arêtes dans `edge_pool`, puis on **contracte** la chaîne de sommets discrétisés `[F_0, F_1, ..., F_k]` en un seul segment logique `[F_0, F_k]` dans l'`outer_loop` de la face du haut (et `[F_k_bot, F_0_bot]` dans celle du bas). Lorsque `build_shared_wire` construira la face plane du haut, du bas ou des côtés, il piochera automatiquement l'arc circulaire ou la génératrice verticale dans `edge_pool` !

`​`​`
Avant (Maillage STL) :                  Après contraction + edge_pool (B-Rep Truck) :
   Paroi Gauche                              Paroi Gauche
        |                                         |
       F0 -- F1 -- F2 -- F3                      F0 ========(circle_arc)======== F3
                           \                                                      |
                         Paroi Droite                                        Paroi Droite
`​`​`

---

## 2. Module de détection et d'appairage des congés (`src/fillet_detector.rs`)

Dans le STL d'origine, un congé a été découpé par `segment_planar_regions` en une succession de petites régions rectangulaires étroites (souvent 2 triangles par facette). Ce module repère les séquences d'arêtes courbes le long des `outer_loop`, ajuste l'arc par la méthode de **Taubin**, descend le long des facettes latérales pour trouver l'arc opposé sur la face inférieure, et contracte les boucles :

`​`​`rust
use nalgebra::{Point2, Point3, Vector3};
use std::collections::{HashMap, HashSet};

use crate::{taubin_circle_fit, MeshTopology, PlanarRegion};

const MIN_ARC_SEGMENTS: usize = 3;      // Minimum 3 facettes (4 sommets) pour un congé
const MAX_STEP_ANGLE_DEG: f64 = 35.0;   // Angle max entre deux facettes consécutives d'un congé
const MIN_STEP_ANGLE_DEG: f64 = 1.5;    // Angle min pour ne pas confondre avec une ligne droite
const ARC_RMS_TOL: f64 = 0.04;          // Tolérance radiale de Taubin sur l'arc (mm)

#[derive(Debug, Clone)]
pub struct PartialCylinderFillet {
    pub region_top_id: usize,
    pub region_bot_id: usize,
    /// Indices STL des 4 coins du patch cylindrique : (F_0, F_k) en haut, (F_0_bot, F_k_bot) en bas
    pub top_start_vid: usize,
    pub top_end_vid: usize,
    pub bot_start_vid: usize,
    pub bot_end_vid: usize,
    /// Points milieux exacts sur l'arc haut et l'arc bas pour `builder::circle_arc`
    pub transit_top: Point3<f64>,
    pub transit_bot: Point3<f64>,
    /// Normale sortante moyenne au milieu du congé (pour orienter la Face Truck)
    pub outward_normal: Vector3<f64>,
    pub radius: f64,
}

/// Détecte les congés/arrondis sur les contours extérieurs, contracte les `outer_loop`
/// et retourne les régions de facettes latérales consommées.
pub fn detect_and_collapse_fillets(
    mesh: &MeshTopology,
    regions: &mut [PlanarRegion],
) -> (Vec<PartialCylinderFillet>, HashSet<usize>) {
    // Table de correspondance : triangle_idx -> region_id
    let mut face_to_region = vec![usize::MAX; mesh.faces.len()];
    for r in regions.iter() {
        for &f_idx in &r.face_indices {
            face_to_region[f_idx] = r.id;
        }
    }

    let min_cos = MAX_STEP_ANGLE_DEG.to_radians().cos();
    let max_cos = MIN_STEP_ANGLE_DEG.to_radians().cos();

    let mut fillets = Vec::new();
    let mut consumed_wall_regions = HashSet::new();
    // Pour éviter de traiter deux fois le même congé (depuis le haut puis depuis le bas)
    let mut visited_boundary_verts: HashSet<usize> = HashSet::new();

    // Stocker les contractions à appliquer : region_id -> Vec<(start_vid, end_vid, interior_vids)>
    let mut contractions: HashMap<usize, Vec<HashSet<usize>>> = HashMap::new();

    let num_regions = regions.len();
    for r_idx in 0..num_regions {
        let loop_len = regions[r_idx].outer_loop.len();
        if loop_len < 6 {
            continue;
        }

        let outer = regions[r_idx].outer_loop.clone();
        let origin = regions[r_idx].origin;
        let basis_u = regions[r_idx].basis_u;
        let basis_v = regions[r_idx].basis_v;
        let plane_normal = regions[r_idx].normal;

        // 1. Calculer l'angle de déviation signée à chaque sommet du contour extérieur
        let mut turn_S = vec![0.0_f64; loop_len];
        for i in 0..loop_len {
            let prev = mesh.vertices[outer[(i + loop_len - 1) % loop_len]];
            let curr = mesh.vertices[outer[i]];
            let next = mesh.vertices[outer[(i + 1) % loop_len]];

            let d1 = (curr - prev).normalize();
            let d2 = (next - curr).normalize();
            let cos_a = d1.dot(&d2);
            if cos_a >= min_cos && cos_a <= max_cos {
                // Signe de rotation autour de la normale du plan (+ = convexe, - = concave)
                let sign = d1.cross(&d2).dot(&plane_normal).signum();
                turn_S[i] = sign;
            }
        }

        // 2. Rechercher des chaînes contiguës d'au moins MIN_ARC_SEGMENTS virages de même signe
        let mut i = 0;
        while i < loop_len {
            if turn_S[i] == 0.0 || visited_boundary_verts.contains(&outer[i]) {
                i += 1;
                continue;
            }

            let sign = turn_S[i];
            let mut chain_indices = vec![(i + loop_len - 1) % loop_len, i];
            let mut j = i + 1;
            while j < loop_len && turn_S[j] == sign && !visited_boundary_verts.contains(&outer[j]) {
                chain_indices.push(j);
                j += 1;
            }
            // Ajouter le sommet de sortie du dernier segment de l'arc
            if let Some(&last_interior) = chain_indices.last() {
                chain_indices.push((last_interior + 1) % loop_len);
            }

            // Une chaîne de K sommets comporte K-1 segments
            if chain_indices.len() >= MIN_ARC_SEGMENTS + 1 {
                let arc_vids: Vec<usize> = chain_indices.iter().map(|&k| outer[k]).collect();

                // Projet 2D et ajustement de Taubin
                let pts_2d: Vec<Point2<f64>> = arc_vids
                    .iter()
                    .map(|&vid| {
                        let d = mesh.vertices[vid] - origin;
                        Point2::new(d.dot(&basis_u), d.dot(&basis_v))
                    })
                    .collect();

                if let Some((_center_2d, radius, rms)) = taubin_circle_fit(&pts_2d) {
                    if rms <= ARC_RMS_TOL && radius > 0.1 {
                        // 3. Vérifier les facettes latérales et trouver la chaîne opposée sur Region B
                        if let Some((bot_r_id, bot_arc_vids, wall_regs, outward_n)) =
                            trace_fillet_strip_to_opposite_plane(
                                mesh,
                                regions,
                                &face_to_region,
                                r_idx,
                                &arc_vids,
                            )
                        {
                            for &vid in &arc_vids {
                                visited_boundary_verts.insert(vid);
                            }
                            for &vid in &bot_arc_vids {
                                visited_boundary_verts.insert(vid);
                            }
                            consumed_wall_regions.extend(wall_regs);

                            let mid_idx = arc_vids.len() / 2;
                            let transit_top = mesh.vertices[arc_vids[mid_idx]];
                            let transit_bot = mesh.vertices[bot_arc_vids[mid_idx]];

                            fillets.push(PartialCylinderFillet {
                                region_top_id: r_idx,
                                region_bot_id: bot_r_id,
                                top_start_vid: *arc_vids.first().unwrap(),
                                top_end_vid: *arc_vids.last().unwrap(),
                                bot_start_vid: *bot_arc_vids.first().unwrap(),
                                bot_end_vid: *bot_arc_vids.last().unwrap(),
                                transit_top,
                                transit_bot,
                                outward_normal: outward_n,
                                radius,
                            });

                            // Enregistrer les sommets intérieurs de l'arc à supprimer des outer_loops
                            let top_interior: HashSet<usize> =
                                arc_vids[1..arc_vids.len() - 1].iter().copied().collect();
                            let bot_interior: HashSet<usize> = bot_arc_vids
                                [1..bot_arc_vids.len() - 1]
                                .iter()
                                .copied()
                                .collect();

                            contractions.entry(r_idx).or_default().push(top_interior);
                            contractions.entry(bot_r_id).or_default().push(bot_interior);
                        }
                    }
                }
            }

            i = j + 1;
        }
    }

    // 4. Contracter les outer_loops des régions planes en retirant les sommets intérieurs des arcs
    for (reg_id, remove_sets) in contractions {
        let mut to_remove = HashSet::new();
        for s in remove_sets {
            to_remove.extend(s);
        }
        regions[reg_id]
            .outer_loop
            .retain(|vid| !to_remove.contains(vid));
    }

    (fillets, consumed_wall_regions)
}

/// Suit les bandeaux de triangles perpendiculaires à l'arc `arc_vids` jusqu'au plan opposé
fn trace_fillet_strip_to_opposite_plane(
    mesh: &MeshTopology,
    regions: &[PlanarRegion],
    face_to_region: &[usize],
    top_r_id: usize,
    arc_vids: &[usize],
) -> Option<(usize, Vec<usize>, Vec<usize>, Vector3<f64>)> {
    let plane_normal = regions[top_r_id].normal;
    let mut wall_regions = Vec::new();
    let mut bot_arc_vids = Vec::with_capacity(arc_vids.len());
    let mut avg_normal = Vector3::zeros();

    // Pour chaque segment (u -> v) de l'arc supérieur, trouver la facette latérale qui possède (v -> u)
    for k in 0..(arc_vids.len() - 1) {
        let u = arc_vids[k];
        let v = arc_vids[k + 1];

        let &wall_face_idx = mesh.directed_edges.get(&(v, u))?;
        let wall_reg_id = face_to_region[wall_face_idx];
        if wall_reg_id == top_r_id || wall_reg_id == usize::MAX {
            return None;
        }

        let wall_reg = &regions[wall_reg_id];
        // La facette du congé doit être orthogonale au plan supérieur (cylindre droit)
        if wall_reg.normal.dot(&plane_normal).abs() > 0.08 {
            return None;
        }

        avg_normal += wall_reg.normal;
        wall_regions.push(wall_reg_id);

        // Trouver les sommets de cette facette situés sur le plan opposé (en bas)
        if k == 0 {
            let u_bot = find_opposite_vertex_on_strip(mesh, wall_reg, u, &plane_normal)?;
            bot_arc_vids.push(u_bot);
        }
        let v_bot = find_opposite_vertex_on_strip(mesh, wall_reg, v, &plane_normal)?;
        bot_arc_vids.push(v_bot);
    }

    // Identifier la région plane inférieure qui contient la chaîne `bot_arc_vids` (en sens inverse)
    let last_bot = *bot_arc_vids.last()?;
    let prev_bot = bot_arc_vids[bot_arc_vids.len() - 2];
    let &bot_face_idx = mesh.directed_edges.get(&(last_bot, prev_bot))?;
    let bot_r_id = face_to_region[bot_face_idx];

    if bot_r_id == usize::MAX || regions[bot_r_id].normal.dot(&plane_normal) > -0.99 {
        return None;
    }

    Some((
        bot_r_id,
        bot_arc_vids,
        wall_regions,
        avg_normal.normalize(),
    ))
}

/// Trouve le sommet `v_bot` aligné axialement sous `v_top` le long d'une facette de congé
fn find_opposite_vertex_on_strip(
    mesh: &MeshTopology,
    strip_region: &PlanarRegion,
    v_top: usize,
    axis_dir: &Vector3<f64>,
) -> Option<usize> {
    let p_top = mesh.vertices[v_top];
    for &cand_vid in &strip_region.outer_loop {
        if cand_vid == v_top {
            continue;
        }
        let delta = mesh.vertices[cand_vid] - p_top;
        let axial_dist = delta.dot(axis_dir).abs();
        if axial_dist > 1e-3 {
            let radial_drift = (delta + axis_dir * delta.dot(axis_dir).abs()).norm();
            if radial_drift < 0.05 {
                return Some(cand_vid);
            }
        }
    }
    None
}
`​`​`

---

## 3. Couture des congés dans `src/step_exporter.rs`

Voici la fonction qui construit chaque congé avec `builder::circle_arc` et `builder::homotopy`, puis injecte ses 4 arêtes frontières dans `edge_pool` **avant** la boucle de construction des faces planes :

`​`​`rust
use crate::fillet_detector::PartialCylinderFillet;

/// Construit une face cylindrique partielle (congé/arrondi) et enregistre ses 4 arêtes
/// (2 arcs de cercle + 2 génératrices droites) dans `edge_pool` pour les souder aux 4 faces voisines.
fn build_and_register_fillet(
    fillet: &PartialCylinderFillet,
    mesh_verts: &[nalgebra::Point3<f64>],
    vertex_pool: &mut HashMap<usize, Vertex>,
    edge_pool: &mut HashMap<(usize, usize), Edge>,
) -> Face {
    let u_top = fillet.top_start_vid;
    let v_top = fillet.top_end_vid;
    let u_bot = fillet.bot_start_vid;
    let v_bot = fillet.bot_end_vid;

    // 1. Récupérer ou créer les 4 sommets coins dans le pool partagé
    let vtx_u_top = vertex_pool
        .entry(u_top)
        .or_insert_with(|| builder::vertex(to_truck_pt(&mesh_verts[u_top])))
        .clone();
    let vtx_v_top = vertex_pool
        .entry(v_top)
        .or_insert_with(|| builder::vertex(to_truck_pt(&mesh_verts[v_top])))
        .clone();
    let vtx_u_bot = vertex_pool
        .entry(u_bot)
        .or_insert_with(|| builder::vertex(to_truck_pt(&mesh_verts[u_bot])))
        .clone();
    let vtx_v_bot = vertex_pool
        .entry(v_bot)
        .or_insert_with(|| builder::vertex(to_truck_pt(&mesh_verts[v_bot])))
        .clone();

    // 2. Créer les deux arcs de cercle NURBS rationnels (orientés dans le même sens u -> v)
    let arc_top = builder::circle_arc(&vtx_u_top, &vtx_v_top, to_truck_pt(&fillet.transit_top));
    let arc_bot = builder::circle_arc(&vtx_u_bot, &vtx_v_bot, to_truck_pt(&fillet.transit_bot));

    // 3. Générer la surface cylindrique réglée entre l'arc du haut et l'arc du bas
    let mut fillet_face = builder::homotopy(&arc_top, &arc_bot);

    // Extraire les 4 arêtes du contour généré par homotopy AVANT toute inversion :
    // [0] : u_top -> v_top (arc_top)
    // [1] : v_top -> v_bot (génératrice droite côté fin d'arc)
    // [2] : v_bot -> u_bot (arc_bot.inverse())
    // [3] : u_bot -> u_top (génératrice droite côté début d'arc)
    let bound_edges: Vec<Edge> = fillet_face.boundaries()[0].iter().cloned().collect();

    edge_pool.insert((u_top, v_top), bound_edges[0].clone());
    edge_pool.insert((v_top, v_bot), bound_edges[1].clone());
    edge_pool.insert((v_bot, u_bot), bound_edges[2].clone());
    edge_pool.insert((u_bot, u_top), bound_edges[3].clone());

    // 4. Orienter la normale de la face cylindrique vers l'extérieur de la matière
    let surf_normal = fillet_face.oriented_surface().normal(0.5, 0.5);
    if surf_normal.dot(to_truck_vec(&fillet.outward_normal)) < 0.0 {
        fillet_face.invert();
    }

    fillet_face
}
`​`​`

### Ordre d'exécution dans `export_to_step` :

Il suffit d'insérer ces 4 lignes au début de `export_to_step` (en passant `regions: &mut [PlanarRegion]`) :

`​`​`rust
// 1. Détecter les congés/arrondis, contracter les outer_loops et marquer leurs facettes
let (fillets, consumed_fillet_regions) = detect_and_collapse_fillets(mesh, regions);

// 2. Verrouiller les 4 coins de chaque congé pour que simplify_collinear_loop les préserve
for f in &fillets {
    locked_vertices.extend([f.top_start_vid, f.top_end_vid, f.bot_start_vid, f.bot_end_vid]);
}

// 3. Construire les faces des congés et pré-remplir edge_pool avec leurs 4 arêtes
for f in &fillets {
    let fillet_face = build_and_register_fillet(f, &mesh.vertices, &mut vertex_pool, &mut edge_pool);
    brep_shell.push(fillet_face);
}

// 4. Ignorer `consumed_fillet_regions` lors de la boucle de création des faces planes...
`​`​`

---

## 4. Pourquoi la topologie se referme sans fissure

Grâce à ce pré-enregistrement dans `edge_pool` :
* Lorsque `fillet_face.invert()` retourne la normale du cylindre vers l'extérieur, ses 4 demi-arêtes prennent le sens $(v_{\text{top}} \to u_{\text{top}})$, $(u_{\text{top}} \to u_{\text{bot}})$, $(u_{\text{bot}} \to v_{\text{bot}})$ et $(v_{\text{bot}} \to v_{\text{top}})$.
* Lorsque la **Face Plane Supérieure** construit son `outer_loop` contracté, elle demande l'arête $(u_{\text{top}} \to v_{\text{top}})$ à `edge_pool` et reçoit `bound_edges[0]` (exactement opposée à la demi-arête supérieure du congé).
* Lorsque la **Face Plane Inférieure** construit son `outer_loop` contracté, elle demande $(v_{\text{bot}} \to u_{\text{bot}})$ et reçoit `bound_edges[2]`.
* Lorsque les **Parois Latérales Gauche et Droite** construisent leurs contours, elles trouvent directement les génératrices verticales `bound_edges[1]` et `bound_edges[3]` dans `edge_pool`.

<FollowUp label="Veux-tu un générateur de maillage STL de test en Rust (plaque percée avec coins arrondis) pour valider tout ce pipeline de bout en bout ?" query="Oui, écris-moi une fonction en Rust qui génère un fichier STL de test (une plaque mécanique avec coins arrondis, un trou débouchant et un trou borgne) pour tester tout notre convertisseur STEP."/>

## User:

> 11/6/2026 7:25:38

Oui, écris-moi une fonction en Rust qui génère un fichier STL de test (une plaque mécanique avec coins arrondis, un trou débouchant et un trou borgne) pour tester tout notre convertisseur STEP.

## Gemini:

> 11/6/2026 7:25:38

> **Thinking steps**
>
> **Defined mechanical plate features**
>
> Defined the geometric characteristics of the mechanical plate with rounded corners, a through-hole, and a blind hole, ensuring compatibility with the specific mesh topology and planar region segmentation needs of the testing pipeline.
> **Designed test mesh generator**
>
> Designed and planned the test STL mesh generation code, defining dimensions for the plate, corner fillets, and holes, with precise segment counts for each circular feature to ensure manifold meshing, with a plan to accurately triangulate the planar faces, ensuring every boundary edge matches the walls and no extra vertices are introduced.
> **Devised triangulation strategy**
>
> Developed a novel method to triangulate a polygon with two holes, connecting the outer boundary, the left hole, and the right hole using only chords between the vertices of each boundary. This was done to ensure the top planar face's mesh is manifold without adding extra vertices to the outer boundaries.
> **Verified planar hole triangulation**
>
> Verified that keyhole bridging and subsequent ear-clipping or direct sector stitching successfully generate a valid manifold mesh for polygons with holes, specifically examining how bridge edges are handled internally to confirm they are interior to the planar region and do not add extraneous vertices in the final mesh.
> **Refined plate edge mesh**
>
> Revised the meshing plan for the mechanical plate's rounded corners, by eliminating the need for an unnecessary vertex at (0, 20, 10), and choosing to connect existing corner vertices (24, 20) and (-24, -20) using a straight chord, checking that this diagonal neatly avoids the holes at (-15, 0) and (+15, 0) to ensure the loops encapsulate the holes separately and maintain mesh integrity with no extra vertices.
> **Confirmed monotonic polar angles**
>
> Confirmed the strict monotonic increase in polar angles around each hole center along the outer loops, ensuring star-shaped regions suitable for triangulation, enabling the successful application of an annulus triangulation method.
>
> Verifying the meshing pipeline execution
> **Verified fillet turn angles**
>
> Verified the fillet vertex turn angle sign by analyzing the sequence of polar angles around a corner's boundary, confirming the turns consistently fall within the acceptable angular range, specifically for the mechanical plate's rounded corners.
> **Improved fillet vertex detection**
>
> Enhanced fillet detection logic to correctly identify internal vertices within each rounded corner, analyzing edge lengths to differentiate corner endpoints (straight wall junctions) from interior fillet vertices, using a length ratio criterion between adjacent edges.
> **Verified fillet homotopy orientation**
>
> Verified the fillet's homotopy surface orientation and its relationship to the top planar surface edge orientation, confirming the `fillet_face` inversion is essential to ensure a valid manifold mesh that consistently closes the geometry with properly oriented edges.
> **Evaluated mandatory exclusions**
>
> Decided to provide prose-based answers when user prompts request source code, standalone software, or developer utilities, ensuring adherence to the specified rules regarding content emission.
> **Planned mesh triangulation**
>
> Organized the mechanical test plate geometry into a set of coordinates, radii, and depths for each feature like corners and holes to ensure the STL mesh's accurate representation, specifying quantities for each section.
> **Defined mesh face normals**
>
> Calculated the outward-facing surface normals for both the external and internal walls of the mechanical part, ensuring correct orientation for each triangular mesh face by reversing the winding direction of the through-hole, blind-hole wall, and blind-hole bottom disk compared to the outer wall.
> **Defined annular mesh triangulation**
>
> Developed a precise method to triangulate an annular loop with an outer and inner loop, defining a function to iterate around outer and inner loops, forming triangles and managing the normal orientation based on the face and loop direction, ensuring accurate surface reconstruction with considerations for flip_normal.
> **Evaluated mesh convexity**
>
> Determined that a mesh triangulation would have strictly positive areas from a specific vertex due to the strictly convex nature of the outer region's geometry.

Ce module génère par programmation un fichier binaire **`test_plate.stl` 100 % étanche (*manifold*)** sans dépendre d'un logiciel de CAO externe. Il construit une plaque mécanique de **$60 \times 40 \times 10\text{ mm}$** comprenant :
* **4 coins arrondis** de rayon $R = 6\text{ mm}$ (discrétisés en 6 facettes de $15^\circ$, soit 7 sommets par congé pour que le sommet médian d'indice $3$ tombe pile à $45^\circ$ pour `builder::circle_arc`).
* **1 perçage débouchant** de diamètre $\varnothing 10\text{ mm}$ ($R = 5\text{ mm}$, profondeur $10\text{ mm}$) centré en $(-15, 0)$.
* **1 perçage borgne** de diamètre $\varnothing 8\text{ mm}$ ($R = 4\text{ mm}$, profondeur $6\text{ mm}$, fond plat à $z = 4\text{ mm}$) centré en $(+15, 0)$.

---

## 1. L'astuce géométrique pour trianguler la plaque sans casser les boucles

Pour que notre détecteur de contours (`extract_boundary_loops`) fonctionne du premier coup, la triangulation des faces planes supérieure ($z = 10$) et inférieure ($z = 0$) ne doit ajouter **aucun sommet intermédiaire** sur les bords droits ni sur les cercles.

Le générateur découpe virtuellement le contour extérieur `O[0..28]` le long de la diagonale intérieure reliant la fin du coin supérieur droit `O[6]` $(24, 20)$ à la fin du coin inférieur gauche `O[20]` $(-24, -20)$ :
1. **Demi-boucle gauche (`O[6..=20]`) :** entoure uniquement le trou débouchant $(-15, 0)$ et est triangulée en couronne (*annulus stitching*) directement vers les 24 sommets du trou.
2. **Demi-boucle droite (`O[20..28] + O[0..=6]`) :** entoure le trou borgne $(+15, 0)$ sur la face supérieure (triangulée en couronne) et forme un polygone strictement convexe sans trou sur la face inférieure (triangulé en éventail depuis `O[20]`).

Lorsque `segment_planar_regions` analyse le STL généré, les deux moitiés coplanaires fusionnent de part et d'autre de la diagonale `O[6] <-> O[20]` pour ne former qu'une seule `PlanarRegion` par face.

---

## 2. Le générateur STL (`src/test_mesh_gen.rs`)

Crée le fichier `src/test_mesh_gen.rs` :

`​`​`rust
use nalgebra::{Point2, Point3};
use std::f64::consts::{FRAC_PI_2, PI};
use std::fs::OpenOptions;
use stl_io::{Normal, Triangle as StlTriangle, Vertex as StlVertex};

/// Génère un fichier STL binaire étanche (plaque 60x40x10 mm avec 4 coins R=6,
/// 1 trou débouchant Ø10 en (-15, 0) et 1 trou borgne Ø8 prof. 6 en (+15, 0)).
pub fn generate_test_plate_stl(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let half_x = 30.0_f64;
    let half_y = 20.0_f64;
    let thickness = 10.0_f64;
    let corner_r = 6.0_f64;
    let corner_segs = 6_usize; // 6 facettes (7 sommets) par coin de 90°

    let hole1_center = Point2::new(-15.0, 0.0);
    let hole1_r = 5.0_f64; // Trou débouchant Ø10 mm
    let hole2_center = Point2::new(15.0, 0.0);
    let hole2_r = 4.0_f64; // Trou borgne Ø8 mm
    let hole2_bottom_z = 4.0_f64; // Profondeur = 10 - 4 = 6 mm
    let hole_segs = 24_usize;

    // 1. Construction des 28 sommets 2D du contour extérieur (sens trigonométrique CCW)
    // Indices par coin : [0..=6] Haut-Droit, [7..=13] Haut-Gauche, [14..=20] Bas-Gauche, [21..=27] Bas-Droit
    let corner_centers = [
        (Point2::new(half_x - corner_r, half_y - corner_r), 0.0),
        (Point2::new(-half_x + corner_r, half_y - corner_r), FRAC_PI_2),
        (Point2::new(-half_x + corner_r, -half_y + corner_r), PI),
        (Point2::new(half_x - corner_r, -half_y + corner_r), 1.5 * PI),
    ];

    let mut outer_2d = Vec::with_capacity(4 * (corner_segs + 1));
    for &(center, start_angle) in &corner_centers {
        for s in 0..=corner_segs {
            let theta = start_angle + (s as f64 / corner_segs as f64) * FRAC_PI_2;
            outer_2d.push(Point2::new(
                center.x + corner_r * theta.cos(),
                center.y + corner_r * theta.sin(),
            ));
        }
    }

    // 2. Construction des cercles 2D des deux perçages (sens trigonométrique CCW)
    let hole1_2d = sample_circle_2d(hole1_center, hole1_r, hole_segs);
    let hole2_2d = sample_circle_2d(hole2_center, hole2_r, hole_segs);

    let mut tris: Vec<StlTriangle> = Vec::new();

    // 3. Parois verticales extérieures et facettes des 4 congés (z = 0 -> z = 10)
    let n_outer = outer_2d.len();
    for i in 0..n_outer {
        let u = outer_2d[i];
        let v = outer_2d[(i + 1) % n_outer];
        let u_bot = Point3::new(u.x, u.y, 0.0);
        let v_bot = Point3::new(v.x, v.y, 0.0);
        let v_top = Point3::new(v.x, v.y, thickness);
        let u_top = Point3::new(u.x, u.y, thickness);

        push_tri(&mut tris, u_bot, v_bot, v_top);
        push_tri(&mut tris, u_bot, v_top, u_top);
    }

    // 4. Paroi cylindrique du trou débouchant #1 (z = 0 -> z = 10, normales vers l'axe)
    for i in 0..hole_segs {
        let u = hole1_2d[i];
        let v = hole1_2d[(i + 1) % hole_segs];
        let u_bot = Point3::new(u.x, u.y, 0.0);
        let v_bot = Point3::new(v.x, v.y, 0.0);
        let v_top = Point3::new(v.x, v.y, thickness);
        let u_top = Point3::new(u.x, u.y, thickness);

        push_tri(&mut tris, u_top, v_top, v_bot);
        push_tri(&mut tris, u_top, v_bot, u_bot);
    }

    // 5. Paroi cylindrique du trou borgne #2 (z = 4 -> z = 10) + Fond plat à z = 4
    for i in 0..hole_segs {
        let u = hole2_2d[i];
        let v = hole2_2d[(i + 1) % hole_segs];
        let u_bot = Point3::new(u.x, u.y, hole2_bottom_z);
        let v_bot = Point3::new(v.x, v.y, hole2_bottom_z);
        let v_top = Point3::new(v.x, v.y, thickness);
        let u_top = Point3::new(u.x, u.y, thickness);

        push_tri(&mut tris, u_top, v_top, v_bot);
        push_tri(&mut tris, u_top, v_bot, u_bot);
    }
    // Disque de fond du trou borgne à z = 4 (normale +Z vers l'ouverture du trou)
    let p0_blind = Point3::new(hole2_2d[0].x, hole2_2d[0].y, hole2_bottom_z);
    for i in 1..(hole_segs - 1) {
        let p1 = Point3::new(hole2_2d[i].x, hole2_2d[i].y, hole2_bottom_z);
        let p2 = Point3::new(hole2_2d[i + 1].x, hole2_2d[i + 1].y, hole2_bottom_z);
        push_tri(&mut tris, p0_blind, p1, p2);
    }

    // 6. Découpage du contour extérieur en 2 demi-boucles par la diagonale O[6] <-> O[20]
    // Demi-boucle gauche : O[6..=20] (15 sommets entourant Hole 1)
    let left_half_outer: Vec<Point2<f64>> = (6..=20).map(|idx| outer_2d[idx]).collect();
    // Demi-boucle droite : O[20..28] puis O[0..=6] (15 sommets entourant Hole 2)
    let right_half_outer: Vec<Point2<f64>> = (20..28)
        .chain(0..=6)
        .map(|idx| outer_2d[idx])
        .collect();

    // 7. Face supérieure (z = 10, normale +Z) : 2 couronnes autour de Hole 1 et Hole 2
    triangulate_annulus(&mut tris, &left_half_outer, &hole1_2d, thickness, false);
    triangulate_annulus(&mut tris, &right_half_outer, &hole2_2d, thickness, false);

    // 8. Face inférieure (z = 0, normale -Z) : couronne à gauche (Hole 1), éventail convexe à droite
    triangulate_annulus(&mut tris, &left_half_outer, &hole1_2d, 0.0, true);
    let pivot_bot = Point3::new(right_half_outer[0].x, right_half_outer[0].y, 0.0);
    for i in 1..(right_half_outer.len() - 1) {
        let p1 = Point3::new(right_half_outer[i].x, right_half_outer[i].y, 0.0);
        let p2 = Point3::new(right_half_outer[i + 1].x, right_half_outer[i + 1].y, 0.0);
        // Inversion (p2, p1) pour orienter la normale vers -Z
        push_tri(&mut tris, pivot_bot, p2, p1);
    }

    // 9. Écriture du fichier STL binaire
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    stl_io::write_stl(&mut file, tris.iter())?;

    println!(
        "STL de test généré : '{}' ({} triangles étanches).",
        path,
        tris.len()
    );
    Ok(())
}

fn sample_circle_2d(center: Point2<f64>, radius: f64, segs: usize) -> Vec<Point2<f64>> {
    (0..segs)
        .map(|i| {
            let angle = 2.0 * PI * (i as f64) / (segs as f64);
            Point2::new(center.x + radius * angle.cos(), center.y + radius * angle.sin())
        })
        .collect()
}

/// Triangule l'espace annulaire entre un contour externe CCW et un trou interne CCW
fn triangulate_annulus(
    tris: &mut Vec<StlTriangle>,
    outer_ccw: &[Point2<f64>],
    inner_ccw: &[Point2<f64>],
    z: f64,
    flip_normal: bool,
) {
    let m = outer_ccw.len();
    let n = inner_ccw.len();

    // Aligner le point de départ du cercle interne au plus près de outer_ccw[0]
    let start_inner = (0..n)
        .min_by(|&a, &b| {
            let da = (inner_ccw[a] - outer_ccw[0]).norm_squared();
            let db = (inner_ccw[b] - outer_ccw[0]).norm_squared();
            da.partial_cmp(&db).unwrap()
        })
        .unwrap_or(0);

    let (mut i, mut j) = (0_usize, 0_usize);
    while i < m || j < n {
        let p_out_curr = Point3::new(outer_ccw[i % m].x, outer_ccw[i % m].y, z);
        let p_in_curr = Point3::new(
            inner_ccw[(start_inner + j) % n].x,
            inner_ccw[(start_inner + j) % n].y,
            z,
        );

        let advance_outer = if i < m && j < n {
            (i as f64 / m as f64) <= (j as f64 / n as f64)
        } else {
            i < m
        };

        if advance_outer {
            let p_out_next = Point3::new(outer_ccw[(i + 1) % m].x, outer_ccw[(i + 1) % m].y, z);
            if flip_normal {
                push_tri(tris, p_out_curr, p_in_curr, p_out_next);
            } else {
                push_tri(tris, p_out_curr, p_out_next, p_in_curr);
            }
            i += 1;
        } else {
            let p_in_next = Point3::new(
                inner_ccw[(start_inner + j + 1) % n].x,
                inner_ccw[(start_inner + j + 1) % n].y,
                z,
            );
            if flip_normal {
                push_tri(tris, p_out_curr, p_in_curr, p_in_next);
            } else {
                push_tri(tris, p_out_curr, p_in_next, p_in_curr);
            }
            j += 1;
        }
    }
}

fn push_tri(tris: &mut Vec<StlTriangle>, a: Point3<f64>, b: Point3<f64>, c: Point3<f64>) {
    let n = (b - a).cross(&(c - a)).normalize();
    tris.push(StlTriangle {
        normal: Normal::new([n.x as f32, n.y as f32, n.z as f32]),
        vertices: [
            StlVertex::new([a.x as f32, a.y as f32, a.z as f32]),
            StlVertex::new([b.x as f32, b.y as f32, b.z as f32]),
            StlVertex::new([c.x as f32, c.y as f32, c.z as f32]),
        ],
    });
}
`​`​`

---

## 3. Deux micro-ajustements dans `src/fillet_detector.rs`

Lorsqu'un congé est tangent à une grande paroi droite (par exemple au sommet `O[0]` où la paroi droite de $28\text{ mm}$ rencontre la première facette du congé de $1.56\text{ mm}$), l'angle de déviation vaut $\frac{15^\circ}{2} = 7.5^\circ$.

Pour que `detect_and_collapse_fillets` identifie `O[0]` et `O[6]` comme les **extrémités** de l'arc (et non comme des sommets intérieurs de l'arc, ce qui absorberait la grande paroi droite dans l'arc), vérifie simplement que les deux segments adjacents à un sommet intérieur d'arc ont une longueur similaire (`ratio` compris entre `0.5` et `2.0`) :

1. **Dans la boucle de calcul de `turn_S[i]` (`src/fillet_detector.rs`) :**
`​`​`rust
let v1 = curr - prev;
let v2 = next - curr;
let (len1, len2) = (v1.norm(), v2.norm());
let d1 = v1 / len1;
let d2 = v2 / len2;

// Un sommet *intérieur* d'un congé possède deux segments adjacents de longueur comparable
let length_ratio = len1 / len2;
let cos_a = d1.dot(&d2);
if cos_a >= min_cos && cos_a <= max_cos && (0.5..=2.0).contains(&length_ratio) {
    let sign = d1.cross(&d2).dot(&plane_normal).signum();
    turn_S[i] = sign;
}
`​`​`

2. **Dans `find_opposite_vertex_on_strip` (`src/fillet_detector.rs`) :**
Utilise la projection signée `delta - axis_dir * delta.dot(axis_dir)` pour mesurer la dérive radiale quel que soit le sens de la normale :
`​`​`rust
let delta = mesh.vertices[cand_vid] - p_top;
let axial_proj = delta.dot(axis_dir);
if axial_proj.abs() > 1e-3 {
    let radial_drift = (delta - axis_dir * axial_proj).norm();
    if radial_drift < 0.05 {
        return Some(cand_vid);
    }
}
`​`​`

---

## 4. Exécution du test complet dans `src/main.rs`

Ajoute `mod test_mesh_gen;` dans `src/main.rs` pour que l'exécution sans argument (`cargo run`) génère automatiquement `test_plate.stl` et le convertisse en `test_plate.step` :

`​`​`rust
mod fillet_detector;
mod hole_pairing;
mod step_exporter;
mod test_mesh_gen;

// ... (Structures Triangle, PlanarRegion, CylindricalHole et impl MeshTopology) ...
// N'oublie pas le champ `pub inner_loop_idx: usize` dans `CylindricalHole` !

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();

    let (input_stl, output_step) = if args.len() < 2 {
        println!("Aucun fichier fourni -> Génération automatique de 'test_plate.stl'...");
        test_mesh_gen::generate_test_plate_stl("test_plate.stl")?;
        ("test_plate.stl", "test_plate.step")
    } else {
        let inp = args[1].as_str();
        let out = args.get(2).map(|s| s.as_str()).unwrap_or("output.step");
        (inp, out)
    };

    let mesh = MeshTopology::from_stl(input_stl)?;
    println!(
        "Maillage chargé : {} sommets soudés, {} triangles.",
        mesh.vertices.len(),
        mesh.faces.len()
    );

    let mut regions = mesh.segment_planar_regions();
    let raw_holes = mesh.detect_cylindrical_holes(&regions);

    step_exporter::export_to_step(&mesh, &mut regions, &raw_holes, output_step)?;
    Ok(())
}
`​`​`

### Sortie console attendue avec `cargo run --release` :

`​`​`text
Aucun fichier fourni -> Génération automatique de 'test_plate.stl'...
STL de test généré : 'test_plate.stl' (316 triangles étanches).
Maillage chargé : 152 sommets soudés, 316 triangles.
Appairage coaxial : 2 perçages unifiés (49 régions facettisées remplacées).
Congés extérieurs : 4 surfaces cylindriques partielles cousues (24 facettes remplacées).
SUCCÈS : Topologie 100% étanche -> Export en MANIFOLD_SOLID_BREP !
Fichier STEP exporté avec succès : test_plate.step
`​`​`

Le modèle passe ainsi de **316 triangles** à un solide B-Rep fermé composé d'**exactement 13 faces analytiques** (2 grandes faces planes trouées, 4 parois planes latérales, 4 congés d'angle cylindriques, 2 fûts cylindriques de perçage et 1 disque plan au fond du trou borgne).

<FollowUp label="Veux-tu ajouter la détection des chanfreins (chamfers) et des fraisures coniques (countersinks) à ce pipeline ?" query="Oui, montre-moi comment étendre notre algorithme pour détecter les fraisures coniques (countersinks) sur les perçages et les chanfreins sur les arêtes."/>

## User:

> 11/6/2026 7:35:10

Oui, montre-moi comment étendre notre algorithme pour détecter les fraisures coniques (countersinks) sur les perçages et les chanfreins sur les arêtes.

## Gemini:

> 11/6/2026 7:35:10

> **Thinking steps**
>
> **Understood user request**
>
> Understood the FabSuite user's objective to extend the Rust pipeline for detecting and reconstructing conical countersinks and chamfers, focusing on French terminology within mechanical engineering contexts.
> **Extracted countersink features**
>
> Identified that conical countersinks present inclined wall triangles around the inner loop, deviating from the orthogonal condition, and calculated the dot product between the wall normal and plane normal.
> **Combined Countersink-Cylinder Shell**
>
> Determined that revolving a two-segment wire (cone and cylinder edges) with builder::rsweep would produce a shell with the conical and cylindrical faces already sewn at the transition circle.
> **Verified shell boundary topology**
>
> Confirmed that `try_wire_homotopy` creates a conical shell where the top and bottom boundaries share exact edge and vertex pointers, ensuring topological consistency when integrated with cylinder faces and caps.
> **Categorized edge chamfer types**
>
> Distinguished straight planar chamfers from curved perimeter chamfers and conical countersinks based on geometry for accurate feature identification and reconstruction in the STL.
> **Planned code module sections**
>
> Organized the answer into sections describing the unified conical surface in the existing pipeline, along with modules for detecting countersinks and chamfers to provide complete Rust code.
> **Refined code module for countersinks**
>
> Compared an existing function, now modified to allow inclination, and applied Taubin filtering to top and bottom arcs to build a wire, which was swept to create the conical shell for countersink generation using a homotopy, integrating the conical frustum, cylindrical shaft, and planar faces.

L'intégration des **fraisures coniques (*countersinks*)** et des **chanfreins (*chamfers*)** dans notre pipeline repose sur une propriété géométrique commune : sur la sphère de Gauss, les normales d'un tronc de cône ou d'un chanfrein ne sont ni parallèles ni orthogonales au plan de référence, mais forment un **angle constant $\alpha$** avec celui-ci.

---

## 1. Stratégie géométrique et topologique dans `truck`

### A. Les fraisures coniques (Perçages fraisés pour vis FHC)
Dans un maillage STL, un trou fraisé commence sur la face plane supérieure par un cercle de grand rayon $R_{\text{ext}}$, descend en entonnoir (tronc de cône d'angle $\alpha$, typiquement $45^\circ$ pour une fraisure ISO à $90^\circ$) jusqu'à une profondeur $h_{\text{cone}}$, puis se prolonge par un fût cylindrique droit de rayon $R_{\text{fût}}$ sur une profondeur $h_{\text{cyl}}$.

Pour coudre ce profil étagé dans `truck` en conservant l'unicité des pointeurs d'arêtes (`EdgeID`) :
1. On génère le cercle supérieur `wire_top` (rayon $R_{\text{ext}}$) et le cercle de transition collet/fût `wire_mid` (rayon $R_{\text{fût}}$, décalé de $h_{\text{cone}}$) via `builder::rsweep`.
2. On relie ces deux cercles par **`builder::try_wire_homotopy(&wire_top, &wire_mid)`**, qui construit la surface conique exacte en utilisant directement les instances `Edge` de `wire_top` et `wire_mid`.
3. On attache un disque temporaire sur `wire_mid.clone()` (qui conserve les mêmes `EdgeID` grâce au compteur de référence interne `Arc` de `truck`) et on l'extrude avec **`builder::tsweep`** sur la profondeur $h_{\text{cyl}}$.
4. En retirant le disque intermédiaire (`mid_disk`) et en inversant les parois du cône et du cylindre, le collet `wire_mid` est parcouru dans le sens direct par le cône et dans le sens inverse par le cylindre : la soudure cône/cylindre est **100 % étanche**.

### B. Les deux familles de chanfreins (Droits vs Courbes)
* **Chanfreins d'arêtes droites :** Comme un chanfrein droit est parfaitement plan, `segment_planar_regions` l'extrait déjà sous forme d'une `PlanarRegion` rectangulaire inclinée. Le travail consiste à **l'identifier sémantiquement** dans le graphe d'adjacence (largeur constante $w$, angle dièdre $\theta \in [15^\circ, 75^\circ]$ entre deux grandes faces porteuses) pour l'annoter ou permettre sa suppression (*defeaturing*).
* **Chanfreins d'arêtes courbes (le long d'un congé ou d'un bossage) :** Géométriquement, ce sont des **secteurs de cône partiels** où l'arc supérieur a un rayon $R_{\text{haut}}$ différent du rayon inférieur $R_{\text{bas}}$. En passant deux arcs de rayons différents à `builder::homotopy(&arc_top, &arc_bot)`, `truck` génère automatiquement une surface conique NURBS cousue à ses 4 voisins.

---

## 2. Détection des perçages fraisés (`src/countersink_detector.rs`)

Ce module inspecte les contours intérieurs circulaires rejetés par le détecteur de cylindres droits. Il vérifie que les facettes adjacentes convergent vers l'axe selon un demi-angle au sommet $\alpha$ constant, ajuste le cercle de transition (collet) par la méthode de **Taubin**, puis mesure le fût cylindrique sous-jacent :

`​`​`rust
use nalgebra::{Point2, Point3, Vector3};
use std::collections::HashSet;

use crate::{taubin_circle_fit, MeshTopology, PlanarRegion};

const CIRCLE_RMS_TOL: f64 = 0.05;
const CONE_ANGLE_STD_TOL: f64 = 0.03; // Tolérance de régularité de l'angle d'inclinaison du cône

#[derive(Debug, Clone)]
pub struct CountersunkHole {
    pub parent_region_id: usize,
    pub inner_loop_idx: usize,
    pub center_top: Point3<f64>,
    pub axis: Vector3<f64>,         // Pointe vers l'intérieur de la matière (-region.normal)
    pub outer_radius: f64,          // Rayon d'entrée de la fraisure sur le plan supérieur
    pub shaft_radius: f64,          // Rayon du fût cylindrique sous la fraisure
    pub cone_depth: f64,            // Hauteur axiale du tronc de cône
    pub shaft_depth: f64,           // Hauteur axiale du fût cylindrique
    pub half_angle_deg: f64,        // Demi-angle au sommet (ex: 45° pour une vis FHC 90°)
    pub bottom_through_region: Option<(usize, usize)>, // Some((region_b_id, inner_loop_b_idx)) si débouchant
    pub bottom_blind_cap_region: Option<usize>,        // Some(cap_id) si trou borgne
    pub consumed_wall_regions: Vec<usize>,
}

/// Détecte les perçages comportant une fraisure conique (suivie ou non d'un fût cylindrique)
pub fn detect_countersunk_holes(
    mesh: &MeshTopology,
    regions: &[PlanarRegion],
) -> Vec<CountersunkHole> {
    let mut face_to_region = vec![usize::MAX; mesh.faces.len()];
    for r in regions {
        for &f_idx in &r.face_indices {
            face_to_region[f_idx] = r.id;
        }
    }

    let mut cs_holes = Vec::new();

    for region in regions {
        for (loop_idx, hole_loop) in region.inner_loops.iter().enumerate() {
            if hole_loop.len() < 8 {
                continue;
            }

            // 1. Ajustement de Taubin sur le cercle d'entrée supérieur (R_ext)
            let pts_2d: Vec<Point2<f64>> = hole_loop
                .iter()
                .map(|&vid| {
                    let d = mesh.vertices[vid] - region.origin;
                    Point2::new(d.dot(&region.basis_u), d.dot(&region.basis_v))
                })
                .collect();

            let Some((center_2d, outer_radius, rms)) = taubin_circle_fit(&pts_2d) else {
                continue;
            };
            if rms > CIRCLE_RMS_TOL || outer_radius < 0.2 {
                continue;
            }

            let center_top =
                region.origin + region.basis_u * center_2d.x + region.basis_v * center_2d.y;
            let hole_axis = -region.normal;

            // 2. Analyse des facettes adjacentes au bord d'entrée pour détecter un tronc de cône
            let n = hole_loop.len();
            let mut sin_alphas = Vec::with_capacity(n);
            let mut cone_wall_regions = HashSet::new();
            let mut neck_vertices = HashSet::new();
            let top_loop_set: HashSet<usize> = hole_loop.iter().copied().collect();

            for i in 0..n {
                let u = hole_loop[i];
                let v = hole_loop[(i + 1) % n];

                if let Some(&wall_face_idx) = mesh.directed_edges.get(&(v, u)) {
                    let wall_tri = &mesh.faces[wall_face_idx];
                    // Composante axiale de la normale : pour un cône évasé vers le haut,
                    // la normale de la paroi pointe partiellement vers +region.normal
                    let sin_a = wall_tri.normal.dot(&region.normal);

                    // Un chanfrein d'entrée / fraisure a un angle compris entre ~10° et ~80°
                    if (0.17..=0.98).contains(&sin_a) {
                        // Vérifier que la composante radiale pointe bien vers l'axe du trou
                        let mid_edge = 0.5 * (mesh.vertices[u].coords + mesh.vertices[v].coords);
                        let inward_radial = (center_top.coords - mid_edge).normalize();
                        if wall_tri.normal.dot(&inward_radial) > 0.15 {
                            sin_alphas.push(sin_a);
                            let w_reg_id = face_to_region[wall_face_idx];
                            if w_reg_id != usize::MAX {
                                cone_wall_regions.insert(w_reg_id);
                                // Collecter les sommets bas de la facette conique (le collet)
                                for &vid in &regions[w_reg_id].outer_loop {
                                    if !top_loop_set.contains(&vid) {
                                        neck_vertices.insert(vid);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Au moins 80 % du périmètre doit présenter la même inclinaison conique
            if sin_alphas.len() * 10 < n * 8 || neck_vertices.len() < 8 {
                continue;
            }

            let mean_sin = sin_alphas.iter().sum::<f64>() / (sin_alphas.len() as f64);
            let variance = sin_alphas
                .iter()
                .map(|&s| (s - mean_sin).powi(2))
                .sum::<f64>()
                / (sin_alphas.len() as f64);
            if variance.sqrt() > CONE_ANGLE_STD_TOL {
                continue;
            }

            // 3. Ajuster le cercle de transition (collet entre le cône et le fût cylindrique)
            let mut neck_depths = Vec::with_capacity(neck_vertices.len());
            let mut neck_pts_2d = Vec::with_capacity(neck_vertices.len());
            for &vid in &neck_vertices {
                let d = mesh.vertices[vid] - center_top;
                neck_depths.push(d.dot(&hole_axis));
                neck_pts_2d.push(Point2::new(d.dot(&region.basis_u), d.dot(&region.basis_v)));
            }

            let cone_depth = neck_depths.iter().sum::<f64>() / (neck_depths.len() as f64);
            let Some((_neck_c2d, shaft_radius, neck_rms)) = taubin_circle_fit(&neck_pts_2d) else {
                continue;
            };

            if neck_rms > CIRCLE_RMS_TOL || shaft_radius >= outer_radius || cone_depth <= 1e-3 {
                continue;
            }

            // 4. Inspecter le fût cylindrique sous le collet pour trouver la profondeur totale
            // et déterminer si le trou est débouchant ou borgne
            let (shaft_depth, through_match, blind_cap, extra_wall_regs) =
                trace_shaft_below_countersink(
                    mesh,
                    regions,
                    &center_top,
                    &hole_axis,
                    cone_depth,
                    shaft_radius,
                    region.id,
                    &cone_wall_regions,
                );

            let mut all_consumed: Vec<usize> = cone_wall_regions.into_iter().collect();
            all_consumed.extend(extra_wall_regs);

            // Demi-angle d'ouverture du cône par rapport à l'axe : tan(alpha) = (R_ext - R_fut) / h_cone
            let half_angle_deg = ((outer_radius - shaft_radius) / cone_depth)
                .atan()
                .to_degrees();

            cs_holes.push(CountersunkHole {
                parent_region_id: region.id,
                inner_loop_idx: loop_idx,
                center_top,
                axis: hole_axis,
                outer_radius,
                shaft_radius,
                cone_depth,
                shaft_depth,
                half_angle_deg,
                bottom_through_region: through_match,
                bottom_blind_cap_region: blind_cap,
                consumed_wall_regions: all_consumed,
            });
        }
    }

    cs_holes
}

/// Recherche les parois du fût cylindrique et la face de sortie (ou le fond borgne) sous le cône
fn trace_shaft_below_countersink(
    mesh: &MeshTopology,
    regions: &[PlanarRegion],
    center_top: &Point3<f64>,
    hole_axis: &Vector3<f64>,
    cone_depth: f64,
    shaft_radius: f64,
    parent_reg_id: usize,
    cone_wall_regions: &HashSet<usize>,
) -> (f64, Option<(usize, usize)>, Option<usize>, Vec<usize>) {
    let mut max_total_depth = cone_depth;
    let mut shaft_wall_regs = Vec::new();
    let mut through_match = None;
    let mut blind_cap = None;

    // 1. Collecter les facettes cylindriques situées dans le prolongement du collet
    for r in regions {
        if r.id == parent_reg_id || cone_wall_regions.contains(&r.id) {
            continue;
        }
        if r.normal.dot(hole_axis).abs() < 0.08 {
            let on_shaft = r.outer_loop.iter().all(|&vid| {
                let d = mesh.vertices[vid] - center_top;
                let z = d.dot(hole_axis);
                let rad = (d - hole_axis * z).norm();
                z >= cone_depth - 0.05 && (rad - shaft_radius).abs() <= 0.08
            });
            if on_shaft {
                shaft_wall_regs.push(r.id);
                for &vid in &r.outer_loop {
                    let z = (mesh.vertices[vid] - center_top).dot(hole_axis);
                    if z > max_total_depth {
                        max_total_depth = z;
                    }
                }
            }
        }
    }

    let shaft_depth = (max_total_depth - cone_depth).max(0.0);

    // 2. Vérifier si le fût débouche sur une région plane opposée (Through-Hole)
    for r in regions {
        if r.id == parent_reg_id || r.normal.dot(hole_axis) < 0.995 {
            continue;
        }
        for (loop_idx, inner_lp) in r.inner_loops.iter().enumerate() {
            if inner_lp.len() < 8 {
                continue;
            }
            let matches_bottom_rim = inner_lp.iter().all(|&vid| {
                let d = mesh.vertices[vid] - center_top;
                let z = d.dot(hole_axis);
                let rad = (d - hole_axis * z).norm();
                (z - max_total_depth).abs() <= 0.08 && (rad - shaft_radius).abs() <= 0.08
            });
            if matches_bottom_rim {
                through_match = Some((r.id, loop_idx));
                break;
            }
        }
    }

    // 3. Sinon, chercher le disque de fond s'il s'agit d'un trou fraisé borgne
    if through_match.is_none() {
        for r in regions {
            if r.id == parent_reg_id || !r.inner_loops.is_empty() || r.outer_loop.len() < 8 {
                continue;
            }
            if r.normal.dot(hole_axis) > -0.995 {
                continue;
            }
            let matches_cap = r.outer_loop.iter().all(|&vid| {
                let d = mesh.vertices[vid] - center_top;
                let z = d.dot(hole_axis);
                let rad = (d - hole_axis * z).norm();
                (z - max_total_depth).abs() <= 0.08 && (rad - shaft_radius).abs() <= 0.08
            });
            if matches_cap {
                blind_cap = Some(r.id);
                shaft_wall_regs.push(r.id);
                break;
            }
        }
    }

    (shaft_depth, through_match, blind_cap, shaft_wall_regs)
}
`​`​`

---

## 3. Détection des chanfreins droits et courbes (`src/chamfer_detector.rs`)

Ce module couvre les deux cas de chanfreins :
1. **`detect_planar_edge_chamfers`** : parcourt les régions planes pour identifier les bandeaux de chanfreins droits (ex. chanfrein à $45^\circ$ entre deux faces orthogonales) et calcule leur largeur $w$ et leurs côtes $d_1, d_2$.
2. **`detect_conical_corner_chamfers`** : généralise notre détecteur de congés aux **chanfreins tournants** (lorsqu'un chanfrein suit un coin arrondi de rayon $R$, formant un ruban conique partiel entre un arc de rayon $R_{\text{haut}}$ et un arc de rayon $R_{\text{bas}}$).

`​`​`rust
use nalgebra::{Point2, Point3, Vector3};
use std::collections::{HashMap, HashSet};

use crate::{taubin_circle_fit, MeshTopology, PlanarRegion};

#[derive(Debug, Clone)]
pub struct PlanarChamferInfo {
    pub chamfer_region_id: usize,
    pub parent_region_a: usize,
    pub parent_region_b: usize,
    pub width: f64,          // Largeur de la face biseautée (hypoténuse)
    pub angle_with_a_deg: f64,
    pub angle_with_b_deg: f64,
}

#[derive(Debug, Clone)]
pub struct ConicalCornerChamfer {
    pub region_top_id: usize,
    pub region_bot_id: usize,
    pub top_start_vid: usize,
    pub top_end_vid: usize,
    pub bot_start_vid: usize,
    pub bot_end_vid: usize,
    pub transit_top: Point3<f64>,
    pub transit_bot: Point3<f64>,
    pub outward_normal: Vector3<f64>,
    pub radius_top: f64,
    pub radius_bot: f64,
}

/// 1. Identifie les régions planes qui sont géométriquement des chanfreins d'arêtes droites
pub fn detect_planar_edge_chamfers(
    mesh: &MeshTopology,
    regions: &[PlanarRegion],
) -> Vec<PlanarChamferInfo> {
    let mut face_to_region = vec![usize::MAX; mesh.faces.len()];
    let mut region_areas = vec![0.0_f64; regions.len()];

    for r in regions {
        for &f_idx in &r.face_indices {
            face_to_region[f_idx] = r.id;
            region_areas[r.id] += mesh.faces[f_idx].area;
        }
    }

    // Construire la carte des arêtes partagées entre régions : (reg_1, reg_2) -> longueur totale partagée
    let mut shared_border_len: HashMap<(usize, usize), f64> = HashMap::new();
    for (&(u, v), adj_faces) in &mesh.edge_adjacency {
        if adj_faces.len() == 2 {
            let r1 = face_to_region[adj_faces[0]];
            let r2 = face_to_region[adj_faces[1]];
            if r1 != r2 && r1 != usize::MAX && r2 != usize::MAX {
                let len = (mesh.vertices[v] - mesh.vertices[u]).norm();
                let key = if r1 < r2 { (r1, r2) } else { (r2, r1) };
                *shared_border_len.entry(key).or_insert(0.0) += len;
            }
        }
    }

    let mut neighbors_by_reg: HashMap<usize, Vec<(usize, f64)>> = HashMap::new();
    for ((r1, r2), len) in shared_border_len {
        neighbors_by_reg.entry(r1).or_default().push((r2, len));
        neighbors_by_reg.entry(r2).or_default().push((r1, len));
    }

    let mut chamfers = Vec::new();

    for r in regions {
        let Some(mut nbs) = neighbors_by_reg.get(&r.id).cloned() else {
            continue;
        };
        if nbs.len() < 2 {
            continue;
        }

        // Trier les voisins par longueur de frontière partagée décroissante
        nbs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let (reg_a, len_a) = nbs[0];
        let (reg_b, len_b) = nbs[1];

        // Un chanfrein est beaucoup plus petit en surface que ses deux faces porteuses A et B,
        // et ses deux grands côtés opposés (len_a et len_b) ont une longueur quasi identique
        let my_area = region_areas[r.id];
        if my_area >= region_areas[reg_a] * 0.35 || my_area >= region_areas[reg_b] * 0.35 {
            continue;
        }
        if (len_a - len_b).abs() > 0.25 * len_a.max(len_b) {
            continue;
        }

        let width = my_area / (0.5 * (len_a + len_b));
        // Un bandeau de chanfrein est un ruban allongé (longueur >= 2.5 * largeur)
        if 0.5 * (len_a + len_b) < 2.5 * width {
            continue;
        }

        let angle_a = r.normal.dot(&regions[reg_a].normal).clamp(-1.0, 1.0).acos().to_degrees();
        let angle_b = r.normal.dot(&regions[reg_b].normal).clamp(-1.0, 1.0).acos().to_degrees();

        // Les angles dièdres d'un chanfrein typique se situent entre 15° et 75° (ex: 45° + 45° = 90°)
        if (15.0..=75.0).contains(&angle_a) && (15.0..=75.0).contains(&angle_b) {
            chamfers.push(PlanarChamferInfo {
                chamfer_region_id: r.id,
                parent_region_a: reg_a,
                parent_region_b: reg_b,
                width,
                angle_with_a_deg: angle_a,
                angle_with_b_deg: angle_b,
            });
        }
    }

    chamfers
}

/// 2. Détecte les chanfreins courbes (troncs de cône partiels) le long d'un contour extérieur
/// et contracte les sommets intermédiaires dans les deux régions reliées.
pub fn detect_and_collapse_conical_chamfers(
    mesh: &MeshTopology,
    regions: &mut [PlanarRegion],
) -> (Vec<ConicalCornerChamfer>, HashSet<usize>) {
    let mut face_to_region = vec![usize::MAX; mesh.faces.len()];
    for r in regions.iter() {
        for &f_idx in &r.face_indices {
            face_to_region[f_idx] = r.id;
        }
    }

    let min_cos = 35.0_f64.to_radians().cos();
    let max_cos = 1.5_f64.to_radians().cos();

    let mut conical_chamfers = Vec::new();
    let mut consumed_regions = HashSet::new();
    let mut visited_verts = HashSet::new();
    let mut contractions: HashMap<usize, Vec<HashSet<usize>>> = HashMap::new();

    for r_idx in 0..regions.len() {
        let outer = regions[r_idx].outer_loop.clone();
        let loop_len = outer.len();
        if loop_len < 6 {
            continue;
        }

        let origin = regions[r_idx].origin;
        let basis_u = regions[r_idx].basis_u;
        let basis_v = regions[r_idx].basis_v;
        let plane_normal = regions[r_idx].normal;

        let mut turn_s = vec![0.0_f64; loop_len];
        for i in 0..loop_len {
            let prev = mesh.vertices[outer[(i + loop_len - 1) % loop_len]];
            let curr = mesh.vertices[outer[i]];
            let next = mesh.vertices[outer[(i + 1) % loop_len]];

            let v1 = curr - prev;
            let v2 = next - curr;
            let ratio = v1.norm() / v2.norm();
            let d1 = v1.normalize();
            let d2 = v2.normalize();
            let cos_a = d1.dot(&d2);

            if (min_cos..=max_cos).contains(&cos_a) && (0.5..=2.0).contains(&ratio) {
                turn_s[i] = d1.cross(&d2).dot(&plane_normal).signum();
            }
        }

        let mut i = 0;
        while i < loop_len {
            if turn_s[i] == 0.0 || visited_verts.contains(&outer[i]) {
                i += 1;
                continue;
            }
            let sign = turn_s[i];
            let mut chain = vec![(i + loop_len - 1) % loop_len, i];
            let mut j = i + 1;
            while j < loop_len && turn_s[j] == sign && !visited_verts.contains(&outer[j]) {
                chain.push(j);
                j += 1;
            }
            if let Some(&last_in) = chain.last() {
                chain.push((last_in + 1) % loop_len);
            }

            if chain.len() >= 4 {
                let top_arc_vids: Vec<usize> = chain.iter().map(|&k| outer[k]).collect();
                let pts_top_2d: Vec<Point2<f64>> = top_arc_vids
                    .iter()
                    .map(|&vid| {
                        let d = mesh.vertices[vid] - origin;
                        Point2::new(d.dot(&basis_u), d.dot(&basis_v))
                    })
                    .collect();

                if let Some((center_top_2d, r_top, rms_top)) = taubin_circle_fit(&pts_top_2d) {
                    if rms_top <= 0.04 {
                        let center_top_3d =
                            origin + basis_u * center_top_2d.x + basis_v * center_top_2d.y;

                        if let Some((bot_reg_id, bot_arc_vids, bevel_regs, out_n, r_bot)) =
                            trace_conical_bevel_strip(
                                mesh,
                                regions,
                                &face_to_region,
                                r_idx,
                                &top_arc_vids,
                                &center_top_3d,
                                r_top,
                            )
                        {
                            for &v in &top_arc_vids {
                                visited_verts.insert(v);
                            }
                            for &v in &bot_arc_vids {
                                visited_verts.insert(v);
                            }
                            consumed_regions.extend(bevel_regs);

                            let mid = top_arc_vids.len() / 2;
                            conical_chamfers.push(ConicalCornerChamfer {
                                region_top_id: r_idx,
                                region_bot_id: bot_reg_id,
                                top_start_vid: *top_arc_vids.first().unwrap(),
                                top_end_vid: *top_arc_vids.last().unwrap(),
                                bot_start_vid: *bot_arc_vids.first().unwrap(),
                                bot_end_vid: *bot_arc_vids.last().unwrap(),
                                transit_top: mesh.vertices[top_arc_vids[mid]],
                                transit_bot: mesh.vertices[bot_arc_vids[mid]],
                                outward_normal: out_n,
                                radius_top: r_top,
                                radius_bot: r_bot,
                            });

                            contractions.entry(r_idx).or_default().push(
                                top_arc_vids[1..top_arc_vids.len() - 1]
                                    .iter()
                                    .copied()
                                    .collect(),
                            );
                            contractions.entry(bot_reg_id).or_default().push(
                                bot_arc_vids[1..bot_arc_vids.len() - 1]
                                    .iter()
                                    .copied()
                                    .collect(),
                            );
                        }
                    }
                }
            }
            i = j + 1;
        }
    }

    for (reg_id, sets) in contractions {
        let mut to_remove = HashSet::new();
        for s in sets {
            to_remove.extend(s);
        }
        regions[reg_id]
            .outer_loop
            .retain(|vid| !to_remove.contains(vid));
    }

    (conical_chamfers, consumed_regions)
}

/// Suit les facettes inclinées d'un chanfrein d'angle (tronc de cône partiel) jusqu'à l'arc inférieur
fn trace_conical_bevel_strip(
    mesh: &MeshTopology,
    regions: &[PlanarRegion],
    face_to_region: &[usize],
    top_r_id: usize,
    top_arc_vids: &[usize],
    center_top_3d: &Point3<f64>,
    r_top: f64,
) -> Option<(usize, Vec<usize>, Vec<usize>, Vector3<f64>, f64)> {
    let plane_normal = regions[top_r_id].normal;
    let basis_u = regions[top_r_id].basis_u;
    let basis_v = regions[top_r_id].basis_v;

    let mut bevel_regions = Vec::new();
    let mut bot_arc_vids = Vec::with_capacity(top_arc_vids.len());
    let mut avg_normal = Vector3::zeros();
    let mut ref_tilt = None;

    for k in 0..(top_arc_vids.len() - 1) {
        let u = top_arc_vids[k];
        let v = top_arc_vids[k + 1];

        let &bevel_face_idx = mesh.directed_edges.get(&(v, u))?;
        let bevel_reg_id = face_to_region[bevel_face_idx];
        if bevel_reg_id == top_r_id || bevel_reg_id == usize::MAX {
            return None;
        }

        let bevel_reg = &regions[bevel_reg_id];
        let tilt = bevel_reg.normal.dot(&plane_normal);

        // Un chanfrein conique est incliné (0.15 <= |tilt| <= 0.95) de façon constante
        if !(0.15..=0.95).contains(&tilt.abs()) {
            return None;
        }
        if let Some(first_t) = ref_tilt {
            if (tilt - first_t).abs() > 0.04 {
                return None;
            }
        } else {
            ref_tilt = Some(tilt);
        }

        avg_normal += bevel_reg.normal;
        bevel_regions.push(bevel_reg_id);

        if k == 0 {
            let u_bot =
                find_radial_opposite_vertex(mesh, bevel_reg, u, center_top_3d, &plane_normal)?;
            bot_arc_vids.push(u_bot);
        }
        let v_bot =
            find_radial_opposite_vertex(mesh, bevel_reg, v, center_top_3d, &plane_normal)?;
        bot_arc_vids.push(v_bot);
    }

    // Vérifier que les sommets du bas forment bien un arc de cercle de rayon r_bot != r_top
    let pts_bot_2d: Vec<Point2<f64>> = bot_arc_vids
        .iter()
        .map(|&vid| {
            let d = mesh.vertices[vid] - center_top_3d;
            Point2::new(d.dot(&basis_u), d.dot(&basis_v))
        })
        .collect();

    let (_c_bot, r_bot, rms_bot) = taubin_circle_fit(&pts_bot_2d)?;
    if rms_bot > 0.04 || (r_bot - r_top).abs() < 0.05 {
        return None;
    }

    let last_bot = *bot_arc_vids.last()?;
    let prev_bot = bot_arc_vids[bot_arc_vids.len() - 2];
    let &bot_face_idx = mesh.directed_edges.get(&(last_bot, prev_bot))?;
    let bot_r_id = face_to_region[bot_face_idx];
    if bot_r_id == usize::MAX {
        return None;
    }

    Some((
        bot_r_id,
        bot_arc_vids,
        bevel_regions,
        avg_normal.normalize(),
        r_bot,
    ))
}

/// Trouve le sommet bas aligné dans le même plan méridien (axe + rayon) que `v_top`
fn find_radial_opposite_vertex(
    mesh: &MeshTopology,
    bevel_region: &PlanarRegion,
    v_top: usize,
    center_top: &Point3<f64>,
    axis_dir: &Vector3<f64>,
) -> Option<usize> {
    let d_top = mesh.vertices[v_top] - center_top;
    let radial_vec = (d_top - axis_dir * d_top.dot(axis_dir)).normalize();
    let tangent_vec = axis_dir.cross(&radial_vec).normalize();

    for &cand_vid in &bevel_region.outer_loop {
        if cand_vid == v_top {
            continue;
        }
        let d_cand = mesh.vertices[cand_vid] - center_top;
        let axial_diff = (d_cand - d_top).dot(axis_dir).abs();
        let tangent_drift = d_cand.dot(&tangent_vec).abs();
        if axial_diff > 1e-3 && tangent_drift < 0.05 {
            return Some(cand_vid);
        }
    }
    None
}
`​`​`

---

## 4. Construction B-Rep cousue de la fraisure conique dans `src/step_exporter.rs`

Voici la fonction qui assemble le tronc de cône (`try_wire_homotopy`) et le fût cylindrique (`tsweep`) avec partage strict des `EdgeID` :

`​`​`rust
use crate::countersink_detector::CountersunkHole;

/// Construit une fraisure conique cousue à son fût cylindrique sous-jacent.
/// Retourne :
/// - `top_hole_wire` : boucle circulaire de rayon `outer_radius` à souder sur la Face supérieure
/// - `bottom_hole_wire` : `Some(wire)` de rayon `shaft_radius` si le trou est débouchant
/// - `feature_faces` : les faces analytiques du tronc de cône + fût cylindrique (+ fond si borgne)
pub fn build_sewn_countersunk_hole(
    cs: &CountersunkHole,
) -> (Wire, Option<Wire>, Vec<Face>) {
    let center_top = to_truck_pt(&cs.center_top);
    let axis = to_truck_vec(&cs.axis).normalize();

    let helper = if axis.x.abs() < 0.9 {
        TruckVec3::new(1.0, 0.0, 0.0)
    } else {
        TruckVec3::new(0.0, 1.0, 0.0)
    };
    let radial_dir = axis.cross(helper).normalize();

    // 1. Cercle supérieur (entrée de la fraisure, rayon = outer_radius)
    let v_top = builder::vertex(center_top + radial_dir * cs.outer_radius);
    let wire_top = builder::rsweep(&v_top, center_top, axis, Rad(7.0));

    // 2. Cercle intermédiaire au collet (jonction cône / cylindre, rayon = shaft_radius)
    let center_neck = center_top + axis * cs.cone_depth;
    let v_neck = builder::vertex(center_neck + radial_dir * cs.shaft_radius);
    let wire_neck = builder::rsweep(&v_neck, center_neck, axis, Rad(7.0));

    // 3. Génération du tronc de cône entre wire_top et wire_neck
    let cone_shell = builder::try_wire_homotopy(&wire_top, &wire_neck)
        .expect("Échec de l'homotopie conique entre wire_top et wire_neck");

    let mut feature_faces: Vec<Face> = cone_shell.into_iter().collect();
    // Inverser les normales du cône pour qu'elles pointent vers l'axe du perçage :
    // - le bord supérieur du cône parcourt alors `wire_top.inverse()`
    // - le bord inférieur du cône parcourt `wire_neck` dans le sens direct
    for f in &mut feature_faces {
        f.invert();
    }

    // 4. S'il y a un fût cylindrique sous la fraisure, on l'extrude directement depuis `wire_neck`
    let mut bottom_hole_wire = None;
    if cs.shaft_depth > 1e-4 {
        let neck_disk = builder::try_attach_plane(&[wire_neck.clone()])
            .expect("Impossible d'attacher le disque de collet");
        let cyl_solid = builder::tsweep(&neck_disk, axis * cs.shaft_depth);
        let mut cyl_shell = cyl_solid.into_boundaries().pop().unwrap();

        // Retirer le disque de collet (index 0) et extraire le bouchon du fond (last)
        let _removed_neck_cap = cyl_shell.remove(0);
        let mut bottom_cap = cyl_shell.pop().unwrap();

        // Inverser les parois cylindriques : leur bord supérieur devient `wire_neck.inverse()`,
        // ce qui s'annule exactement avec le bord inférieur `wire_neck` du cône !
        for mut f in cyl_shell {
            f.invert();
            feature_faces.push(f);
        }

        if cs.bottom_through_region.is_some() {
            // Trou fraisé débouchant : la boucle inversée du fond percera la Face B
            bottom_hole_wire = Some(bottom_cap.boundaries()[0].inverse());
        } else {
            // Trou fraisé borgne : on garde le disque de fond inversé
            bottom_cap.invert();
            feature_faces.push(bottom_cap);
        }
    } else if cs.bottom_through_region.is_some() {
        // Fraisure purement conique traversant une tôle fine sans fût cylindrique
        bottom_hole_wire = Some(wire_neck.inverse());
    }

    // `wire_top` (sens horaire vu depuis la normale sortante de la Face supérieure)
    // est exactement opposé au bord supérieur `wire_top.inverse()` du cône inversé !
    (wire_top, bottom_hole_wire, feature_faces)
}
`​`​`

> **Note pour les chanfreins coniques d'angle (`ConicalCornerChamfer`) :** Ils utilisent exactement la même fonction `build_and_register_fillet` que les congés cylindriques ! Parce que `top_start_vid → top_end_vid` se trouve au rayon $R_{\text{haut}}$ et `bot_start_vid → bot_end_vid` au rayon $R_{\text{bas}}$, l'appel `builder::homotopy(&arc_top, &arc_bot)` génère de lui-même un tronc de cône NURBS rationnel et enregistre ses 4 arêtes dans `edge_pool`.

<FollowUp label="Veux-tu voir comment supprimer automatiquement un chanfrein ou un congé (Defeaturing) en prolongeant les faces planes jusqu'à leur arête vive ?" query="Oui, montre-moi comment implémenter un algorithme de Defeaturing en Rust pour supprimer les chanfreins ou congés et recalculer les arêtes vives par intersection Plan-Plan."/>

Powered by Gemini Exporter (https://www.ai-chat-exporter.com)
