# Handoff — continue Unbound on the Mac Studio

**Stopped:** 2026-09-06 ~22:25 UTC (Linux Omarchy box).  
**Repo:** https://github.com/antonybstack/unbound (public)  
**Branch:** `main`  
**HEAD at handoff:** `78ddee8` `Flash the shield white when a block chips.`  
(Plus this commit: `handoff.md` and Linux-only Wayland on the native client.)

The 24-hour autonomous loop is **cancelled**. Do not restart it unless you want another long unattended push. Working tree was clean before this file.

MVP slices 0–3 are **Playable**. Remaining work is combat/feel polish and later art (glTF), not missing systems. Vision: [docs/VISION.md](docs/VISION.md).

---

## What this is

Browser-first persistent action RPG: **Rust everywhere**, **Bevy 0.19** client (native + WASM/WebGPU via Trunk), **SpacetimeDB 2.10** module, **shared** crate for sim math. `bevy_stdb` 0.13.

Constraints that still apply:

- Browser-first, few moving parts, full E2E on one machine, performance first.
- Client sends **intents** (`set_input`), never positions. Server is authority; client predicts WASD and swing start.
- One native window per identity. Two natives sharing one token is **not** 1v1.
- Do **not** `pkill -f unbound` (matches agent argv and random shells). Kill only the client binary, e.g. `pkill -f 'target/.*/unbound$'`.
- `spacetime generate --lang rust` owns `crates/client/src/module_bindings` and `crates/bot/src/module_bindings`. Do not hand-edit those.
- Bevy 0.19 `Update` system tuples overflow around ~21 items. The client is split into **three chained groups**. Stay at ≤15–20 per group.

Combat protocol (already shipped): `set_input(dir_x, dir_z, yaw, drawn, buttons, loadout)` plus pitch for bows/staffs. Buttons: LIGHT / HEAVY / DODGE / BLOCK / SPRINT / INTERACT. Loadouts: `0` sword, `1` bow, `2` staff.

---

## Clone and tools (Mac Studio)

```bash
git clone https://github.com/antonybstack/unbound.git
cd unbound
git checkout main
git pull
```

You need:

| Tool | Notes |
|---|---|
| Rust **1.95** + `wasm32-unknown-unknown` | `rustup target add wasm32-unknown-unknown` |
| SpacetimeDB **2.10** CLI | `curl -sSf https://install.spacetimedb.com \| sh -s -- -y` then ensure `spacetime` is on PATH |
| Trunk | `cargo install trunk` or a binary at `~/.local/bin/trunk` / `~/bin/trunk` |
| wasm-opt (optional, release web) | binaryen; `scripts/web-release.sh` looks for it |
| Chrome (optional) | `./scripts/web_smoke.sh` |

Linux native enabled Bevy `wayland`. That feature is now **Linux-only** so macOS uses Metal defaults. If native still fails on Mac, check `crates/client/Cargo.toml`.

---

## Run locally (one machine)

### 1. Spacetime host

```bash
spacetime start --non-interactive
```

Listens on `http://127.0.0.1:3000`. HTTP 404 on `/` is normal.

### 2. Publish the module (first time, or after `crates/module` changes)

```bash
cd unbound
spacetime publish unbound -p crates/module -s local -y
spacetime generate --lang rust -o crates/client/src/module_bindings -p crates/module
spacetime generate --lang rust -o crates/bot/src/module_bindings -p crates/module
```

Always pass **`-s local`**. The CLI default server may be `maincloud`.

### 3. Native client (one window)

```bash
cargo run -p unbound-client
```

Binary name is `unbound` (`crates/client`). Identity file (this crate does **not** use macOS Application Support):

```
~/.local/share/unbound/identity.token
```

On Mac that is still `~/.local/share/...` unless you set `XDG_DATA_HOME`. Browser uses `localStorage` key `unbound.identity.token`.

Keep **one** native window per token. For 1v1 use a second identity (`unbound-bot` with `--token /tmp/unbound-alpha.token`) or a browser tab.

### 4. Browser (WebGPU)

```bash
# spacetime already running
./scripts/web.sh            # http://127.0.0.1:8080  debug wasm, ~180MB
# or
./scripts/web-release.sh    # ~52MB after wasm-opt / ~14MB gzip
```

`scripts/web.sh` expects Trunk at `$TRUNK_BIN` or `$HOME/.local/bin/trunk`. Point `TRUNK_BIN` at your Trunk if it lives elsewhere.

```bash
export TRUNK_BIN="$(which trunk)"
./scripts/web.sh
```

Unset `NO_COLOR` if Trunk’s clap rejects it.

### 5. Proofs

```bash
./scripts/mvp_check.sh
# fails the process unless dummy XP, persist reconnect, gather XP, bow XP,
# 1v1 damage, AND a bow bag surviving logout all happen

cargo test -p unbound-shared   # 78 tests at 78ddee8; more if you added slices
cargo run -p unbound-bot -- --name Alpha --token /tmp/unbound-alpha.token --mode pvp --seconds 12
```

Bot modes include dummy / pvp / gather / bow. RESULT includes `loadout`.

---

## Controls

