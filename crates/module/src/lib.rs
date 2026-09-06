use std::time::Duration;

use spacetimedb::{table, Identity, ReducerContext, ScheduleAt, Table};
use unbound_shared::{
    action_busy, blocking, dist_xz, dodge_ticks, facing_dot, gather_ticks, hitstun_ticks,
    integrate, invulnerable, knockback, loadout, move_lock, node_respawn_ticks, node_xp, push_apart,
    scaled_damage, skill_for_loadout, skill_level, stamina_ok, swap_ticks, yaw_forward, ACTION_BLOCK,
    ACTION_DEAD, ACTION_DODGE, ACTION_GATHER, ACTION_HEAVY, ACTION_HIT, ACTION_LIGHT, ACTION_NONE,
    ACTION_SWAP, BODY_SEPARATION, BTN_BLOCK, BTN_DODGE, BTN_HEAVY, BTN_INTERACT, BTN_LIGHT, BTN_SPRINT,
    DODGE_SPEED, GATHER_RANGE, HP_REGEN_PER_SEC, KNOCKBACK_HEAVY, KNOCKBACK_LIGHT, LOADOUT_SWORD,
    MAX_HP, MAX_STAMINA, MOVE_SPEED, NODE_ORE, NODE_WOOD, PLAYER_RADIUS, SKILL_DEFENCE, SKILL_GATHERING,
    SKILL_HITPOINTS, SKILL_MAGIC, SKILL_MELEE, SKILL_RANGED, SPRINT_SPEED, SPRINT_STAMINA_PER_SEC,
    STAMINA_REGEN_PER_SEC, TICK_DT, WORLD_HALF,
};

#[table(accessor = player, public)]
#[derive(Clone, Debug)]
pub struct Player {
    #[primary_key]
    pub identity: Identity,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub yaw: f32,
    pub drawn: bool,
    pub hp: f32,
    pub stamina: f32,
    pub loadout: u8,
    pub action: u8,
    pub action_ticks: u8,
    pub pending_hit: bool,
    pub alive: bool,
    pub name: String,
}

#[table(accessor = player_input)]
#[derive(Clone, Debug)]
pub struct PlayerInput {
    #[primary_key]
    pub identity: Identity,
    pub dir_x: f32,
    pub dir_z: f32,
    pub yaw: f32,
    pub drawn: bool,
    pub buttons: u32,
    pub loadout: u8,
}

#[table(accessor = character, public)]
#[derive(Clone, Debug)]
pub struct Character {
    #[primary_key]
    pub identity: Identity,
    pub name: String,
    pub melee_xp: u64,
    pub ranged_xp: u64,
    pub magic_xp: u64,
    pub defence_xp: u64,
    pub hitpoints_xp: u64,
    pub gather_xp: u64,
}

#[table(accessor = dummy, public)]
#[derive(Clone, Debug)]
pub struct Dummy {
    #[primary_key]
    pub id: u32,
    pub x: f32,
    pub z: f32,
    pub yaw: f32,
    pub hp: f32,
    pub action: u8,
    pub action_ticks: u8,
    pub pending_hit: bool,
    pub alive: bool,
    pub cooldown: u8,
}

#[table(accessor = projectile, public)]
#[derive(Clone, Debug)]
pub struct Projectile {
    #[primary_key]
    #[auto_inc]
    pub id: u32,
    pub x: f32,
    pub z: f32,
    pub vx: f32,
    pub vz: f32,
    pub owner: Identity,
    pub from_dummy: bool,
    pub dummy_owner: u32,
    pub damage: f32,
    pub ttl: u8,
    pub skill: u8,
}

#[table(accessor = gather_node, public)]
#[derive(Clone, Debug)]
pub struct GatherNode {
    #[primary_key]
    pub id: u32,
    pub kind: u8,
    pub x: f32,
    pub z: f32,
    pub charges: u8,
    pub cooldown: u16,
}

#[table(accessor = combat_event, public, event)]
#[derive(Clone, Debug)]
pub struct CombatEvent {
    pub kind: u8,
    pub attacker_is_dummy: bool,
    pub attacker: Identity,
    pub dummy_id: u32,
    pub target_is_dummy: bool,
    pub target: Identity,
    pub target_dummy: u32,
    pub damage: f32,
    pub x: f32,
    pub z: f32,
}

