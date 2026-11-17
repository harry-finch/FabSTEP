# ADR-0029 — Reconstruction automatique locale et sélection de paroi

Date : 16 novembre 2026. Statut : acceptée dans R-057, parcours global approuvé par l’utilisateur.

## Contexte

Les stratégies explicites échouent lorsqu’un candidat local n’est pas prouvé. Le guidage imposait une base facettisée et la sélection par voisinage ne distinguait pas les parois. L’utilisateur demande une reconstruction automatique, une sélection rapide et des corrections conservant les autres formes.

## Décision

Ajouter une politique `Auto` distincte, sans changer les modes historiques. Les candidats arrondis sont prouvés avant assemblage ; les refus géométriques locaux restent facettisés et sont diagnostiqués. Les limites de travail, défauts source/topologie, options invalides et contrôles natifs ne sont pas absorbés. Aucun paramètre n’est augmenté implicitement. Les régions planes polygonales non constructibles sont subdivisées en triangles source avant la génération de provenance ; un support de cercle incompatible exclut son candidat avant assemblage.

Le modèle local unifié est `RoundedRecognition` avec attribution exhaustive. Le noyau expose construction suivie additive pour plans, cylindres complets/partiels, cônes, sphères et tores : jetons associés aux faces effectivement construites puis suivis après couture et transfert STEP. Le cas entièrement plan réutilise le constructeur planaire. Aucun objet OCCT ne traverse l’ABI. Le modèle spécifique RoundedBox reste dans la stratégie historique avancée ; pas de fausse provenance ni correction locale promise pour celui-ci.

Les indications Auto/Plan/Cylindre/Cône/Tore sont rejouées sur la source immuable. En Auto, la reconstruction automatique indépendante est conservée ; en base Plans, les parois courbes extérieures aux indications restent facettisées. Un groupe Tore inclut sa portée cylindrique directe ; les chaînes impliquant un cône sont refusées en guidage torique. Les anciennes sessions Cylindre/Cône/Plan gardent leur parcours historique. Le retrait recalcule la base et les autres indications, sans restauration d’octets STEP anciens.

`suggest_region` lit un snapshot du résultat, résout les triangles source puis leur groupe géométrique, et renvoie seulement des identifiants de faces entières du même résultat. La sélection est refusée si un groupe coupe une face existante. Révision, source, époque et identité du résultat sont contrôlées avant/après calcul ; aucun cache export n’est invalidé. Côté UI, un double-clic est distingué du second toggle ; une seule requête native active et la dernière attente, puis une sélection groupée validée dans le worker. Les réponses périmées sont ignorées.

## Conséquences et limites

Auto est le choix initial du bureau ; migration de préférence versionnée conservant unité et tolérances. CLI/Rust gardent le défaut Planar pour compatibilité. Les modes explicites restent disponibles dans Avancé. Les refus locaux ne garantissent pas toute reconnaissance possible : les contours ouverts couplés ambigus peuvent conserver plusieurs arcs facettisés. Une sélection Auto peut confirmer une face plane sans inventer de courbe. Les preuves finales et l’audit restent obligatoires ; ce parcours n’est pas une reconnaissance universelle ni un historique paramétrique.
