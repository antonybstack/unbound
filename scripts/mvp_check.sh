#!/usr/bin/env bash
# Prove slices 2 and 3 against a running local host: 1v1 PvP, dummy XP, persistence, gather.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
eval "$(mise activate bash)" 2>/dev/null || true
# shellcheck disable=SC1090
. "$HOME/.cargo/env" 2>/dev/null || true
export PATH="$HOME/.local/share/mise/shims:$HOME/.cargo/bin:$PATH"

cd "$root"
sql() {
  spacetime sql unbound -s local "$1" 2>/dev/null | tail -n +1
}

echo "== dummy farm =="
cargo run -q -p unbound-bot -- --name DummyFarmer --token /tmp/unbound-dummy.token --mode dummy --seconds 10
echo
echo "== persistence reconnect =="
before=$(sql "SELECT melee_xp FROM character" | awk 'NR>2 && $1 ~ /[0-9]/ {print $1; exit}')
echo "melee_xp after farm (any character row sample): ${before:-unknown}"
cargo run -q -p unbound-bot -- --name DummyFarmer --token /tmp/unbound-dummy.token --mode dummy --seconds 3
echo
echo "== gather =="
cargo run -q -p unbound-bot -- --name Gatherer --token /tmp/unbound-gather.token --mode gather --seconds 8
echo
echo "== 1v1 PvP =="
cargo run -q -p unbound-bot -- --name Alpha --token /tmp/unbound-alpha.token --mode pvp --seconds 14 &
pid_a=$!
sleep 0.6
cargo run -q -p unbound-bot -- --name Bravo --token /tmp/unbound-bravo.token --mode pvp --seconds 14 &
pid_b=$!
wait "$pid_a" "$pid_b"
echo
echo "== world snapshot =="
echo "-- players --"
sql "SELECT name, x, z, hp, stamina, loadout, alive FROM player"
echo "-- dummy --"
sql "SELECT id, x, z, hp, alive, action FROM dummy"
echo "-- characters --"
sql "SELECT name, melee_xp, ranged_xp, magic_xp, defence_xp, hitpoints_xp, gather_xp FROM character"
echo "-- nodes --"
sql "SELECT id, kind, x, z, charges, cooldown FROM gather_node"
