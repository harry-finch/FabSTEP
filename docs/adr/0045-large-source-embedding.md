# ADR-0045 — Vérification spatiale d’une source triangulée de grande taille

## Contexte

Le helper dispose de 4 Gio d’empreinte physique. Transformer les 540 170 triangles de Flansch en autant de faces OCCT pour le seul contrôle de la source dépassait déjà 7 Gio. Avant une reconstruction analytique, il faut prouver que la géométrie du maillage respecte la topologie validée par le core.

## Décision

Dans le profil `large-stl`, `fabstep-verify` peut certifier un corps triangulé déjà analysé. Le core établit l’orientation, la connexité de la coque et des liens de sommets, l’incidence et le volume. Un arbre de boîtes examine ensuite les paires dont la séparation spatiale reste à démontrer. Les prédicats contrôlent séparément les triangles disjoints, ceux qui partagent un sommet et ceux qui partagent une arête. Une séparation numérique incertaine ou un budget épuisé refuse le certificat.

Le certificat concerne un seul corps source. L’assemblage multi-corps conserve ses contrôles entre corps. La construction CAO, la réimportation du STEP, les mesures de volume et dimensions, la tessellation et le contrôle de fidélité restent nécessaires pour l’export. Le certificat ne requiert aucune IO, aucun objet OCCT ni nouvelle dépendance. Le profil ordinaire garde son chemin de validation historique.

## Vérifications

V-1246 consigne les essais de recouvrement, de contact, de budget et les mesures sur Flansch.