| Input | Action |
|---|---|
| WASD | Move (camera-relative) |
| Hold RMB | Look (sheathed) |
| F | Draw / sheathe |
| LMB / RMB | Light / heavy (drawn) |
| Space | Dodge |
| Shift | Sprint |
| E | Gather (sheathed, in range) |
| 1 / 2 / 3 | Sword / bow / staff |
| Tab | Lock nearest dummy or wanderer |
| Q or MMB | Block (sword) |
| Esc | Free cursor |
| H or F1 | Hold for controls, tap to pin |

---

## Layout

```
docs/VISION.md     pillars, stack, slices 0–3
handoff.md         this file
crates/shared      movement / combat math (no Bevy, no SpacetimeDB)
crates/module      SpacetimeDB WASM (tables + reducers + 30 Hz world_tick)
crates/client      Bevy 0.19 native + WASM
crates/bot         headless SDK proofs
scripts/mvp_check.sh
scripts/web.sh
scripts/web-release.sh
scripts/web_smoke.sh
```

Module tick is 33 ms / 30 Hz (`WorldTickTimer` — do not name the table `world_tick`). Shared combat: MOVE 8, SPRINT 11, DODGE 16, MAX_HP/STAMINA 100. `skill_level = 1 + sqrt(xp/80)` cap 50.

Client SFX: `crates/client/assets/sfx/*.wav`. AssetPlugin file_path is `CARGO_MANIFEST_DIR/assets` native and `"assets"` wasm (Trunk `copy-dir`).

---

## Already on `main` (do not redo)

Slices 0–3 playable, plus a long feel pass:

- Predicted WASD, sheathe/draw camera, Tab lock (yaw+pitch at chest), camera push-out (`CamBlock`)
- Walk / sprint / remote / dummy footfalls
- H/F1 control card (`Pickable::IGNORE`)
- Dummy telegraph whoosh, capsule puff on heavy windup, armor flash, heavy rumble, light+heavy slam dust, death-drop dust
- Draw/sheathe by loadout (steel / string / wood); remote draw and remote swing whoosh
- Shot whoosh + crosshair kick; block air-sweep + shield chip flash
- Sprint camera +0.55 m; dodge camera +0.45 m; spawn-grace camera ease + local spawn dust
- Lock click, acquire pulse, drop shrink
- Node deplete crack + flatten; respawn sparkle + 0.32 s mesh restore
- Death thud, respawn sting, skill-up ding + gold XP-bar flash
- Nameplate fade on death; death veil ease
- Hotbar 1/2/3 flash; gather-hint pulse; player / dummy / remote HP bar flash
- Local + remote dodge dust; local + remote melee lunge dust
- Persist `character.loadout`; `mvp_check.sh` requires bow bag surviving logout
- Chrome/WebGPU smoke script

`ACTION_SPAWN` walk-out grace ~1.25 s: invulnerable, ghosted, can walk, cannot swing.

Persistent names seen on the Linux host (local DB, not in git): DummyFarmer, Gatherer, Wanderer-4888, Archer (`loadout=1`), Alpha, Bravo, etc.

---

## Good next holes (not on `main`)

Skip anything already in `git log`. Highest leverage left:

1. **glTF / a readable human with a sword** — the real art bottleneck. C# stays off unless this is the reason.
2. **Guard-break extra shield pop** (EVT kind 6) besides chip flash — a slice was started and **cancelled**; not on `main`.
3. Dummy **light** telegraph vs heavy (heavy already puffs more).
4. Mac-native playtest (Metal, `~/.local/share/unbound/identity.token`).
5. Release WASM size / `wasm-opt` on Mac if binaryen is missing.

Do **not** add systems the vision called out as not-v1 (auction house, quest hubs, raids, OAuth, world streaming, navmesh armies, deep character creator).

---

## Pitfalls (already burned)

- `pkill -f unbound` SIGTERM’d the agent. Kill `target/.*/unbound$` only.
- Two native clients + one persisted token ≠ 1v1.
- Spacetime CLI default server can be **maincloud**. Always `-s local`.
- `spacetime sql` aggregates need aliases (`SELECT COUNT(*) AS n FROM dummy`).
- `NO_COLOR=1` breaks Trunk clap.
- wasm-opt may be missing; install binaryen or skip release web.
- Native `AssetPlugin` path is baked from `CARGO_MANIFEST_DIR` at compile time.
- Module table cannot be named `world_tick` (clash) — use `WorldTickTimer`.
- Bevy 0.19: `CursorOptions`, `TextLayout::no_wrap`, `Mesh3d`, `Camera3d`, `ChildOf`, `MessageReader`.
- `matches!` on imported `ACTION_*` treated them as bindings — use `==`.
- Local pawn must snap to server spawn (`bind_local_player`) or hits miss from origin.
- HUD HP used to rubber-band; snap-only reconcile >2 m.
- Dummy chase only when **drawn** (or in strike range) so sheathed gatherers are not clubbed.
- Shield block is a **front cone**, not 360°. Guard-break at 0 stam.

---

## Suggested first hour on the Studio

1. `git pull` `main`.
2. Start spacetime, publish `unbound` `-s local`, `cargo run -p unbound-client`.
3. Walk, draw, 1/2/3, dummy, E gather, Tab lock. Confirm Metal window, audio, identity file.
4. `./scripts/mvp_check.sh`.
5. Either start glTF, or finish guard-break shield pop (EVT 6) as a small feel slice.

If you continue with an autonomous agent: one focused commit per slice, `git log -15` first, worktree for client edits, headless tests, **do not** steal the desktop, keep one native window per token.
