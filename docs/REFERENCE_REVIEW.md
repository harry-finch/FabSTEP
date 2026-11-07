# Relecture du brainstorming

Le fichier `FabSTEP-Brainstorming.md` reste la référence exploratoire d'origine. Ses extraits de code et ses sorties annoncées ne constituent pas des résultats vérifiés.

## Points retenus

Séparer import, diagnostic, reconnaissance, construction BRep et export. Commencer par la fusion de régions planes, puis les cylindres. Des méthodes 2,5D peuvent aider sur des pièces réellement extrudées, à condition de vérifier l'invariance du profil.

## Corrections qui orientent l'implémentation

| Emplacement du document d'origine | Défaut | Exigence FabSTEP |
| --- | --- | --- |
| Lignes 340–358 | Arrondi des sommets dans une seule cellule sans mesure de distance | Rechercher les cellules voisines et mesurer la distance réelle |
| Ligne 382 | Une insertion peut écraser une incidence d'arête | Conserver et compter toutes les incidences |
| Lignes 525–598 | Marche interrompue conservée comme contour | Vérifier la fermeture et refuser les branches ambiguës |
| Lignes 680–707 | Vérification de paroi sur le premier anneau seulement | Vérifier la totalité du support d'une primitive |
| Lignes 1389–1485 | Premier trou coaxial apparié, fond borgne parfois supposé | Prouver la connectivité, les limites et le fond avant reconstruction |
| Lignes 1755–1773 | Échec de construction de solide suivi d'un succès de coque | Bloquer la conversion |

Un maillage fermé n'est pas nécessairement sans auto-intersections. Une topologie manifold n'est pas une preuve de validité géométrique. Une forme exacte au sens CAO n'est pas une preuve des dimensions nominales d'origine. Le STL ne fournit pas de budget d'erreur de tessellation.

`ShapeUpgrade_UnifySameDomain` peut fusionner des domaines géométriques compatibles ; il ne devine pas le cylindre original derrière une suite de facettes planes. La couture n'est pas non plus une reconstruction automatique de surfaces infinies.

## Sources techniques consultées

- [truck : contrat de Solid](https://docs.rs/truck-topology/latest/truck_topology/struct.Solid.html) : contrôle topologique distinct d'une validation géométrique complète.
- [OCCT : Sewing](https://occt3d.com/dev/doc/refman/html/class_b_rep_builder_a_p_i___sewing.html) et [BRepCheck_Analyzer](https://occt3d.com/dev/doc/refman/html/class_b_rep_check___analyzer.html) : couture et diagnostic.
- [Bindings opencascade-rs](https://github.com/bschwind/opencascade-rs) : leur couverture ne se confond pas avec celle du moteur C++.
- [Spécification 3MF Core](https://github.com/3MFConsortium/spec_core/blob/master/3MF%20Core%20Specification.md) : unités, objets, transformations et extensions.

Audit réalisé le 6 novembre 2026 par un sous-agent géométrie, puis intégré par l'architecte.
