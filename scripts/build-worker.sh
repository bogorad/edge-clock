#!/usr/bin/env bash

trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

project_root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd) || exit "$?"
cd -- "$project_root" || exit "$?"

for tool in cargo rustc emcc wasm-bindgen; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    printf 'Missing %s. Enter the pinned project dev shell.\n' "$tool" >&2
    exit 1
  fi
done

emcc_version=$(emcc --version) || exit "$?"
emcc_version=${emcc_version%%$'\n'*}
if [[ ! "$emcc_version" =~ \ 6\.0\.10(-git)?($|[[:space:]]) ]]; then
  printf 'Emscripten 6.0.10 is required. Enter the pinned project dev shell.\n' >&2
  exit 1
fi
bindgen_version=$(wasm-bindgen --version) || exit "$?"
if [[ "$bindgen_version" != 'wasm-bindgen 0.2.129' ]]; then
  printf 'wasm-bindgen 0.2.129 is required. Enter the pinned project dev shell.\n' >&2
  exit 1
fi

export EM_CACHE="$project_root/.cache/emscripten"
export CARGO_TARGET_WASM32_UNKNOWN_EMSCRIPTEN_LINKER=emcc
# Keep inherited shell flags from replacing the declared target settings.
unset RUSTFLAGS CARGO_ENCODED_RUSTFLAGS
export CARGO_TARGET_WASM32_UNKNOWN_EMSCRIPTEN_RUSTFLAGS='-C panic=abort -C llvm-args=-enable-emscripten-cxx-exceptions=0 -C relocation-model=static -C link-arg=-sWASM_BINDGEN -C link-arg=-sMODULARIZE=instance -C link-arg=-sEXPORT_ES6 -C link-arg=-sAUTO_INIT -C link-arg=-sSOURCE_PHASE_IMPORTS -C link-arg=-sENVIRONMENT=node -C link-arg=-sASSERTIONS=0 -C link-arg=-sALLOW_MEMORY_GROWTH=1'

cargo build --locked --manifest-path rust/clock-wasm/Cargo.toml \
  --target-dir rust/clock-wasm/target \
  --target wasm32-unknown-emscripten --bin clock_wasm --release || exit "$?"

build_dir=rust/clock-wasm/target/wasm32-unknown-emscripten/release
output_dir=rust/clock-wasm/pkg
if [[ ! -s "$build_dir/clock_wasm.js" || ! -s "$build_dir/clock_wasm.wasm" ]]; then
  printf 'Emscripten did not produce the Worker JavaScript and WASM pair.\n' >&2
  exit 1
fi
for symbol in typed_handle_http typed_render_sse; do
  if ! rg -U -q "export[[:space:]]+(function[[:space:]]+$symbol\\b|\\{[^}]*\\b$symbol\\b)" "$build_dir/clock_wasm.js"; then
    printf 'Generated Worker module is missing %s.\n' "$symbol" >&2
    exit 1
  fi
done

mkdir -p -- "$output_dir" || exit "$?"
cp -- "$build_dir/clock_wasm.js" "$build_dir/clock_wasm.wasm" "$output_dir/" || exit "$?"
for dependency_dir in snippets deps/snippets; do
  if [[ -d "$build_dir/$dependency_dir" ]]; then
    mkdir -p -- "$output_dir/$dependency_dir" || exit "$?"
    cp -R -- "$build_dir/$dependency_dir/." "$output_dir/$dependency_dir/" || exit "$?"
  fi
done
printf 'Built Emscripten Worker module: %s/clock_wasm.js\n' "$output_dir"
