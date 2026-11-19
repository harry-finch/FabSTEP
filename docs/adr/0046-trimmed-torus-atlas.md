# ADR-0046 — Domaines toriques sur STL dense

Le mode Auto du helper peut essayer un second constructeur sur un STL à un corps lorsque la reconnaissance habituelle dépasse son budget de travail ou produirait plus de 100 000 faces. Le core déduit les supports et leurs boucles isoparamétriques du maillage. Chaque domaine expose ses triangles source et une borne de déviation. Chaque triangle résiduel garde sa propre face. Aucune fermeture supplémentaire n’est créée.

La CAO partage les arêtes des domaines avec leurs voisins, recoud le corps et contrôle les identités des faces au transfert STEP. Le helper certifie la source, vérifie le solide réimporté, ses dimensions et son volume puis exécute l’audit de distance bidirectionnel habituel. Le calcul est refusé si l’une des preuves ou un plafond échoue. Les budgets du bureau et les contrats de fichier du helper sont conservés. Voir V-1244 à V-1250.
