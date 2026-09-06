mod camera;
mod combat;
mod module_bindings;
mod net;
mod persist;

use bevy::prelude::*;
use bevy_stdb::prelude::*;
use unbound_shared::{
    BTN_BLOCK, BTN_DODGE, BTN_HEAVY, BTN_INTERACT, BTN_LIGHT, BTN_SPRINT, GATHER_RANGE, MAX_HP,
    MAX_STAMINA, PLAYER_HEIGHT, action_label, loadout,
};

use crate::camera::{ControlState, update_camera, update_cursor};
use crate::combat::{
    DummyPawn, HitFlash, LocalVitals, WeaponState, apply_predicted_starts, flash_hits, fly_shots,
    interpolate_dummy, pose_dummy_club, pose_hp_bars, pose_weapons, refresh_remote_weapons,
    refresh_weapon, subscribe_world, sync_dummy, sync_nameplates, sync_nodes, sync_projectiles,
    sync_vitals, tick_dummy_pose, tick_hit_flash, tick_prediction, update_floaters,
    update_nameplates,
};
use crate::module_bindings::{
    CharacterTableAccessor, CombatEventTableAccessor, DbConnection, DummyTableAccessor,
    GatherNodeTableAccessor, PlayerTableAccessor, ProjectileTableAccessor, RemoteModule,
};
use crate::net::{
    LocalPlayer, RemotePlayer, apply_player_deletes, apply_player_inserts, apply_player_updates,
    bind_local_player, connect, interpolate_remotes, predict_local, send_input,
    spawn_pawns_from_cache,
};
use crate::persist::persist_on_connect;

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
    Nodes,
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
        .add_table::<GatherNodeTableAccessor>()
        .add_event_table::<CombatEventTableAccessor>()
        .with_subscriptions::<SubKey>()
        .with_reconnect(StdbReconnectOptions::default());
    if let Some(token) = persist::load_token() {
        stdb = stdb.with_token(token);
    }

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
        .insert_resource(HitFlash::default())
        .add_systems(Startup, (setup_scene, connect, setup_hud))
        .add_systems(
            Update,
            (
                (
                    persist_on_connect,
                    subscribe_world,
                    spawn_pawns_from_cache,
                    bind_local_player,
                    apply_player_inserts,
                    apply_player_updates,
                    apply_player_deletes,
                    interpolate_remotes,
                    sync_dummy,
                    tick_dummy_pose,
                    interpolate_dummy,
                    pose_hp_bars,
                    sync_projectiles,
                    fly_shots,
                    sync_nodes,
                    sync_vitals,
                    flash_hits,
                    update_floaters,
                    sync_nameplates,
                    update_nameplates,
                )
                    .chain(),
                (
                    tick_hit_flash,
                    read_combat_input,
                    apply_predicted_starts,
                    tick_prediction,
                    predict_local,
                    send_input,
                    refresh_weapon,
                    refresh_remote_weapons,
                    pose_weapons,
                    pose_dummy_club,
                    update_cursor,
                    update_camera,
                    update_hud,
                    update_crosshair,
                )
                    .chain(),
            )
                .chain(),
        )
        .run();
}

fn stdb_uri() -> String {
    "http://127.0.0.1:3000".into()
}

