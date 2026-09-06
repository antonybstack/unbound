#!/usr/bin/env bash
# Prove slices 1–3 against a running local host. Exit non-zero if a proof fails.
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

field() {
  # field KEY < result-line
  sed -n "s/.*${1}=\\([^ ]*\\).*/\\1/p"
}

need() {
  local label="$1" got="$2" op="$3" want="$4"
  python3 - "$label" "$got" "$op" "$want" <<'PY'
import sys
label, got, op, want = sys.argv[1:5]
def as_num(s):
    try:
        return float(s)
    except ValueError:
        return None
g, w = as_num(got), as_num(want)
if op == "eq" and g is None:
    ok = got == want or got.lower() == want.lower()
elif g is None or w is None:
    print(f"FAIL {label}: not a number ({got!r})")
    sys.exit(1)
else:
    ok = {"gt": g > w, "ge": g >= w, "eq": g == w}.get(op)
if not ok:
    print(f"FAIL {label}: {got} {op} {want}")
    sys.exit(1)
print(f"ok {label}: {got} {op} {want}")
PY
}

run_bot() {
  local log res
  log="$(mktemp)"
  cargo run -q -p unbound-bot -- "$@" | tee "$log" >&2
  res=$(grep '^RESULT ' "$log" | tail -1)
  rm -f "$log"
  printf '%s\n' "$res"
}

echo "== dummy farm =="
dummy=$(run_bot --name DummyFarmer --token /tmp/unbound-dummy.token --mode dummy --seconds 10)
echo "$dummy"
need "dummy saw_self" "$(echo "$dummy" | field saw_self)" eq true
need "dummy melee_xp" "$(echo "$dummy" | field melee_xp)" gt 0

echo
echo "== persistence reconnect =="
before=$(echo "$dummy" | field melee_xp)
echo "melee_xp after farm: ${before}"
again=$(run_bot --name DummyFarmer --token /tmp/unbound-dummy.token --mode dummy --seconds 3)
echo "$again"
need "persist melee_xp" "$(echo "$again" | field melee_xp)" ge "$before"

echo
echo "== gather =="
gather=$(run_bot --name Gatherer --token /tmp/unbound-gather.token --mode gather --seconds 8)
echo "$gather"
need "gather_xp" "$(echo "$gather" | field gather_xp)" gt 0

echo
echo "== bow =="
bow=$(run_bot --name Archer --token /tmp/unbound-bow.token --mode bow --seconds 8)
echo "$bow"
need "ranged_xp" "$(echo "$bow" | field ranged_xp)" gt 0

echo
echo "== 1v1 PvP =="
pvp_a=$(mktemp)
pvp_b=$(mktemp)
cargo run -q -p unbound-bot -- --name Alpha --token /tmp/unbound-alpha.token --mode pvp --seconds 14 | tee "$pvp_a" &
pid_a=$!
sleep 0.6
cargo run -q -p unbound-bot -- --name Bravo --token /tmp/unbound-bravo.token --mode pvp --seconds 14 | tee "$pvp_b" &
pid_b=$!
wait "$pid_a" "$pid_b"
res_a=$(grep '^RESULT ' "$pvp_a" | tail -1)
res_b=$(grep '^RESULT ' "$pvp_b" | tail -1)
echo "$res_a"
echo "$res_b"
rm -f "$pvp_a" "$pvp_b"
need "alpha saw_self" "$(echo "$res_a" | field saw_self)" eq true
need "bravo saw_self" "$(echo "$res_b" | field saw_self)" eq true
hp_a=$(echo "$res_a" | field min_hp)
hp_b=$(echo "$res_b" | field min_hp)
python3 - "$hp_a" "$hp_b" <<'PY'
import sys
a, b = float(sys.argv[1]), float(sys.argv[2])
if min(a, b) >= 99:
    print(f"FAIL pvp: neither fighter dropped HP (min {a}, {b})")
    sys.exit(1)
print(f"ok pvp damage: min_hp {a} / {b}")
PY

echo
echo "== world snapshot =="
echo "-- players --"
sql "SELECT name, x, z, hp, stamina, loadout, alive FROM player"
echo "-- dummy --"
sql "SELECT id, x, z, hp, alive, action FROM dummy"
echo "-- characters --"
sql "SELECT name, melee_xp, ranged_xp, magic_xp, defence_xp, hitpoints_xp, gather_xp, loadout FROM character"
echo "-- nodes --"
sql "SELECT id, kind, x, z, charges, cooldown FROM gather_node"
echo
echo "mvp_check: all proofs passed"
