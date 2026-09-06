//! Simulation constants and movement used by both the module and the client.
//! Keep this crate free of Bevy and SpacetimeDB so the same math runs in both places.

pub const MOVE_SPEED: f32 = 8.0;
pub const TICK_HZ: f32 = 30.0;
pub const TICK_DT: f32 = 1.0 / TICK_HZ;
pub const WORLD_HALF: f32 = 32.0;
pub const PLAYER_RADIUS: f32 = 0.4;
pub const PLAYER_HEIGHT: f32 = 1.6;
pub const RECONCILE_SNAP: f32 = 2.0;
pub const RECONCILE_BLEND: f32 = 12.0;
pub const INPUT_SEND_HZ: f32 = 20.0;

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

pub fn integrate(x: f32, z: f32, yaw: f32, dir_x: f32, dir_z: f32, dt: f32) -> (f32, f32) {
    let (dx, dz) = move_offset(yaw, dir_x, dir_z, dt, MOVE_SPEED);
    clamp_world(x + dx, z + dz)
}
