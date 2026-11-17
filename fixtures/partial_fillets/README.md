# Arrondis locaux et bac ouvert — R-062

Corpus indépendant en millimètres, généré par `python3 fixtures/generate_partial_fillets.py`. Les paramètres, volumes analytiques, sondes de matière/vide et empreintes sont fixés dans `manifest.json`.

- `one_edge.stl` : boîte 40 × 30 × 12 mm avec une seule arête verticale arrondie R4 ; six plans et un cylindre partiel.
- `two_edges.stl` : deux arêtes disjointes R4/R2 ; six plans et deux cylindres partiels. Variantes tournée et triangulation décalée.
- `open_tray.stl` : extérieur 40 × 30 × 12 mm, coins R4 ; intérieur 34 × 24 mm, coins R2 ; fond de 3 mm. Onze plans et huit cylindres partiels, avec plancher et rebord réels. Les jonctions du fond et du rebord restent vives. Variante tournée disponible.

Le moteur existant suffit pour ces formes en Automatique ou Cylindres ; ce lot ajoute leur qualification, pas un algorithme général de congé. Les tangences sont vérifiées pour ces cas nominaux, sans présenter tout cylindre partiel reconnu comme un congé tangent certifié.

Les sources `one_edge_missing_end.stl` et `tray_missing_floor.stl` sont ouvertes et doivent être refusées. `one_edge_variable_radius.stl` est fermée mais son rayon médian diffère : refus analytique en Cylindres, facettes en Auto. Il ne s’agit pas d’une enveloppe de congé variable qualifiée.

Les tests core/app vérifient surfaces, paramètres, provenance, volume, dimensions et sondes de matière/vide sur le STEP réimporté. Résultats et limites : V-1186 dans `docs/VALIDATION.md`.
