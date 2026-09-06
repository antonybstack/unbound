use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};

use crate::{LocalPlayer, MainCamera};
use unbound_shared::{
    ACTION_NONE, CAM_SHEATHED, CAM_SHOULDER, MAX_STAMINA, PLAYER_HEIGHT, camera_distance,
    camera_shake_amp, lock_focus_xz,
};

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
    let sprinting =
        (control.buttons & unbound_shared::BTN_SPRINT) != 0 && control.pred_stamina > 1.0;
    let want = camera_distance(control.drawn, control.lock_on, sprinting);
    let blend = (8.0 * dt).min(1.0);
    control.cam_dist += (want - control.cam_dist) * blend;
    control.shake = (control.shake - dt).max(0.0);

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
    camera.look_at(focus, Vec3::Y);

    if let Ok(mut ring) = reticle.single_mut() {
        if let Some(target) = control.lock_focus {
            let spin = time.elapsed_secs() * 2.2;
            ring.translation = Vec3::new(target.x, target.y + 1.15, target.z);
            ring.rotation =
                Quat::from_rotation_x(std::f32::consts::FRAC_PI_2) * Quat::from_rotation_z(spin);
            ring.scale = Vec3::ONE;
        } else {
            ring.scale = Vec3::ZERO;
        }
    }
}
