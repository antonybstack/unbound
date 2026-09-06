//! Simulation constants and rules used by both the module and the client.
//! Keep this crate free of Bevy and SpacetimeDB.

mod combat;

pub use combat::*;

pub const MOVE_SPEED: f32 = 8.0;
pub const SPRINT_SPEED: f32 = 11.0;
pub const DODGE_SPEED: f32 = 16.0;
pub const TICK_HZ: f32 = 30.0;
pub const TICK_DT: f32 = 1.0 / TICK_HZ;
pub const WORLD_HALF: f32 = 40.0;
pub const PLAYER_RADIUS: f32 = 0.4;
pub const PLAYER_HEIGHT: f32 = 1.6;
pub const RECONCILE_SNAP: f32 = 2.0;
pub const INPUT_SEND_HZ: f32 = 20.0;

pub const MAX_HP: f32 = 100.0;
pub const MAX_STAMINA: f32 = 100.0;
pub const STAMINA_REGEN_PER_SEC: f32 = 22.0;
pub const HP_REGEN_PER_SEC: f32 = 1.5;
pub const SPRINT_STAMINA_PER_SEC: f32 = 16.0;
pub const KNOCKBACK_LIGHT: f32 = 0.45;
pub const KNOCKBACK_HEAVY: f32 = 0.85;
pub const BODY_SEPARATION: f32 = PLAYER_RADIUS * 2.0 + 0.2;

pub const BTN_LIGHT: u32 = 1 << 0;
pub const BTN_HEAVY: u32 = 1 << 1;
pub const BTN_DODGE: u32 = 1 << 2;
pub const BTN_BLOCK: u32 = 1 << 3;
pub const BTN_SPRINT: u32 = 1 << 4;
pub const BTN_INTERACT: u32 = 1 << 5;

pub const ACTION_NONE: u8 = 0;
pub const ACTION_LIGHT: u8 = 1;
pub const ACTION_HEAVY: u8 = 2;
pub const ACTION_DODGE: u8 = 3;
pub const ACTION_BLOCK: u8 = 4;
pub const ACTION_SWAP: u8 = 5;
pub const ACTION_HIT: u8 = 6;
pub const ACTION_DEAD: u8 = 7;
pub const ACTION_GATHER: u8 = 8;
pub const ACTION_SPAWN: u8 = 9;

pub const SKILL_MELEE: u8 = 0;
pub const SKILL_RANGED: u8 = 1;
pub const SKILL_MAGIC: u8 = 2;
pub const SKILL_DEFENCE: u8 = 3;
pub const SKILL_HITPOINTS: u8 = 4;
pub const SKILL_GATHERING: u8 = 5;

pub const LOADOUT_SWORD: u8 = 0;
pub const LOADOUT_BOW: u8 = 1;
pub const LOADOUT_STAFF: u8 = 2;

pub const NODE_WOOD: u8 = 0;
pub const NODE_ORE: u8 = 1;
pub const GATHER_RANGE: f32 = 2.4;
pub const NODE_WOOD_XP: u64 = 10;
pub const NODE_ORE_XP: u64 = 14;

#[derive(Clone, Copy, Debug)]
pub struct NodeHome {
    pub id: u32,
    pub kind: u8,
    pub x: f32,
    pub z: f32,
    pub charges: u8,
}

/// Wood flanks the walk to the dummy; ore sits past it. All three read from spawn look.
pub const NODE_HOMES: [NodeHome; 3] = [
    NodeHome {
        id: 1,
        kind: NODE_WOOD,
        x: -5.2,
        z: -4.5,
        charges: 4,
    },
    NodeHome {
        id: 2,
        kind: NODE_WOOD,
        x: 5.2,
        z: -4.5,
        charges: 4,
    },
    NodeHome {
        id: 3,
        kind: NODE_ORE,
        x: 0.0,
        z: -15.0,
        charges: 3,
    },
];

/// Stable short tag from a 32-byte identity. Mixes several words so last-byte collisions
/// (common in local STDB identities) do not produce identical wanderer names.
pub fn wanderer_tag(bytes: &[u8; 32]) -> u16 {
    let a = u16::from_le_bytes([bytes[0], bytes[1]]);
    let b = u16::from_le_bytes([bytes[14], bytes[15]]);
    let c = u16::from_le_bytes([bytes[30], bytes[31]]);
    a ^ b ^ c
}

/// Yaw 0 looks down -Z (Bevy camera default).
pub fn yaw_forward(yaw: f32) -> (f32, f32) {
    (-yaw.sin(), -yaw.cos())
}

/// Camera-forward in XZY. Pitch 0 is level; negative looks down (Bevy YXZ).
pub fn aim_dir(yaw: f32, pitch: f32) -> (f32, f32, f32) {
    let (fx, fz) = yaw_forward(yaw);
    let cp = pitch.cos();
    (fx * cp, pitch.sin(), fz * cp)
}

pub const SHOT_SPAWN_Y: f32 = 1.15;
pub const SHOT_GROUND_Y: f32 = 0.08;
pub const SHOT_CEILING_Y: f32 = 12.0;
pub const BOW_GRAVITY: f32 = 2.2;
pub const STAFF_GRAVITY: f32 = 24.0;

pub fn shot_gravity(skill: u8) -> f32 {
    if skill == SKILL_MAGIC {
        STAFF_GRAVITY
    } else {
        BOW_GRAVITY
    }
}

pub fn shot_hits_height(y: f32) -> bool {
    y > 0.2 && y < PLAYER_HEIGHT + 0.35
}

pub fn yaw_right(yaw: f32) -> (f32, f32) {
    (yaw.cos(), -yaw.sin())
}

/// `dir_x` is strafe (A/D, -1..1), `dir_z` is forward (W/S, -1..1).
pub fn move_offset(yaw: f32, dir_x: f32, dir_z: f32, dt: f32, speed: f32) -> (f32, f32) {
    let (fx, fz) = yaw_forward(yaw);
    let (rx, rz) = yaw_right(yaw);
    let mut mx = rx * dir_x + fx * dir_z;
    let mut mz = rz * dir_x + fz * dir_z;
    let mag = (mx * mx + mz * mz).sqrt();
    if mag > 1e-4 {
        mx /= mag;
        mz /= mag;
        (mx * speed * dt, mz * speed * dt)
    } else {
        (0.0, 0.0)
    }
}

pub fn clamp_world(x: f32, z: f32) -> (f32, f32) {
    let limit = WORLD_HALF - PLAYER_RADIUS;
    (x.clamp(-limit, limit), z.clamp(-limit, limit))
}

pub fn integrate(
    x: f32,
    z: f32,
    yaw: f32,
    dir_x: f32,
    dir_z: f32,
    dt: f32,
    speed: f32,
) -> (f32, f32) {
    let (dx, dz) = move_offset(yaw, dir_x, dir_z, dt, speed);
    clamp_world(x + dx, z + dz)
}

pub fn dist_xz(ax: f32, az: f32, bx: f32, bz: f32) -> f32 {
    let dx = ax - bx;
    let dz = az - bz;
    (dx * dx + dz * dz).sqrt()
}