#[table(accessor = world_tick_timer, scheduled(world_tick))]
pub struct WorldTickTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: ScheduleAt,
}

const EVT_HIT: u8 = 1;
const EVT_KILL: u8 = 2;
const EVT_BLOCK: u8 = 3;
const EVT_DODGE: u8 = 4;
const EVT_GATHER: u8 = 5;
const DUMMY_ID: u32 = 1;
const DUMMY_HOME_X: f32 = 0.0;
const DUMMY_HOME_Z: f32 = -10.0;

#[spacetimedb::reducer(init)]
pub fn init(ctx: &ReducerContext) -> Result<(), String> {
    ctx.db.world_tick_timer().try_insert(WorldTickTimer {
        scheduled_id: 0,
        scheduled_at: ScheduleAt::Interval(Duration::from_millis(33).into()),
    })?;
    ctx.db.dummy().try_insert(Dummy {
        id: DUMMY_ID,
        x: DUMMY_HOME_X,
        z: DUMMY_HOME_Z,
        yaw: 0.0,
        hp: MAX_HP,
        action: ACTION_NONE,
        action_ticks: 0,
        pending_hit: false,
        alive: true,
        cooldown: 20,
    })?;
    spawn_node(ctx, 1, NODE_WOOD, -12.0, 2.0, 4)?;
    spawn_node(ctx, 2, NODE_WOOD, 12.0, 2.0, 4)?;
    spawn_node(ctx, 3, NODE_ORE, 0.0, 14.0, 3)?;
    log::info!("unbound module initialized (combat yard + gather)");
    Ok(())
}

fn spawn_node(
    ctx: &ReducerContext,
    id: u32,
    kind: u8,
    x: f32,
    z: f32,
    charges: u8,
) -> Result<(), String> {
    ctx.db.gather_node().try_insert(GatherNode {
        id,
        kind,
        x,
        z,
        charges,
        cooldown: 0,
    })?;
    Ok(())
}

#[spacetimedb::reducer(client_connected)]
pub fn connected(ctx: &ReducerContext) -> Result<(), String> {
    log::info!("client connected: {}", ctx.sender());
    Ok(())
}

#[spacetimedb::reducer(client_disconnected)]
pub fn disconnected(ctx: &ReducerContext) -> Result<(), String> {
    let identity = ctx.sender();
    let had = ctx.db.player().identity().delete(&identity);
    ctx.db.player_input().identity().delete(&identity);
    if had {
        log::info!("player left yard: {identity}");
    }
    Ok(())
}

#[spacetimedb::reducer]
pub fn set_name(ctx: &ReducerContext, name: String) -> Result<(), String> {
    let name = sanitize_name(name)?;
    let identity = ctx.sender();
    if let Some(mut c) = ctx.db.character().identity().find(&identity) {
        c.name = name.clone();
        ctx.db.character().identity().update(c);
    } else {
        ctx.db
            .character()
            .insert(default_character(identity, name.clone()));
    }
    if let Some(mut p) = ctx.db.player().identity().find(&identity) {
        p.name = name;
        ctx.db.player().identity().update(p);
    }
    Ok(())
}

#[spacetimedb::reducer]
pub fn set_input(
    ctx: &ReducerContext,
    dir_x: f32,
    dir_z: f32,
    yaw: f32,
    drawn: bool,
    buttons: u32,
    loadout: u8,
) -> Result<(), String> {
    let identity = ctx.sender();
    let dir_x = dir_x.clamp(-1.0, 1.0);
    let dir_z = dir_z.clamp(-1.0, 1.0);
    let loadout = if loadout > 2 { LOADOUT_SWORD } else { loadout };

    if ctx.db.player_input().identity().find(&identity).is_some() {
        ctx.db.player_input().identity().update(PlayerInput {
            identity,
            dir_x,
            dir_z,
            yaw,
            drawn,
            buttons,
            loadout,
        });
    } else {
        ctx.db.player_input().insert(PlayerInput {
            identity,
            dir_x,
            dir_z,
            yaw,
            drawn,
            buttons,
            loadout,
        });
    }

    if ctx.db.player().identity().find(&identity).is_none() {
        let character = ensure_character(ctx, identity);
        ctx.db.player().insert(Player {
            identity,
            x: spawn_x(identity),
            y: 0.0,
            z: 4.0,
            yaw,
            drawn,
            hp: MAX_HP,
            stamina: MAX_STAMINA,
            loadout,
            action: ACTION_NONE,
            action_ticks: 0,
            pending_hit: false,
            alive: true,
            name: character.name,
        });
        log::info!("player entered yard: {identity}");
    }
    Ok(())
}

