---
id: R-115
titre: "Récupérer les plans fragmentés par la quantification du STL"
resume: "Essayer une graine de grande aire sur les régions planes fragmentées en f32. Vérifier le contour, la provenance et les plis sur des solides avant de retenir une seconde segmentation."
statut: idee
cree: 2026-11-19
maj: 2026-11-19
livree:
---

## But

Retrouver les plans d’un solide quand la quantification du STL binaire fragmente une face dont les premiers triangles sont petits. Sur une pièce pliée, les deux côtés du pli doivent rester sélectionnables séparément.

## Inclus

- Essai déterministe avec des graines choisies par aire décroissante, en conservant les indices source, l’ordre des sommets, les coordonnées et les tolérances du document.
- Sélection du résultat uniquement si les contours fermés, le plan effectivement utilisé par le noyau, la couverture des triangles et les contrôles du STEP réimporté sont démontrés.
- Mesure du coût et de la qualité sur un parallélépipède tourné, sa variante pliée et un solide qui échoue aux deux ordres. Un second essai consomme un budget borné.

## Hors périmètre

Le lissage de normales relève de R-116. Les raccords toriques de Flansch et la réparation générale d’un contour ramifié relèvent d’autres travaux.

## Dépendances

Les essais de R-112 fournissent la référence de segmentation. La comparaison porte sur le même maillage soudé pour les deux ordres de graines.

## Critères d'acceptation

- Sur les deux petits STL binaires fermés, obtenir respectivement six et sept faces planes en sortie, avec les 96 triangles source attribués au bon plan sur chaque pièce.
- Réimporter chacun des STEP, vérifier le nombre de solides, le volume, le contour des faces et la fidélité dans les deux directions.
- Préserver un pli au-delà de la tolérance. Si aucun ordre ne donne une partition valide, conserver le refus ou le repli documenté du parcours initial.
- Comparer les références existantes et le temps de calcul, y compris près du plafond ordinaire de triangles. Rejeter les variantes qui dégradent les contrats existants.

## Vérifications de l'utilisateur

- Ouvrir les deux témoins dans FabSTEP, inspecter les six et sept faces du résultat puis contrôler le pli de la seconde pièce dans Fusion.
