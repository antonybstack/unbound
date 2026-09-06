mod camera;
mod module_bindings;
mod net;

use bevy::prelude::*;
use bevy_stdb::prelude::*;
use unbound_shared::PLAYER_HEIGHT;

use crate::camera::{ControlState, update_camera, update_cursor};
use crate::module_bindings::{DbConnection, PlayerTableAccessor, RemoteModule};
use crate::net::{
    LocalPlayer, apply_player_deletes, apply_player_inserts, apply_player_updates, connect,
    interpolate_remotes, predict_local, send_input, spawn_pawns_from_cache, subscribe_on_connect,
};

pub type StdbConn = StdbConnection<DbConnection>;
pub type StdbSubs = StdbSubscriptions<SubKey, RemoteModule>;
pub type StdbCmds<'w, 's> = StdbCommands<'w, 's, DbConnection, RemoteModule>;

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub enum SubKey {
    Players,
}

fn main() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    let mut stdb = StdbPlugin::<DbConnection, RemoteModule>::default()
        .with_database_name("unbound")
        .with_uri(stdb_uri())
        .add_table::<PlayerTableAccessor>()
        .with_subscriptions::<SubKey>()
        .with_reconnect(StdbReconnectOptions::default());

    #[cfg(target_arch = "wasm32")]
    {
        stdb = stdb.with_background_driver(DbConnection::run_background_task);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        stdb = stdb.with_background_driver(DbConnection::run_threaded);
    }

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Unbound".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(stdb)
        .insert_resource(ControlState::default())
        .add_systems(Startup, (setup_scene, connect, setup_hud))
        .add_systems(
            Update,
            (
                subscribe_on_connect,
                spawn_pawns_from_cache,
                apply_player_inserts,
                apply_player_updates,
                apply_player_deletes,
                interpolate_remotes,
                read_movement_keys,
                predict_local,
                send_input,
                update_cursor,
                update_camera,
                update_hud,
            )
                .chain(),
        )
        .run();
}

fn stdb_uri() -> String {
    "http://127.0.0.1:3000".into()
}

#[derive(Component)]
struct HudText;

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(64.0, 64.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.18, 0.20, 0.16))),
    ));

    let pillar = meshes.add(Cuboid::new(1.2, 3.0, 1.2));
    let pillar_mat = materials.add(Color::srgb(0.32, 0.28, 0.24));
    for (x, z) in [(-10.0, -10.0), (10.0, -10.0), (-10.0, 10.0), (10.0, 10.0)] {
        commands.spawn((
            Mesh3d(pillar.clone()),
            MeshMaterial3d(pillar_mat.clone()),
            Transform::from_xyz(x, 1.5, z),
        ));
    }

    commands.spawn((
        DirectionalLight {
            illuminance: 12_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(12.0, 24.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 4.0, 8.0)
            .looking_at(Vec3::new(0.0, PLAYER_HEIGHT * 0.5, 0.0), Vec3::Y),
        MainCamera,
    ));
}

#[derive(Component)]
pub struct MainCamera;

fn setup_hud(mut commands: Commands) {
    commands.spawn((
        Text::new(hud_copy(false, false)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(14.0),
            left: Val::Px(16.0),
            ..default()
        },
        HudText,
    ));
}

fn read_movement_keys(keys: Res<ButtonInput<KeyCode>>, mut control: ResMut<ControlState>) {
    if keys.just_pressed(KeyCode::KeyF) {
        control.drawn = !control.drawn;
    }
    let mut dir_x = 0.0;
    let mut dir_z = 0.0;
    if keys.pressed(KeyCode::KeyD) {
        dir_x += 1.0;
    }
    if keys.pressed(KeyCode::KeyA) {
        dir_x -= 1.0;
    }
    if keys.pressed(KeyCode::KeyW) {
        dir_z += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) {
        dir_z -= 1.0;
    }
    control.dir_x = dir_x;
    control.dir_z = dir_z;
}

fn update_hud(
    control: Res<ControlState>,
    local: Query<(), With<LocalPlayer>>,
    mut text: Query<&mut Text, With<HudText>>,
) {
    let Ok(mut text) = text.single_mut() else {
        return;
    };
    *text = Text::new(hud_copy(control.drawn, !local.is_empty()));
}

fn hud_copy(drawn: bool, connected: bool) -> String {
    let stance = if drawn { "DRAWN" } else { "SHEATHED" };
    let net = if connected { "online" } else { "connecting…" };
    format!(
        "UNBOUND  {stance}  [{net}]\nWASD move   RMB look (sheathed)   F draw/sheathe   Esc cursor"
    )
}