#[spacetimedb::reducer]
pub fn world_tick(ctx: &ReducerContext, _tick: WorldTickTimer) -> Result<(), String> {
    if ctx.sender() != ctx.database_identity() {
        return Err("world_tick is scheduler-only".into());
    }

    tick_players(ctx);
    tick_dummy(ctx);
    separate_occupants(ctx);
    tick_projectiles(ctx);
    tick_nodes(ctx);
    Ok(())
}

fn tick_players(ctx: &ReducerContext) {
    let inputs: Vec<PlayerInput> = ctx.db.player_input().iter().collect();
    for input in inputs {
        let Some(mut player) = ctx.db.player().identity().find(&input.identity) else {
            continue;
        };
        if !player.alive {
            if player.action_ticks > 0 {
                player.action_ticks -= 1;
            }
            if player.action_ticks == 0 {
                respawn_player(&mut player);
            }
            ctx.db.player().identity().update(player);
            continue;
        }

        player.yaw = input.yaw;
        player.drawn = input.drawn;

        if player.action_ticks > 0 {
            player.action_ticks -= 1;
        }

        if player.pending_hit && player.action_ticks == 0 {
            resolve_player_attack(ctx, &player);
            player.pending_hit = false;
            let def = loadout(player.loadout);
            player.action_ticks = if player.action == ACTION_HEAVY {
                def.heavy_recover_ticks
            } else {
                def.light_recover_ticks
            };
        }

        if player.action == ACTION_GATHER && player.action_ticks == 0 {
            resolve_gather(ctx, &player);
            player.action = ACTION_NONE;
        }

        if player.action == ACTION_GATHER {
            if player.drawn || nearest_node_range(ctx, player.x, player.z) > GATHER_RANGE {
                player.action = ACTION_NONE;
                player.action_ticks = 0;
            }
        }

        if player.action_ticks == 0 && player.action != ACTION_BLOCK {
            player.action = ACTION_NONE;
        }

        if player.drawn && !action_busy(player.action) {
            try_start_player_action(ctx, &mut player, &input);
        } else if !player.drawn && !action_busy(player.action) {
            try_start_gather(&mut player, &input, ctx);
        } else if !player.drawn && player.action != ACTION_GATHER {
            player.action = ACTION_NONE;
            player.pending_hit = false;
        }

        if player.action == ACTION_BLOCK && (input.buttons & BTN_BLOCK) == 0 {
            player.action = ACTION_NONE;
        }

        let sprinting = (input.buttons & BTN_SPRINT) != 0
            && player.stamina > 1.0
            && !move_lock(player.action)
            && player.action != ACTION_DODGE;
        let speed = if player.action == ACTION_DODGE {
            DODGE_SPEED
        } else if sprinting {
            SPRINT_SPEED
        } else {
            MOVE_SPEED
        };
        if !move_lock(player.action) {
            let (x, z) = integrate(
                player.x,
                player.z,
                input.yaw,
                input.dir_x,
                input.dir_z,
                TICK_DT,
                speed,
            );
            player.x = x;
            player.z = z;
        }

        if sprinting {
            player.stamina = (player.stamina - SPRINT_STAMINA_PER_SEC * TICK_DT).max(0.0);
        } else if player.action != ACTION_DODGE && player.action != ACTION_BLOCK {
            player.stamina = (player.stamina + STAMINA_REGEN_PER_SEC * TICK_DT).min(MAX_STAMINA);
        }
        if player.hp < MAX_HP {
            player.hp = (player.hp + HP_REGEN_PER_SEC * TICK_DT).min(MAX_HP);
        }

        ctx.db.player().identity().update(player);
    }
}