#[derive(Component)]
struct HudTitle;
#[derive(Component)]
struct HudLog;
#[derive(Component)]
struct HpFill;
#[derive(Component)]
struct StamFill;
#[derive(Component)]
struct DummyHpFill;
#[derive(Component)]
struct Crosshair;
#[derive(Component)]
pub struct MainCamera;

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(400.0, 400.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.22, 0.28, 0.20))),
    ));

    // PvP circle so the yard reads as a place, not an infinite lawn.
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(16.0, 0.04))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.18, 0.22, 0.16),
            perceptual_roughness: 1.0,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.02, 0.0),
    ));
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(4.5, 0.05))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.28, 0.24, 0.16),
            perceptual_roughness: 1.0,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.03, 0.0),
    ));

    let pillar = meshes.add(Cuboid::new(1.6, 4.0, 1.6));
    let pillar_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.62, 0.38, 0.22),
        perceptual_roughness: 0.85,
        ..default()
    });
    for (x, z) in [(-10.0, -10.0), (10.0, -10.0), (-10.0, 10.0), (10.0, 10.0)] {
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

fn setup_hud(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(12.0),
                left: Val::Px(14.0),
                width: Val::Px(380.0),
                padding: UiRect::all(Val::Px(10.0)),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.04, 0.05, 0.06, 0.62)),
        ))
        .with_children(|root| {
            root.spawn((
                Text::new("UNBOUND"),
                TextFont::from_font_size(15.0),
                TextColor(Color::srgb(0.95, 0.95, 0.90)),
                TextLayout::no_wrap(),
                HudTitle,
            ));
            spawn_bar(
                root,
                Color::srgb(0.18, 0.07, 0.07),
                Color::srgb(0.78, 0.20, 0.16),
                HpFill,
            );
            spawn_bar(
                root,
                Color::srgb(0.16, 0.14, 0.05),
                Color::srgb(0.82, 0.72, 0.22),
                StamFill,
            );
            spawn_bar(
                root,
                Color::srgb(0.16, 0.06, 0.05),
                Color::srgb(0.72, 0.22, 0.18),
                DummyHpFill,
            );
            root.spawn((
                Text::new(""),
                TextFont::from_font_size(13.0),
                TextColor(Color::srgb(0.90, 0.88, 0.80)),
                TextLayout::no_wrap(),
                HudLog,
            ));
        });

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Percent(50.0),
            top: Val::Percent(50.0),
            width: Val::Px(10.0),
            height: Val::Px(10.0),
            margin: UiRect {
                left: Val::Px(-5.0),
                top: Val::Px(-5.0),
                ..default()
            },
            border: UiRect::all(Val::Px(1.5)),
            ..default()
        },
        BorderColor::all(Color::srgba(0.95, 0.95, 0.9, 0.0)),
        BackgroundColor(Color::NONE),
        Crosshair,
    ));
}

fn spawn_bar(parent: &mut ChildSpawnerCommands, back: Color, fill: Color, marker: impl Bundle) {
    parent
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(12.0),
                ..default()
            },
            BackgroundColor(back),
        ))
        .with_children(|bar| {
            bar.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(fill),
                marker,
            ));
        });
}

fn read_combat_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut control: ResMut<ControlState>,
    dummy: Query<&Transform, With<DummyPawn>>,
    local: Query<&Transform, With<LocalPlayer>>,
    remotes: Query<&Transform, With<RemotePlayer>>,
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
        if let Ok(me) = local.single() {
            let mut best: Option<(f32, Vec3)> = None;
            let consider = |best: &mut Option<(f32, Vec3)>, pos: Vec3| {
                let d = me.translation.distance(pos);
                if best.map(|(bd, _)| d < bd).unwrap_or(true) {
                    *best = Some((d, pos));
                }
            };
            if let Ok(dummy) = dummy.single() {
                consider(&mut best, dummy.translation);
            }
            for remote in &remotes {
                consider(&mut best, remote.translation);
            }
            if let Some((_, pos)) = best {
                let dx = pos.x - me.translation.x;
                let dz = pos.z - me.translation.z;
                control.yaw = (-dx).atan2(-dz);
                control.lock_focus = Some(pos);
            } else {
                control.lock_focus = None;
            }
        }
    } else {
        control.lock_focus = None;
    }

    let mut buttons = 0u32;
    if control.drawn {
        if mouse.pressed(MouseButton::Left) {
            buttons |= BTN_LIGHT;
        }
        if mouse.just_pressed(MouseButton::Left) {
            control.latched |= BTN_LIGHT;
        }
        if mouse.pressed(MouseButton::Right) {
            buttons |= BTN_HEAVY;
        }
        if mouse.just_pressed(MouseButton::Right) {
            control.latched |= BTN_HEAVY;
        }
        if keys.pressed(KeyCode::Space) {
            buttons |= BTN_DODGE;
        }
        if keys.just_pressed(KeyCode::Space) {
            control.latched |= BTN_DODGE;
        }
        if mouse.pressed(MouseButton::Middle) || keys.pressed(KeyCode::KeyQ) {
            buttons |= BTN_BLOCK;
        }
    }
    if keys.pressed(KeyCode::KeyE) {
        buttons |= BTN_INTERACT;
    }
    if keys.just_pressed(KeyCode::KeyE) {
        control.latched |= BTN_INTERACT;
    }
    if keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight) {
        buttons |= BTN_SPRINT;
    }
    control.buttons = buttons;
}