/// Push two discs apart so they sit at least `min_dist` from each other.
/// Equal share. Identical positions split along +X.
pub fn push_apart(ax: f32, az: f32, bx: f32, bz: f32, min_dist: f32) -> (f32, f32, f32, f32) {
    let dx = bx - ax;
    let dz = bz - az;
    let d = (dx * dx + dz * dz).sqrt();
    if d >= min_dist {
        return (ax, az, bx, bz);
    }
    let (nx, nz) = if d < 1e-4 {
        (1.0, 0.0)
    } else {
        (dx / d, dz / d)
    };
    let push = (min_dist - d) * 0.5;
    let (ax, az) = clamp_world(ax - nx * push, az - nz * push);
    let (bx, bz) = clamp_world(bx + nx * push, bz + nz * push);
    (ax, az, bx, bz)
}

pub fn knockback(x: f32, z: f32, from_x: f32, from_z: f32, amount: f32) -> (f32, f32) {
    let dx = x - from_x;
    let dz = z - from_z;
    let d = (dx * dx + dz * dz).sqrt();
    let (nx, nz) = if d < 1e-3 {
        (0.0, 1.0)
    } else {
        (dx / d, dz / d)
    };
    clamp_world(x + nx * amount, z + nz * amount)
}

pub fn facing_dot(yaw: f32, toward_x: f32, toward_z: f32) -> f32 {
    let (fx, fz) = yaw_forward(yaw);
    let mag = (toward_x * toward_x + toward_z * toward_z).sqrt();
    if mag < 1e-4 {
        return 1.0;
    }
    (toward_x / mag) * fx + (toward_z / mag) * fz
}

pub fn skill_level(xp: u64) -> u8 {
    // Fast OSRS-ish curve, capped at 50 for the yard.
    let lvl = (1.0 + (xp as f32 / 80.0).sqrt()).floor() as u32;
    lvl.clamp(1, 50) as u8
}

pub fn skill_xp_floor(level: u8) -> u64 {
    let n = u64::from(level.max(1).saturating_sub(1));
    80 * n * n
}

/// Level, XP into this level, XP needed for next, fraction 0..1.
pub fn skill_progress(xp: u64) -> (u8, u64, u64, f32) {
    let lvl = skill_level(xp);
    if lvl >= 50 {
        return (50, 0, 0, 1.0);
    }
    let lo = skill_xp_floor(lvl);
    let hi = skill_xp_floor(lvl.saturating_add(1));
    let span = hi.saturating_sub(lo).max(1);
    let into = xp.saturating_sub(lo).min(span);
    (lvl, into, span, into as f32 / span as f32)
}

pub fn skill_label(skill: u8) -> &'static str {
    match skill {
        SKILL_RANGED => "Ranged",
        SKILL_MAGIC => "Magic",
        SKILL_DEFENCE => "Defence",
        SKILL_HITPOINTS => "Hitpoints",
        SKILL_GATHERING => "Gathering",
        _ => "Melee",
    }
}

pub fn skill_for_loadout(loadout: u8) -> u8 {
    match loadout {
        LOADOUT_BOW => SKILL_RANGED,
        LOADOUT_STAFF => SKILL_MAGIC,
        _ => SKILL_MELEE,
    }
}

