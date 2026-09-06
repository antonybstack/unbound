mod camera;
mod combat;
mod module_bindings;
mod net;

use bevy::prelude::*;
use bevy_stdb::prelude::*;
use unbound_shared::{
    BTN_BLOCK, BTN_DODGE, BTN_HEAVY, BTN_LIGHT, BTN_SPRINT, MAX_HP, MAX_STAMINA, PLAYER_HEIGHT,
    loadout,
};

use crate::camera::{ControlState, update_camera, update_cursor};
use crate::combat::{
    LocalVitals, WeaponState, flash_hits, refresh_weapon, subscribe_world, sync_dummy,
    sync_projectiles, sync_vitals,
};
use crate::module_bindings::{
    CharacterTableAccessor, CombatEventTableAccessor, DbConnection, DummyTableAccessor,
    PlayerTableAccessor, ProjectileTableAccessor, RemoteModule,
};
use crate::net::{
    LocalPlayer, apply_player_deletes, apply_player_inserts, apply_player_updates, connect,
    interpolate_remotes, predict_local, send_input, spawn_pawns_from_cache,
};

pub type StdbConn = StdbConnection<DbConnection>;
pub type StdbSubs = StdbSubscriptions<SubKey, RemoteModule>;
pub type StdbCmds<'w, 's> = StdbCommands<'w, 's, DbConnection, RemoteModule>;

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub enum SubKey {
    Players,
    Dummies,
    Projectiles,
    Characters,
    Events,
}

fn main() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    let mut stdb = StdbPlugin::<DbConnection, RemoteModule>::default()
        .with_database_name("unbound")
        .with_uri(stdb_uri())
        .add_table::<PlayerTableAccessor>()
        .add_table::<DummyTableAccessor>()
        .add_table::<ProjectileTableAccessor>()
        .add_table::<CharacterTableAccessor>()
        .add_event_table::<CombatEventTableAccessor>()
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
        .insert_resource(ClearColor(Color::srgb(0.52, 0.62, 0.72)))
        .insert_resource(GlobalAmbientLight {
            color: Color::srgb(0.85, 0.88, 0.95),
            brightness: 220.0,
            ..default()
        })
        .add_plugins(stdb)
        .insert_resource(ControlState::default())
        .insert_resource(LocalVitals::default())
        .insert_resource(WeaponState::default())
        .add_systems(Startup, (setup_scene, connect, setup_hud))
        .add_systems(
            Update,
            (
                subscribe_world,
                spawn_pawns_from_cache,
                apply_player_inserts,
                apply_player_updates,
                apply_player_deletes,
                interpolate_remotes,
                sync_dummy,
                sync_projectiles,
                sync_vitals,
                flash_hits,
                read_combat_input,
                predict_local,
                send_input,
                refresh_weapon,
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
        Mesh3d(meshes.add(Plane3d::default().mesh().size(400.0, 400.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.22, 0.28, 0.20))),
    ));

    let pillar = meshes.add(Cuboid::new(1.6, 4.0, 1.6));
    let pillar_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.62, 0.38, 0.22),
        perceptual_roughness: 0.85,
        ..default()
    });
    for (x, z) in [
        (-6.0, -8.0),
        (6.0, -8.0),
        (-6.0, 8.0),
        (6.0, 8.0),
        (0.0, -12.0),
    ] {
        commands.spawn((
            Mesh3d(pillar.clone()),
            MeshMaterial3d(pillar_mat.clone()),
            Transform::from_xyz(x, 2.0, z),
        ));
    }

    commands.spawn((
        Mesh3d(meshes.add(Capsule3d::new(0.35, 0.9))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.82, 0.62, 0.28),
            perceptual_roughness: 0.7,
            ..default()
        })),
        Transform::from_xyz(0.0, PLAYER_HEIGHT * 0.5, 0.0),
        LocalPlayer,
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 18_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(18.0, 28.0, 12.0).looking_at(Vec3::ZERO, Vec3::Y),
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
        Text::new("UNBOUND"),
        TextFont::from_font_size(16.0),
        TextColor(Color::srgb(0.95, 0.95, 0.90)),
        TextLayout::no_wrap(),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(14.0),
            width: Val::Auto,
            height: Val::Auto,
            ..default()
        },
        BackgroundColor(Color::NONE),
        HudText,
    ));
}

fn read_combat_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut control: ResMut<ControlState>,
    dummy: Query<&Transform, With<crate::combat::DummyPawn>>,
    local: Query<&Transform, With<LocalPlayer>>,
) {
    if keys.just_pressed(KeyCode::KeyF) {
        control.drawn = !control.drawn;
    }
    if keys.just_pressed(KeyCode::Digit1) {
        control.loadout = 0;
        control.drawn = true;
    }
    if keys.just_pressed(KeyCode::Digit2) {
        control.loadout = 1;
        control.drawn = true;
    }
    if keys.just_pressed(KeyCode::Digit3) {
        control.loadout = 2;
        control.drawn = true;
    }
    if keys.just_pressed(KeyCode::Tab) {
        control.lock_on = !control.lock_on;
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

    if control.lock_on {
        if let (Ok(me), Ok(dummy)) = (local.single(), dummy.single()) {
            let dx = dummy.translation.x - me.translation.x;
            let dz = dummy.translation.z - me.translation.z;
            control.yaw = (-dx).atan2(-dz);
        }
    }

    let mut buttons = 0u32;
    if control.drawn {
        if mouse.pressed(MouseButton::Left) {
            buttons |= BTN_LIGHT;
        }
        if mouse.pressed(MouseButton::Right) {
            buttons |= BTN_HEAVY;
        }
        if keys.pressed(KeyCode::Space) {
            buttons |= BTN_DODGE;
        }
        if mouse.pressed(MouseButton::Middle) || keys.pressed(KeyCode::KeyQ) {
            buttons |= BTN_BLOCK;
        }
    } else if mouse.pressed(MouseButton::Right) {
        // sheathed RMB is look, not heavy
    }
    if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
        buttons |= BTN_SPRINT;
    }
    control.buttons = buttons;
}

fn update_hud(
    control: Res<ControlState>,
    vitals: Res<LocalVitals>,
    mut text: Query<&mut Text, With<HudText>>,
) {
    let Ok(mut text) = text.single_mut() else {
        return;
    };
    let stance = if !vitals.alive {
        "DEAD"
    } else if control.drawn {
        "DRAWN"
    } else {
        "SHEATHED"
    };
    let def = loadout(control.loadout);
    let hp = if vitals.hp == 0.0 && vitals.name.is_empty() {
        MAX_HP
    } else {
        vitals.hp
    };
    let stam = if vitals.stamina == 0.0 && vitals.name.is_empty() {
        MAX_STAMINA
    } else {
        vitals.stamina
    };
    text.0 = format!(
        "UNBOUND  {stance}  {}  [{}]\nHP {hp:3.0}/{MAX_HP:.0}   ST {stam:3.0}/{MAX_STAMINA:.0}   lock {}\nMelee {}  Range {}  Magic {}  Def {}  HP-skill {}\nWASD move  F draw  LMB light  RMB heavy  Space dodge  Shift sprint\n1 sword  2 bow  3 staff  Tab lock dummy  Q/MMB block",
        def.name,
        if vitals.name.is_empty() {
            "online"
        } else {
            vitals.name.as_str()
        },
        if control.lock_on { "ON" } else { "off" },
        vitals.melee.max(1),
        vitals.ranged.max(1),
        vitals.magic.max(1),
        vitals.defence.max(1),
        vitals.hitpoints.max(1),
    );
}
