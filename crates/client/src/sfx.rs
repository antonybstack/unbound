use bevy::prelude::*;
use bevy_stdb::prelude::*;
use unbound_shared::{ACTION_DEAD, ACTION_DODGE, move_lock};

use crate::StdbConn;
use crate::module_bindings::{CombatEvent, Projectile};
use crate::net::{LocalPlayer, RemotePlayer, RemoteStep, ServerPose};

#[derive(Resource)]
pub struct Sfx {
    pub hit: Handle<AudioSource>,
    pub heavy: Handle<AudioSource>,
    pub dodge: Handle<AudioSource>,
    pub block: Handle<AudioSource>,
    pub brk: Handle<AudioSource>,
    pub gather: Handle<AudioSource>,
    pub kill: Handle<AudioSource>,
}

pub fn load_sfx(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(Sfx {
        hit: assets.load("sfx/hit.wav"),
        heavy: assets.load("sfx/heavy.wav"),
        dodge: assets.load("sfx/dodge.wav"),
        block: assets.load("sfx/block.wav"),
        brk: assets.load("sfx/break.wav"),
        gather: assets.load("sfx/gather.wav"),
        kill: assets.load("sfx/kill.wav"),
    });
}

pub fn play_combat_sfx(
    mut commands: Commands,
    sfx: Option<Res<Sfx>>,
    mut events: ReadInsertMessage<CombatEvent>,
    conn: Option<Res<StdbConn>>,
) {
    let Some(sfx) = sfx else {
        return;
    };
    let me = conn.and_then(|c| c.try_identity());
    for msg in events.read() {
        let row = &msg.row;
        if me != Some(row.attacker) && me != Some(row.target) {
            continue;
        }
        let handle = match row.kind {
            1 if row.damage >= 20.0 => sfx.heavy.clone(),
            1 => sfx.hit.clone(),
            2 => sfx.kill.clone(),
            3 => sfx.block.clone(),
            4 => sfx.dodge.clone(),
            5 => sfx.gather.clone(),
            6 => sfx.brk.clone(),
            _ => continue,
        };
        commands.spawn((AudioPlayer::new(handle), PlaybackSettings::DESPAWN));
    }
}

pub fn play_local_sfx(
    mut commands: Commands,
    sfx: Option<Res<Sfx>>,
    mut control: ResMut<crate::camera::ControlState>,
) {
    let Some(sfx) = sfx else {
        return;
    };
    if control.sfx_dodge {
        commands.spawn((
            AudioPlayer::new(sfx.dodge.clone()),
            PlaybackSettings::DESPAWN,
        ));
        control.sfx_dodge = false;
    }
    if control.sfx_swing != 0 {
        let heavy = control.sfx_swing == 2;
        let handle = if heavy {
            sfx.heavy.clone()
        } else {
            sfx.dodge.clone()
        };
        let mut settings = PlaybackSettings::DESPAWN;
        settings.volume = bevy::audio::Volume::Linear(if heavy { 0.45 } else { 0.32 });
        settings.speed = if heavy { 0.85 } else { 1.35 };
        commands.spawn((AudioPlayer::new(handle), settings));
        control.sfx_swing = 0;
    }
    if control.sfx_draw != 0 {
        let draw = control.sfx_draw > 0;
        let mut settings = PlaybackSettings::DESPAWN;
        settings.volume = bevy::audio::Volume::Linear(0.4);
        settings.speed = if draw { 1.15 } else { 0.75 };
        commands.spawn((AudioPlayer::new(sfx.block.clone()), settings));
        control.sfx_draw = 0;
    }
    if control.sfx_foot != 0 {
        let sprint = control.sfx_foot == 2;
        play_foot(
            &mut commands,
            sfx.gather.clone(),
            sprint,
            if sprint { 0.18 } else { 0.12 },
        );
        control.sfx_foot = 0;
    }
    if control.sfx_spawn {
        let mut settings = PlaybackSettings::DESPAWN;
        settings.volume = bevy::audio::Volume::Linear(0.38);
        settings.speed = 1.55;
        commands.spawn((AudioPlayer::new(sfx.kill.clone()), settings));
        control.sfx_spawn = false;
    }
    if control.sfx_gather {
        let mut settings = PlaybackSettings::DESPAWN;
        settings.volume = bevy::audio::Volume::Linear(0.4);
        settings.speed = 0.9;
        commands.spawn((AudioPlayer::new(sfx.gather.clone()), settings));
        control.sfx_gather = false;
    }
    if control.sfx_block {
        // Predicted raise: air sweep. Rim clack stays on a blocked hit.
        let mut settings = PlaybackSettings::DESPAWN;
        settings.volume = bevy::audio::Volume::Linear(0.34);
        settings.speed = 0.92;
        commands.spawn((AudioPlayer::new(sfx.dodge.clone()), settings));
        control.sfx_block = false;
    }
    if control.sfx_dummy != 0 {
        let heavy = control.sfx_dummy == 2;
        let handle = if heavy {
            sfx.heavy.clone()
        } else {
            sfx.dodge.clone()
        };
        let mut settings = PlaybackSettings::DESPAWN;
        settings.volume = bevy::audio::Volume::Linear(if heavy { 0.42 } else { 0.28 });
        settings.speed = if heavy { 0.62 } else { 0.95 };
        commands.spawn((AudioPlayer::new(handle), settings));
        control.sfx_dummy = 0;
    }
    if control.sfx_lock != 0 {
        let acquire = control.sfx_lock > 0;
        let mut settings = PlaybackSettings::DESPAWN;
        settings.volume = bevy::audio::Volume::Linear(if acquire { 0.32 } else { 0.22 });
        settings.speed = if acquire { 1.65 } else { 0.7 };
        commands.spawn((AudioPlayer::new(sfx.block.clone()), settings));
        control.sfx_lock = 0;
    }
    if control.sfx_shot != 0 {
        play_shot_whoosh(&mut commands, &sfx, control.sfx_shot == 2);
        control.sfx_shot = 0;
    }
}

