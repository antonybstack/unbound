use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};

use crate::{LocalPlayer, MainCamera};
use unbound_shared::{
    camera_distance, camera_push_out, camera_shake_amp, lock_focus_xz, lock_reticle_lost_scale,
    lock_reticle_scale, ACTION_DEAD, ACTION_DODGE, ACTION_NONE, CAM_BLOCK_RADIUS, CAM_SHEATHED,
    CAM_SHOULDER, MAX_STAMINA, PLAYER_HEIGHT,
};

#[derive(Component, Clone, Copy)]
pub struct CamBlock {
    pub radius: f32,
}

impl Default for CamBlock {
    fn default() -> Self {
        Self {
            radius: CAM_BLOCK_RADIUS,
        }
    }
}

const LOOK_SENS: f32 = 0.004;

#[derive(Component)]
pub struct LockReticle;

#[derive(Resource)]
pub struct ControlState {
    pub drawn: bool,
    pub yaw: f32,
    pub pitch: f32,
    pub dir_x: f32,
    pub dir_z: f32,
    pub send_accum: f32,
    pub buttons: u32,
    pub latched: u32,
    pub loadout: u8,
    pub lock_on: bool,
    pub pred_action: u8,
    pub pred_ticks: f32,
    pub pred_stamina: f32,
    pub pred_loadout: u8,
    pub cam_dist: f32,
    pub shake: f32,
    pub lock_focus: Option<Vec3>,
    pub pred_shot: bool,
    pub hitstop: f32,
    pub sfx_dodge: bool,
    pub sfx_swing: u8,
    pub sfx_draw: i8,
    pub sfx_foot: u8,
    pub foot_accum: f32,
    pub sfx_spawn: bool,
    pub sfx_gather: bool,
    pub sfx_block: bool,
    pub sfx_dummy: u8,
    pub sfx_lock: i8,
    pub sfx_shot: u8,
    pub sfx_deplete: u8,
    pub sfx_respawn: u8,
    pub sfx_death: bool,
    pub sfx_rise: bool,
    pub sfx_level: bool,
    pub lock_pulse: f32,
    pub lock_lost: f32,
    pub lock_lost_at: Option<Vec3>,
    pub shot_kick: f32,
    pub pip_pulse: f32,
}

impl Default for ControlState {
    fn default() -> Self {
        Self {
            drawn: false,
            yaw: 0.0,
            pitch: -0.48,
            dir_x: 0.0,
            dir_z: 0.0,
            send_accum: 0.0,
            buttons: 0,
            latched: 0,
            loadout: 0,
            lock_on: false,
            pred_action: ACTION_NONE,
            pred_ticks: 0.0,
            pred_stamina: MAX_STAMINA,
            pred_loadout: 0,
            cam_dist: CAM_SHEATHED,
            shake: 0.0,
            lock_focus: None,
            pred_shot: false,
            hitstop: 0.0,
            sfx_dodge: false,
            sfx_swing: 0,
            sfx_draw: 0,
            sfx_foot: 0,
            foot_accum: 0.2,
            sfx_spawn: false,
            sfx_gather: false,
            sfx_block: false,
            sfx_dummy: 0,
            sfx_lock: 0,
            sfx_shot: 0,
            sfx_deplete: 0,
            sfx_respawn: 0,
            sfx_death: false,
            sfx_rise: false,
            sfx_level: false,
            lock_pulse: 0.0,
            lock_lost: 0.0,
            lock_lost_at: None,
            shot_kick: 0.0,
            pip_pulse: 0.0,
        }
    }
}

pub fn update_cursor(
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    control: Res<ControlState>,
    mut cursors: Query<&mut CursorOptions>,
) {
    let looking = control.drawn || buttons.pressed(MouseButton::Right);
    let free = keys.pressed(KeyCode::Escape) || (!looking && !control.drawn);

    for mut cursor in &mut cursors {
        cursor.grab_mode = if free {
            CursorGrabMode::None
        } else {
            CursorGrabMode::Locked
        };
        cursor.visible = free;
    }
}

