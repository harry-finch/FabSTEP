# ADR-0037 — Chaînes de révolution à frontières mesurées

Statut : décision retenue pour R-076 et R-078 dans la poursuite globale autorisée par l’utilisateur. La validation et ses limites figurent dans V-1201 ; l’acceptation utilisateur reste distincte.

## Décision

Étendre les profils reconnus par les modèles existants, sans nouveau DTO, ABI, dépendance ou seuil. Chaque changement de surface repose sur un anneau source complet et un découpage unique. Les reconnaissances historiques, dont R-060 et R-075, gardent leur priorité.

R-076 admet deux grammaires supplémentaires sur un tour complet : plan–quart de tore–cylindre–cône–cylindre–cône–plan avec cônes de signes opposés, et plan–tore–cône–plan avec arc compris entre 90° et 180°, axialement monotone. Les paramètres du tore et de sa jonction sont calculés conjointement depuis le plan et les portions droites. Le second cas exige un véritable témoin du retournement radial ; le domaine angulaire traverse la couture ±π sans changer de branche. Les bandes droites à deux anneaux exigent les mêmes preuves continues et topologiques que R-075. Une tangence ne se déduit pas d’un résidu faible aux seuls sommets.

R-078 admet deux à huit cônes adjacents ou mêlés à des portées résolues, avec au moins deux anneaux témoins intérieurs séparés dans chaque nouvelle portion. Les pentes signées sont séparées au-delà de leurs incertitudes ; tous les découpages admissibles concurrents sont recherchés dans le budget commun. Un deuxième découpage, une portion non résolue ou un dépassement du domaine de huit cônes rend cette extension non concluante, même si un candidat a déjà été trouvé ; Auto conserve son repli historique. Un véritable épuisement du budget de travail reste bloquant. Aucun intervalle de pente flottant non encadré ne supprime de candidat : la revue a fait retirer un tel raccourci, avec une régression à grandes coordonnées et très faible tolérance.

Les rayons mesurés sont conservés avant les essais historiques qui peuvent canonicaliser une portée. Ils sont restaurés pour les chemins supplémentaires, sans les recalculer dans un ordre modifié. Chaque triangle doit relier des anneaux consécutifs et appartient à une seule portion ; la preuve native, la couverture UV et la borne continue restent obligatoires.

## Travail et limites

Le contrôle Auto des supports plans indexe les ensembles exacts d’arêtes de leurs anneaux : longueur et première arête ne servent que de filtre, l’égalité complète décide. Les doublons conservent leurs indices et l’ordre des contrôles. L’index est paresseux et sa construction/recherche est comptabilisée. Cette optimisation enlève des constructions et comparaisons répétées ; elle ne retire aucun support du contrôle de coplanarité.

Le préfiltre sphérique facture une unité de préparation puis quatre unités par sommet réellement visité, avant son examen. Il ne réserve plus quatre unités pour tous les sommets après un rejet précoce. Le plafond reste dix millions ; un parcours complet coûte une unité de plus, et un budget épuisé n’est jamais renouvelé. C’est une correction de comptabilisation, pas un gain de vitesse revendiqué.

Le guidage Tore avec voisins coniques, les méridiennes libres, les retours axiaux et les lois arbitraires restent distincts. La réduction des faces de Flansch ne remplace ni son contrôle de capacité ni sa validation source/native. Les cas et mesures effectifs sont consignés dans VALIDATION, sans promesse d’export du fichier entier.
