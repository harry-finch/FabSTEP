# Paquet macOS local

La version locale **1.0.0** inclut les quatre pièces du [corpus mécanique indépendant](../fixtures/complex_v1/README.md) et leurs régressions. L’utilisateur a confirmé l’aperçu et l’export du boîtier à 26 faces sur RC1 ; la finale conserve exactement le même code fonctionnel. Les essais manuels et automatisés sont détaillés dans [VALIDATION](VALIDATION.md).

## Construire et vérifier

Cible actuellement qualifiée : **Apple Silicon, macOS 26.0 ou ultérieur**. Le minimum est vérifié sur l’exécutable et chaque bibliothèque, pas seulement déclaré dans le plist. La fermeture Homebrew de ce poste impose cette cible ; Intel et les macOS antérieurs nécessitent une chaîne de compilation et une qualification distinctes.

Prérequis de construction : Rust 1.88 avec sa cible `aarch64-apple-darwin`, Node 26, npm, Python 3.11 ou ultérieur, outils de développement Apple et OCCT Homebrew 7.9.3. Le verrou npm est autonome et utilise les archives du registre ; aucun lien vers le dossier FabWeave n’est nécessaire.

```sh
npm --prefix ui ci
sh scripts/check.sh
sh scripts/build-macos-release.sh
```

Le script fixe `MACOSX_DEPLOYMENT_TARGET=26.0` pour Rust et C++, compile avec les verrous et sans téléchargement Cargo, puis produit un nouveau dossier `artifacts/release/<version>/`. Il refuse de remplacer un dossier de livraison existant. Pour une nouvelle qualification du même numéro, passer un nouveau chemin au script.

Le dossier contient l’application, le ZIP, son empreinte SHA-256 et le rapport de validation. Le bundle intègre ses bibliothèques dans `Contents/Frameworks`, leurs liens `@loader_path`, un inventaire et les notices sous `Contents/Resources/Licenses`. Les recherches de bibliothèques héritées du poste de construction sont supprimées. Les textes supplémentaires de licences sont épinglés avec leur provenance sous `packaging/macos/`.

```sh
python3 scripts/package-macos.py verify \
  artifacts/release/1.0.0/FabSTEP-1.0.0-macOS-arm64.zip \
  --smoke-cli artifacts/macos-build/aarch64-apple-darwin/release/fabstep \
  --require-no-homebrew
```

Le vérificateur contrôle le checksum, les chemins de l’archive, la signature, chaque lien Mach-O et la conversion de quatre documents après extraction dans un chemin contenant des espaces. Le CLI de contrôle est relocalisé avec les mêmes bibliothèques ; il ne fait pas partie du ZIP utilisateur. L’option stricte exige que les lectures de Homebrew soient effectivement bloquées pendant les conversions. Elle échoue si cette isolation n’est pas disponible. Elle ne remplace pas les essais d’interface du vrai bundle ni un essai sur un autre Mac.

## Qualification 1.0.0

Le ZIP local de 18 Mio embarque 29 bibliothèques et un bundle d’environ 49 Mio. Il inclut `TKFillet` et `TKBool`, nécessaires aux congés du boîtier. Deux contrôles stricts ont réussi : une sphère STL, deux instances 3MF, un tore entier et un boîtier tourné. Les faces, volumes et boîtes attendus sont vérifiés ; les cinq solides sont fermés et valides après réimport. Les chargements constatés restent dans le bundle et les bibliothèques système Apple. Le contrôle indépendant confirme également l’empreinte et la signature. [Rapport strict final](../artifacts/release/1.0.0/verification-stricte.json)

Le ZIP final contient 19 196 918 octets et a pour SHA-256 `9bac6a8dc8ae02e5d91487b898a164cb22f43a2fe51dc5204851a3af190c88be`. Le validateur avait également refusé une empreinte modifiée et un chemin sortant du bundle pendant la qualification 0.10 ; ces essais négatifs n’ont pas été réexécutés sans changement de ces contrôles.

La signature est **ad hoc**, sans Developer ID ni notarisation. Aucun paquet n’a été publié. La notarisation est une étape distincte de distribution Apple ; voir la [documentation Apple](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution).

## Sources et automatisation

L’archive locale `artifacts/release/1.0.0/FabSTEP-1.0.0-sources.tar.gz` accompagne le paquet ; son manifeste permet de comparer chaque fichier au commit de livraison marqué `v1.0.0`.

`scripts/package-source.py <nouveau-fichier.tar.gz>` produit une archive déterministe des sources FabSTEP et un manifeste SHA-256. Les entrées utilisateur sous `artifacts/`, les caches, les builds et `node_modules` sont exclus. Les fixtures doivent rester synthétiques ; vérifier le manifeste avant toute diffusion. L’archive contient les verrous mais n’embarque pas les sources des dépendances. Elle ne constitue donc pas à elle seule un kit complet de distribution binaire avec ses sources correspondantes ; voir la [licence GPLv3](https://www.gnu.org/licenses/gpl.en.html#section6).

La définition [GitHub Actions](../.github/workflows/check.yml) exécute les contrôles sur le runner `macos-26`, avec droits de lecture et sans publication. Elle n’a pas été exécutée à distance : le dépôt local n’a actuellement aucun distant. La liste des runners provient de la [documentation GitHub](https://docs.github.com/en/actions/how-tos/write-workflows/choose-where-workflows-run/choose-the-runner-for-a-job). Les contrôles réellement exécutés restent ceux de [VALIDATION](VALIDATION.md).
