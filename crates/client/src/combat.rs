use bevy::prelude::*;
use bevy_stdb::prelude::*;
use unbound_shared::{PLAYER_HEIGHT, loadout};

use crate::module_bindings::{
    Character, CharacterTableAccess, CombatEvent, Dummy, DummyTableAccess, PlayerTableAccess,
    Projectile, ProjectileTableAccess, character_table::characterQueryTableAccess,
    combat_event_table::combat_eventQueryTableAccess, dummy_table::dummyQueryTableAccess,
    player_table::playerQueryTableAccess, projectile_table::projectileQueryTableAccess,
};
use crate::net::LocalPlayer;
use crate::{StdbConn, StdbSubs, SubKey};
use spacetimedb_sdk::Table;

#[derive(Component)]
pub struct DummyPawn;

#[derive(Component)]
pub struct ShotPawn {
    pub id: u32,
}

#[derive(Component)]
pub struct WeaponVisual;

#[derive(Resource, Default)]
pub struct LocalVitals {
    pub hp: f32,
    pub stamina: f32,
    pub loadout: u8,
    pub name: String,
    pub melee: u8,
    pub ranged: u8,
    pub magic: u8,
    pub defence: u8,
    pub hitpoints: u8,
    pub alive: bool,
}

pub fn subscribe_world(mut connected: ReadStdbConnectedMessage, mut subs: ResMut<StdbSubs>) {
    if connected.read().next().is_some() {
        subs.subscribe_query(SubKey::Players, |q| q.from.player());
        subs.subscribe_query(SubKey::Dummies, |q| q.from.dummy());
        subs.subscribe_query(SubKey::Projectiles, |q| q.from.projectile());
        subs.subscribe_query(SubKey::Characters, |q| q.from.character());
        subs.subscribe_query(SubKey::Events, |q| q.from.combat_event());
    }
}

pub fn sync_dummy(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut inserts: ReadInsertMessage<Dummy>,
    mut updates: ReadUpdateMessage<Dummy>,
    mut deletes: ReadDeleteMessage<Dummy>,
    conn: Option<Res<StdbConn>>,
    mut dummies: Query<
        (
            Entity,
            &mut Transform,
            &mut MeshMaterial3d<StandardMaterial>,
        ),
        With<DummyPawn>,
    >,
) {
    for msg in inserts.read() {
        spawn_dummy(&mut commands, &mut meshes, &mut materials, &msg.row);
    }
    if let Some(conn) = conn.as_ref() {
        if dummies.is_empty() {
            for row in conn.db().dummy().iter() {
                spawn_dummy(&mut commands, &mut meshes, &mut materials, &row);
            }
        }
    }
    for msg in updates.read() {
        for (_e, mut transform, mat) in &mut dummies {
            transform.translation = Vec3::new(msg.new.x, PLAYER_HEIGHT * 0.5, msg.new.z);
            transform.rotation = Quat::from_rotation_y(msg.new.yaw);
            let color = if !msg.new.alive {
                Color::srgb(0.15, 0.15, 0.15)
            } else if msg.new.action == 6 {
                Color::srgb(0.9, 0.35, 0.2)
            } else {
                Color::srgb(0.72, 0.22, 0.18)
            };
            if let Some(mut m) = materials.get_mut(&mat.0) {
                m.base_color = color;
            }
        }
    }
    for _ in deletes.read() {
        for (e, _, _) in &dummies {
            commands.entity(e).despawn();
        }
    }
}

pub fn sync_projectiles(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut inserts: ReadInsertMessage<Projectile>,
    mut updates: ReadUpdateMessage<Projectile>,
    mut deletes: ReadDeleteMessage<Projectile>,
    conn: Option<Res<StdbConn>>,
    mut shots: Query<(Entity, &ShotPawn, &mut Transform)>,
) {
    if let Some(conn) = conn.as_ref() {
        for row in conn.db().projectile().iter() {
            if shots.iter().any(|(_, s, _)| s.id == row.id) {
                continue;
            }
            spawn_shot(&mut commands, &mut meshes, &mut materials, &row);
        }
    }
    for msg in inserts.read() {
        if shots.iter().any(|(_, s, _)| s.id == msg.row.id) {
            continue;
        }
        spawn_shot(&mut commands, &mut meshes, &mut materials, &msg.row);
    }
    for msg in updates.read() {
        for (_, shot, mut transform) in &mut shots {
            if shot.id == msg.new.id {
                transform.translation = Vec3::new(msg.new.x, 1.1, msg.new.z);
            }
        }
    }
    for msg in deletes.read() {
        for (e, shot, _) in &shots {
            if shot.id == msg.row.id {
                commands.entity(e).despawn();
            }
        }
    }
}

