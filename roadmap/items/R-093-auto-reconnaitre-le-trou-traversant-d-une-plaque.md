---
id: R-093
titre: "Auto — reconnaître le trou traversant d’une plaque inclinée"
resume: "Sur la plaque inclinée à un perçage de V-1220, Cylindres donne sept faces, dont le cylindre du trou. Auto choisit encore 252 faces planes."
statut: idee
jalon: post-v1
cree: 2026-11-18
maj: 2026-11-18
livree:
---
## Cas reproduit

Sur une plaque de 24 × 20 × 6 mm avec un trou traversant de Ø6 mm, le mode Cylindres produit six plans et un cylindre. Auto sauvegarde un solide à 252 faces planes. Les deux STEP se relisent dans FreeCAD. Aux tolérances par défaut le volume en mode Cylindres est de 2 710,354265 mm³, pour 2 710,353997 mm³ attendus.

## Reproduction

Dans le repère local du script `scripts/validate-cross-drill-ratios-freecad.py`, centrer la plaque sur l’origine et soustraire un cylindre de rayon 3 mm suivant Z. Appliquer la pose de `f_ratio_mid` du manifeste, écrire et relire le STEP, puis mailler comme le témoin grossier de V-1220 (déflexions 0,025 mm et 0,20 rad). Comparer les conversions avec `--unit mm --mode auto` et `--unit mm --mode cylinders`.

## À vérifier

Identifier pourquoi Auto retient le résultat facettisé. Pour retenir le cylindre, contrôler sa fidélité et le volume du solide entier. Vérifier les négatifs historiques de reconnaissance circulaire et les attributions source avant de modifier le choix automatique. Les perçages qui se croisent sont suivis dans R-036.
