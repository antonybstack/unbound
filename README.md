# Unbound

Browser-first action RPG slice: **Bevy** client, **SpacetimeDB** backend, **Rust** everywhere.

Slice 0 is locomotion. WASD is predicted locally. The module is authoritative at 30 Hz. Sheathed = WoW camera (hold RMB to look). Drawn (`F`) = captured mouse.

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
| Esc | Free the cursor |

## Layout

```
crates/shared   movement math used by module and client
crates/module   SpacetimeDB WASM module (tables + reducers + tick)
crates/client   Bevy client (native + WASM/WebGPU)
```
