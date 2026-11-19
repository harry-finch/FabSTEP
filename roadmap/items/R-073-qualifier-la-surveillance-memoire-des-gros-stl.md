---
id: R-073
titre: "Qualifier la surveillance mémoire des gros STL sur macOS"
resume: "La sonde R-072 rapporte une empreinte physique de 6.9G malgré un RSS maximal échantillonné de 2,35 Gio. Comparer les métriques macOS et qualifier une surveillance adaptée avant toute augmentation de capacité ; préserver arrêt et nettoyage du helper."
statut: livree
jalon: post-v1
cree: 2026-11-17
maj: 2026-11-19
livree: 2026-11-19
---

## But

Surveiller la charge mémoire du helper sur macOS sans perdre la protection RSS existante. La poursuite demandée par l’utilisateur autorise la qualification et une correction locale de la mesure, sans relever les plafonds ni modifier la reconstruction.

## Inclus

- Vérifier le contrat du SDK macOS et mesurer RSS et empreinte physique.
- Refuser dès que l’une de ces deux mesures atteint 4 Gio ; conserver le code stable et les chemins de nettoyage.
- Tester les seuils sans allocation massive, les erreurs de lecture et l’arrêt d’un vrai enfant.
- Mettre à jour les preuves, la documentation et le build de développement.

## Hors périmètre

Hausse de capacité, nouvelles familles géométriques, modification des fichiers, surveillance Windows/Linux ou limite mémoire matérielle stricte. L’échantillonnage ne garantit pas l’absence de dépassement entre deux lectures.

## Dépendances

R-072, V-1196 ; SDK macOS local et API libproc existante. Aucune nouvelle dépendance.

## Critères d'acceptation

- ABI vérifiée contre le SDK ; mesures natives non nulles, PID invalide refusé.
- Chacune des deux mesures déclenche worker.memory_limit à 4 Gio, même si l’autre est faible.
- Erreur de suivi ou dépassement : enfant récolté, aucun résultat publié, réservation libérée.
- Contrôles automatisés pertinents et paquet reconstruit ; limites explicitement documentées.

## Vérifications de l'utilisateur

- Vérifier que le build mis à jour permet toujours de lancer puis annuler un gros STL et conserve sa source ; aucun dépassement mémoire volontaire n’est nécessaire.

## Implémentation

`supervisor.rs` utilise `proc_pid_rusage` avec un tampon vérifié contre le SDK (taille, alignement, offsets). Le maximum du RSS et de l’empreinte physique est comparé au plafond existant de 4 Gio. Aucun contrat géométrique, format de données ou protocole helper ne change. La revue indépendante ne relève pas de défaut ; les preuves et limites sont dans V-1197 de `docs/VALIDATION.md`.

Qualification locale : lectures réelles sur soi et un enfant, PID invalide, seuils injectés sans allocation massive, arrêt et récolte avant nettoyage. La sonde de 64 Mio confirme l’augmentation d’empreinte. L’échantillonnage reste à une seconde ; il ne garantit pas un plafond matériel.