fn try_start_player_action(ctx: &ReducerContext, player: &mut Player, input: &PlayerInput) {
    if input.loadout != player.loadout {
        player.loadout = input.loadout;
        player.action = ACTION_SWAP;
        player.action_ticks = swap_ticks();
        player.pending_hit = false;
        return;
    }
    if (input.buttons & BTN_DODGE) != 0 {
        let cost = loadout(player.loadout).dodge_stamina;
        if stamina_ok(player.stamina, cost) {
            player.stamina -= cost;
            player.action = ACTION_DODGE;
            player.action_ticks = dodge_ticks();
            player.pending_hit = false;
            let (x, z) = integrate(
                player.x,
                player.z,
                input.yaw,
                if input.dir_x.abs() + input.dir_z.abs() < 0.1 {
                    0.0
                } else {
                    input.dir_x
                },
                if input.dir_x.abs() + input.dir_z.abs() < 0.1 {
                    1.0
                } else {
                    input.dir_z
                },
                TICK_DT * dodge_ticks() as f32 * 0.35,
                DODGE_SPEED,
            );
            player.x = x;
            player.z = z;
        }
        return;
    }
    if (input.buttons & BTN_BLOCK) != 0 && player.loadout == LOADOUT_SWORD {
        player.action = ACTION_BLOCK;
        player.action_ticks = 1;
        return;
    }
    let def = loadout(player.loadout);
    if (input.buttons & BTN_HEAVY) != 0 && stamina_ok(player.stamina, def.heavy_stamina) {
        player.stamina -= def.heavy_stamina;
        player.action = ACTION_HEAVY;
        player.action_ticks = def.heavy_windup_ticks;
        player.pending_hit = true;
        return;
    }
    if (input.buttons & BTN_LIGHT) != 0 && stamina_ok(player.stamina, def.light_stamina) {
        player.stamina -= def.light_stamina;
        player.action = ACTION_LIGHT;
        player.action_ticks = def.light_windup_ticks;
        player.pending_hit = true;
    }
    let _ = ctx;
}

fn try_start_gather(player: &mut Player, input: &PlayerInput, ctx: &ReducerContext) {
    if (input.buttons & BTN_INTERACT) == 0 {
        return;
    }
    if nearest_node_range(ctx, player.x, player.z) > GATHER_RANGE {
        return;
    }
    player.action = ACTION_GATHER;
    player.action_ticks = gather_ticks();
    player.pending_hit = false;
}

fn resolve_gather(ctx: &ReducerContext, player: &Player) {
    let mut best: Option<(f32, GatherNode)> = None;
    for node in ctx.db.gather_node().iter() {
        if node.charges == 0 {
            continue;
        }
        let d = dist_xz(player.x, player.z, node.x, node.z);
        if d <= GATHER_RANGE && best.as_ref().map(|(bd, _)| d < *bd).unwrap_or(true) {
            best = Some((d, node));
        }
    }
    let Some((_, mut node)) = best else {
        return;
    };
    node.charges = node.charges.saturating_sub(1);
    if node.charges == 0 {
        node.cooldown = node_respawn_ticks();
    }
    grant_xp(ctx, player.identity, SKILL_GATHERING, node_xp(node.kind));
    emit(
        ctx,
        EVT_GATHER,
        false,
        player.identity,
        0,
        false,
        player.identity,
        node.id,
        node_xp(node.kind) as f32,
        node.x,
        node.z,
    );
    ctx.db.gather_node().id().update(node);
}

fn nearest_node_range(ctx: &ReducerContext, x: f32, z: f32) -> f32 {
    let mut best = f32::MAX;
    for node in ctx.db.gather_node().iter() {
        if node.charges == 0 {
            continue;
        }
        let d = dist_xz(x, z, node.x, node.z);
        if d < best {
            best = d;
        }
    }
    best
}