fn update_hud(
    control: Res<ControlState>,
    vitals: Res<LocalVitals>,
    mut title: Query<&mut Text, With<HudTitle>>,
    mut log: Query<&mut Text, (With<HudLog>, Without<HudTitle>)>,
    mut bars: ParamSet<(
        Query<&mut Node, With<HpFill>>,
        Query<&mut Node, With<StamFill>>,
        Query<&mut Node, With<DummyHpFill>>,
    )>,
) {
    let stance = if !vitals.alive || control.pred_action == unbound_shared::ACTION_DEAD {
        "DEAD"
    } else if control.drawn {
        "DRAWN"
    } else {
        "SHEATHED"
    };
    let act = action_label(control.pred_action);
    let def = loadout(control.loadout);
    let hp = if vitals.hp == 0.0 && vitals.name.is_empty() {
        MAX_HP
    } else {
        vitals.hp
    };
    let stam = if vitals.name.is_empty() && control.pred_stamina <= 0.0 {
        MAX_STAMINA
    } else {
        control.pred_stamina
    };
    let name = if vitals.name.is_empty() {
        "online"
    } else {
        vitals.name.as_str()
    };
    if let Ok(mut text) = title.single_mut() {
        text.0 = format!(
            "UNBOUND  {stance}  {}  [{name}]  lock {}{}",
            def.name,
            if control.lock_on { "ON" } else { "off" },
            if act.is_empty() {
                String::new()
            } else {
                format!("  {act}")
            },
        );
    }
    if let Ok(mut fill) = bars.p0().single_mut() {
        fill.width = Val::Percent((100.0 * (hp / MAX_HP)).clamp(0.0, 100.0));
    }
    if let Ok(mut fill) = bars.p1().single_mut() {
        fill.width = Val::Percent((100.0 * (stam / MAX_STAMINA)).clamp(0.0, 100.0));
    }
    if let Ok(mut fill) = bars.p2().single_mut() {
        fill.width = Val::Percent((100.0 * (vitals.dummy_hp / MAX_HP)).clamp(0.0, 100.0));
    }
    if let Ok(mut text) = log.single_mut() {
        let gather_hint = if !control.drawn && vitals.node_dist <= GATHER_RANGE {
            format!(
                "E gather {} ({:.1}m)\n",
                if vitals.node_kind == 1 { "ore" } else { "wood" },
                vitals.node_dist
            )
        } else if !control.drawn && vitals.dummy_dist > unbound_shared::DUMMY_STRIKE_RANGE {
            "sheathed — dummy hunts drawn steel\n".into()
        } else {
            String::new()
        };
        let others = if vitals.others.is_empty() {
            "no other wanderers".into()
        } else {
            vitals.others.clone()
        };
        text.0 = format!(
            "HP {hp:3.0}   ST {stam:3.0}   Dummy {dummy:3.0}{dummy_state}\n\
             Melee {melee}  Range {ranged}  Magic {magic}  Def {defence}  HP {hitpoints}  Gather {gather}\n\
             {gather_hint}{others}\n\
             {log}\n\
             WASD  F draw  LMB/RMB  Space dodge  Shift sprint  E gather\n\
             1 sword  2 bow  3 staff  Tab lock  Q/MMB block",
            dummy = vitals.dummy_hp,
            dummy_state = if vitals.dummy_alive { "" } else { "  (down)" },
            melee = vitals.melee.max(1),
            ranged = vitals.ranged.max(1),
            magic = vitals.magic.max(1),
            defence = vitals.defence.max(1),
            hitpoints = vitals.hitpoints.max(1),
            gather = vitals.gather.max(1),
            log = vitals.log,
        );
    }
}

fn update_crosshair(control: Res<ControlState>, mut q: Query<&mut BorderColor, With<Crosshair>>) {
    let Ok(mut border) = q.single_mut() else {
        return;
    };
    let alpha = if control.drawn { 0.85 } else { 0.0 };
    *border = BorderColor::all(Color::srgba(0.95, 0.95, 0.88, alpha));
}
