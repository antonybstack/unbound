use crate::{
    facing_dot, ACTION_BLOCK, ACTION_DEAD, ACTION_DODGE, ACTION_GATHER, ACTION_HEAVY, ACTION_HIT,
    ACTION_LIGHT, ACTION_SPAWN, ACTION_SWAP, BTN_BLOCK, BTN_DODGE, BTN_HEAVY, BTN_INTERACT,
    BTN_LIGHT, LOADOUT_BOW, LOADOUT_STAFF, LOADOUT_SWORD, TICK_DT, TICK_HZ,
};

#[derive(Clone, Copy, Debug)]
pub struct LoadoutDef {
    pub id: u8,
    pub name: &'static str,
    pub light_stamina: f32,
    pub heavy_stamina: f32,
    pub dodge_stamina: f32,
    pub light_damage: f32,
    pub heavy_damage: f32,
    pub light_windup_ticks: u8,
    pub heavy_windup_ticks: u8,
    pub light_recover_ticks: u8,
    pub heavy_recover_ticks: u8,
    pub melee_range: f32,
    pub projectile_speed: f32,
    pub is_projectile: bool,
}

pub const SWORD: LoadoutDef = LoadoutDef {
    id: LOADOUT_SWORD,
    name: "Sword & board",
    light_stamina: 12.0,
    heavy_stamina: 28.0,
    dodge_stamina: 26.0,
    light_damage: 14.0,
    heavy_damage: 28.0,
    light_windup_ticks: 5,
    heavy_windup_ticks: 11,
    light_recover_ticks: 6,
    heavy_recover_ticks: 10,
    melee_range: 2.3,
    projectile_speed: 0.0,
    is_projectile: false,
};

pub const BOW: LoadoutDef = LoadoutDef {
    id: LOADOUT_BOW,
    name: "Bow",
    light_stamina: 10.0,
    heavy_stamina: 24.0,
    dodge_stamina: 22.0,
    light_damage: 12.0,
    heavy_damage: 22.0,
    light_windup_ticks: 6,
    heavy_windup_ticks: 14,
    light_recover_ticks: 8,
    heavy_recover_ticks: 12,
    melee_range: 0.0,
    projectile_speed: 28.0,
    is_projectile: true,
};

pub const STAFF: LoadoutDef = LoadoutDef {
    id: LOADOUT_STAFF,
    name: "Staff",
    light_stamina: 14.0,
    heavy_stamina: 32.0,
    dodge_stamina: 18.0,
    light_damage: 13.0,
    heavy_damage: 26.0,
    light_windup_ticks: 7,
    heavy_windup_ticks: 13,
    light_recover_ticks: 8,
    heavy_recover_ticks: 12,
    melee_range: 0.0,
    projectile_speed: 22.0,
    is_projectile: true,
};

pub fn loadout(id: u8) -> LoadoutDef {
    match id {
        LOADOUT_BOW => BOW,
        LOADOUT_STAFF => STAFF,
        _ => SWORD,
    }
}

pub fn dodge_ticks() -> u8 {
    dodge_ticks_for(LOADOUT_SWORD)
}

/// Sword & board rolls heavier; staff rolls lighter. Bag is the armor.
pub fn dodge_ticks_for(loadout: u8) -> u8 {
    let sec = match loadout {
        LOADOUT_STAFF => 0.22,
        LOADOUT_BOW => 0.26,
        _ => 0.30,
    };
    (sec * TICK_HZ).max(1.0) as u8
}

pub fn swap_ticks() -> u8 {
    (0.45 * TICK_HZ) as u8
}

pub fn hitstun_ticks() -> u8 {
    4
}

pub fn death_respawn_ticks() -> u8 {
    (3.0 * TICK_HZ) as u8
}

pub fn spawn_protect_ticks() -> u8 {
    (1.25 * TICK_HZ) as u8
}

pub fn gather_ticks() -> u8 {
    (1.35 * TICK_HZ) as u8
}

pub fn node_respawn_ticks() -> u16 {
    (8.0 * TICK_HZ) as u16
}

pub fn stamina_ok(stamina: f32, cost: f32) -> bool {
    stamina + 0.01 >= cost
}

pub fn scaled_damage(base: f32, skill_level: u8) -> f32 {
    (base * (1.0 + 0.025 * (skill_level.saturating_sub(1) as f32))).round()
}

