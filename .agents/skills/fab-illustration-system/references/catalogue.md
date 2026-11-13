# Catalogue

Le paquet `@lucasmarkes/hairline` (version 0.5.1 à la rédaction) exporte trente-trois figures, une fonction par figure : `import { drawer } from '@lucasmarkes/hairline'`. Un import ne charge que la figure nommée (quelques kilo-octets).

## Les figures de la famille, par état

**Même état, même figure dans toutes les applications** : l'utilisateur qui passe de FabSpeed à FabVents reconnaît la situation avant de lire la phrase.

| État | Figure | Ce qu'elle montre | Description (`figure.<nom>`) |
| --- | --- | --- | --- |
| Bibliothèque vide (fraises, matières, machines, gabarits) | `drawer` | Un meuble à trois tiroirs ; celui qu'on ouvre n'a que ses séparateurs | « Un meuble à trois tiroirs vides. » |
| Recherche sans résultat | `loupe` | Une loupe sur pied au-dessus d'une feuille lignée vierge | « Une loupe posée sur une feuille vierge. » |
| Filtre qui ne laisse rien passer | `sieve` | Trois tamis empilés, toutes les toiles nues | « Trois tamis empilés, vides. » |
| Aucune mesure, aucun essai, aucun historique | `plot` | Un graphique en barres dont les barres sont à plat | « Un graphique sans données. » |
| Moteur indisponible, machine déconnectée | `plug` | Une prise murale, et une fiche restée au sol au bout de son cordon | « Une fiche débranchée devant sa prise. » |
| Fichier illisible, format inconnu | `query` | Un point d'interrogation bâti comme un objet, son point en bille libre | « Un point d'interrogation. » |
| Accès verrouillé (droits, machine réservée) | `padlock` | Un cadenas dont l'anse se lève à l'approche | « Un cadenas fermé. » |
| Aucun document ouvert, premier lancement, site | **Figure du métier** (maison) | L'objet que l'application fabrique ou règle | à écrire avec la figure |

Un état qui n'est pas dans ce tableau : chercher dans la liste ci-dessous ; s'il n'y a rien de juste, garder l'icône et la phrase, ou proposer une figure maison à l'utilisateur. Ne pas détourner une figure vers un sens qu'elle n'a pas (un cadenas pour « enregistré », un tamis pour « export »).

## Les trente-trois figures

`intensity` règle la force de la réponse ; la colonne « Dans l'application » dit si la figure convient à un des emplacements de l'application, ou seulement au site et à la documentation.

### Tenir ou chercher, et ne rien trouver

| Fonction | Ce que c'est, ce que fait le pointeur | Dans l'application |
| --- | --- | --- |
| `drawer` | Meuble à trois tiroirs ; la hauteur du pointeur en choisit un, qui s'ouvre sur deux séparateurs et rien entre eux | Oui : bibliothèque vide |
| `loupe` | Loupe sur pied sur une feuille lignée vierge ; le pointeur la promène | Oui : recherche sans résultat |
| `sieve` | Trois tamis sur un bac ; la hauteur du pointeur en soulève un | Oui : filtre sans résultat |
| `plot` | Graphique en barres à sept onglets plats ; ils se soulèvent un peu et retombent à zéro | Oui : aucune mesure |
| `plug` | Prise murale et fiche au sol ; le pointeur tire la fiche vers la prise, elle s'arrête avant | Oui : moteur ou machine hors service |
| `query` | Point d'interrogation en barre cintrée sur une bille ; le crochet tourne vers le pointeur | Oui : fichier ou format inconnu |
| `rail` | Portant à sept cintres nus ; le pointeur les fait balancer | Possible : collection vide |
| `basket` | Panier en fil sous une anse ; il penche vers le pointeur et montre son fond nu | Site (objet de commerce) |

### Objets et matériel