fn resolve_player_attack(ctx: &ReducerContext, player: &Player) {
    let def = loadout(player.loadout);
    let skill_id = skill_for_loadout(player.loadout);
    let level = character_skill(ctx, player.identity, skill_id);
    let heavy = player.action == ACTION_HEAVY;
    let dmg = scaled_damage(
        if heavy {
            def.heavy_damage
        } else {
            def.light_damage
        },
        level,
    );
    if def.is_projectile {
        let (fx, fz) = yaw_forward(player.yaw);
        ctx.db.projectile().insert(Projectile {
            id: 0,
            x: player.x + fx * 0.9,
            z: player.z + fz * 0.9,
            vx: fx * def.projectile_speed,
            vz: fz * def.projectile_speed,
            owner: player.identity,
            from_dummy: false,
            dummy_owner: 0,
            damage: dmg,
            ttl: 45,
            skill: skill_id,
        });
        return;
    }
    melee_strike(
        ctx,
        player.x,
        player.z,
        player.yaw,
        def.melee_range,
        dmg,
        false,
        player.identity,
        0,
        skill_id,
        if heavy {
            KNOCKBACK_HEAVY
        } else {
            KNOCKBACK_LIGHT
        },
    );
}

fn tick_dummy(ctx: &ReducerContext) {
    let Some(mut dummy) = ctx.db.dummy().id().find(&DUMMY_ID) else {
        return;
    };
    if !dummy.alive {
        if dummy.action_ticks > 0 {
            dummy.action_ticks -= 1;
        } else {
            dummy.hp = MAX_HP;
            dummy.alive = true;
            dummy.action = ACTION_NONE;
            dummy.cooldown = 40;
            dummy.x = DUMMY_HOME_X;
            dummy.z = DUMMY_HOME_Z;
        }
        ctx.db.dummy().id().update(dummy);
        return;
    }
    if dummy.action_ticks > 0 {
        dummy.action_ticks -= 1;
    }
    if dummy.pending_hit && dummy.action_ticks == 0 {
        melee_strike(
            ctx,
            dummy.x,
            dummy.z,
            dummy.yaw,
            2.2,
            11.0,
            true,
            Identity::from_byte_array([0; 32]),
            dummy.id,
            SKILL_MELEE,
            KNOCKBACK_LIGHT,
        );
        dummy.pending_hit = false;
        dummy.action = ACTION_NONE;
        dummy.cooldown = 24;
    }
    if dummy.cooldown > 0 {
        dummy.cooldown -= 1;
    }

    let mut nearest: Option<(f32, f32, f32, f32)> = None;
    for p in ctx.db.player().iter() {
        if !p.alive {
            continue;
        }
        let d = dist_xz(dummy.x, dummy.z, p.x, p.z);
        if nearest.map(|(nd, _, _, _)| d < nd).unwrap_or(true) {
            let yaw = (-(p.x - dummy.x)).atan2(-(p.z - dummy.z));
            nearest = Some((d, p.x, p.z, yaw));
        }
    }
    let home_d = dist_xz(dummy.x, dummy.z, DUMMY_HOME_X, DUMMY_HOME_Z);
    if let Some((d, _px, _pz, yaw)) = nearest {
        dummy.yaw = yaw;
        let chase = d < 12.0 && home_d < 14.0;
        if chase && d > 2.05 && dummy.action_ticks == 0 {
            let (x, z) = integrate(dummy.x, dummy.z, dummy.yaw, 0.0, 1.0, TICK_DT, 3.2);
            dummy.x = x;
            dummy.z = z;
        } else if !chase && home_d > 0.6 && dummy.action_ticks == 0 {
            let yaw_home = (-(DUMMY_HOME_X - dummy.x)).atan2(-(DUMMY_HOME_Z - dummy.z));
            dummy.yaw = yaw_home;
            let (x, z) = integrate(dummy.x, dummy.z, dummy.yaw, 0.0, 1.0, TICK_DT, 3.6);
            dummy.x = x;
            dummy.z = z;
        }
        if chase && d < 2.55 && dummy.cooldown == 0 && dummy.action_ticks == 0 {
            dummy.action = ACTION_LIGHT;
            dummy.action_ticks = 9;
            dummy.pending_hit = true;
        }
    } else if home_d > 0.6 && dummy.action_ticks == 0 {
        let yaw_home = (-(DUMMY_HOME_X - dummy.x)).atan2(-(DUMMY_HOME_Z - dummy.z));
        dummy.yaw = yaw_home;
        let (x, z) = integrate(dummy.x, dummy.z, dummy.yaw, 0.0, 1.0, TICK_DT, 3.6);
        dummy.x = x;
        dummy.z = z;
    }
    ctx.db.dummy().id().update(dummy);
}

