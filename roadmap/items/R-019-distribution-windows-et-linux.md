---
id: R-019
titre: "Distribution — Windows et Linux"
resume: "Construire et qualifier la distribution sur Windows et Linux."
statut: a-tester
jalon: distribution
cree: 2026-11-13
maj: 2026-11-20
livree:
---
## But

Installer FabSTEP sur un PC Windows ou Ubuntu et reconstruire localement un solide STEP à partir d’un STL ou d’un 3MF.

## Inclus

- Windows x64. Distribuer l’atelier et le CLI dans un ZIP portable, avec les bibliothèques OCCT nécessaires et leurs notices.
- Ubuntu 26.04 amd64. Installer l’atelier et le CLI avec un paquet Debian dont les dépendances natives sont résolues par APT.
- Mettre les paquets, les empreintes SHA-256 et les sources à disposition dans la release GitHub.

## Hors périmètre

Les autres architectures et distributions, la signature Authenticode et le portage du superviseur gros STL feront l’objet de qualifications distinctes. Le superviseur mémoire de la v1 est propre à macOS.

## Dépendances

La révision v1.0 du moteur, le SDK OCCT 7.9.x, WebView2 sous Windows et WebKitGTK sous Ubuntu.

## Critères d'acceptation

Compiler l’application avec l’interface embarquée sur chaque système. Lancer le bureau depuis les fichiers distribués et observer sa fenêtre. Convertir le cube et l’octant, vérifier le nombre de solides et de faces après réimport. Installer le paquet Ubuntu dans un environnement distinct de celui de compilation. Vérifier les empreintes des fichiers publiés sur GitHub.

## Vérifications de l'utilisateur

Ouvrir un fichier personnel dans le paquet correspondant à son système puis exporter le résultat STEP. Signaler le système, l’architecture et le message affiché en cas de difficulté.
