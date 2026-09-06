# Unbound — vision and stack

This is the north star from the original design conversation. Implementation follows it; if a change fights this document, change the document on purpose.

## What the game is

A **browser-first persistent action RPG** whose class is your inventory, whose camera and locomotion feel like retail WoW, and whose hits have Souls weight.

The product is **feel in a Chrome tab**, not triangle count and not a 2,000-player shard.

**Sheathe your weapon and you are in WoW. Draw it and you are in Elden Ring. Your bag decides whether that fight is a sword, a bow, or a staff.**

## What we steal, what we leave

| Source | Take | Leave |
|---|---|---|
| **RuneScape** | No class lock. Skills are independent. Gear in the bag *is* the spec. Master-of-all is a long-term identity. | 600 ms ticks, click-to-move, the feeling that the server is thinking about your click. |
| **World of Warcraft** | Hold RMB to look, WASD starts this frame, camera and body are one, PvP reads as a duel of motion. | Class trinity, 30-button bars, tab-target as the only way to fight. |
| **Dark Souls / Elden Ring** | Stamina as the pacing resource. Light vs committed heavy. Optional lock-on. Readable telegraphs. Armor weight affecting dodge. | Bonfire-as-the-whole-loop, default slow walk, “the cursor never exists.” |

## Controls

RMB cannot be both “look” and “heavy attack.” Context solves it:

- **Sheathed:** cursor free for world, inventory, gathering. Hold RMB = mouselook + WASD. Instant start/stop.
- **Drawn:** mouse captured. LMB light, RMB heavy/block, WASD, Space dodge, Shift sprint, optional lock-on. Camera is always live.
- **Gear swap** is a real action (~0.4–0.6 s). No class restriction. No “you didn’t spec this.”

No global cooldown on movement. Ever.

Slice 0 ships the sheathe/draw camera and predicted WASD. Combat buttons come with loadouts.

## Combat and progression (not all shipped)

**Motion** is 100% WoW: almost no acceleration, direction change is immediate, camera is the aiming device.

**Offense** is Souls-shaped: light attacks near-instant, heavies and charged spells have startup you can read. Stamina gates sprint, dodge i-frames, block, and heavies — not walking.

**Range and magic** are action: bows and spells aim with the camera, not a nameplate.

**Progression is OSRS-honest:**

- Skills are independent (Melee, Defence, Ranged, Magic, Hitpoints, plus a tiny gathering/crafting set).
- Equipment is the class. A robe + staff *is* mage. Plate + 2H *is* melee.
- No penalty for being a generalist other than time.

**Combat authority** is MMO, not fighting-game rollback:

- Client predicts *your* locomotion and swing start.
- Attacks are reducers. The module checks stamina, range from authoritative positions, dodge windows.
- Souls lives on the screen. The database does not store per-bone hitboxes every frame.

PvP in a small yard is the feel-target, not raids.

## The browser boundary worth pushing

1. Sub-frame movement start (WASD is not a click the sim notices later).
2. Client prediction so 40–80 ms RTT still feels local.
3. WebGPU that looks like a game, not a canvas demo.
4. One simulation compiled two ways: native-speed module, WASM client.

Do not also try to be the biggest open world, the prettiest GI, *and* a megaserver.

Art is **readable stylized**. Silhouettes and animation sell Souls combat; photoreal hero assets eat the project.

## Technology

Constraints that picked this stack: browser-first, high performance as a hard requirement, few moving parts, full E2E on one machine.

| Layer | Choice | Role |
|---|---|---|
| Language | **Rust everywhere** | Client, module, shared math. No TypeScript module, no Node. |
| Client | **Bevy** → native + **WASM / WebGPU** | Camera, WASD, animation, HUD. Native for iteration, browser for the real target. |
| Backend | **SpacetimeDB** (`spacetime start`) | Game server + database + replication + identity. |
| Module | **Rust WASM module** (`spacetimedb` 2.10) | Tables, reducers, 30 Hz `world_tick`, later nearby-player views. |
| Sync | **Subscriptions** + generated **Rust SDK** | Clients subscribe to rows; SDK keeps a local cache. |
| Persistence | **STDB commit log** | In-memory for speed, durable on disk. No SQLite, no Postgres in v1. |
| Auth (dev) | **STDB identity** | One identity per client. OIDC later if accounts ship. |
| Prediction | **Client-owned** | Apply *your* input this frame; reconcile from the `Player` row. Interpolate everyone else. |
| Physics (v1) | **Kinematic move in `world_tick`** | Capsule vs plane. Not Avian-on-the-server. |
| Assets | **glTF** in `assets/` when we have them | Blender in, game out. |
| Browser shell | **Trunk** + thin `index.html` | No React. |

### What you run on this machine

```text
spacetime start                     # host on :3000
spacetime publish unbound -p crates/module -s local
cargo run -p unbound-client         # native Bevy
./scripts/web.sh                    # WASM + WebGPU at :8080
```

