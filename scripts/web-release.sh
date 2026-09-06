#!/usr/bin/env bash
# Release WASM/WebGPU build. Debug wasm is ~180MB; this is the shippable target.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
export RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }--cfg getrandom_backend=\"wasm_js\""
cd "$root/crates/client"
trunk_bin="${TRUNK_BIN:-$HOME/.local/bin/trunk}"
unset NO_COLOR
exec "$trunk_bin" build --release --public-url /
