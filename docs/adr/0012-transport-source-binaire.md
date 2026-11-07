# ADR-0012 — Transport binaire de la source

Statut : acceptée pour 1.0.0 ; implémentation issue de la candidate 1.0.0-rc.1

Date : 7 novembre 2026

## Contexte

L’aperçu et la conversion utilisaient chacun les octets complets de la source. Leur passage sous forme de tableau JSON multipliait les allocations pour des fichiers pouvant atteindre 64 Mio. Les changements rapides de fichier ou de réglage imposent aussi d’écarter les travaux obsolètes et de ne jamais exporter un ancien STEP.

## Décision

L’interface transmet une fois par identité de fichier une vue `Uint8Array` en corps binaire brut à `upload_source`. L’identité monotone et le format passent par des en-têtes bornés ; les commandes d’aperçu et de conversion ne transmettent ensuite que l’identité, la révision et les options. Le navigateur seul ne dispose pas du moteur natif.

Le bureau conserve au plus une source courante dans un `Arc<Vec<u8>>`. La limite de 64 Mio est contrôlée avant la copie applicative ; une seule copie entrante est traitée à la fois. Un travail en cours conserve son instantané partagé hors du verrou. Changer de fichier retire la source courante du cache ; changer une option ou une unité invalide le STEP tout en conservant la source. Un transfert tardif n’est accepté que pour la source attendue. Une identité réutilisée avec d’autres octets ou un autre format est refusée et invalide le résultat. L’interface abandonne les réponses obsolètes ; le bureau vérifie identité et révision au début et à la fin des travaux, puis contrôle de nouveau le résultat après le dialogue d’export.

Aucune permission générique de lecture de fichier n’est ajoutée : la source provient du fichier déjà choisi dans l’interface. L’écriture STEP reste effectuée par le dialogue et le cache de résultat existants.

## Alternatives examinées

- Envoyer deux tableaux JSON de nombres est simple mais coûteux en mémoire et en sérialisation.
- Faire lire au bureau un chemin arbitraire introduirait un accès système supplémentaire et couplerait le transport au choix du fichier.
- Conserver plusieurs sources en cache augmenterait le budget mémoire et compliquerait leur invalidation sans bénéfice pour le document courant.

## Conséquences

La copie applicative est bornée, mais Tauri et la WebView ont déjà reçu le corps avant son contrôle natif : la mémoire totale n’est pas bornée à 64 Mio. Un ancien travail peut aussi conserver son instantané jusqu’à sa fin, en plus de la nouvelle source ; les maillages, rapports et STEP occupent leur propre mémoire. La copie et la comparaison d’un doublon restent proportionnelles à la taille du fichier. Un transport non brut est refusé avec une erreur explicite ; aucune compatibilité n’est supposée pour un éventuel repli Tauri sérialisé en JSON.

Le remplacement des options conserve l’aperçu du même fichier lorsque son unité ne change pas ; une erreur de conversion laisse l’aperçu obtenu disponible sans autoriser l’export. Le transfert lui-même ne garantit ni l’interprétation ni la validité géométrique.

## Validation

Les tests UI couvrent une sous-vue binaire et le refus d’une taille excessive avant IPC. Les tests bureau couvrent les limites, l’ordre des invalidations et transferts, les changements de source ou de réglage, les résultats tardifs et les doublons conflictuels. Ces tests ne mesurent pas les allocations de la WebView. Sur la candidate, l’utilisateur confirme l’aperçu et l’export natifs d’un boîtier à 26 faces ; ce parcours est déclaré par l’utilisateur, puis le fichier exporté a été réimporté avec succès dans FreeCAD de manière indépendante. La version 1.0.0 utilise le même code que la candidate, hors métadonnées de version. Voir [VALIDATION](../VALIDATION.md) pour les résultats et essais manuels effectivement consignés.