Two processes plus a client. No Redis, Docker compose, auth provider, or separate physics server.

### Feel budget (this is part of the stack)

- Server tick: **30 Hz** scheduled reducer
- Input send: **20 Hz** intents (`set_input`), **never positions**
- Client render: **60/120 Hz**
- Authority: module. Presentation: Bevy.

Never let the renderer wait on a reducer. Never let the client say “I am at (x, z).”

### Repo shape

```text
crates/shared    movement / combat math — no Bevy, no SpacetimeDB
crates/module    tables + reducers + tick (own Cargo workspace so publish does not build Bevy to WASM)
crates/client    Bevy app + generated module_bindings/
```

`spacetime generate --lang rust` owns `crates/client/src/module_bindings`. Do not hand-edit those files.

### Rejected on purpose

- **C# / Unity / Godot** — Unity is the content-fast path and loses the browser-performance fight. Godot 4 cannot officially export C# to the web.
- **Lightyear + headless Bevy + SQLite** — right if we wanted frame-perfect rollback. Wrong once SpacetimeDB owns persistence and the RS world.
- **TypeScript client or Three.js** — two sims the moment a real server exists.
- **Tab-target, RS ticks, trusting the browser with damage**

C# stays off the table unless the bottleneck becomes *shipping a readable human with a sword*, not *making WASD feel like WoW in Chrome*.

## Slices

An MMO is a studio. A yard that feels right is a side project.

| Slice | Status | Bar |
|---|---|---|
| **0 — locomotion** | Playable | Sheathe/draw camera (far WoW vs close shoulder + lock-on frame), predicted WASD with walk and sprint footfalls, local pawn snaps to server spawn, 30 Hz module, native + browser against one local host. Sprint opens the camera a half-meter. Tab puts a gold ring on the lock target, yaws and pitches at them, clicks and pulses on grab, shrinks on drop, and drops if they die or leave range. Camera pushes out of dummy and other wanderers. H/F1 holds or pins a control card that never steals WASD. |
| **1 — three loadouts** | Playable | Sword, bow, staff (`1/2/3`). Stamina, light/heavy, dodge i-frames in the roll (ghosted), frontal block (chip + hold) that guard-breaks at 0 stam. Dummy holds a pocket, shuffles sideways between swings, and winds a club (heavy under 70 HP) through pokes; the windup whooshes and puffs the capsule so you hear and see the telegraph; armored hits flash white; chase footfalls tick while he closes; heavies have hyperarmor; stam waits out the swing; club facing commits so a sidestep works. Loadout is armor weight (sword rolls heavy, staff light). Draw/sheathe is steel, string, or wood by bag. Melee steps in on the swing. Client predicts swing start and the bolt leaving the bow (whoosh and crosshair kick on release); bows/staffs aim with camera pitch; hits shake the camera, dummy heavies rumble more than lights. A body drop thuds; emptying a node cracks and flattens; charges return with a sparkle and the mesh eases back in. Dummy heavies rumble the camera longer. |
| **2 — 1v1 PvP** | Playable | Same combat rules on every player capsule. Two clients (or headless bots) share the yard; hits, blocks, deaths, and respawns apply to both. Remote swings interpolate on the client instead of stepping at tick rate. Remote dodges ghost during i-frames. Remote walk/sprint, windup, and draw are audible at range. Respawn has a short walk-out grace; the camera eases to spawn instead of snapping the yard. |
| **3 — a character that persists** | Playable | `character` row keeps XP/name/loadout across disconnect. Client stores the STDB access token so a relaunch is the same wanderer with the same bag. Skills go up when you land hits (level-up toast). HUD shows XP into the current level. Sheathed `E` gathers wood/ore into Gathering. |

Proof gate: `./scripts/mvp_check.sh` must fail the process if dummy XP, persist reconnect, gather XP, bow XP, or 1v1 damage don't happen.

**Not v1:** auction house, quest hubs, raids, OAuth, world streaming, navmesh armies, a character creator deep enough to ship a trailer.

## Risks that actually kill this

1. Building systems before feel.
2. Too much animation-lock (lose WoW) or too little (lose Souls).
3. Bevy / SpacetimeDB version churn — pin, upgrade on purpose, keep `shared` engine-light.
4. Calling it an MMO too early. Ship a persistent action brawler with a world door.

## Open product calls (already decided)

- Drawn-weapon camera: **sheathe/draw hybrid** (not always-WoW keys, not always-captured Souls).
- v1 scope: **PvP yard + persistence**, not always-on megaworld from day one, not solo-Souls-then-netcode.
- Performance budget: **input-to-photon first**, stylized cheap graphics. Atmosphere can come after the walk feels right.
- Combat authority: **predicted movement, reducer-resolved attacks**. Do not reinvent Lightyear on a database.
