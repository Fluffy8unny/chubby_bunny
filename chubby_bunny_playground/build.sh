#!/usr/bin/env sh
set -eu
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
cd "$REPO_ROOT"

ENABLE_PROFILING=0
while [ "$#" -gt 0 ]; do
  case "$1" in
    --enable_profiling)
      ENABLE_PROFILING=1
      ;;
    -h|--help)
      printf '%s\n' "Usage: $0 [--enable_profiling]"
      exit 0
      ;;
    *)
      printf '%s\n' "Unknown argument: $1" >&2
      exit 1
      ;;
  esac
  shift
done


if ! command -v wasm-bindgen >/dev/null 2>&1; then
  cargo install wasm-bindgen-cli
fi

WASM_BINDGEN_BIN=$(command -v wasm-bindgen || true)
if [ -z "$WASM_BINDGEN_BIN" ]; then
  WASM_BINDGEN_BIN="$HOME/.cargo/bin/wasm-bindgen"
fi

rustup target add wasm32-unknown-unknown
if [ "$ENABLE_PROFILING" -eq 1 ]; then
  cargo build -p chubby_bunny_playground --features profiling --target wasm32-unknown-unknown --release
else
  cargo build -p chubby_bunny_playground --target wasm32-unknown-unknown --release
fi
"$WASM_BINDGEN_BIN" target/wasm32-unknown-unknown/release/chubby_bunny_playground.wasm \
  --out-dir chubby_bunny_playground/pkg \
  --target web

python3 -m http.server 8000