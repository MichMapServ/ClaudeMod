# ClaudeMod — collection de mods Horizon

Dépôt multi-mods pour [Horizon](https://github.com/horizon-rust/horizon). Chaque dossier `mods/<nom>/` est un mod autonome, publié en fichier `.hmod`. Les mods Rust partagent un workspace Cargo et une dépendance au SDK ; les mods de contenu n'ont pas de crate Rust.

## Structure

```text
ClaudeMod/
├── Cargo.toml
├── rust-toolchain.toml
├── scripts/
│   ├── new_mod.py
│   └── tests/test_new_mod.py
├── mods/
│   └── <nom-du-mod>/
│       ├── horizon-mod.toml
│       ├── Cargo.toml       # seulement pour les mods Rust
│       ├── src/             # pour les mods Rust
│       ├── tests/           # tests optionnels
│       ├── assets/          # ressources optionnelles
│       └── data/            # données optionnelles
└── .github/workflows/hmod.yml
```

Le dossier `mods/` est volontairement vide à l'initialisation. On peut y ajouter autant de mods que souhaité.

## Préparation de la machine

Installer Rust ([rustup](https://rustup.rs/)), Python 3.10+ et l'outil officiel Horizon :

```sh
rustup target add wasm32-unknown-unknown
cargo install --locked --git https://github.com/horizon-rust/horizon cargo-horizon-mod
```

## Créer des mods

Depuis la racine du dépôt :

```sh
# Mod Rust : commande /hello et test fournis par le modèle officiel
python scripts/new_mod.py mon-premier-mod --name "Mon premier mod"

# Mod purement contenu : blocs, recettes, modèles, assets
python scripts/new_mod.py nouveaux-blocs --name "Nouveaux blocs" --template content
```

Le générateur appelle `cargo horizon-mod new` et modifie le manifeste Cargo du nouveau mod Rust pour partager les paramètres de la racine. Il ajoute **seulement** les mods Rust à `[workspace].members`, afin que les mods de contenu sans `Cargo.toml` restent compatibles.

## Construire, vérifier et tester

```sh
cd mods/mon-premier-mod
cargo horizon-mod build
cargo horizon-mod test
cargo horizon-mod check ../../target/horizon-mod/mon_premier_mod.hmod
```

Pour un mod de contenu, lancer `cargo horizon-mod build` depuis son dossier. Les `.hmod` Rust sont généralement générés sous `target/horizon-mod/` à la racine, et ceux de contenu sous `mods/<nom>/target/horizon-mod/`. Toujours vérifier le chemin réellement affiché par la commande.

Pour charger un mod dans le client Horizon en développement, voir la [documentation officielle](https://playhorizon.dev/docs/make-a-mod).

## Intégration continue GitHub Actions

[Workflow hmod](.github/workflows/hmod.yml) : détection automatique des `mods/*/horizon-mod.toml`, compilation individuelle, vérification `.hmod`, tests quand `tests/*.toml` existent, et mise à disposition des artifacts `hmod-<nom>`. Une tâche vérifie le générateur même lorsque la collection ne contient aucun mod.

Déclenchement sur les changements de mods, outils ou configuration ; exécution manuelle dans **Actions → hmod → Run workflow** avec `mod=all` ou un nom de dossier. Le contrôle `result` peut être déclaré obligatoire pour la branche principale.

## Versions du SDK

Le SDK central est défini dans `Cargo.toml`. Le compilateur CLI utilise `HORIZON_GIT` et `HORIZON_REF` dans les variables du dépôt GitHub (défaut : `horizon-rust/horizon`, branche `main`). **Aligner la version du SDK et celle du CLI** pour éviter les incompatibilités, en particulier avec les forks.

Ne pas committer les ressources propriétaires du jeu, les dossiers `target/` ou les fichiers `.hmod` générés.

Documentation : [Horizon](https://playhorizon.dev/docs/make-a-mod) · [Cargo workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html).
