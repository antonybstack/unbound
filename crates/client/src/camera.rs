use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};

use crate::{LocalPlayer, MainCamera};
use unbound_shared::PLAYER_HEIGHT;

const LOOK_SENS: f32 = 0.004;
const CAMERA_DISTANCE: f32 = 5.5;

#[derive(Resource)]
pub struct ControlState {
    pub drawn: bool,
    pub yaw: f32,
    pub pitch: f32,
    pub dir_x: f32,
    pub dir_z: f32,
    pub send_accum: f32,
    pub buttons: u32,
    pub loadout: u8,
    pub lock_on: bool,
}

impl Default for ControlState {
    fn default() -> Self {
        Self {
            drawn: false,
            yaw: 0.0,
            pitch: -0.35,
            dir_x: 0.0,
            dir_z: 0.0,
            send_accum: 0.0,
            buttons: 0,
            loadout: 0,
            lock_on: false,
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
    mut motion: MessageReader<MouseMotion>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut control: ResMut<ControlState>,
    mut camera: Query<&mut Transform, With<MainCamera>>,
    local: Query<&Transform, (With<LocalPlayer>, Without<MainCamera>)>,
) {
    let looking = control.drawn || buttons.pressed(MouseButton::Right);
    if looking {
        for ev in motion.read() {
            control.yaw -= ev.delta.x * LOOK_SENS;
            control.pitch -= ev.delta.y * LOOK_SENS;
            control.pitch = control.pitch.clamp(-1.15, 0.45);
        }
    } else {
        motion.clear();
    }

    let Ok(mut camera) = camera.single_mut() else {
        return;
    };
    let focus = local
        .single()
        .ok()
        .map(|t| t.translation)
        .unwrap_or(Vec3::new(0.0, PLAYER_HEIGHT * 0.5, 0.0));
    let rot = Quat::from_euler(EulerRot::YXZ, control.yaw, control.pitch, 0.0);
    let offset = rot * Vec3::new(0.0, 0.0, CAMERA_DISTANCE);
    camera.translation = focus + offset;
    camera.look_at(focus, Vec3::Y);
}