pub fn action_busy(action: u8) -> bool {
    action == ACTION_LIGHT
        || action == ACTION_HEAVY
        || action == ACTION_DODGE
        || action == ACTION_SWAP
        || action == ACTION_HIT
        || action == ACTION_GATHER
        || action == ACTION_DEAD
        || action == ACTION_SPAWN
}

pub fn move_lock(action: u8) -> bool {
    action == ACTION_HEAVY
        || action == ACTION_SWAP
        || action == ACTION_GATHER
        || action == ACTION_DEAD
}

pub fn blocking(action: u8) -> bool {
    action == ACTION_BLOCK
}

/// Cosine of the shield arc. ~78° half-angle, same as the melee strike cone.
pub const BLOCK_COVER_DOT: f32 = 0.2;
pub const BLOCK_CHIP: f32 = 0.3;
pub const BLOCK_STAMINA_HIT: f32 = 8.0;

pub fn guard_break_ticks() -> u8 {
    10
}

/// True when the victim's facing covers the attacker (front cone).
pub fn block_covers(yaw: f32, x: f32, z: f32, from_x: f32, from_z: f32) -> bool {
    facing_dot(yaw, from_x - x, from_z - z) > BLOCK_COVER_DOT
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuardResult {
    Open,
    Covered,
    GuardBreak,
    OpenFlank,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GuardHit {
    pub result: GuardResult,
    pub damage_mul: f32,
    pub stamina_after: f32,
    pub knockback_mul: f32,
    pub hitstun: bool,
}

/// Shield only works while you face the blow. Empty stamina shatters the guard.
pub fn resolve_guard(
    action: u8,
    yaw: f32,
    x: f32,
    z: f32,
    from_x: f32,
    from_z: f32,
    stamina: f32,
) -> GuardHit {
    if !blocking(action) {
        return GuardHit {
            result: GuardResult::Open,
            damage_mul: 1.0,
            stamina_after: stamina,
            knockback_mul: 1.0,
            hitstun: true,
        };
    }
    if !block_covers(yaw, x, z, from_x, from_z) {
        return GuardHit {
            result: GuardResult::OpenFlank,
            damage_mul: 1.0,
            stamina_after: stamina,
            knockback_mul: 1.0,
            hitstun: true,
        };
    }
    let after = (stamina - BLOCK_STAMINA_HIT).max(0.0);
    if after <= 0.01 {
        GuardHit {
            result: GuardResult::GuardBreak,
            damage_mul: BLOCK_CHIP,
            stamina_after: 0.0,
            knockback_mul: 1.5,
            hitstun: true,
        }
    } else {
        GuardHit {
            result: GuardResult::Covered,
            damage_mul: BLOCK_CHIP,
            stamina_after: after,
            knockback_mul: 0.2,
            hitstun: false,
        }
    }
}

pub fn dodge_iframe(ticks_left: u8) -> bool {
    dodge_iframe_for(ticks_left, LOADOUT_SWORD)
}

pub fn dodge_iframe_for(ticks_left: u8, loadout: u8) -> bool {
    let total = dodge_ticks_for(loadout);
    ticks_left >= 2 && ticks_left + 1 <= total
}

pub fn invulnerable(action: u8, ticks_left: u8) -> bool {
    invulnerable_for(action, ticks_left, LOADOUT_SWORD)
}

pub fn invulnerable_for(action: u8, ticks_left: u8, loadout: u8) -> bool {
    (action == ACTION_DODGE && dodge_iframe_for(ticks_left, loadout))
        || (action == ACTION_SPAWN && ticks_left > 0)
}

/// Buttons that are meaningful as a 1-frame press and must be latched until `set_input`.
pub const BTN_EDGE: u32 = BTN_LIGHT | BTN_HEAVY | BTN_DODGE | BTN_INTERACT;

pub const DUMMY_AGGRO_RANGE: f32 = 12.0;
pub const DUMMY_LEASH_RANGE: f32 = 14.0;
pub const DUMMY_STRIKE_RANGE: f32 = 2.55;
pub const DUMMY_MELEE_RANGE: f32 = 2.2;
pub const DUMMY_CHASE_SPEED: f32 = 3.2;
pub const DUMMY_HOME_SPEED: f32 = 3.6;
pub const DUMMY_STRAFE_SPEED: f32 = 2.2;
pub const DUMMY_LIGHT_DAMAGE: f32 = 11.0;
pub const DUMMY_HEAVY_DAMAGE: f32 = 20.0;
pub const DUMMY_KEEP_OUT: f32 = 1.75;
pub const DUMMY_APPROACH: f32 = 2.35;

/// Sheathed wanderers are world-mode (gather, walk). The dummy only hunts drawn
/// weapons, or anyone who already stepped into strike range.
pub fn dummy_should_chase(drawn: bool, dist: f32) -> bool {
    if dist <= DUMMY_STRIKE_RANGE {
        return true;
    }
    drawn && dist < DUMMY_AGGRO_RANGE
}

pub const CAM_SHEATHED: f32 = 6.8;
pub const CAM_DRAWN: f32 = 4.35;
pub const CAM_LOCK: f32 = 5.15;
pub const CAM_SPRINT_EXTRA: f32 = 0.55;
pub const CAM_LOCK_MIX: f32 = 0.32;
pub const CAM_SHOULDER: f32 = 0.42;
pub const CAM_SHAKE_TIME: f32 = 0.16;
pub const CAM_BLOCK_RADIUS: f32 = 0.95;
pub const LOCK_RANGE: f32 = 16.0;

pub fn dummy_light_windup() -> u8 {
    12
}

pub fn dummy_heavy_windup() -> u8 {
    18
}

pub fn dummy_cooldown_ticks() -> u8 {
    22
}

pub fn dummy_windup_ticks(action: u8) -> u8 {
    if action == ACTION_HEAVY {
        dummy_heavy_windup()
    } else {
        dummy_light_windup()
    }
}

/// Club raise just committed. None while the same windup is still pending.
pub fn dummy_telegraph_started(
    prev_action: u8,
    prev_pending: bool,
    new_action: u8,
    new_pending: bool,
) -> Option<u8> {
    if !new_pending {
        return None;
    }
    if new_action != ACTION_LIGHT && new_action != ACTION_HEAVY {
        return None;
    }
    if prev_pending && prev_action == new_action {
        return None;
    }
    Some(new_action)
}

/// Body just dropped. True only on the alive true→false edge so respawn and
/// staying dead do not re-fire.
pub fn death_started(prev_alive: bool, new_alive: bool) -> bool {
    prev_alive && !new_alive
}

/// Body just stood up. True only on the alive false→true edge so idle living
/// and the death fall do not re-fire.
pub fn life_started(prev_alive: bool, new_alive: bool) -> bool {
    !prev_alive && new_alive
}

/// +1 chase, 0 hold the pocket, -1 step back. Dummy should not glue to the player.
pub fn dummy_move_dir(dist: f32) -> f32 {
    if dist > DUMMY_APPROACH {
        1.0
    } else if dist < DUMMY_KEEP_OUT {
        -1.0
    } else {
        0.0
    }
}

/// Sideways shuffle while holding the pocket. Zero when chasing, backing off, or about to swing.
pub fn dummy_strafe_dir(pocket: f32, cooldown: u8) -> f32 {
    if pocket.abs() > 0.01 || cooldown == 0 {
        return 0.0;
    }
    if (cooldown / 10) % 2 == 0 {
        1.0
    } else {
        -1.0
    }
}

pub fn camera_distance(drawn: bool, lock_on: bool, sprinting: bool) -> f32 {
    let base = if lock_on {
        CAM_LOCK
    } else if drawn {
        CAM_DRAWN
    } else {
        CAM_SHEATHED
    };
    if sprinting {
        base + CAM_SPRINT_EXTRA
    } else {
        base
    }
}

pub fn lock_focus_xz(px: f32, pz: f32, tx: f32, tz: f32, mix: f32) -> (f32, f32) {
    let mix = mix.clamp(0.0, 1.0);
    (px + (tx - px) * mix, pz + (tz - pz) * mix)
}

/// If the camera sits inside a body, push it to the shell so the fight stays on screen.
pub fn camera_push_out(
    cam_x: f32,
    cam_y: f32,
    cam_z: f32,
    ox: f32,
    oy: f32,
    oz: f32,
    radius: f32,
) -> (f32, f32, f32) {
    let dx = cam_x - ox;
    let dy = cam_y - oy;
    let dz = cam_z - oz;
    let d = (dx * dx + dy * dy + dz * dz).sqrt();
    if d >= radius || d < 1e-4 {
        return (cam_x, cam_y, cam_z);
    }
    let s = radius / d;
    (ox + dx * s, oy + dy * s, oz + dz * s)
}

/// Camera pitch that looks at a lock target. Negative looks down (Bevy YXZ).
pub fn lock_aim_pitch(from_y: f32, to_y: f32, dist_xz: f32) -> f32 {
    (to_y - from_y).atan2(dist_xz.max(0.2)).clamp(-1.15, 0.45)
}

pub fn camera_shake_amp(t: f32) -> f32 {
    let a = (t / CAM_SHAKE_TIME).clamp(0.0, 1.0);
    a * a * 0.16
}

pub fn hp_regen_ok(action: u8) -> bool {
    !action_busy(action) && action != ACTION_BLOCK
}

/// Stamina waits for the swing, roll, block, or stun to finish — Souls pacing.
pub fn stamina_regen_ok(action: u8) -> bool {
    hp_regen_ok(action)
}

/// Knockback while a committed strike is in the air. Enough to read, not enough to miss.
pub const HYPERARMOR_KNOCKBACK: f32 = 0.2;

/// Dummy windups and player heavies don't flinch. Lights still do.
pub fn hyperarmor(action: u8, pending_hit: bool, dummy: bool) -> bool {
    if !pending_hit {
        return false;
    }
    if dummy {
        action == ACTION_LIGHT || action == ACTION_HEAVY
    } else {
        action == ACTION_HEAVY
    }
}

/// Hold the club at the bottom of the slam so the impact reads.
pub fn dummy_recover_ticks() -> u8 {
    8
}

/// A committed swing keeps the facing it started with so you can step out of the cone.
pub fn swing_locks_facing(action: u8, pending_hit: bool) -> bool {
    pending_hit && (action == ACTION_LIGHT || action == ACTION_HEAVY)
}

pub fn merge_input_buttons(held: u32, latched: u32) -> u32 {
    held | latched
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActionStart {
    pub action: u8,
    pub ticks: u8,
    pub stamina: f32,
    pub pending_hit: bool,
    pub loadout: u8,
}

/// Same rules the module uses to start a drawn-weapon action.
pub fn start_drawn_action(
    current_action: u8,
    current_loadout: u8,
    wanted_loadout: u8,
    stamina: f32,
    buttons: u32,
) -> Option<ActionStart> {
    if action_busy(current_action) {
        return None;
    }
    if wanted_loadout != current_loadout {
        return Some(ActionStart {
            action: ACTION_SWAP,
            ticks: swap_ticks(),
            stamina,
            pending_hit: false,
            loadout: wanted_loadout,
        });
    }
    if (buttons & BTN_DODGE) != 0 {
        let cost = loadout(current_loadout).dodge_stamina;
        if stamina_ok(stamina, cost) {
            return Some(ActionStart {
                action: ACTION_DODGE,
                ticks: dodge_ticks_for(current_loadout),
                stamina: stamina - cost,
                pending_hit: false,
                loadout: current_loadout,
            });
        }
        return None;
    }
    if (buttons & BTN_BLOCK) != 0 && current_loadout == LOADOUT_SWORD {
        if stamina > 0.5 {
            return Some(ActionStart {
                action: ACTION_BLOCK,
                ticks: 1,
                stamina,
                pending_hit: false,
                loadout: current_loadout,
            });
        }
        return None;
    }
    let def = loadout(current_loadout);
    if (buttons & BTN_HEAVY) != 0 && stamina_ok(stamina, def.heavy_stamina) {
        return Some(ActionStart {
            action: ACTION_HEAVY,
            ticks: def.heavy_windup_ticks,
            stamina: stamina - def.heavy_stamina,
            pending_hit: true,
            loadout: current_loadout,
        });
    }
    if (buttons & BTN_LIGHT) != 0 && stamina_ok(stamina, def.light_stamina) {
        return Some(ActionStart {
            action: ACTION_LIGHT,
            ticks: def.light_windup_ticks,
            stamina: stamina - def.light_stamina,
            pending_hit: true,
            loadout: current_loadout,
        });
    }
    None
}

pub fn start_gather_action(
    current_action: u8,
    buttons: u32,
    in_range: bool,
) -> Option<ActionStart> {
    if action_busy(current_action) {
        return None;
    }
    if (buttons & BTN_INTERACT) == 0 || !in_range {
        return None;
    }
    Some(ActionStart {
        action: ACTION_GATHER,
        ticks: gather_ticks(),
        stamina: 0.0,
        pending_hit: false,
        loadout: 0,
    })
}

pub fn dodge_dir(dir_x: f32, dir_z: f32) -> (f32, f32) {
    if dir_x.abs() + dir_z.abs() < 0.1 {
        (0.0, 1.0)
    } else {
        (dir_x, dir_z)
    }
}

pub fn dodge_burst_dt() -> f32 {
    TICK_DT * dodge_ticks() as f32 * 0.35
}

/// Forward step on a melee swing. Heavies commit further; bows and staffs stay planted.
pub fn melee_lunge_dt(action: u8, is_projectile: bool) -> f32 {
    if is_projectile {
        return 0.0;
    }
    match action {
        ACTION_HEAVY => TICK_DT * 3.5,
        ACTION_LIGHT => TICK_DT * 2.0,
        _ => 0.0,
    }
}

/// Ticks left on a predicted attack when the projectile/melee should release.
pub fn predicted_release_ticks(action: u8, loadout_id: u8) -> u8 {
    let def = loadout(loadout_id);
    match action {
        ACTION_HEAVY => def.heavy_recover_ticks,
        ACTION_LIGHT => def.light_recover_ticks,
        _ => 0,
    }
}

/// Client-side busy window: attacks include recover so the chop plays through.
pub fn predicted_busy_ticks(start: &ActionStart) -> u8 {
    let def = loadout(start.loadout);
    match start.action {
        ACTION_LIGHT => start.ticks.saturating_add(def.light_recover_ticks),
        ACTION_HEAVY => start.ticks.saturating_add(def.heavy_recover_ticks),
        _ => start.ticks,
    }
}

/// 0 at windup start, 1 at impact, >1 through recover.
pub fn swing_progress(action: u8, ticks_left: f32, loadout_id: u8) -> f32 {
    let def = loadout(loadout_id);
    let (windup, recover) = match action {
        ACTION_HEAVY => (
            def.heavy_windup_ticks as f32,
            def.heavy_recover_ticks as f32,
        ),
        ACTION_LIGHT => (
            def.light_windup_ticks as f32,
            def.light_recover_ticks as f32,
        ),
        _ => return 0.0,
    };
    let total = windup + recover;
    if total <= 1e-3 {
        return 1.0;
    }
    (total - ticks_left.max(0.0)) / windup.max(1.0)
}

pub fn weapon_extra_rotation(action: u8, ticks_left: f32, loadout_id: u8) -> (f32, f32, f32) {
    match action {
        ACTION_BLOCK => (0.85, 0.15, -0.9),
        ACTION_DODGE => (0.35, 0.0, 0.4),
        ACTION_SWAP => {
            let total = swap_ticks() as f32;
            let t = if total > 1e-3 {
                (1.0 - ticks_left / total).clamp(0.0, 1.0)
            } else {
                1.0
            };
            // Holster dip, then the new weapon comes up.
            let pitch = if t < 0.5 {
                -t * 2.4
            } else {
                -1.2 + (t - 0.5) * 2.4
            };
            (pitch, 0.25, 0.15)
        }
        ACTION_HIT => (0.55, 0.0, 0.2),
        ACTION_GATHER => (0.4, 0.0, 0.15),
        ACTION_LIGHT | ACTION_HEAVY => {
            let p = swing_progress(action, ticks_left, loadout_id);
            if p < 1.0 {
                (-0.2 - p * 0.95, 0.0, p * 0.25)
            } else {
                let rec = (p - 1.0).clamp(0.0, 1.2);
                (-1.15 + rec * 1.85, 0.0, 0.25 + rec * 0.35)
            }
        }
        _ => (0.0, 0.0, 0.0),
    }
}

/// Dummy club pitch. Raises through most of the windup, slams in the last 30%.
pub fn dummy_club_pitch(action: u8, ticks_left: f32, windup_ticks: f32) -> f32 {
    if action == ACTION_HIT {
        return 0.4;
    }
    if action != ACTION_LIGHT && action != ACTION_HEAVY {
        return 0.0;
    }
    let t = if windup_ticks > 1e-3 {
        (1.0 - ticks_left / windup_ticks).clamp(0.0, 1.0)
    } else {
        1.0
    };
    if t < 0.7 {
        -(t / 0.7) * 1.25
    } else {
        -1.25 + ((t - 0.7) / 0.3) * 2.2
    }
}

pub fn action_label(action: u8) -> &'static str {
    match action {
        ACTION_LIGHT => "LIGHT",
        ACTION_HEAVY => "HEAVY",
        ACTION_DODGE => "DODGE",
        ACTION_BLOCK => "BLOCK",
        ACTION_SWAP => "SWAP",
        ACTION_HIT => "HIT",
        ACTION_DEAD => "DEAD",
        ACTION_SPAWN => "GRACE",
        ACTION_GATHER => "GATHER",
        _ => "",
    }
}
