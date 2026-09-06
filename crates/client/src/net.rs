use bevy::prelude::*;
use bevy_stdb::prelude::*;
use unbound_shared::{
    death_started, integrate, life_started, merge_input_buttons, move_lock, ACTION_DEAD,
    ACTION_DODGE, ACTION_HIT, ACTION_NONE, ACTION_SPAWN, BTN_SPRINT, DODGE_SPEED, INPUT_SEND_HZ,
    MOVE_SPEED, PLAYER_HEIGHT, RECONCILE_SNAP, SPRINT_SPEED, TICK_HZ,
};

use crate::camera::ControlState;
use crate::module_bindings::{set_input_reducer::set_input, Player, PlayerTableAccess};
use crate::{StdbCmds, StdbConn};
use spacetimedb_sdk::Table;

#[derive(Component)]
pub struct LocalPlayer;

#[derive(Component)]
pub struct RemotePlayer;

#[derive(Component)]
pub struct RemoteStep {
    pub accum: f32,
    pub last_x: f32,
    pub last_z: f32,
    pub speed: f32,
    pub since: f32,
    pub last_action: u8,
    pub last_pending: bool,
    pub last_drawn: bool,
}

impl RemoteStep {
    pub fn new(x: f32, z: f32, action: u8, pending: bool, drawn: bool) -> Self {
        Self {
            accum: 0.18,
            last_x: x,
            last_z: z,
            speed: 0.0,
            since: 0.0,
            last_action: action,
            last_pending: pending,
            last_drawn: drawn,
        }
    }
}

#[derive(Component, Clone, Copy)]
pub struct NetworkedIdentity {
    pub identity: spacetimedb_sdk::Identity,
}

#[derive(Component, Clone)]
#[allow(dead_code)]
pub struct ServerPose {
    pub x: f32,
    pub z: f32,
    pub yaw: f32,
    pub drawn: bool,
    pub loadout: u8,
    pub hp: f32,
    pub stamina: f32,
    pub alive: bool,
    pub action: u8,
    pub action_ticks: f32,
    pub pending_hit: bool,
    pub name: String,
}

impl ServerPose {
    pub fn from_player(player: &Player) -> Self {
        Self {
            x: player.x,
            z: player.z,
            yaw: player.yaw,
            drawn: player.drawn,
            loadout: player.loadout,
            hp: player.hp,
            stamina: player.stamina,
            alive: player.alive,
            action: player.action,
            action_ticks: player.action_ticks as f32,
            pending_hit: player.pending_hit,
            name: player.name.clone(),
        }
    }

    pub fn apply_row(&mut self, player: &Player) {
        *self = Self::from_player(player);
    }
}

pub fn connect(mut cmds: StdbCmds) {
    let options = match crate::persist::load_token() {
        Some(token) => StdbConnectOptions::from_token(token),
        None => StdbConnectOptions::default(),
    };
    cmds.connect(options);
}

pub fn bind_local_player(
    mut commands: Commands,
    conn: Option<Res<StdbConn>>,
    mut local: Query<(Entity, &mut Transform, Option<&NetworkedIdentity>), With<LocalPlayer>>,
    mut control: ResMut<ControlState>,
) {
    let Some(conn) = conn else {
        return;
    };
    let Some(me) = conn.try_identity() else {
        return;
    };
    let Ok((entity, mut transform, net)) = local.single_mut() else {
        return;
    };
    let Some(player) = conn.db().player().iter().find(|p| p.identity == me) else {
        return;
    };
    if net.is_some() {
        return;
    }
    transform.translation.x = player.x;
    transform.translation.z = player.z;
    transform.translation.y = PLAYER_HEIGHT * 0.5;
    transform.rotation = Quat::from_rotation_y(player.yaw);
    control.yaw = player.yaw;
    control.pred_stamina = player.stamina;
    control.pred_action = player.action;
    control.pred_ticks = player.action_ticks as f32;
    control.loadout = player.loadout;
    control.pred_loadout = player.loadout;
    commands.entity(entity).insert((
        NetworkedIdentity {
            identity: player.identity,
        },
        ServerPose::from_player(&player),
    ));
}