pub fn update_camera(
    time: Res<Time>,
    mut motion: MessageReader<MouseMotion>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut control: ResMut<ControlState>,
    mut camera: Query<&mut Transform, With<MainCamera>>,
    local: Query<&Transform, (With<LocalPlayer>, Without<MainCamera>)>,
    blockers: Query<
        (&Transform, &CamBlock),
        (
            Without<MainCamera>,
            Without<LocalPlayer>,
            Without<LockReticle>,
        ),
    >,
    mut reticle: Query<
        &mut Transform,
        (With<LockReticle>, Without<MainCamera>, Without<LocalPlayer>),
    >,
) {
    let dt = time.delta_secs();
    let looking = control.drawn || buttons.pressed(MouseButton::Right);
    if looking {
        for ev in motion.read() {
            if !control.lock_on {
                control.yaw -= ev.delta.x * LOOK_SENS;
                control.pitch -= ev.delta.y * LOOK_SENS;
                control.pitch = control.pitch.clamp(-1.15, 0.45);
            }
        }
    } else {
        motion.clear();
    }

    let Ok(mut camera) = camera.single_mut() else {
        return;
    };
    let player = local
        .single()
        .ok()
        .map(|t| t.translation)
        .unwrap_or(Vec3::new(0.0, PLAYER_HEIGHT * 0.5, 0.0));
    let can_step = control.pred_action != ACTION_DODGE
        && control.pred_action != ACTION_DEAD
        && !unbound_shared::move_lock(control.pred_action);
    let sprinting = (control.buttons & unbound_shared::BTN_SPRINT) != 0
        && control.pred_stamina > 1.0
        && can_step;
    let dodging = control.pred_action == ACTION_DODGE;
    let want = camera_distance(control.drawn, control.lock_on, sprinting, dodging);
    let blend = (8.0 * dt).min(1.0);
    control.cam_dist += (want - control.cam_dist) * blend;
    control.shake = (control.shake - dt).max(0.0);
    control.lock_pulse = (control.lock_pulse - dt).max(0.0);
    control.lock_lost = (control.lock_lost - dt).max(0.0);
    if control.lock_lost <= 0.0 {
        control.lock_lost_at = None;
    }
    control.shot_kick = (control.shot_kick - dt).max(0.0);
    control.pip_pulse = (control.pip_pulse - dt).max(0.0);

    let mut focus = player;
    if control.lock_on {
        if let Some(target) = control.lock_focus {
            let (fx, fz) = lock_focus_xz(
                player.x,
                player.z,
                target.x,
                target.z,
                unbound_shared::CAM_LOCK_MIX,
            );
            focus = Vec3::new(fx, player.y + 0.12, fz);
        }
    }
    let amp = camera_shake_amp(control.shake);
    if amp > 0.0 {
        let w = control.shake * 70.0;
        focus += Vec3::new(w.sin() * amp, (w * 1.3).cos() * amp * 0.6, 0.0);
    }

    let rot = Quat::from_euler(EulerRot::YXZ, control.yaw, control.pitch, 0.0);
    let shoulder = if control.drawn {
        Vec3::new(CAM_SHOULDER, 0.0, 0.0)
    } else {
        Vec3::ZERO
    };
    let offset = rot * (Vec3::new(0.0, 0.0, control.cam_dist) + shoulder);
    camera.translation = focus + offset;
    for (body, block) in &blockers {
        let p = body.translation;
        let (x, y, z) = camera_push_out(
            camera.translation.x,
            camera.translation.y,
            camera.translation.z,
            p.x,
            p.y,
            p.z,
            block.radius,
        );
        camera.translation = Vec3::new(x, y, z);
    }
    camera.look_at(focus, Vec3::Y);

    if let Ok(mut ring) = reticle.single_mut() {
        let shrinking = control.lock_lost > 0.0;
        let target = control
            .lock_focus
            .or(control.lock_lost_at.filter(|_| shrinking));
        if let Some(target) = target {
            let spin = time.elapsed_secs() * 2.2;
            ring.translation = Vec3::new(target.x, target.y + 1.15, target.z);
            ring.rotation =
                Quat::from_rotation_x(std::f32::consts::FRAC_PI_2) * Quat::from_rotation_z(spin);
            let scale = if control.lock_focus.is_some() {
                lock_reticle_scale(control.lock_pulse)
            } else {
                lock_reticle_lost_scale(control.lock_lost)
            };
            ring.scale = Vec3::splat(scale);
        } else {
            ring.scale = Vec3::ZERO;
        }
    }
}
