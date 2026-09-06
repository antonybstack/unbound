#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
export RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }--cfg getrandom_backend=\"wasm_js\""
cd "$root/crates/client"
# Prefer the Trunk WASM bundler binary; a different `trunk` may be on PATH.
trunk_bin="${TRUNK_BIN:-$HOME/.local/bin/trunk}"
# Trunk's clap env mapping rejects NO_COLOR=1 (the agent harness sets that).
unset NO_COLOR
exec "$trunk_bin" serve --address 127.0.0.1 --port 8080
