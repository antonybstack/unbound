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

pub const BTN_LIGHT: u32 = 1 << 0;
pub const BTN_HEAVY: u32 = 1 << 1;
pub const BTN_DODGE: u32 = 1 << 2;
pub const BTN_BLOCK: u32 = 1 << 3;
pub const BTN_SPRINT: u32 = 1 << 4;

pub const ACTION_NONE: u8 = 0;
pub const ACTION_LIGHT: u8 = 1;
pub const ACTION_HEAVY: u8 = 2;
pub const ACTION_DODGE: u8 = 3;
pub const ACTION_BLOCK: u8 = 4;
pub const ACTION_SWAP: u8 = 5;
pub const ACTION_HIT: u8 = 6;
pub const ACTION_DEAD: u8 = 7;

pub const SKILL_MELEE: u8 = 0;
pub const SKILL_RANGED: u8 = 1;
pub const SKILL_MAGIC: u8 = 2;
pub const SKILL_DEFENCE: u8 = 3;
pub const SKILL_HITPOINTS: u8 = 4;

pub const LOADOUT_SWORD: u8 = 0;
pub const LOADOUT_BOW: u8 = 1;
pub const LOADOUT_STAFF: u8 = 2;

/// Yaw 0 looks down -Z (Bevy camera default).
pub fn yaw_forward(yaw: f32) -> (f32, f32) {
    (-yaw.sin(), -yaw.cos())
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

pub fn skill_level(xp: u64) -> u8 {
    // Fast OSRS-ish curve, capped at 50 for the yard.
    let lvl = (1.0 + (xp as f32 / 80.0).sqrt()).floor() as u32;
    lvl.clamp(1, 50) as u8
}

pub fn skill_for_loadout(loadout: u8) -> u8 {
    match loadout {
        LOADOUT_BOW => SKILL_RANGED,
        LOADOUT_STAFF => SKILL_MAGIC,
        _ => SKILL_MELEE,
    }
}
