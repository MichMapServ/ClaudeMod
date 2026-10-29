#!/usr/bin/env bash
# Requires Rust, wasm32-unknown-unknown and cargo-horizon-mod from Horizon.
set -euo pipefail
repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
mod="$repo/mods/HorizonUtilities"
command -v cargo >/dev/null || { echo 'Rust/Cargo is required' >&2; exit 1; }
cd "$mod"
cargo fmt --all
cargo test --lib
cargo horizon-mod build
for file in "$repo/target/horizon-mod/horizon_utilities.hmod" "$mod/target/horizon-mod/horizon_utilities.hmod"; do
  if [[ -s "$file" ]]; then
    cargo horizon-mod check "$file"
    mkdir -p "$repo/dist"
    cp -- "$file" "$repo/dist/HorizonUtilities.hmod"
    (cd "$repo/dist" && sha256sum HorizonUtilities.hmod > HorizonUtilities.hmod.sha256)
    echo "Validated: $repo/dist/HorizonUtilities.hmod"
    exit 0
  fi
done
echo 'No HorizonUtilities.hmod produced' >&2
exit 1
