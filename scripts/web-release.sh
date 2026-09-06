#!/usr/bin/env bash
# Release WASM/WebGPU build. Debug wasm is ~180MB; this is the shippable target.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
export RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }--cfg getrandom_backend=\"wasm_js\""
cd "$root/crates/client"
trunk_bin="${TRUNK_BIN:-$HOME/.local/bin/trunk}"
wasm_opt="${WASM_OPT:-$HOME/.local/bin/wasm-opt}"
unset NO_COLOR
"$trunk_bin" build --release --public-url /
wasm="$root/crates/client/dist/unbound_bg.wasm"
if [ -x "$wasm_opt" ] && [ -f "$wasm" ]; then
  echo "wasm-opt -Os $wasm"
  "$wasm_opt" -Os --strip-debug -o "$wasm.opt" "$wasm"
  mv "$wasm.opt" "$wasm"
fi
ls -lh "$wasm"