pub fn spawn_pawns_from_cache(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    conn: Option<Res<StdbConn>>,
    existing: Query<&NetworkedIdentity>,
) {
    let Some(conn) = conn else {
        return;
    };
    let Some(me) = conn.try_identity() else {
        return;
    };
    for player in conn.db().player().iter() {
        if existing.iter().any(|id| id.identity == player.identity) {
            continue;
        }
        if player.identity == me {
            continue;
        }
        spawn_pawn(&mut commands, &mut meshes, &mut materials, &player, false);
    }
}

pub fn apply_player_inserts(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut inserts: ReadInsertMessage<Player>,
    conn: Option<Res<StdbConn>>,
    existing: Query<&NetworkedIdentity>,
) {
    let me = conn.and_then(|c| c.try_identity());
    for msg in inserts.read() {
        if existing.iter().any(|id| id.identity == msg.row.identity) {
            continue;
        }
        if me == Some(msg.row.identity) {
            continue;
        }
        spawn_pawn(&mut commands, &mut meshes, &mut materials, &msg.row, false);
    }
}

pub fn apply_player_updates(
    mut updates: ReadUpdateMessage<Player>,
    mut players: Query<(
        &NetworkedIdentity,
        &mut ServerPose,
        Option<&LocalPlayer>,
        &mut Transform,
    )>,
    mut control: ResMut<ControlState>,
) {
    for msg in updates.read() {
        for (id, mut pose, local, mut transform) in &mut players {
            if id.identity != msg.new.identity {
                continue;
            }
            if death_started(pose.alive, msg.new.alive) {
                control.sfx_death = true;
            }
            if local.is_none() && life_started(pose.alive, msg.new.alive) {
                control.sfx_rise = true;
            }
            pose.apply_row(&msg.new);
            if local.is_some() {
                // Keep prediction authoritative unless we have clearly desynced.
                // Blending toward a delayed snapshot every tick makes WASD feel sticky.
                let predicted = Vec3::new(transform.translation.x, 0.0, transform.translation.z);
                let server = Vec3::new(msg.new.x, 0.0, msg.new.z);
                let dead = !msg.new.alive || msg.new.action == ACTION_DEAD;
                let spawning = msg.new.action == ACTION_SPAWN;
                if spawning && control.pred_action != ACTION_SPAWN {
                    control.sfx_spawn = true;
                }
                if dead || spawning || predicted.distance(server) > RECONCILE_SNAP {
                    transform.translation.x = msg.new.x;
                    transform.translation.z = msg.new.z;
                }
                if msg.new.action == ACTION_HIT
                    || msg.new.action == ACTION_DEAD
                    || msg.new.action == ACTION_SPAWN
                {
                    control.pred_action = msg.new.action;
                    control.pred_ticks = msg.new.action_ticks as f32;
                } else if msg.new.action == ACTION_NONE
                    && (control.pred_action == ACTION_HIT
                        || control.pred_action == ACTION_DEAD
                        || control.pred_action == ACTION_SPAWN)
                {
                    control.pred_action = ACTION_NONE;
                    control.pred_ticks = 0.0;
                }
                if control.pred_action == ACTION_NONE || msg.new.stamina < control.pred_stamina {
                    control.pred_stamina = msg.new.stamina;
                }
            }
        }
    }
}

pub fn apply_player_deletes(
    mut commands: Commands,
    mut deletes: ReadDeleteMessage<Player>,
    players: Query<(Entity, &NetworkedIdentity, Option<&LocalPlayer>)>,
) {
    for msg in deletes.read() {
        for (entity, id, local) in &players {
            if id.identity == msg.row.identity && local.is_none() {
                commands.entity(entity).despawn();
            }
        }
    }
}

pub fn tick_remote_pose(time: Res<Time>, mut remotes: Query<&mut ServerPose, With<RemotePlayer>>) {
    let dt = time.delta_secs() * TICK_HZ;
    for mut pose in &mut remotes {
        if pose.action_ticks > 0.0 {
            pose.action_ticks = (pose.action_ticks - dt).max(0.0);
        }
    }
}

pub fn interpolate_remotes(
    time: Res<Time>,
    mut remotes: Query<(&ServerPose, &mut Transform), With<RemotePlayer>>,
) {
    let t = (10.0 * time.delta_secs()).min(1.0);
    for (pose, mut transform) in &mut remotes {
        let y = if pose.alive {
            PLAYER_HEIGHT * 0.5
        } else {
            0.22
        };
        let target = Vec3::new(pose.x, y, pose.z);
        transform.translation = transform.translation.lerp(target, t);
        transform.rotation = transform.rotation.slerp(Quat::from_rotation_y(pose.yaw), t);
        let scale = if pose.alive {
            Vec3::ONE
        } else {
            Vec3::new(1.0, 0.22, 1.0)
        };
        transform.scale = transform.scale.lerp(scale, t);
    }
}