/// Server bolts from anyone else. Local predicted shots already whooshed at spawn.
pub fn play_remote_shot_sfx(
    mut commands: Commands,
    sfx: Option<Res<Sfx>>,
    mut inserts: ReadInsertMessage<Projectile>,
    conn: Option<Res<StdbConn>>,
) {
    let Some(sfx) = sfx else {
        return;
    };
    let Some(me) = conn.and_then(|c| c.try_identity()) else {
        return;
    };
    for msg in inserts.read() {
        if msg.row.owner == me {
            continue;
        }
        play_shot_whoosh(&mut commands, &sfx, msg.row.skill == 2);
    }
}

const REMOTE_WALK_SPEED: f32 = 1.5;
const REMOTE_SPRINT_SPEED: f32 = 6.5;
const FOOT_NEAR: f32 = 12.0;
const FOOT_MUTE: f32 = 22.0;

pub fn tick_remote_steps(
    mut commands: Commands,
    time: Res<Time>,
    sfx: Option<Res<Sfx>>,
    local: Query<&Transform, With<LocalPlayer>>,
    mut remotes: Query<(&ServerPose, &Transform, &mut RemoteStep), With<RemotePlayer>>,
) {
    let Some(sfx) = sfx else {
        return;
    };
    let Ok(local_tf) = local.single() else {
        return;
    };
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    let lx = local_tf.translation.x;
    let lz = local_tf.translation.z;
    for (pose, tf, mut step) in &mut remotes {
        step.since += dt;
        let dx = pose.x - step.last_x;
        let dz = pose.z - step.last_z;
        let moved = (dx * dx + dz * dz).sqrt();
        if moved > 0.02 {
            let elapsed = step.since.clamp(1.0 / 60.0, 0.2);
            step.speed = moved / elapsed;
            step.last_x = pose.x;
            step.last_z = pose.z;
            step.since = 0.0;
        } else if step.since > 0.12 {
            step.speed = 0.0;
        }

        let can_step = pose.alive
            && pose.action != ACTION_DODGE
            && pose.action != ACTION_DEAD
            && !move_lock(pose.action);
        let sprinting = can_step && step.speed > REMOTE_SPRINT_SPEED;
        let walking = can_step && !sprinting && step.speed > REMOTE_WALK_SPEED;
        let foot = if sprinting {
            step.accum += dt;
            if step.accum >= 0.28 {
                step.accum = 0.0;
                2
            } else {
                0
            }
        } else if walking {
            step.accum += dt;
            if step.accum >= 0.42 {
                step.accum = 0.0;
                1
            } else {
                0
            }
        } else {
            step.accum = 0.18;
            0
        };
        if foot == 0 {
            continue;
        }
        let rdx = tf.translation.x - lx;
        let rdz = tf.translation.z - lz;
        let range = (rdx * rdx + rdz * rdz).sqrt();
        if range >= FOOT_MUTE {
            continue;
        }
        let atten = if range <= FOOT_NEAR {
            1.0
        } else {
            1.0 - (range - FOOT_NEAR) / (FOOT_MUTE - FOOT_NEAR)
        };
        let sprint = foot == 2;
        let base = if sprint { 0.18 } else { 0.12 };
        play_foot(
            &mut commands,
            sfx.gather.clone(),
            sprint,
            base * 0.7 * atten,
        );
    }
}

fn play_shot_whoosh(commands: &mut Commands, sfx: &Sfx, staff: bool) {
    let handle = if staff {
        sfx.heavy.clone()
    } else {
        sfx.dodge.clone()
    };
    let mut settings = PlaybackSettings::DESPAWN;
    // Bow: higher/quieter. Staff: lower/thicker. Reuse swing samples, pitched.
    settings.volume = bevy::audio::Volume::Linear(if staff { 0.36 } else { 0.2 });
    settings.speed = if staff { 0.68 } else { 1.75 };
    commands.spawn((AudioPlayer::new(handle), settings));
}

fn play_foot(commands: &mut Commands, handle: Handle<AudioSource>, sprint: bool, volume: f32) {
    if volume <= 0.001 {
        return;
    }
    let mut settings = PlaybackSettings::DESPAWN;
    settings.volume = bevy::audio::Volume::Linear(volume);
    settings.speed = if sprint { 1.85 } else { 1.4 };
    commands.spawn((AudioPlayer::new(handle), settings));
}
