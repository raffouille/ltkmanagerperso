# Version perso de LTK Manager

Copie privée de [LeagueToolkit/ltk-manager](https://github.com/LeagueToolkit/ltk-manager), partie de
la v1.29.0.

## Ce que cette version ajoute

- **Settings > Built-in mods > Kept skins** : sous **Base skins**, une liste de skins que Base skins
  ne remplace pas. On ajoute un champion, puis on choisit ses skins (chromas compris). Le
  compagnon d'un champion garde le même skin (par exemple Tibbers pour Annie).
- **Settings > Built-in mods > Keep the skin I pick** : pendant que le patcher tourne, il lit
  dans le client League le champion et le skin que vous choisissez en sélection, et ne remplace
  pas ce skin. Seule l'archive de ce champion est reconstruite, en quelques secondes. Choisissez
  le skin avant les toutes dernières secondes de la sélection.
- Changer **Kept skins** pendant que le patcher tourne reconstruit le patch tout de suite, sans
  le relancer.
- **Patcher** : cette version ne peut pas télécharger elle-même le patcher corrigé de
  LeagueToolkit. Elle réutilise celui que la version officielle a téléchargé dans le même
  dossier de données, après avoir vérifié sa signature. Si les mods ne s'affichent plus après un
  patch de League, ouvrez la version officielle une fois et lancez son patcher pour qu'elle
  télécharge le nouveau, puis revenez à cette version.
- Les mises à jour automatiques sont coupées (`OFFERS_RELEASES` dans `src-tauri/src/updater.rs`),
  sinon une version officielle remplacerait celle-ci.
- Le workflow `CI` officiel est désactivé dans l'onglet Actions, pour ne pas consommer de minutes.

## Construire l'installateur

1. Onglet **Actions** > **Build installer** > **Run workflow**.
2. À la fin (environ 30 à 60 minutes la première fois), télécharger l'artefact
   `ltk-manager-installer` en bas de la page du run et lancer le `.exe` qu'il contient.
3. L'installateur n'est pas signé : Windows SmartScreen affiche un avertissement
   (**Informations complémentaires** > **Exécuter quand même**).

Sur son PC : Rust, Node.js 22 et pnpm, puis `pnpm install` et
`pnpm tauri build --config '{"bundle":{"createUpdaterArtifacts":false,"windows":{"signCommand":null}}}'`.

## Récupérer une nouvelle version officielle

```bash
git remote add upstream https://github.com/LeagueToolkit/ltk-manager   # une seule fois
git fetch upstream --tags
git merge v1.30.0   # le tag de la version voulue
git push
```

Puis reconstruire l'installateur.
