use bevy::prelude::*;
use bevy_stdb::prelude::*;

use crate::StdbConn;
use crate::module_bindings::CombatEvent;

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
        let mut settings = PlaybackSettings::DESPAWN;
        settings.volume = bevy::audio::Volume::Linear(if sprint { 0.18 } else { 0.12 });
        settings.speed = if sprint { 1.85 } else { 1.4 };
        commands.spawn((AudioPlayer::new(sfx.gather.clone()), settings));
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
        let mut settings = PlaybackSettings::DESPAWN;
        settings.volume = bevy::audio::Volume::Linear(0.35);
        settings.speed = 0.85;
        commands.spawn((AudioPlayer::new(sfx.block.clone()), settings));
        control.sfx_block = false;
    }
}