fn separate_occupants(ctx: &ReducerContext) {
    let mut players: Vec<Player> = ctx.db.player().iter().collect();
    let n = players.len();
    for i in 0..n {
        for j in (i + 1)..n {
            if !players[i].alive || !players[j].alive {
                continue;
            }
            let (ax, az, bx, bz) = push_apart(
                players[i].x,
                players[i].z,
                players[j].x,
                players[j].z,
                BODY_SEPARATION,
            );
            players[i].x = ax;
            players[i].z = az;
            players[j].x = bx;
            players[j].z = bz;
        }
    }

    if let Some(mut dummy) = ctx.db.dummy().id().find(&DUMMY_ID) {
        if dummy.alive {
            for p in players.iter_mut() {
                if !p.alive {
                    continue;
                }
                let (dx, dz, px, pz) = push_apart(
                    dummy.x,
                    dummy.z,
                    p.x,
                    p.z,
                    BODY_SEPARATION + 0.15,
                );
                dummy.x = dx;
                dummy.z = dz;
                p.x = px;
                p.z = pz;
            }
        }
        ctx.db.dummy().id().update(dummy);
    }

    for p in players {
        ctx.db.player().identity().update(p);
    }
}

fn tick_projectiles(ctx: &ReducerContext) {
    let shots: Vec<Projectile> = ctx.db.projectile().iter().collect();
    for mut shot in shots {
        shot.x += shot.vx * TICK_DT;
        shot.z += shot.vz * TICK_DT;
        shot.ttl = shot.ttl.saturating_sub(1);
        if shot.x.abs() > WORLD_HALF || shot.z.abs() > WORLD_HALF || shot.ttl == 0 {
            ctx.db.projectile().id().delete(&shot.id);
            continue;
        }
        let mut consumed = false;
        if !shot.from_dummy {
            if let Some(mut dummy) = ctx.db.dummy().id().find(&DUMMY_ID) {
                if dummy.alive && dist_xz(shot.x, shot.z, dummy.x, dummy.z) < 0.7 {
                    apply_dummy_damage(ctx, &mut dummy, shot.damage, shot.owner, shot.skill);
                    ctx.db.dummy().id().update(dummy);
                    consumed = true;
                }
            }
        }
        if !consumed {
            let victims: Vec<Player> = ctx.db.player().iter().collect();
            for mut victim in victims {
                if !victim.alive {
                    continue;
                }
                if !shot.from_dummy && victim.identity == shot.owner {
                    continue;
                }
                if dist_xz(shot.x, shot.z, victim.x, victim.z) <= PLAYER_RADIUS + 0.35 {
                    apply_player_damage(
                        ctx,
                        &mut victim,
                        shot.damage,
                        shot.from_dummy,
                        shot.owner,
                        shot.dummy_owner,
                        shot.skill,
                        shot.x - shot.vx * TICK_DT,
                        shot.z - shot.vz * TICK_DT,
                        KNOCKBACK_LIGHT,
                    );
                    ctx.db.player().identity().update(victim);
                    consumed = true;
                    break;
                }
            }
        }
        if consumed {
            ctx.db.projectile().id().delete(&shot.id);
        } else {
            ctx.db.projectile().id().update(shot);
        }
    }
}

fn tick_nodes(ctx: &ReducerContext) {
    let nodes: Vec<GatherNode> = ctx.db.gather_node().iter().collect();
    for mut node in nodes {
        if node.charges == 0 && node.cooldown > 0 {
            node.cooldown -= 1;
            if node.cooldown == 0 {
                node.charges = if node.kind == NODE_ORE { 3 } else { 4 };
            }
            ctx.db.gather_node().id().update(node);
        }
    }
}