pub fn predict_local(
    time: Res<Time>,
    control: Res<ControlState>,
    mut local: Query<&mut Transform, With<LocalPlayer>>,
) {
    let Ok(mut transform) = local.single_mut() else {
        return;
    };
    if move_lock(control.pred_action) {
        let dead = control.pred_action == ACTION_DEAD;
        transform.translation.y = if dead { 0.22 } else { PLAYER_HEIGHT * 0.5 };
        transform.rotation = Quat::from_rotation_y(control.yaw);
        transform.scale = if dead {
            Vec3::new(1.0, 0.22, 1.0)
        } else {
            Vec3::ONE
        };
        return;
    }
    if control.hitstop > 0.0 {
        transform.translation.y = PLAYER_HEIGHT * 0.5;
        transform.rotation = Quat::from_rotation_y(control.yaw);
        return;
    }
    let speed = if control.pred_action == ACTION_DODGE {
        DODGE_SPEED
    } else if (control.buttons & BTN_SPRINT) != 0 && control.pred_stamina > 1.0 {
        SPRINT_SPEED
    } else {
        MOVE_SPEED
    };
    let (x, z) = integrate(
        transform.translation.x,
        transform.translation.z,
        control.yaw,
        control.dir_x,
        control.dir_z,
        time.delta_secs(),
        speed,
    );
    transform.translation.x = x;
    transform.translation.z = z;
    let dead = control.pred_action == ACTION_DEAD;
    transform.translation.y = if dead { 0.22 } else { PLAYER_HEIGHT * 0.5 };
    transform.rotation = Quat::from_rotation_y(control.yaw);
    transform.scale = if dead {
        Vec3::new(1.0, 0.22, 1.0)
    } else {
        Vec3::ONE
    };
}

pub fn send_input(time: Res<Time>, mut control: ResMut<ControlState>, conn: Option<Res<StdbConn>>) {
    let Some(conn) = conn else {
        return;
    };
    control.send_accum += time.delta_secs();
    let interval = 1.0 / INPUT_SEND_HZ;
    let flush = control.latched != 0 || control.send_accum >= interval;
    if !flush {
        return;
    }
    control.send_accum = 0.0;
    let buttons = merge_input_buttons(control.buttons, control.latched);
    if let Err(err) = conn.reducers().set_input(
        control.dir_x,
        control.dir_z,
        control.yaw,
        control.pitch,
        control.drawn,
        buttons,
        control.loadout,
    ) {
        warn!("set_input failed: {err}");
        return;
    }
    control.latched = 0;
}

fn spawn_pawn(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    player: &Player,
    is_local: bool,
) {
    let color = if is_local {
        Color::srgb(0.82, 0.62, 0.28)
    } else {
        Color::srgb(0.35, 0.48, 0.62)
    };
    let parent = commands
        .spawn((
            Mesh3d(meshes.add(Capsule3d::new(0.35, 0.9))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                perceptual_roughness: 0.7,
                ..default()
            })),
            Transform::from_xyz(player.x, PLAYER_HEIGHT * 0.5, player.z)
                .with_rotation(Quat::from_rotation_y(player.yaw)),
            NetworkedIdentity {
                identity: player.identity,
            },
            ServerPose::from_player(player),
        ))
        .id();
    if is_local {
        commands.entity(parent).insert(LocalPlayer);
    } else {
        commands.entity(parent).insert((
            RemotePlayer,
            crate::camera::CamBlock::default(),
            RemoteStep::new(
                player.x,
                player.z,
                player.action,
                player.pending_hit,
                player.drawn,
            ),
        ));
        let bar = commands
            .spawn((
                Mesh3d(meshes.add(Cuboid::new(1.0, 0.1, 0.04))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.25, 0.55, 0.85),
                    unlit: true,
                    ..default()
                })),
                Transform::from_xyz(0.0, 1.2, 0.0),
                crate::combat::HpBar {
                    fade: if player.alive { 1.0 } else { 0.0 },
                    flash: 0.0,
                },
            ))
            .id();
        commands.entity(parent).add_child(bar);
    }
}