| Fonction | Ce que c'est, ce que fait le pointeur | Dans l'application |
| --- | --- | --- |
| `padlock` | Cadenas ; l'anse sort et pivote à l'approche du pointeur | Oui : accès verrouillé |
| `vault` | Porte de coffre à cadran et trois pênes ; le pointeur tourne le cadran | Possible : accès, secrets |
| `lockers` | Douze casiers, un entrouvert ; celui sous le pointeur s'ouvre | Possible : réservations, casiers d'atelier |
| `cabinet` | Baie de douze lames, quelques-unes à moitié sorties | Site |
| `keyboard` | Clavier de soixante touches ; la touche sous le pointeur s'enfonce, ses voisines suivent | Site : raccourcis |
| `laptop` | Ordinateur portable ; la hauteur du pointeur règle l'écran | Site |
| `phone` | Téléphone en couches (verre, carte, batterie, coque) | Site |
| `terminal` | Fenêtre de terminal ; la ligne sous le pointeur se soulève | Site |
| `elevator` | Quatre étages et une cage ouverte ; la cabine va à l'étage du pointeur | Site |
| `router` | Routeur ; les antennes penchent vers le pointeur | Site : réseau |
| `dish` | Parabole sur cardan ; le pointeur l'oriente | Site |
| `patch` | Panneau de brassage à vingt-quatre ports | Site |

### Structures

| Fonction | Ce que c'est, ce que fait le pointeur | Dans l'application |
| --- | --- | --- |
| `stack` | Pile de cinq cadres ; ceux du dessus se soulèvent pour ouvrir celui qu'on vise | Possible : couches, passes |
| `hub` | Un moyeu et huit tuiles reliées en pointillé ; la tuile visée monte, son lien devient plein | Site : machines reliées |
| `relay` | Un moyeu et quatre branches ; le chemin vers la feuille visée s'allume pas à pas | Site |
| `settle` | Douze tuiles de travers autour d'un moyeu, qui se rangent en arbre à l'approche | Site |
| `rebuild` | Arbre de paquets ; celui qu'on touche monte, puis tout ce qui en dépend | Site |
| `branches` | Graphe de commits avec une branche qui part et revient | Site : développement |
| `format` | Fichier de dix lignes de travers, que le pointeur remet d'équerre | Site |

### Les six d'origine

| Fonction | Ce que c'est, ce que fait le pointeur | Dans l'application |
| --- | --- | --- |
| `terrain` | Quatre-vingt-un piliers sur un socle, qui montent autour du pointeur ; au repos, une dune | Possible : relief, carte de hauteur |
| `exploded` | Fenêtre d'application en quatre couches ; on écarte les couches, on en choisit une | Possible : couches |
| `turntable` | Blocs sur un plateau tournant ; une chiquenaude le lance, il s'arrête au quart de tour | Possible : rotation |
| `riffle` | Bac de huit fiches ; celle sous le pointeur se lève ; les flèches du clavier les parcourent (groupe focalisable) | Possible : fiches, essais |
| `phosphor` | Matrice de points qui joue une boucle et garde la trace du pointeur | **Non** : ambiante |
| `slow` | Caisses sur un tapis roulant ; le survol ralentit le temps | **Non** : ambiante |

« Possible » : la figure est admise si son objet dit vraiment l'état ; sinon préférer une figure du tableau de la famille ou une figure maison.

## Ce que le catalogue n'a pas

Le catalogue vient du logiciel et du bureau. Les objets de l'atelier sont à dessiner avec `hairline-create` ([creer-et-adopter.md](creer-et-adopter.md)), une figure par application, celle de son métier :

| Application | Objet | Ce que le pointeur règle |
| --- | --- | --- |
| FabSpeed | **`endmill`** (faite) : une fraise couchée, pointe vers le lecteur | Le pointeur la fait rouler ; paramétrique, elle prend la forme de la fiche affichée |
| FabVents | Une plaque perforée | La taille des perforations autour du pointeur |
| FabWeave | Une buse au-dessus d'une pièce tissée en couches | La couche montrée |
| FabCAM | Un portique au-dessus d'un martyr | La position de la broche |

Hors `endmill`, ce sont des points de départ : la skill propose deux ou trois concepts, l'utilisateur choisit.