fn melee_strike(
    ctx: &ReducerContext,
    x: f32,
    z: f32,
    yaw: f32,
    range: f32,
    damage: f32,
    from_dummy: bool,
    attacker: Identity,
    dummy_id: u32,
    skill: u8,
    kb: f32,
) {
    if !from_dummy {
        if let Some(mut dummy) = ctx.db.dummy().id().find(&DUMMY_ID) {
            if dummy.alive {
                let dx = dummy.x - x;
                let dz = dummy.z - z;
                let dist = (dx * dx + dz * dz).sqrt();
                if dist <= range + 0.4 && facing_dot(yaw, dx, dz) > 0.25 {
                    apply_dummy_damage(ctx, &mut dummy, damage, attacker, skill);
                    let (nx, nz) = knockback(dummy.x, dummy.z, x, z, kb * 0.6);
                    dummy.x = nx;
                    dummy.z = nz;
                    ctx.db.dummy().id().update(dummy);
                }
            }
        }
    }
    let victims: Vec<Player> = ctx.db.player().iter().collect();
    for mut victim in victims {
        if !victim.alive {
            continue;
        }
        if !from_dummy && victim.identity == attacker {
            continue;
        }
        let dx = victim.x - x;
        let dz = victim.z - z;
        let dist = (dx * dx + dz * dz).sqrt();
        if dist <= range + PLAYER_RADIUS && facing_dot(yaw, dx, dz) > 0.2 {
            apply_player_damage(
                ctx, &mut victim, damage, from_dummy, attacker, dummy_id, skill, x, z, kb,
            );
            ctx.db.player().identity().update(victim);
        }
    }
}

fn apply_player_damage(
    ctx: &ReducerContext,
    victim: &mut Player,
    damage: f32,
    from_dummy: bool,
    attacker: Identity,
    dummy_id: u32,
    skill: u8,
    from_x: f32,
    from_z: f32,
    kb: f32,
) {
    if invulnerable(victim.action) {
        emit(
            ctx,
            EVT_DODGE,
            from_dummy,
            attacker,
            dummy_id,
            false,
            victim.identity,
            0,
            0.0,
            victim.x,
            victim.z,
        );
        return;
    }
    let mut dealt = damage;
    if blocking(victim.action) {
        dealt *= 0.3;
        victim.stamina = (victim.stamina - 8.0).max(0.0);
        emit(
            ctx,
            EVT_BLOCK,
            from_dummy,
            attacker,
            dummy_id,
            false,
            victim.identity,
            0,
            dealt,
            victim.x,
            victim.z,
        );
    }
    let def_lvl = character_skill(ctx, victim.identity, SKILL_DEFENCE);
    dealt *= 1.0 - 0.01 * def_lvl as f32;
    dealt = dealt.max(1.0);
    victim.hp -= dealt;
    let (nx, nz) = knockback(victim.x, victim.z, from_x, from_z, kb);
    victim.x = nx;
    victim.z = nz;
    grant_xp(ctx, victim.identity, SKILL_DEFENCE, 4);
    grant_xp(ctx, victim.identity, SKILL_HITPOINTS, 2);
    if !from_dummy {
        grant_xp(ctx, attacker, skill, 8);
        grant_xp(ctx, attacker, SKILL_HITPOINTS, 2);
    }
    if victim.hp <= 0.0 {
        victim.hp = 0.0;
        victim.alive = false;
        victim.action = ACTION_DEAD;
        victim.action_ticks = unbound_shared::death_respawn_ticks();
        emit(
            ctx,
            EVT_KILL,
            from_dummy,
            attacker,
            dummy_id,
            false,
            victim.identity,
            0,
            dealt,
            victim.x,
            victim.z,
        );
    } else {
        if !blocking(victim.action) {
            victim.action = ACTION_HIT;
            victim.action_ticks = hitstun_ticks();
            victim.pending_hit = false;
        }
        emit(
            ctx,
            EVT_HIT,
            from_dummy,
            attacker,
            dummy_id,
            false,
            victim.identity,
            0,
            dealt,
            victim.x,
            victim.z,
        );
    }
}

