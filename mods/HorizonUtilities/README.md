# Horizon Utilities

Mod client Horizon (Rust → WebAssembly `.hmod`), API `0.1`, id `horizon_utilities`.
Analysé contre `horizon-rust/horizon` au commit `f66f66c` (`crates/horizon-mod-sdk/wit/horizon.wit`, package `horizon:mods@0.1.0`).

## Ce qui est disponible

| Module | État | Détail |
|---|---|---|
| **Nutrition HUD** | ✅ implémenté | Faim exacte (`game::get-player().food`, 0–20), barre de 10 icônes, aperçu des points de l'aliment en main, nom/icône/points, avertissement de points perdus, note d'effet. |
| Saturation | ❌ non exposée | Aucune API : **aucune valeur n'est affichée**. |
| **Inventory Manager** | ❌ bloqué par l'API | Aucune lecture/déplacement d'emplacements (`input::held` = main principale seulement ; `recipes::give` = créatif seulement). Aucun bouton factice. |
| **Armor HUD** | ❌ bloqué par l'API | `game::player` ne contient ni équipement ni durabilité. |
| **Pickup Notifier** | ❌ bloqué par l'API | Aucun événement de ramassage. Une détection par variation de la main n'est pas fiable : non implémentée. |

Limite importante du Nutrition HUD : les points d'un aliment viennent d'une **table vanilla intégrée** (`src/food.rs`), car l'API n'expose pas les valeurs nutritionnelles. Un aliment absent de la table (mod, datapack) n'a pas d'aperçu. Le texte ne peut pas être mis à l'échelle par l'API : l'échelle agit sur la barre et l'icône.

## Capacités déclarées

`game-read`, `storage`, `ui`, `render`, `input` ; `ui`, `render` et `input` sont expérimentales (`experimental = [...]`) et peuvent changer à toute version de Horizon.

## Utilisation

- **Page de réglages** : Options → Mods → Horizon Utilities (activer/désactiver chaque élément, coin, échelle, opacité, réinitialiser).
- **Raccourci** : `N` (modifiable dans la page du mod) — afficher/masquer le HUD nutrition.
- **Commande** : `/utilities [status | nutrition on|off|toggle | reset]`.
- Réglages persistants (`storage`, clé `config.v1`, format versionné ; valeur illisible → défauts).

## Structure

`config.rs` (réglages, sans SDK) · `food.rs` (table) · `nutrition.rs` (logique pure) · `hud.rs` (formes `render`) · `settings.rs` (page `ui`) · `unavailable.rs` (limites) · `lib.rs` (hooks).

## Compilation

```sh
cd mods/HorizonUtilities
cargo fmt --check && cargo test --lib
cargo horizon-mod build
cargo horizon-mod check <chemin-affiché>.hmod
```

Le workflow `.github/workflows/hmod.yml` compile, vérifie et publie l'artifact `hmod-horizon-utilities`.

## Validation

Tests unitaires de la logique (config, table, aperçu, mise en page, réglages) ; **aucune vérification en jeu** n'a été faite : rendu, raccourci, page Options, persistance et changement de monde sont à valider avec `--dev-mod`.
