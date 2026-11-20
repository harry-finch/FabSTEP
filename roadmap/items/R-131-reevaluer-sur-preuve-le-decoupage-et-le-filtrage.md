---
id: R-131
titre: "Réévaluer sur preuve le découpage et le filtrage des groupes OCCT"
resume: "F3 et F4 · Rouvrir séparément les deux essais de V-1276 si un groupe chaud représentatif permet de démontrer un gain."
statut: en-cours
jalon: audit
cree: 2026-11-20
maj: 2026-11-20
livree:
---
## But

Déterminer si un groupe coûteux peut réellement soumettre moins de faces au noyau. Sur le secteur de V-1276, découper les composés ajoute 18 appels et écarter les 15 paires candidates laisse les mêmes faces à contrôler.

## Inclus

- F3. À partir de données archivées exploitables du groupe (1880, 1881), ou d’un témoin public équivalent, compter les composantes du graphe biparti, leurs tailles et les appels nécessaires. Conserver les groupes diagonaux entiers.
- F4. Pour les colliers bilinéaires à poids positifs, borner le support et ses bords avec placements et tolérances physiques. Prouver la séparation pour les géométries réellement relues après transfert. Un cas tangent, incertain ou hors domaine reste candidat.
- Mesurer séparément les deux essais, puis leur combinaison seulement si les deux sont concluants. Compter les faces retirées des composés et le coût de l’ensemble des appels.

## Hors périmètre

Généralisation aux B-splines quelconques, exclusion par simple distance entre triangles et relance privée pour produire les données de départ.

## Dépendances

V-1262, V-1275 et V-1276. Réévaluer après R-123 si le profil de coût résiduel le justifie. Le résultat de R-130 donne la référence de production s’il a été retenu.

## Critères d'acceptation

- F3 conserve chaque paire requise dans un même composé mono-forme. F4 fournit sur un témoin reproductible la preuve d’exclusion et retire effectivement des faces d’un composé.
- Comparer au contrôle exhaustif sur petits solides fermés, avec triangles sécants, contacts proches de la tolérance, sommets partagés et colliers obliques. Exiger les mêmes verdicts avant et après transfert.
- Trois répétitions sur groupe chaud, rampe et secteur. Pour intégrer une piste, documenter une réduction de coût au-delà de la dispersion sur le témoin chaud, sans régression significative sur les contrôles complets publics et sous 4 Gio. Si les préconditions manquent ou le gain ne se confirme pas, maintenir le report correspondant dans AUD-202.

## Vérifications de l'utilisateur

Aucune.