fn apply_dummy_damage(
    ctx: &ReducerContext,
    dummy: &mut Dummy,
    damage: f32,
    attacker: Identity,
    skill: u8,
) {
    dummy.hp -= damage;
    grant_xp(ctx, attacker, skill, 6);
    grant_xp(ctx, attacker, SKILL_HITPOINTS, 2);
    if dummy.hp <= 0.0 {
        dummy.hp = 0.0;
        dummy.alive = false;
        dummy.action = ACTION_DEAD;
        dummy.action_ticks = unbound_shared::death_respawn_ticks();
        dummy.pending_hit = false;
        emit(
            ctx,
            EVT_KILL,
            false,
            attacker,
            0,
            true,
            Identity::from_byte_array([0; 32]),
            dummy.id,
            damage,
            dummy.x,
            dummy.z,
        );
    } else {
        dummy.action = ACTION_HIT;
        dummy.action_ticks = hitstun_ticks();
        dummy.pending_hit = false;
        emit(
            ctx,
            EVT_HIT,
            false,
            attacker,
            0,
            true,
            Identity::from_byte_array([0; 32]),
            dummy.id,
            damage,
            dummy.x,
            dummy.z,
        );
    }
}

fn emit(
    ctx: &ReducerContext,
    kind: u8,
    attacker_is_dummy: bool,
    attacker: Identity,
    dummy_id: u32,
    target_is_dummy: bool,
    target: Identity,
    target_dummy: u32,
    damage: f32,
    x: f32,
    z: f32,
) {
    ctx.db.combat_event().insert(CombatEvent {
        kind,
        attacker_is_dummy,
        attacker,
        dummy_id,
        target_is_dummy,
        target,
        target_dummy,
        damage,
        x,
        z,
    });
}

fn grant_xp(ctx: &ReducerContext, identity: Identity, skill: u8, amount: u64) {
    if identity == Identity::from_byte_array([0; 32]) {
        return;
    }
    let mut c = ensure_character(ctx, identity);
    match skill {
        SKILL_RANGED => c.ranged_xp = c.ranged_xp.saturating_add(amount),
        SKILL_MAGIC => c.magic_xp = c.magic_xp.saturating_add(amount),
        SKILL_DEFENCE => c.defence_xp = c.defence_xp.saturating_add(amount),
        SKILL_HITPOINTS => c.hitpoints_xp = c.hitpoints_xp.saturating_add(amount),
        SKILL_GATHERING => c.gather_xp = c.gather_xp.saturating_add(amount),
        _ => c.melee_xp = c.melee_xp.saturating_add(amount),
    }
    ctx.db.character().identity().update(c);
}

fn character_skill(ctx: &ReducerContext, identity: Identity, skill: u8) -> u8 {
    let Some(c) = ctx.db.character().identity().find(&identity) else {
        return 1;
    };
    let xp = match skill {
        SKILL_RANGED => c.ranged_xp,
        SKILL_MAGIC => c.magic_xp,
        SKILL_DEFENCE => c.defence_xp,
        SKILL_HITPOINTS => c.hitpoints_xp,
        SKILL_GATHERING => c.gather_xp,
        _ => c.melee_xp,
    };
    skill_level(xp)
}

fn ensure_character(ctx: &ReducerContext, identity: Identity) -> Character {
    if let Some(c) = ctx.db.character().identity().find(&identity) {
        c
    } else {
        let c = default_character(identity, default_name(identity));
        ctx.db.character().insert(c.clone());
        c
    }
}

fn default_character(identity: Identity, name: String) -> Character {
    Character {
        identity,
        name,
        melee_xp: 0,
        ranged_xp: 0,
        magic_xp: 0,
        defence_xp: 0,
        hitpoints_xp: 0,
        gather_xp: 0,
    }
}

fn default_name(identity: Identity) -> String {
    let bytes = identity.to_byte_array();
    format!("Wanderer-{:02x}{:02x}", bytes[30], bytes[31])
}

fn sanitize_name(name: String) -> Result<String, String> {
    let trimmed: String = name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(16)
        .collect();
    if trimmed.len() < 2 {
        return Err("name too short".into());
    }
    Ok(trimmed)
}

fn spawn_x(identity: Identity) -> f32 {
    let b = identity.to_byte_array();
    let n = i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0;
    (n * 6.0).clamp(-6.0, 6.0)
}

fn respawn_player(player: &mut Player) {
    player.hp = MAX_HP;
    player.stamina = MAX_STAMINA;
    player.alive = true;
    player.action = ACTION_NONE;
    player.pending_hit = false;
    player.x = 0.0;
    player.z = 6.0;
    player.yaw = 0.0;
}