pub fn node_xp(kind: u8) -> u64 {
    if kind == NODE_ORE {
        NODE_ORE_XP
    } else {
        NODE_WOOD_XP
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yaw_zero_looks_down_neg_z() {
        let (x, z) = yaw_forward(0.0);
        assert!(x.abs() < 1e-5);
        assert!((z + 1.0).abs() < 1e-5);
    }

    #[test]
    fn integrate_walks_forward() {
        let (x, z) = integrate(0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 8.0);
        assert!(x.abs() < 1e-4);
        assert!((z + 8.0).abs() < 1e-3);
    }

    #[test]
    fn world_clamp_holds() {
        let (x, z) = clamp_world(100.0, -100.0);
        let limit = WORLD_HALF - PLAYER_RADIUS;
        assert!((x - limit).abs() < 1e-4);
        assert!((z + limit).abs() < 1e-4);
    }

    #[test]
    fn skill_curve_is_osrs_ish() {
        assert_eq!(skill_label(SKILL_MELEE), "Melee");
        assert_eq!(skill_label(SKILL_GATHERING), "Gathering");
        assert_eq!(skill_level(0), 1);
        assert_eq!(skill_level(80), 2);
        assert_eq!(skill_progress(0), (1, 0, 80, 0.0));
        let (l, into, span, frac) = skill_progress(40);
        assert_eq!((l, into, span), (1, 40, 80));
        assert!((frac - 0.5).abs() < 1e-4);
        assert_eq!(skill_progress(80), (2, 0, 240, 0.0));
        assert_eq!(skill_level(720), 4);
        assert_eq!(skill_level(80 * 49 * 49), 50);
        assert_eq!(skill_level(u64::MAX), 50);
    }

    #[test]
    fn scaled_damage_grows_with_level() {
        let base = 14.0;
        assert_eq!(scaled_damage(base, 1), 14.0);
        assert!(scaled_damage(base, 10) > scaled_damage(base, 1));
        assert!(scaled_damage(base, 50) > scaled_damage(base, 10));
    }

    #[test]
    fn push_apart_separates_overlap() {
        let (ax, az, bx, bz) = push_apart(0.0, 0.0, 0.1, 0.0, 1.0);
        assert!((dist_xz(ax, az, bx, bz) - 1.0).abs() < 1e-3);
    }

    #[test]
    fn push_apart_leaves_ok_pairs() {
        let (ax, az, bx, bz) = push_apart(0.0, 0.0, 5.0, 0.0, 1.0);
        assert_eq!((ax, az, bx, bz), (0.0, 0.0, 5.0, 0.0));
    }

    #[test]
    fn knockback_pushes_away() {
        let (x, z) = knockback(1.0, 0.0, 0.0, 0.0, 0.5);
        assert!((x - 1.5).abs() < 1e-3);
        assert!(z.abs() < 1e-3);
    }

    #[test]
    fn action_flags() {
        assert!(action_busy(ACTION_LIGHT));
        assert!(action_busy(ACTION_GATHER));
        assert!(action_busy(ACTION_DEAD));
        assert!(action_busy(ACTION_SPAWN));
        assert!(!action_busy(ACTION_NONE));
        assert!(move_lock(ACTION_HEAVY));
        assert!(move_lock(ACTION_GATHER));
        assert!(!move_lock(ACTION_LIGHT));
        assert!(!move_lock(ACTION_SPAWN));
        assert!(invulnerable(ACTION_DODGE, 4));
        assert!(!invulnerable(ACTION_DODGE, dodge_ticks()));
        assert!(!invulnerable(ACTION_DODGE, 0));
        assert!(!invulnerable(ACTION_LIGHT, 4));
        assert!(invulnerable_for(
            ACTION_SPAWN,
            spawn_protect_ticks(),
            LOADOUT_SWORD
        ));
        assert!(!invulnerable_for(ACTION_SPAWN, 0, LOADOUT_SWORD));
        assert!(spawn_protect_ticks() > dodge_ticks());
        assert!(blocking(ACTION_BLOCK));
    }

    #[test]
    fn aim_dir_level_matches_yaw_forward() {
        let (x, y, z) = aim_dir(0.0, 0.0);
        let (fx, fz) = yaw_forward(0.0);
        assert!((x - fx).abs() < 1e-5);
        assert!(y.abs() < 1e-5);
        assert!((z - fz).abs() < 1e-5);
    }

    #[test]
    fn aim_dir_looks_down_when_pitch_negative() {
        let (_x, y, _z) = aim_dir(0.0, -0.5);
        assert!(y < -0.4);
    }

    #[test]
    fn dodge_iframes_are_the_middle_of_the_roll() {
        let total = dodge_ticks();
        assert!(!dodge_iframe(total));
        assert!(dodge_iframe(total.saturating_sub(2)));
        assert!(dodge_iframe(2));
        assert!(!dodge_iframe(0));
        assert!(!dodge_iframe(1));
    }

    #[test]
    fn staff_rolls_lighter_than_sword() {
        assert!(dodge_ticks_for(LOADOUT_STAFF) < dodge_ticks_for(LOADOUT_SWORD));
        assert!(STAFF.dodge_stamina < SWORD.dodge_stamina);
        assert!(SWORD.dodge_stamina > BOW.dodge_stamina);
    }

    #[test]
    fn shots_miss_the_ground_and_sky() {
        assert!(shot_hits_height(1.0));
        assert!(!shot_hits_height(0.05));
        assert!(!shot_hits_height(8.0));
    }

    #[test]
    fn facing_requires_forward_cone() {
        assert!(facing_dot(0.0, 0.0, -1.0) > 0.9);
        assert!(facing_dot(0.0, 0.0, 1.0) < -0.9);
        assert!(facing_dot(0.0, 1.0, 0.0).abs() < 0.1);
    }

    #[test]
    fn merge_buttons_keeps_one_frame_press() {
        let held = BTN_SPRINT;
        let latched = BTN_LIGHT | BTN_DODGE;
        let merged = merge_input_buttons(held, latched);
        assert_eq!(merged & BTN_LIGHT, BTN_LIGHT);
        assert_eq!(merged & BTN_DODGE, BTN_DODGE);
        assert_eq!(merged & BTN_SPRINT, BTN_SPRINT);
    }

    #[test]
    fn light_attack_starts_when_free() {
        let start = start_drawn_action(ACTION_NONE, LOADOUT_SWORD, LOADOUT_SWORD, 100.0, BTN_LIGHT)
            .expect("light");
        assert_eq!(start.action, ACTION_LIGHT);
        assert!(start.pending_hit);
        assert!(start.stamina < 100.0);
        assert_eq!(start.ticks, SWORD.light_windup_ticks);
    }

    #[test]
    fn busy_actor_cannot_start() {
        assert!(
            start_drawn_action(ACTION_LIGHT, LOADOUT_SWORD, LOADOUT_SWORD, 100.0, BTN_LIGHT)
                .is_none()
        );
        assert!(
            start_drawn_action(ACTION_HIT, LOADOUT_SWORD, LOADOUT_SWORD, 100.0, BTN_DODGE)
                .is_none()
        );
    }

    #[test]
    fn swap_beats_attack() {
        let start =
            start_drawn_action(ACTION_NONE, LOADOUT_SWORD, LOADOUT_BOW, 100.0, BTN_LIGHT).unwrap();
        assert_eq!(start.action, ACTION_SWAP);
        assert_eq!(start.loadout, LOADOUT_BOW);
        assert!(!start.pending_hit);
        let mid = weapon_extra_rotation(ACTION_SWAP, swap_ticks() as f32 * 0.5, LOADOUT_BOW);
        let start_pose = weapon_extra_rotation(ACTION_SWAP, swap_ticks() as f32, LOADOUT_BOW);
        assert!(mid.0 < start_pose.0 - 0.3, "swap should dip the weapon");
    }

    #[test]
    fn melee_lunge_steps_in() {
        let light = melee_lunge_dt(ACTION_LIGHT, false);
        let heavy = melee_lunge_dt(ACTION_HEAVY, false);
        assert!(light > 0.0);
        assert!(heavy > light);
        assert_eq!(melee_lunge_dt(ACTION_LIGHT, true), 0.0);
        assert_eq!(melee_lunge_dt(ACTION_DODGE, false), 0.0);
        let (_, lz) = move_offset(0.0, 0.0, 1.0, light, MOVE_SPEED);
        let (_, hz) = move_offset(0.0, 0.0, 1.0, heavy, MOVE_SPEED);
        assert!(lz < -0.4);
        assert!(hz < lz);
        assert!(hz.abs() < DODGE_SPEED * dodge_burst_dt());
    }

    #[test]
    fn dodge_costs_stamina_and_defaults_forward() {
        let start = start_drawn_action(ACTION_NONE, LOADOUT_SWORD, LOADOUT_SWORD, 100.0, BTN_DODGE)
            .expect("dodge");
        assert_eq!(start.action, ACTION_DODGE);
        assert!((start.stamina - (100.0 - SWORD.dodge_stamina)).abs() < 1e-3);
        assert_eq!(dodge_dir(0.0, 0.0), (0.0, 1.0));
        assert_eq!(dodge_dir(-1.0, 0.0), (-1.0, 0.0));
    }

    #[test]
    fn empty_stamina_skips_light() {
        assert!(
            start_drawn_action(ACTION_NONE, LOADOUT_SWORD, LOADOUT_SWORD, 0.0, BTN_LIGHT).is_none()
        );
    }

    #[test]
    fn predicted_busy_covers_recover() {
        let start = start_drawn_action(ACTION_NONE, LOADOUT_SWORD, LOADOUT_SWORD, 100.0, BTN_HEAVY)
            .unwrap();
        assert_eq!(
            predicted_busy_ticks(&start),
            SWORD.heavy_windup_ticks + SWORD.heavy_recover_ticks
        );
        assert_eq!(
            predicted_release_ticks(ACTION_LIGHT, LOADOUT_BOW),
            BOW.light_recover_ticks
        );
        assert!(
            predicted_busy_ticks(
                &start_drawn_action(ACTION_NONE, LOADOUT_BOW, LOADOUT_BOW, 100.0, BTN_LIGHT)
                    .unwrap()
            ) > predicted_release_ticks(ACTION_LIGHT, LOADOUT_BOW)
        );
    }

    #[test]
    fn swing_progress_hits_one_at_impact() {
        let left = SWORD.heavy_recover_ticks as f32;
        let p = swing_progress(ACTION_HEAVY, left, LOADOUT_SWORD);
        assert!((p - 1.0).abs() < 0.05, "progress {p}");
    }

    #[test]
    fn dummy_club_raises_then_slams() {
        let windup = dummy_light_windup() as f32;
        let raised = dummy_club_pitch(ACTION_LIGHT, windup * 0.4, windup);
        let slam = dummy_club_pitch(ACTION_LIGHT, 0.0, windup);
        assert!(raised < -0.5, "raise {raised}");
        assert!(slam > 0.5, "slam {slam}");
        assert_eq!(dummy_club_pitch(ACTION_NONE, 0.0, windup), 0.0);
    }

    #[test]
    fn dummy_heavy_puffs_the_capsule() {
        let (hx, hy, hz) = dummy_body_scale(ACTION_HEAVY, true);
        assert!((hx - DUMMY_HEAVY_SCALE_XZ).abs() < 1e-5);
        assert!((hz - hx).abs() < 1e-5);
        assert!(hx >= 1.08 && hx <= 1.14);
        assert!((hy - 1.0).abs() < 0.06);
        let (lx, ly, lz) = dummy_body_scale(ACTION_LIGHT, true);
        assert!(lx > 1.0 && lx < hx);
        assert!((ly - 1.0).abs() < 1e-5);
        assert!((lz - lx).abs() < 1e-5);
        assert_eq!(dummy_body_scale(ACTION_HIT, true), (1.0, 1.0, 1.0));
        assert_eq!(dummy_body_scale(ACTION_NONE, true), (1.0, 1.0, 1.0));
        assert_eq!(
            dummy_body_scale(ACTION_HEAVY, false),
            (1.0, DUMMY_DEAD_SCALE_Y, 1.0)
        );
        assert!((DUMMY_DEAD_SCALE_Y - 0.22).abs() < 1e-5);
    }

    #[test]
    fn dummy_telegraph_fires_once_per_windup() {
        assert_eq!(
            dummy_telegraph_started(ACTION_NONE, false, ACTION_LIGHT, true),
            Some(ACTION_LIGHT)
        );
        assert_eq!(
            dummy_telegraph_started(ACTION_LIGHT, true, ACTION_LIGHT, true),
            None
        );
        assert_eq!(
            dummy_telegraph_started(ACTION_LIGHT, true, ACTION_HIT, false),
            None
        );
        assert_eq!(
            dummy_telegraph_started(ACTION_HIT, false, ACTION_HEAVY, true),
            Some(ACTION_HEAVY)
        );
        assert!(dummy_telegraph_started(ACTION_NONE, false, ACTION_LIGHT, false).is_none());
    }

    #[test]
    fn dummy_heavy_slam_fires_once_on_impact() {
        assert!(dummy_heavy_slammed(ACTION_HEAVY, true, ACTION_HIT, false));
        assert!(!dummy_heavy_slammed(ACTION_HEAVY, true, ACTION_HEAVY, true));
        assert!(!dummy_heavy_slammed(ACTION_NONE, false, ACTION_HEAVY, true));
        assert!(!dummy_heavy_slammed(ACTION_LIGHT, true, ACTION_HIT, false));
        assert!(!dummy_heavy_slammed(ACTION_HEAVY, true, ACTION_DEAD, false));
        assert!(!dummy_heavy_slammed(ACTION_HIT, false, ACTION_HIT, false));
        assert!(dummy_telegraph_started(ACTION_NONE, false, ACTION_HEAVY, true).is_some());
        assert!(!dummy_heavy_slammed(ACTION_NONE, false, ACTION_HEAVY, true));
    }

    #[test]
    fn death_thud_fires_once_when_alive_falls() {
        assert!(death_started(true, false));
        assert!(!death_started(false, false));
        assert!(!death_started(false, true));
        assert!(!death_started(true, true));
    }

    #[test]
    fn life_started_fires_once_when_dead_stands() {
        assert!(life_started(false, true));
        assert!(!life_started(true, true));
        assert!(!life_started(false, false));
        assert!(!life_started(true, false));
    }

    #[test]
    fn nameplate_fades_out_on_death_and_in_on_respawn() {
        assert!(NAMEPLATE_FADE_TIME >= 0.25 && NAMEPLATE_FADE_TIME <= 0.35);
        assert!((nameplate_alpha(true, 1.0, 0.1) - 1.0).abs() < 1e-4);
        assert!((nameplate_alpha(false, 0.0, 0.1) - 0.0).abs() < 1e-4);
        assert!((nameplate_alpha(false, 1.0, NAMEPLATE_FADE_TIME * 0.5) - 0.5).abs() < 1e-4);
        assert!((nameplate_alpha(true, 0.0, NAMEPLATE_FADE_TIME * 0.5) - 0.5).abs() < 1e-4);
        assert!((nameplate_alpha(false, 1.0, NAMEPLATE_FADE_TIME) - 0.0).abs() < 1e-4);
        assert!((nameplate_alpha(true, 0.0, NAMEPLATE_FADE_TIME) - 1.0).abs() < 1e-4);
        assert!((nameplate_alpha(false, 1.0, NAMEPLATE_FADE_TIME * 2.0) - 0.0).abs() < 1e-4);
        assert!((nameplate_alpha(true, 0.0, -0.1) - 0.0).abs() < 1e-4);
    }

    #[test]
    fn death_veil_eases_in_on_drop_and_out_on_respawn() {
        assert!(DEATH_VEIL_FADE_TIME >= 0.22 && DEATH_VEIL_FADE_TIME <= 0.35);
        assert!(DEATH_VEIL_BG_ALPHA >= 0.4 && DEATH_VEIL_BG_ALPHA <= 0.55);
        assert!((death_veil_mix(false, 0.0, 0.1) - 0.0).abs() < 1e-4);
        assert!((death_veil_mix(true, 1.0, 0.1) - 1.0).abs() < 1e-4);
        assert!((death_veil_mix(true, 0.0, DEATH_VEIL_FADE_TIME * 0.5) - 0.5).abs() < 1e-4);
        assert!((death_veil_mix(false, 1.0, DEATH_VEIL_FADE_TIME * 0.5) - 0.5).abs() < 1e-4);
        assert!((death_veil_mix(true, 0.0, DEATH_VEIL_FADE_TIME) - 1.0).abs() < 1e-4);
        assert!((death_veil_mix(false, 1.0, DEATH_VEIL_FADE_TIME) - 0.0).abs() < 1e-4);
        assert!((death_veil_mix(true, 0.0, DEATH_VEIL_FADE_TIME * 2.0) - 1.0).abs() < 1e-4);
        assert!((death_veil_mix(true, 0.0, -0.1) - 0.0).abs() < 1e-4);
        assert!((death_veil_bg_alpha(0.0) - 0.0).abs() < 1e-4);
        assert!((death_veil_bg_alpha(1.0) - DEATH_VEIL_BG_ALPHA).abs() < 1e-4);
        assert!((death_veil_bg_alpha(0.5) - DEATH_VEIL_BG_ALPHA * 0.5).abs() < 1e-4);
        assert!((death_veil_text_alpha(0.0) - 0.0).abs() < 1e-4);
        assert!((death_veil_text_alpha(1.0) - 1.0).abs() < 1e-4);
        assert!((death_veil_text_alpha(0.5) - 0.5).abs() < 1e-4);
        let mut mix = 0.0;
        mix = death_veil_mix(true, mix, DEATH_VEIL_FADE_TIME * 0.25);
        assert!(mix > 0.0 && mix < 1.0);
        assert!(death_veil_bg_alpha(mix) > 0.0 && death_veil_bg_alpha(mix) < DEATH_VEIL_BG_ALPHA);
        mix = death_veil_mix(false, 1.0, DEATH_VEIL_FADE_TIME);
        assert!((mix - 0.0).abs() < 1e-4);
        assert!((death_veil_bg_alpha(mix) - 0.0).abs() < 1e-4);
    }

    #[test]
    fn node_respawn_fires_once_when_charges_return() {
        assert!(node_respawned(0, 4));
        assert!(node_respawned(0, 3));
        assert!(!node_respawned(0, 0));
        assert!(!node_respawned(4, 4));
        assert!(!node_respawned(3, 3));
        assert!(!node_respawned(4, 3));
        assert!(!node_respawned(1, 0));
    }

    #[test]
    fn node_mesh_lerps_in_on_restore() {
        assert!(NODE_RESTORE_TIME >= 0.25 && NODE_RESTORE_TIME <= 0.4);
        assert!((node_restore_mix(0, 1.0, 0.1) - 0.0).abs() < 1e-4);
        assert!((node_restore_mix(0, 0.0, 0.1) - 0.0).abs() < 1e-4);
        assert!((node_restore_mix(4, 1.0, 0.1) - 1.0).abs() < 1e-4);
        assert!((node_restore_mix(4, 0.0, NODE_RESTORE_TIME * 0.5) - 0.5).abs() < 1e-4);
        assert!((node_restore_mix(3, 0.0, NODE_RESTORE_TIME) - 1.0).abs() < 1e-4);
        assert!((node_restore_mix(4, 0.0, NODE_RESTORE_TIME * 2.0) - 1.0).abs() < 1e-4);
        assert!((node_restore_mix(4, 0.0, -0.1) - 0.0).abs() < 1e-4);
        let (sx, sy, sz) = node_mesh_scale(0.0);
        assert!((sx - 1.0).abs() < 1e-4 && (sz - 1.0).abs() < 1e-4);
        assert!((sy - NODE_EMPTY_SCALE_Y).abs() < 1e-4);
        assert_eq!(node_mesh_scale(1.0), (1.0, 1.0, 1.0));
        let (_, mid, _) = node_mesh_scale(0.5);
        assert!((mid - (NODE_EMPTY_SCALE_Y + 0.5 * (1.0 - NODE_EMPTY_SCALE_Y))).abs() < 1e-4);
        let mut mix = 0.0;
        mix = node_restore_mix(0, mix, 0.1);
        assert!((mix - 0.0).abs() < 1e-4);
        assert_eq!(node_mesh_scale(mix), (1.0, NODE_EMPTY_SCALE_Y, 1.0));
    }

    #[test]
    fn gather_requires_range_and_button() {
        assert!(start_gather_action(ACTION_NONE, BTN_INTERACT, false).is_none());
        let start = start_gather_action(ACTION_NONE, BTN_INTERACT, true).unwrap();
        assert_eq!(start.action, ACTION_GATHER);
        assert_eq!(start.ticks, gather_ticks());
    }

    #[test]
    fn dummy_heavy_shakes_more_than_a_light() {
        let light = incoming_hit_shake(1, DUMMY_LIGHT_DAMAGE, true);
        let heavy = incoming_hit_shake(1, DUMMY_HEAVY_DAMAGE, true);
        let kill = incoming_hit_shake(2, DUMMY_HEAVY_DAMAGE, true);
        let brk = incoming_hit_shake(6, DUMMY_LIGHT_DAMAGE, true);
        let pvp = incoming_hit_shake(1, 28.0, false);
        assert!((light - CAM_SHAKE_HIT).abs() < 1e-5);
        assert!((heavy - CAM_SHAKE_HEAVY).abs() < 1e-5);
        assert!(heavy > light);
        assert!((kill - CAM_SHAKE_HEAVY).abs() < 1e-5);
        assert!((brk - CAM_SHAKE_BREAK).abs() < 1e-5);
        assert!((pvp - CAM_SHAKE_HIT).abs() < 1e-5);
        assert!(CAM_SHAKE_HEAVY >= 0.22 && CAM_SHAKE_HEAVY <= 0.28);
        assert!((CAM_SHAKE_HIT - CAM_SHAKE_TIME).abs() < 1e-5);
        assert!((camera_shake_amp(CAM_SHAKE_HEAVY) - camera_shake_amp(CAM_SHAKE_HIT)).abs() < 1e-5);
    }

    #[test]
    fn camera_closes_when_drawn() {
        assert!((camera_distance(false, false, false, false) - CAM_SHEATHED).abs() < 1e-4);
        assert!(
            camera_distance(true, false, false, false)
                < camera_distance(false, false, false, false)
        );
        assert!((camera_distance(true, true, false, false) - CAM_LOCK).abs() < 1e-4);
    }

    #[test]
    fn camera_opens_when_sprinting() {
        let walk = camera_distance(false, false, false, false);
        let run = camera_distance(false, false, true, false);
        let extra = run - walk;
        assert!(extra > 0.0);
        assert!((extra - CAM_SPRINT_EXTRA).abs() < 1e-4);
        assert!(extra >= 0.4 && extra <= 0.8);
        assert!(
            (camera_distance(true, false, true, false)
                - camera_distance(true, false, false, false)
                - CAM_SPRINT_EXTRA)
                .abs()
                < 1e-4
        );
        assert!(
            (camera_distance(false, true, true, false)
                - camera_distance(false, true, false, false)
                - CAM_SPRINT_EXTRA)
                .abs()
                < 1e-4
        );
    }

    #[test]
    fn camera_opens_when_dodging() {
        let walk = camera_distance(false, false, false, false);
        let roll = camera_distance(false, false, false, true);
        let extra = roll - walk;
        assert!(extra > 0.0);
        assert!((extra - CAM_DODGE_EXTRA).abs() < 1e-4);
        assert!(extra >= 0.35 && extra <= 0.55);
        assert!(
            (camera_distance(true, false, false, true)
                - camera_distance(true, false, false, false)
                - CAM_DODGE_EXTRA)
                .abs()
                < 1e-4
        );
        assert!(
            (camera_distance(false, true, false, true)
                - camera_distance(false, true, false, false)
                - CAM_DODGE_EXTRA)
                .abs()
                < 1e-4
        );
        assert!(
            (camera_distance(false, false, true, false)
                - camera_distance(false, false, false, false)
                - CAM_SPRINT_EXTRA)
                .abs()
                < 1e-4
        );
    }

    #[test]
    fn camera_eases_to_spawn_instead_of_snapping() {
        assert!((spawn_camera_blend(0.0) - 0.0).abs() < 1e-4);
        assert!((spawn_camera_blend(-0.1) - 0.0).abs() < 1e-4);
        let step = spawn_camera_blend(0.1);
        assert!((step - CAM_SPAWN_BLEND * 0.1).abs() < 1e-4);
        assert!(step > 0.0 && step < 1.0);
        assert!((spawn_camera_blend(1.0) - 1.0).abs() < 1e-4);
        assert!(CAM_SPAWN_BLEND >= 3.0 && CAM_SPAWN_BLEND <= 5.0);
        let (x, y, z) = spawn_camera_focus(0.0, 0.8, 0.0, 10.0, 0.8, 6.0, 0.0);
        assert!(x.abs() < 1e-4 && (y - 0.8).abs() < 1e-4 && z.abs() < 1e-4);
        let (x, y, z) = spawn_camera_focus(0.0, 0.8, 0.0, 10.0, 0.8, 6.0, 1.0);
        assert!((x - 10.0).abs() < 1e-4 && (y - 0.8).abs() < 1e-4 && (z - 6.0).abs() < 1e-4);
        let (x, _, z) = spawn_camera_focus(0.0, 0.8, 0.0, 10.0, 0.8, 6.0, 0.5);
        assert!((x - 5.0).abs() < 1e-4);
        assert!((z - 3.0).abs() < 1e-4);
        let b = spawn_camera_blend(1.0 / 60.0);
        let (_, _, z) = spawn_camera_focus(0.0, 0.8, -8.0, 0.0, 0.8, 6.0, b);
        assert!(z > -8.0 && z < 6.0);
        let mut z = -8.0;
        let dt = 1.0 / TICK_HZ;
        for _ in 0..spawn_protect_ticks() {
            let p = spawn_camera_focus(0.0, 0.8, z, 0.0, 0.8, 6.0, spawn_camera_blend(dt));
            z = p.2;
        }
        assert!((z - 6.0).abs() < 0.5);
    }

    #[test]
    fn lock_focus_sits_between() {
        let (x, z) = lock_focus_xz(0.0, 0.0, 10.0, 0.0, 0.3);
        assert!((x - 3.0).abs() < 1e-3);
        assert!(z.abs() < 1e-4);
        assert!(LOCK_RANGE > DUMMY_STRIKE_RANGE);
        assert!(LOCK_RANGE < WORLD_HALF);
    }

    #[test]
    fn lock_aim_pitch_looks_at_the_chest() {
        assert!(lock_aim_pitch(0.8, 0.8, 8.0).abs() < 0.05);
        assert!(lock_aim_pitch(1.15, 0.3, 6.0) < -0.1);
        assert!(lock_aim_pitch(0.8, 2.0, 4.0) > 0.1);
        let p = lock_aim_pitch(0.8, 20.0, 0.1);
        assert!(p <= 0.45);
    }

    #[test]
    fn lock_reticle_pops_when_tab_grabs() {
        let rest = lock_reticle_scale(0.0);
        let pop = lock_reticle_scale(LOCK_PULSE_TIME);
        assert!((rest - 1.0).abs() < 1e-4);
        assert!(pop > rest);
        assert!((pop - (1.0 + LOCK_PULSE_EXTRA)).abs() < 1e-4);
        assert!(pop >= 1.4 && pop <= 2.0);
        let mid = lock_reticle_scale(LOCK_PULSE_TIME * 0.5);
        assert!(mid > rest && mid < pop);
        assert!((lock_reticle_scale(-0.5) - 1.0).abs() < 1e-4);
        assert!((lock_reticle_scale(LOCK_PULSE_TIME * 2.0) - pop).abs() < 1e-4);
    }

    #[test]
    fn lock_reticle_shrinks_when_tab_drops() {
        assert!(LOCK_LOST_TIME >= 0.14 && LOCK_LOST_TIME <= 0.22);
        assert!((lock_reticle_lost_scale(0.0) - 0.0).abs() < 1e-4);
        assert!((lock_reticle_lost_scale(LOCK_LOST_TIME) - 1.0).abs() < 1e-4);
        let mid = lock_reticle_lost_scale(LOCK_LOST_TIME * 0.5);
        assert!((mid - 0.5).abs() < 1e-4);
        assert!(mid > 0.0 && mid < 1.0);
        assert!(lock_reticle_lost_scale(LOCK_LOST_TIME * 0.25) < mid);
        assert!((lock_reticle_lost_scale(-0.5) - 0.0).abs() < 1e-4);
        assert!((lock_reticle_lost_scale(LOCK_LOST_TIME * 2.0) - 1.0).abs() < 1e-4);
        let grab = lock_reticle_scale(0.0);
        assert!(lock_reticle_lost_scale(LOCK_LOST_TIME) <= grab);
        assert!(lock_reticle_lost_scale(0.0) < grab);
    }

    #[test]
    fn crosshair_kicks_when_a_bolt_leaves() {
        assert!(CROSSHAIR_KICK_TIME >= 0.12 && CROSSHAIR_KICK_TIME <= 0.18);
        let rest = crosshair_border_px(0.0);
        let kick = crosshair_border_px(CROSSHAIR_KICK_TIME);
        assert!((rest - CROSSHAIR_BORDER).abs() < 1e-4);
        assert!(kick > rest);
        let mid = crosshair_border_px(CROSSHAIR_KICK_TIME * 0.5);
        assert!(mid > rest && mid < kick);
        assert!((crosshair_border_px(-0.5) - rest).abs() < 1e-4);
        assert!((crosshair_border_px(CROSSHAIR_KICK_TIME * 2.0) - kick).abs() < 1e-4);
        let (_, _, _, sheathed) = crosshair_tint(false, CROSSHAIR_KICK_TIME);
        assert!(sheathed.abs() < 1e-4);
        let (rr, rg, rb, ra) = crosshair_tint(true, 0.0);
        let (kr, kg, kb, ka) = crosshair_tint(true, CROSSHAIR_KICK_TIME);
        assert!((ra - CROSSHAIR_ALPHA).abs() < 1e-4);
        assert!(ka > ra);
        assert!(kr >= rr && kg >= rg && kb >= rb);
        assert!((crosshair_kick(0.0)).abs() < 1e-4);
        assert!((crosshair_kick(CROSSHAIR_KICK_TIME) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn dummy_pip_pulses_when_a_swing_starts() {
        assert!(DUMMY_PIP_PULSE_TIME >= 0.16 && DUMMY_PIP_PULSE_TIME <= 0.24);
        let rest = dummy_pip_size(0.0);
        let pop = dummy_pip_size(DUMMY_PIP_PULSE_TIME);
        assert!((rest - DUMMY_PIP_SIZE).abs() < 1e-4);
        assert!(pop > rest);
        assert!((pop - (DUMMY_PIP_SIZE + DUMMY_PIP_PULSE_EXTRA)).abs() < 1e-4);
        assert!(pop >= 14.0 && pop <= 20.0);
        let mid = dummy_pip_size(DUMMY_PIP_PULSE_TIME * 0.5);
        assert!(mid > rest && mid < pop);
        assert!((dummy_pip_size(-0.5) - rest).abs() < 1e-4);
        assert!((dummy_pip_size(DUMMY_PIP_PULSE_TIME * 2.0) - pop).abs() < 1e-4);
        let (_, _, _, hidden) = dummy_pip_tint(false, DUMMY_PIP_PULSE_TIME);
        assert!(hidden.abs() < 1e-4);
        let (rr, rg, rb, ra) = dummy_pip_tint(true, 0.0);
        let (kr, kg, kb, ka) = dummy_pip_tint(true, DUMMY_PIP_PULSE_TIME);
        assert!((ra - DUMMY_PIP_ALPHA).abs() < 1e-4);
        assert!(ka > ra);
        assert!(kr >= rr && kg >= rg && kb >= rb);
        assert!((dummy_pip_pulse(0.0)).abs() < 1e-4);
        assert!((dummy_pip_pulse(DUMMY_PIP_PULSE_TIME) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn hotbar_flashes_when_a_bag_is_picked() {
        assert!(HOTBAR_FLASH_TIME >= 0.14 && HOTBAR_FLASH_TIME <= 0.22);
        let rest = hotbar_border_px(true, 0.0);
        let pop = hotbar_border_px(true, HOTBAR_FLASH_TIME);
        assert!((rest - HOTBAR_BORDER).abs() < 1e-4);
        assert!(pop > rest);
        assert!((pop - (HOTBAR_BORDER + HOTBAR_FLASH_EXTRA)).abs() < 1e-4);
        let mid = hotbar_border_px(true, HOTBAR_FLASH_TIME * 0.5);
        assert!(mid > rest && mid < pop);
        assert!((hotbar_border_px(true, -0.5) - rest).abs() < 1e-4);
        assert!((hotbar_border_px(true, HOTBAR_FLASH_TIME * 2.0) - pop).abs() < 1e-4);
        assert!((hotbar_border_px(false, HOTBAR_FLASH_TIME) - HOTBAR_BORDER).abs() < 1e-4);
        let (_, _, _, off) = hotbar_border_tint(0, false, HOTBAR_FLASH_TIME);
        assert!((off - 0.45).abs() < 1e-4);
        let (rr, rg, rb, ra) = hotbar_border_tint(0, true, 0.0);
        let (kr, kg, kb, ka) = hotbar_border_tint(0, true, HOTBAR_FLASH_TIME);
        assert!((ra - 1.0).abs() < 1e-4);
        assert!(ka >= ra);
        assert!(kr >= rr && kg >= rg && kb >= rb);
        let (br, bg, bb, ba) = hotbar_bg_tint(true, true, 0.0);
        let (fr, fg, fb, fa) = hotbar_bg_tint(true, true, HOTBAR_FLASH_TIME);
        assert!(fr >= br && fg >= bg && fb >= bb && fa >= ba);
        let off_bg = hotbar_bg_tint(false, false, HOTBAR_FLASH_TIME);
        let off_rest = hotbar_bg_tint(false, false, 0.0);
        assert!((off_bg.0 - off_rest.0).abs() < 1e-4);
        assert!((off_bg.3 - off_rest.3).abs() < 1e-4);
        assert!((hotbar_flash(0.0)).abs() < 1e-4);
        assert!((hotbar_flash(HOTBAR_FLASH_TIME) - 1.0).abs() < 1e-4);
        let bow = hotbar_border_tint(1, true, 0.0);
        let staff = hotbar_border_tint(2, true, 0.0);
        assert!(bow.0 > bow.1);
        assert!(staff.2 > staff.1);
    }

    #[test]
    fn hp_bar_flashes_white_when_hp_chips() {
        assert!((HP_FLASH_TIME - 0.16).abs() < 1e-4);
        assert!((hp_bar_flash(0.0)).abs() < 1e-4);
        assert!((hp_bar_flash(HP_FLASH_TIME) - 1.0).abs() < 1e-4);
        let mid = hp_bar_flash(HP_FLASH_TIME * 0.5);
        assert!(mid > 0.0 && mid < 1.0);
        assert!((hp_bar_flash(-0.5)).abs() < 1e-4);
        assert!((hp_bar_flash(HP_FLASH_TIME * 2.0) - 1.0).abs() < 1e-4);
        let (rr, rg, rb) = hp_bar_tint(0.0);
        assert!((rr - 0.78).abs() < 1e-4);
        assert!((rg - 0.20).abs() < 1e-4);
        assert!((rb - 0.16).abs() < 1e-4);
        let (kr, kg, kb) = hp_bar_tint(HP_FLASH_TIME);
        assert!(kr > rr && kg > rg && kb > rb);
        assert!(kr >= 0.98 && kg >= 0.90 && kb >= 0.86);
        let (mr, mg, mb) = hp_bar_tint(HP_FLASH_TIME * 0.5);
        assert!(mr > rr && mr < kr);
        assert!(mg > rg && mg < kg);
        assert!(mb > rb && mb < kb);
    }

    #[test]
    fn gather_hint_pulses_when_sheathed_range_enters() {
        assert!((GATHER_HINT_FLASH_TIME - 0.2).abs() < 1e-4);
        assert!(gather_hint_in_range(true, GATHER_RANGE));
        assert!(gather_hint_in_range(true, 0.0));
        assert!(!gather_hint_in_range(true, GATHER_RANGE + 0.01));
        assert!(!gather_hint_in_range(false, 0.0));
        assert!(gather_hint_entered(false, true));
        assert!(!gather_hint_entered(true, true));
        assert!(!gather_hint_entered(false, false));
        assert!(!gather_hint_entered(true, false));
        assert!((gather_hint_flash(0.0)).abs() < 1e-4);
        assert!((gather_hint_flash(GATHER_HINT_FLASH_TIME) - 1.0).abs() < 1e-4);
        let mid = gather_hint_flash(GATHER_HINT_FLASH_TIME * 0.5);
        assert!(mid > 0.0 && mid < 1.0);
        assert!((gather_hint_flash(-0.5)).abs() < 1e-4);
        assert!((gather_hint_flash(GATHER_HINT_FLASH_TIME * 2.0) - 1.0).abs() < 1e-4);
        let (_, _, _, hidden) = gather_hint_tint(false, GATHER_HINT_FLASH_TIME);
        assert!(hidden.abs() < 1e-4);
        let (rr, rg, rb, ra) = gather_hint_tint(true, 0.0);
        let (kr, kg, kb, ka) = gather_hint_tint(true, GATHER_HINT_FLASH_TIME);
        assert!(ka > ra);
        assert!(kr >= rr && kg >= rg && kb >= rb);
        let drawn = gather_hint_in_range(false, 1.0);
        assert!(!gather_hint_entered(false, drawn));
    }

    #[test]
    fn camera_push_out_leaves_the_shell() {
        let (x, y, z) = camera_push_out(0.1, 0.8, 0.0, 0.0, 0.8, 0.0, 1.0);
        let d = (x * x + (y - 0.8) * (y - 0.8) + z * z).sqrt();
        assert!((d - 1.0).abs() < 1e-3);
        let ok = camera_push_out(5.0, 0.8, 0.0, 0.0, 0.8, 0.0, 1.0);
        assert_eq!(ok, (5.0, 0.8, 0.0));
        assert!(CAM_BLOCK_RADIUS > PLAYER_RADIUS);
    }

    #[test]
    fn dummy_holds_the_pocket() {
        assert_eq!(dummy_move_dir(4.0), 1.0);
        assert_eq!(dummy_move_dir(2.0), 0.0);
        assert_eq!(dummy_move_dir(1.2), -1.0);
    }

    #[test]
    fn dummy_strafes_in_the_pocket() {
        assert_eq!(dummy_strafe_dir(1.0, 15), 0.0);
        assert_eq!(dummy_strafe_dir(-1.0, 15), 0.0);
        assert_eq!(dummy_strafe_dir(0.0, 0), 0.0);
        let right = dummy_strafe_dir(0.0, 5);
        let left = dummy_strafe_dir(0.0, 15);
        assert_eq!(right, 1.0);
        assert_eq!(left, -1.0);
        assert!(DUMMY_STRAFE_SPEED < DUMMY_CHASE_SPEED);
    }

    #[test]
    fn hp_does_not_regen_through_a_combo() {
        assert!(!hp_regen_ok(ACTION_HIT));
        assert!(!hp_regen_ok(ACTION_LIGHT));
        assert!(!hp_regen_ok(ACTION_BLOCK));
        assert!(hp_regen_ok(ACTION_NONE));
    }

    #[test]
    fn stamina_holds_through_a_swing() {
        assert!(!stamina_regen_ok(ACTION_LIGHT));
        assert!(!stamina_regen_ok(ACTION_HEAVY));
        assert!(!stamina_regen_ok(ACTION_HIT));
        assert!(!stamina_regen_ok(ACTION_DODGE));
        assert!(!stamina_regen_ok(ACTION_BLOCK));
        assert!(!stamina_regen_ok(ACTION_SWAP));
        assert!(stamina_regen_ok(ACTION_NONE));
    }

    #[test]
    fn committed_heavies_have_hyperarmor() {
        assert!(hyperarmor(ACTION_HEAVY, true, false));
        assert!(!hyperarmor(ACTION_HEAVY, false, false));
        assert!(!hyperarmor(ACTION_LIGHT, true, false));
        assert!(hyperarmor(ACTION_LIGHT, true, true));
        assert!(hyperarmor(ACTION_HEAVY, true, true));
        assert!(!hyperarmor(ACTION_NONE, false, true));
        assert!(!hyperarmor(ACTION_HIT, false, true));
        assert!(HYPERARMOR_KNOCKBACK < 0.5);
    }

    #[test]
    fn hyperarmor_flash_pings_besides_color() {
        assert!((hyperarmor_flash_mix(0.0)).abs() < 1e-4);
        assert!((hyperarmor_flash_mix(-0.5)).abs() < 1e-4);
        assert!((hyperarmor_flash_mix(HYPERARMOR_FLASH_TIME) - 1.0).abs() < 1e-4);
        assert!((hyperarmor_flash_mix(HYPERARMOR_FLASH_TIME * 2.0) - 1.0).abs() < 1e-4);
        let mid = hyperarmor_flash_mix(HYPERARMOR_FLASH_TIME * 0.5);
        assert!(mid > 0.0 && mid < 1.0);
        let rest = hyperarmor_flash_scale(0.0);
        let pop = hyperarmor_flash_scale(HYPERARMOR_FLASH_TIME);
        assert!((rest - 1.0).abs() < 1e-4);
        assert!(pop > rest);
        assert!((pop - (1.0 + HYPERARMOR_FLASH_SCALE)).abs() < 1e-4);
        assert!((hyperarmor_flash_emissive(0.0)).abs() < 1e-4);
        assert!(
            (hyperarmor_flash_emissive(HYPERARMOR_FLASH_TIME) - HYPERARMOR_FLASH_EMISSIVE).abs()
                < 1e-4
        );
        assert!(HYPERARMOR_FLASH_TIME >= 0.12 && HYPERARMOR_FLASH_TIME <= 0.24);
    }

    #[test]
    fn dummy_holds_the_slam() {
        assert!(dummy_recover_ticks() >= 6);
        assert!(dummy_club_pitch(ACTION_HIT, dummy_recover_ticks() as f32, 12.0) > 0.2);
    }

    #[test]
    fn committed_swing_does_not_track() {
        assert!(swing_locks_facing(ACTION_LIGHT, true));
        assert!(swing_locks_facing(ACTION_HEAVY, true));
        assert!(!swing_locks_facing(ACTION_LIGHT, false));
        assert!(!swing_locks_facing(ACTION_HEAVY, false));
        assert!(!swing_locks_facing(ACTION_NONE, false));
        assert!(!swing_locks_facing(ACTION_HIT, false));
        // yaw 0 faces -Z. A sidestep onto +X leaves the strike cone.
        assert!(facing_dot(0.0, 0.0, -2.0) > 0.9);
        assert!(facing_dot(0.0, 2.0, 0.0) < 0.2);
    }

    #[test]
    fn block_only_on_sword() {
        assert!(
            start_drawn_action(ACTION_NONE, LOADOUT_BOW, LOADOUT_BOW, 100.0, BTN_BLOCK).is_none()
        );
        let start = start_drawn_action(ACTION_NONE, LOADOUT_SWORD, LOADOUT_SWORD, 100.0, BTN_BLOCK)
            .unwrap();
        assert_eq!(start.action, ACTION_BLOCK);
        assert!(!action_busy(ACTION_BLOCK));
    }

    #[test]
    fn empty_stamina_drops_the_shield() {
        assert!(
            start_drawn_action(ACTION_NONE, LOADOUT_SWORD, LOADOUT_SWORD, 0.0, BTN_BLOCK).is_none()
        );
        assert!(
            start_drawn_action(ACTION_NONE, LOADOUT_SWORD, LOADOUT_SWORD, 0.4, BTN_BLOCK).is_none()
        );
        assert!(
            start_drawn_action(ACTION_NONE, LOADOUT_SWORD, LOADOUT_SWORD, 1.0, BTN_BLOCK).is_some()
        );
    }

    #[test]
    fn shield_covers_the_front_not_the_back() {
        // yaw 0 faces -Z.
        assert!(block_covers(0.0, 0.0, 0.0, 0.0, -4.0));
        assert!(!block_covers(0.0, 0.0, 0.0, 0.0, 4.0));
        assert!(!block_covers(0.0, 0.0, 0.0, 4.0, 0.0));
    }

    #[test]
    fn frontal_block_chips_and_holds() {
        let hit = resolve_guard(ACTION_BLOCK, 0.0, 0.0, 0.0, 0.0, -3.0, 40.0);
        assert_eq!(hit.result, GuardResult::Covered);
        assert!((hit.damage_mul - BLOCK_CHIP).abs() < 1e-4);
        assert!((hit.stamina_after - (40.0 - BLOCK_STAMINA_HIT)).abs() < 1e-3);
        assert!(!hit.hitstun);
        assert!(hit.knockback_mul < 0.5);
    }

    #[test]
    fn empty_guard_breaks() {
        let hit = resolve_guard(ACTION_BLOCK, 0.0, 0.0, 0.0, 0.0, -3.0, 8.0);
        assert_eq!(hit.result, GuardResult::GuardBreak);
        assert_eq!(hit.stamina_after, 0.0);
        assert!(hit.hitstun);
        assert!(hit.knockback_mul > 1.0);
        assert!(guard_break_ticks() > hitstun_ticks());
    }

    #[test]
    fn block_from_behind_is_open() {
        let hit = resolve_guard(ACTION_BLOCK, 0.0, 0.0, 0.0, 0.0, 4.0, 80.0);
        assert_eq!(hit.result, GuardResult::OpenFlank);
        assert_eq!(hit.damage_mul, 1.0);
        assert!(hit.hitstun);
        assert!((hit.stamina_after - 80.0).abs() < 1e-3);
        let idle = resolve_guard(ACTION_NONE, 0.0, 0.0, 0.0, 0.0, -3.0, 80.0);
        assert_eq!(idle.result, GuardResult::Open);
    }

    #[test]
    fn wanderer_tag_mixes_more_than_tail_bytes() {
        let mut a = [0u8; 32];
        a[30] = 0x00;
        a[31] = 0xc2;
        let mut b = a;
        b[0] = 0x11;
        b[1] = 0x22;
        assert_ne!(wanderer_tag(&a), wanderer_tag(&b));
        assert_eq!(wanderer_tag(&a), wanderer_tag(&a));
    }

    #[test]
    fn node_homes_sit_in_spawn_look() {
        for node in NODE_HOMES {
            assert!(
                node.z < 0.0,
                "node {} should be down -Z from spawn",
                node.id
            );
            assert!(node.x.abs() < 12.0);
        }
    }

    #[test]
    fn dummy_ignores_sheathed_gatherers() {
        assert!(dummy_should_chase(true, 8.0));
        assert!(!dummy_should_chase(false, 8.0));
        assert!(dummy_should_chase(false, 2.0));
        assert!(dummy_should_chase(true, 2.0));
    }

    #[test]
    fn staff_bolts_drop_harder_than_arrows() {
        assert!(shot_gravity(SKILL_MAGIC) > shot_gravity(SKILL_RANGED) * 5.0);
        assert!(shot_gravity(SKILL_RANGED) > 0.0);
    }
}
