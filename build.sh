if ! command -v wasm-bindgen >/dev/null 2>&1; then
  cargo install wasm-bindgen-cli
fi
rustup target add wasm32-unknown-unknown
cargo test
cargo build --target wasm32-unknown-unknown --release
~/.cargo/bin/wasm-bindgen target/wasm32-unknown-unknown/release/chubby_bunny_playground.wasm \
  --out-dir pkg \
  --target web
python3 -m http.server 8000