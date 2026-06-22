#!/usr/bin/env sh
set -eu
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
cd "$REPO_ROOT"


if ! command -v wasm-bindgen >/dev/null 2>&1; then
  cargo install wasm-bindgen-cli
fi

WASM_BINDGEN_BIN=$(command -v wasm-bindgen || true)
if [ -z "$WASM_BINDGEN_BIN" ]; then
  WASM_BINDGEN_BIN="$HOME/.cargo/bin/wasm-bindgen"
fi

rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown --release
"$WASM_BINDGEN_BIN" target/wasm32-unknown-unknown/release/chubby_bunny_playground.wasm \
  --out-dir chubby_bunny_playground/pkg \
  --target web

python3 -m http.server 8000