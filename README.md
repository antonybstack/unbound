# Unbound

Browser-first persistent action RPG: **Bevy** client, **SpacetimeDB** backend, **Rust** everywhere.

Sheathe and you are in WoW. Draw and you are in Elden Ring. Your bag is your class.

**MVP (slices 0–3) is playable.** Predicted WASD and swing start, sheathe/draw camera, three loadouts, a dummy with a readable windup, 1v1 PvP, and a `character` row that keeps XP and bag after you leave the yard. Native identity lives in `~/.local/share/unbound/identity.token`; the browser uses `localStorage`.

Vision, inspirations, stack, and slice plan: **[docs/VISION.md](docs/VISION.md)**.  
Continue from another machine: **[handoff.md](handoff.md)**.

## One-machine loop

```bash
# terminal 1
spacetime start --non-interactive

# terminal 2
cd unbound
spacetime publish unbound -p crates/module -s local -y
spacetime generate --lang rust -o crates/client/src/module_bindings -p crates/module
spacetime generate --lang rust -o crates/bot/src/module_bindings -p crates/module
cargo run -p unbound-client
```

Browser (WebGPU):

```bash
spacetime start --non-interactive   # if the host is down
./scripts/web.sh                    # http://127.0.0.1:8080 (debug wasm, ~180MB)
./scripts/web-release.sh            # release WebGPU (~52MB after wasm-opt / ~14MB gzip)
./scripts/web_smoke.sh              # Chrome: wasm + canvas + WebGPU window, or fail
```

Open a second native or browser client to see another capsule interpolate in. Or prove the yard headless:

```bash
./scripts/mvp_check.sh              # dummy, persist, gather, bow, 1v1 — fails if XP/HP don't move
cargo run -p unbound-bot -- --name Alpha --token /tmp/unbound-alpha.token --mode pvp --seconds 12
```

## Controls

| Input | Action |
|---|---|
| WASD | Move (camera-relative, instant) |
| Hold RMB | Look (while sheathed) |
| F | Draw / sheathe |
| LMB / RMB | Light / heavy (while drawn) |
| Space | Dodge |
| Shift | Sprint (drains stamina) |
| E | Gather wood/ore (while sheathed, in range) |
| 1 / 2 / 3 | Sword / bow / staff |
| Tab | Lock onto nearest dummy or wanderer |
| Q or MMB | Block (sword) |
| Esc | Free the cursor |
| H or F1 | Hold for controls, tap to pin |

## Layout

```
docs/VISION.md     game pillars, stack, what we are not building
crates/shared      movement / combat math (no Bevy, no SpacetimeDB)
crates/module      SpacetimeDB WASM module (tables + reducers + tick)
crates/client      Bevy client (native + WASM/WebGPU)
crates/bot         headless SDK client for 1v1 / dummy / gather proofs
scripts/mvp_check.sh   dummy, persistence, gather, two-bot PvP
```

`spacetime generate --lang rust` owns `module_bindings/` in the client and bot. Do not hand-edit those files.
