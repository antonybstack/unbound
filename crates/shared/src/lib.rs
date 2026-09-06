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
    fn gather_requires_range_and_button() {
        assert!(start_gather_action(ACTION_NONE, BTN_INTERACT, false).is_none());
        let start = start_gather_action(ACTION_NONE, BTN_INTERACT, true).unwrap();
        assert_eq!(start.action, ACTION_GATHER);
        assert_eq!(start.ticks, gather_ticks());
    }

    #[test]
    fn camera_closes_when_drawn() {
        assert!((camera_distance(false, false, false) - CAM_SHEATHED).abs() < 1e-4);
        assert!(camera_distance(true, false, false) < camera_distance(false, false, false));
        assert!((camera_distance(true, true, false) - CAM_LOCK).abs() < 1e-4);
        assert!(camera_distance(false, false, true) < camera_distance(false, false, false));
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
