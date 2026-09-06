# Unbound

Browser-first persistent action RPG: **Bevy** client, **SpacetimeDB** backend, **Rust** everywhere.

Sheathe and you are in WoW. Draw and you are in Elden Ring. Your bag is your class.

Slice 0 is locomotion. WASD is predicted locally. The module is authoritative at 30 Hz. Sheathed = hold RMB to look. Drawn (`F`) = captured mouse.

Vision, inspirations, stack, and slice plan: **[docs/VISION.md](docs/VISION.md)**.

## One-machine loop

```bash
# terminal 1
spacetime start --non-interactive

# terminal 2
cd unbound
spacetime publish unbound -p crates/module -s local -y
spacetime generate --lang rust -o crates/client/src/module_bindings -p crates/module
cargo run -p unbound-client
```

Browser (WebGPU):

```bash
spacetime start --non-interactive   # if the host is down
./scripts/web.sh                    # http://127.0.0.1:8080
```

Open a second native or browser client to see another capsule interpolate in.

## Controls

| Input | Action |
|---|---|
| WASD | Move (camera-relative, instant) |
| Hold RMB | Look (while sheathed) |
| F | Draw / sheathe |
| LMB / RMB | Light / heavy (while drawn) |
| Space | Dodge |
| Shift | Sprint |
| 1 / 2 / 3 | Sword / bow / staff |
| Tab | Lock onto the dummy |
| Q or MMB | Block (sword) |
| Esc | Free the cursor |

## Layout

```
docs/VISION.md  game pillars, stack, what we are not building
crates/shared   movement math used by module and client
crates/module   SpacetimeDB WASM module (tables + reducers + tick)
crates/client   Bevy client (native + WASM/WebGPU)
```
