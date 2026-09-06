use bevy::prelude::*;
use bevy_stdb::prelude::*;
use unbound_shared::{INPUT_SEND_HZ, PLAYER_HEIGHT, RECONCILE_BLEND, RECONCILE_SNAP, integrate};

use crate::camera::ControlState;
use crate::module_bindings::{
    Player, PlayerTableAccess, player_table::playerQueryTableAccess, set_input_reducer::set_input,
};
use crate::{StdbCmds, StdbConn, StdbSubs, SubKey};
use spacetimedb_sdk::Table;

#[derive(Component)]
pub struct LocalPlayer;

#[derive(Component)]
pub struct RemotePlayer;

#[derive(Component, Clone, Copy)]
pub struct NetworkedIdentity {
    pub identity: spacetimedb_sdk::Identity,
}

#[derive(Component)]
pub struct ServerPose {
    pub x: f32,
    pub z: f32,
    pub yaw: f32,
    pub drawn: bool,
}

pub fn connect(mut cmds: StdbCmds) {
    cmds.connect(StdbConnectOptions::default());
}

pub fn subscribe_on_connect(mut connected: ReadStdbConnectedMessage, mut subs: ResMut<StdbSubs>) {
    if connected.read().next().is_some() {
        subs.subscribe_query(SubKey::Players, |q| q.from.player());
    }
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
        spawn_pawn(
            &mut commands,
            &mut meshes,
            &mut materials,
            &player,
            player.identity == me,
        );
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
        let is_local = me == Some(msg.row.identity);
        spawn_pawn(
            &mut commands,
            &mut meshes,
            &mut materials,
            &msg.row,
            is_local,
        );
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
    time: Res<Time>,
) {
    for msg in updates.read() {
        for (id, mut pose, local, mut transform) in &mut players {
            if id.identity != msg.new.identity {
                continue;
            }
            pose.x = msg.new.x;
            pose.z = msg.new.z;
            pose.yaw = msg.new.yaw;
            pose.drawn = msg.new.drawn;
            if local.is_some() {
                let predicted = Vec3::new(transform.translation.x, 0.0, transform.translation.z);
                let server = Vec3::new(msg.new.x, 0.0, msg.new.z);
                let err = predicted.distance(server);
                if err > RECONCILE_SNAP {
                    transform.translation.x = msg.new.x;
                    transform.translation.z = msg.new.z;
                } else if err > 0.02 {
                    let t = (RECONCILE_BLEND * time.delta_secs()).min(1.0);
                    transform.translation.x = predicted.x.lerp(server.x, t);
                    transform.translation.z = predicted.z.lerp(server.z, t);
                }
            }
        }
    }
}

pub fn apply_player_deletes(
    mut commands: Commands,
    mut deletes: ReadDeleteMessage<Player>,
    players: Query<(Entity, &NetworkedIdentity)>,
) {
    for msg in deletes.read() {
        for (entity, id) in &players {
            if id.identity == msg.row.identity {
                commands.entity(entity).despawn();
            }
        }
    }
}

pub fn interpolate_remotes(
    time: Res<Time>,
    mut remotes: Query<(&ServerPose, &mut Transform), With<RemotePlayer>>,
) {
    let t = (10.0 * time.delta_secs()).min(1.0);
    for (pose, mut transform) in &mut remotes {
        let target = Vec3::new(pose.x, PLAYER_HEIGHT * 0.5, pose.z);
        transform.translation = transform.translation.lerp(target, t);
        transform.rotation = transform.rotation.slerp(Quat::from_rotation_y(pose.yaw), t);
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
    let (x, z) = integrate(
        transform.translation.x,
        transform.translation.z,
        control.yaw,
        control.dir_x,
        control.dir_z,
        time.delta_secs(),
    );
    transform.translation.x = x;
    transform.translation.z = z;
    transform.translation.y = PLAYER_HEIGHT * 0.5;
    transform.rotation = Quat::from_rotation_y(control.yaw);
}

pub fn send_input(time: Res<Time>, mut control: ResMut<ControlState>, conn: Option<Res<StdbConn>>) {
    let Some(conn) = conn else {
        return;
    };
    control.send_accum += time.delta_secs();
    let interval = 1.0 / INPUT_SEND_HZ;
    if control.send_accum < interval {
        return;
    }
    control.send_accum = 0.0;
    if let Err(err) =
        conn.reducers()
            .set_input(control.dir_x, control.dir_z, control.yaw, control.drawn)
    {
        warn!("set_input failed: {err}");
    }
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
    let mut entity = commands.spawn((
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
        ServerPose {
            x: player.x,
            z: player.z,
            yaw: player.yaw,
            drawn: player.drawn,
        },
    ));
    if is_local {
        entity.insert(LocalPlayer);
    } else {
        entity.insert(RemotePlayer);
    }
}
