---
id: A-062
type: valider
titre: "Valider les conclusions de qualification des gros STL"
element: R-072
statut: a-faire
cree: 2026-11-17
par: agent
fait:
---

Lire V-1196 dans docs/VALIDATION.md et le résultat de R-072. Aucun nouveau build : Flansch conserve 501 912 faces et reste refusé. Le regroupement des plans ne suffit pas ; quatre zones courbes totalisent 278 642 triangles encore non reconstruits. Suite recommandée : qualifier d’abord la surveillance mémoire (R-073, empreinte physique observée de 6.9G malgré un RSS inférieur au plafond), puis leurs profils et le coût de validation native source avant de proposer une extension précise. Confirmer ces conclusions et cette suite, ou indiquer une autre priorité. Aucune hausse de limite ni simplification du maillage proposée à ce stade.
