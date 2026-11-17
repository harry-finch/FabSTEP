# ADR-0033 — Profil exact à deux arcs toriques et une portée

Date : 17 novembre 2026. Statut : retenue pour R-061, validation automatisée réussie (V-1186), essais utilisateur ouverts.

## Contexte

Le modèle actuel représente des tores à domaine méridien borné, mais le classificateur limite une chaîne à un seul raccord. Autoriser deux raccords sans traitement commun peut réécrire deux fois le rayon de leur cylindre partagé et rendre les frontières incompatibles.

## Décision

Admettre uniquement la chaîne quart de tore extérieur–cylindre–quart de tore extérieur, sur un tour complet entre deux plans source, avec portée résolue et témoins intérieurs. Les deux rayons de raccord peuvent différer ; une même géométrie cylindrique doit satisfaire les deux transitions. Construire les paramètres ensemble, puis contrôler tous les sommets, frontières et déplacements par les preuves existantes. Aucun écrasement successif d’un rayon déjà fixé.

Conserver le chemin historique à un tore. Les autres chaînes et méridiennes restent refusées selon leur contrat. Aucun ajustement libre ni nouvelle surface paramétrique, ABI ou dépendance. Le volume résulte de l’intégrale analytique existante, additionnée sur les surfaces ; le réimport, les dimensions et l’audit bidirectionnel restent nécessaires.

## Conséquences et limites

Cette tranche réalise un premier profil exact ligne/arcs de R-033. Elle ne promet pas un profil arbitraire, un secteur angulaire, des retours axiaux ou une méridienne lisse ajustée. Les profils non tangents, portées absentes/ambiguës et dépassements de budgets restent refusés. Le corpus et les résultats exécutés sont distingués dans VALIDATION.
