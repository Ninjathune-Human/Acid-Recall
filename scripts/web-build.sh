#!/usr/bin/env bash
# Builds the browser version of ACID RECALL (docs/web.md) into OUT_DIR (default target/web):
# web/index.html, web/manifest.webmanifest, icon.png (the brand PNG) and pkg/ (the wasm module
# and its JavaScript glue, from wasm-bindgen).
#
#   scripts/web-build.sh [OUT_DIR]
#
# Needs the wasm32-unknown-unknown target (rustup target add wasm32-unknown-unknown) and
# wasm-bindgen-cli at the version of wasm-bindgen in Cargo.lock
# (cargo install wasm-bindgen-cli --version VERSION --locked). Serve OUT_DIR over HTTP: a page
# opened from a file cannot load its wasm module.
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
out=${1:-$root/target/web}

cargo build --manifest-path "$root/Cargo.toml" --release --locked \
    --target wasm32-unknown-unknown -p orb-ui --bin OpenRB-338
wasm=${CARGO_TARGET_DIR:-$root/target}/wasm32-unknown-unknown/release/OpenRB-338.wasm

rm -rf "$out"
mkdir -p "$out"
wasm-bindgen --target web --no-typescript --out-name acid_recall --out-dir "$out/pkg" "$wasm"
cp "$root/web/index.html" "$root/web/manifest.webmanifest" "$out/"
cp "$root/assets/brand/acid-recall.png" "$out/icon.png"
du -sh "$out/pkg"/*