pub fn sync_vitals(
    mut vitals: ResMut<LocalVitals>,
    conn: Option<Res<StdbConn>>,
    mut characters: ReadInsertMessage<Character>,
    mut char_updates: ReadUpdateMessage<Character>,
) {
    let Some(conn) = conn else {
        return;
    };
    let Some(me) = conn.try_identity() else {
        return;
    };
    for p in conn.db().player().iter() {
        if p.identity == me {
            vitals.hp = p.hp;
            vitals.stamina = p.stamina;
            vitals.loadout = p.loadout;
            vitals.name = p.name.clone();
            vitals.alive = p.alive;
        }
    }
    let apply_char = |vitals: &mut LocalVitals, c: &Character| {
        if c.identity != me {
            return;
        }
        vitals.name = c.name.clone();
        vitals.melee = unbound_shared::skill_level(c.melee_xp);
        vitals.ranged = unbound_shared::skill_level(c.ranged_xp);
        vitals.magic = unbound_shared::skill_level(c.magic_xp);
        vitals.defence = unbound_shared::skill_level(c.defence_xp);
        vitals.hitpoints = unbound_shared::skill_level(c.hitpoints_xp);
    };
    for c in conn.db().character().iter() {
        apply_char(&mut vitals, &c);
    }
    for msg in characters.read() {
        apply_char(&mut vitals, &msg.row);
    }
    for msg in char_updates.read() {
        apply_char(&mut vitals, &msg.new);
    }
}

pub fn flash_hits(
    mut events: ReadInsertMessage<CombatEvent>,
    mut local: Query<&mut Transform, With<LocalPlayer>>,
) {
    for msg in events.read() {
        if msg.row.kind == 1 || msg.row.kind == 2 {
            if let Ok(mut t) = local.single_mut() {
                t.translation.y = PLAYER_HEIGHT * 0.5 + 0.05;
            }
        }
    }
}

#[derive(Resource, Default)]
pub struct WeaponState {
    loadout: u8,
    drawn: bool,
}

pub fn refresh_weapon(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    control: Res<crate::camera::ControlState>,
    mut state: ResMut<WeaponState>,
    local: Query<Entity, With<LocalPlayer>>,
    weapons: Query<Entity, With<WeaponVisual>>,
) {
    if state.loadout == control.loadout && state.drawn == control.drawn {
        return;
    }
    state.loadout = control.loadout;
    state.drawn = control.drawn;
    let Ok(local_e) = local.single() else {
        return;
    };
    for w in &weapons {
        commands.entity(w).despawn();
    }
    attach_weapon(
        &mut commands,
        &mut meshes,
        &mut materials,
        local_e,
        control.loadout,
        control.drawn,
        true,
    );
}

fn attach_weapon(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    parent: Entity,
    loadout_id: u8,
    drawn: bool,
    _local: bool,
) {
    if !drawn {
        return;
    }
    let def = loadout(loadout_id);
    let (mesh, color, tf) = match def.id {
        1 => (
            meshes.add(Cuboid::new(0.08, 0.08, 1.15)),
            Color::srgb(0.45, 0.28, 0.12),
            Transform::from_xyz(0.25, 0.15, -0.55),
        ),
        2 => (
            meshes.add(Cylinder::new(0.04, 1.4)),
            Color::srgb(0.35, 0.2, 0.55),
            Transform::from_xyz(0.28, 0.1, -0.2).with_rotation(Quat::from_rotation_x(0.2)),
        ),
        _ => (
            meshes.add(Cuboid::new(0.12, 0.04, 0.9)),
            Color::srgb(0.75, 0.75, 0.8),
            Transform::from_xyz(0.38, 0.15, -0.35),
        ),
    };
    let child = commands
        .spawn((
            Mesh3d(mesh),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                metallic: 0.4,
                perceptual_roughness: 0.4,
                ..default()
            })),
            tf,
            WeaponVisual,
        ))
        .id();
    commands.entity(parent).add_child(child);
}

fn spawn_dummy(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    dummy: &Dummy,
) {
    commands.spawn((
        Mesh3d(meshes.add(Capsule3d::new(0.4, 1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.72, 0.22, 0.18),
            perceptual_roughness: 0.65,
            ..default()
        })),
        Transform::from_xyz(dummy.x, PLAYER_HEIGHT * 0.5, dummy.z)
            .with_rotation(Quat::from_rotation_y(dummy.yaw)),
        DummyPawn,
    ));
}

fn spawn_shot(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    shot: &Projectile,
) {
    let color = if shot.skill == 2 {
        Color::srgb(0.55, 0.35, 0.95)
    } else {
        Color::srgb(0.85, 0.7, 0.25)
    };
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.12))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            emissive: LinearRgba::from(color) * 4.0,
            ..default()
        })),
        Transform::from_xyz(shot.x, 1.1, shot.z),
        ShotPawn { id: shot.id },
    ));
}
