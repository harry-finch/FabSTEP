---
id: R-120
titre: "Proposer une correction guidée sur les parois similaires"
resume: "Après une correction guidée acceptée, rechercher en lecture seule les parois géométriquement analogues du document, les prévisualiser puis appliquer celles qui passent les preuves strictes après accord explicite."
statut: livree
jalon: atelier
cree: 2026-11-19
maj: 2026-11-20
livree: 2026-11-20
---


## But

Sur une pièce comportant des formes répétées (ex. 8 perçages identiques ou plusieurs congés de même rayon sur une bride), corriger une paroi par guidage puis répéter le même geste sur chaque occurrence est fastidieux. Proposer une recherche explicite des **parois similaires** après une correction guidée réussie permet de repérer et d’appliquer en un geste les corrections analogues, tout en conservant les preuves strictes et la validation complète du document.

## Inclus

- Action contextuelle **« Chercher des parois similaires »** disponible uniquement après une correction guidée acceptée (familles guidées prises en charge : `Cylindre`, `Cône`, `Tore`, `Sphère`, `Plan`), hors mode gros STL.
- Phase 1 en **lecture seule** hors fil UI (`find_similar_guided_regions` ou équivalent) : à partir de la dernière indication acceptée (famille, rayon/demi-angle/type de raccord mesuré), le moteur recherche sur le document courant les autres régions non encore attribuées à cette famille dont les paramètres géométriques concordent dans la tolérance active, sans modifier le STEP ni invalider l’export courant.
- Présentation claire des candidats trouvés dans l’inspecteur (nombre de parois trouvées, prévisualisation surlignée dans la vue 3D, paramètres mesurés) avec choix explicite de l’utilisateur (**Appliquer** ou **Annuler**), et possibilité d’exclure des candidats avant application.
- Phase 2 d’application : chaque paroi retenue entre dans la session guidée existante et passe les **mêmes preuves géométriques strictes** et la **revalidation complète du solide STEP** (réimport, volume, dimensions, fidélité). En cas de refus, aucune tolérance n’est relâchée et la session acceptée précédente reste intacte sans republier d’export périmé.
- Respect des limites existantes : budget de travail borné, plafond de 16 indications guidées par document (si davantage de candidats existent, la proposition est bornée à la capacité restante et le signale explicitement), gardes de révision/source/époque monotones et traductions FR/EN complètes.

## Hors périmètre

- Application automatique et silencieuse des corrections similaires sans validation explicite de l’utilisateur.
- Relèvement silencieux du plafond de 16 indications ou assouplissement des tolérances radiales/planaires.
- Propagation sur des familles ou groupes couplés déjà refusés en guidage (boîtier arrondi complet, coin triédrique couplé, perçages sécants, révolution cubique).
- Prise en charge sur le mode gros STL isolé (`large-stl`, ADR-0034).

## Dépendances

- S’appuie sur le parcours de guidage multi-indications et de provenance exacte existant (`R-047`, `R-048`, `R-050`, `R-056`, `R-057`) ; compatible avec l’édition future des indications (`R-049`).

## Critères d'acceptation

- Sur un témoin comportant au moins 4 trous cylindriques identiques (ou 4 congés identiques) initialement facettisés, guider un premier trou puis lancer « Chercher des parois similaires » propose les 3 autres en lecture seule sans invalider l’export, puis **Appliquer** reconstruit les 4 parois en un seul passage validé après réimport STEP.
- Un perçage de rayon différent (au-delà de la tolérance active) ou une paroi déformée n’est pas proposé, ou est refusé avec un code stable si forcé ; aucun export partiel n’est publié.
- Un changement de source, d’options ou de révision pendant la recherche ou avant l’application annule/rejette toute réponse obsolète sans altérer la session courante.
- Si plus de candidats que la capacité restante (plafond 16) sont trouvés, le nombre retenu et la limite sont signalés explicitement sans erreur silencieuse.
- `sh scripts/check.sh` passe sans erreur ni avertissement.

## Vérifications de l'utilisateur

- Sur une pièce à perçages ou congés répétés dans FabSTEP, guider une première paroi, lancer « Chercher des parois similaires », vérifier la prévisualisation des candidats puis confirmer l’application et contrôler le STEP exporté dans Fusion.
- Vérifier que l’annulation ou un changement de réglage pendant la recherche laisse l’interface cohérente et ne republie pas un ancien STEP.
