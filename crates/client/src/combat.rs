use bevy::prelude::*;
use bevy_stdb::prelude::*;
use unbound_shared::{ACTION_HEAVY, ACTION_HIT, ACTION_LIGHT, MAX_HP, PLAYER_HEIGHT, loadout};

use crate::module_bindings::{
    Character, CharacterTableAccess, CombatEvent, Dummy, DummyTableAccess, GatherNode,
    GatherNodeTableAccess, Player, PlayerTableAccess, Projectile, ProjectileTableAccess,
    character_table::characterQueryTableAccess, combat_event_table::combat_eventQueryTableAccess,
    dummy_table::dummyQueryTableAccess, gather_node_table::gather_nodeQueryTableAccess,
    player_table::playerQueryTableAccess, projectile_table::projectileQueryTableAccess,
};
use crate::net::{LocalPlayer, RemotePlayer, ServerPose};
use crate::{StdbConn, StdbSubs, SubKey};
use spacetimedb_sdk::Table;

#[derive(Component)]
pub struct DummyPawn;

#[derive(Component)]
pub struct DummyHpBar;

#[derive(Component)]
pub struct ShotPawn {
    pub id: u32,
}

#[derive(Component)]
pub struct WeaponVisual;

#[derive(Component)]
pub struct NodePawn {
    pub id: u32,
}

#[derive(Resource)]
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
    pub gather: u8,
    pub alive: bool,
    pub dummy_hp: f32,
    pub dummy_alive: bool,
    pub node_kind: u8,
    pub node_dist: f32,
    pub others: String,
    pub log: String,
}

impl Default for LocalVitals {
    fn default() -> Self {
        Self {
            hp: MAX_HP,
            stamina: unbound_shared::MAX_STAMINA,
            loadout: 0,
            name: String::new(),
            melee: 1,
            ranged: 1,
            magic: 1,
            defence: 1,
            hitpoints: 1,
            gather: 1,
            alive: true,
            dummy_hp: MAX_HP,
            dummy_alive: true,
            node_kind: 0,
            node_dist: 999.0,
            others: String::new(),
            log: String::new(),
        }
    }
}

#[derive(Resource, Default)]
pub struct WeaponState {
    pub loadout: u8,
    pub drawn: bool,
}

pub fn subscribe_world(mut connected: ReadStdbConnectedMessage, mut subs: ResMut<StdbSubs>) {
    if connected.read().next().is_some() {
        subs.subscribe_query(SubKey::Players, |q| q.from.player());
        subs.subscribe_query(SubKey::Dummies, |q| q.from.dummy());
        subs.subscribe_query(SubKey::Projectiles, |q| q.from.projectile());
        subs.subscribe_query(SubKey::Characters, |q| q.from.character());
        subs.subscribe_query(SubKey::Events, |q| q.from.combat_event());
        subs.subscribe_query(SubKey::Nodes, |q| q.from.gather_node());
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
    mut bars: Query<&mut Transform, (With<DummyHpBar>, Without<DummyPawn>)>,
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
            let color = dummy_color(&msg.new);
            if let Some(mut m) = materials.get_mut(&mat.0) {
                m.base_color = color;
            }
        }
        for mut bar in &mut bars {
            let ratio = (msg.new.hp / MAX_HP).clamp(0.05, 1.0);
            bar.scale.x = if msg.new.alive { ratio } else { 0.05 };
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

pub fn sync_nodes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut inserts: ReadInsertMessage<GatherNode>,
    mut updates: ReadUpdateMessage<GatherNode>,
    mut deletes: ReadDeleteMessage<GatherNode>,
    conn: Option<Res<StdbConn>>,
    mut nodes: Query<(
        Entity,
        &NodePawn,
        &mut Transform,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
) {
    if let Some(conn) = conn.as_ref() {
        for row in conn.db().gather_node().iter() {
            if nodes.iter().any(|(_, n, _, _)| n.id == row.id) {
                continue;
            }
            spawn_node(&mut commands, &mut meshes, &mut materials, &row);
        }
    }
    for msg in inserts.read() {
        if nodes.iter().any(|(_, n, _, _)| n.id == msg.row.id) {
            continue;
        }
        spawn_node(&mut commands, &mut meshes, &mut materials, &msg.row);
    }
    for msg in updates.read() {
        for (_e, node, mut transform, mat) in &mut nodes {
            if node.id != msg.new.id {
                continue;
            }
            let depleted = msg.new.charges == 0;
            transform.scale = if depleted {
                Vec3::new(1.0, 0.45, 1.0)
            } else {
                Vec3::ONE
            };
            if let Some(mut m) = materials.get_mut(&mat.0) {
                m.base_color = node_color(msg.new.kind, depleted);
            }
        }
    }
    for msg in deletes.read() {
        for (e, node, _, _) in &nodes {
            if node.id == msg.row.id {
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
    mut player_updates: ReadUpdateMessage<Player>,
    mut events: ReadInsertMessage<CombatEvent>,
    local: Query<&Transform, With<LocalPlayer>>,
) {
    let Some(conn) = conn else {
        return;
    };
    let Some(me) = conn.try_identity() else {
        return;
    };

    for msg in player_updates.read() {
        if msg.new.identity == me {
            apply_player_vitals(&mut vitals, &msg.new);
        }
    }
    for p in conn.db().player().iter() {
        if p.identity == me {
            apply_player_vitals(&mut vitals, &p);
        }
    }

    vitals.dummy_hp = MAX_HP;
    vitals.dummy_alive = true;
    for d in conn.db().dummy().iter() {
        vitals.dummy_hp = d.hp;
        vitals.dummy_alive = d.alive;
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
        vitals.gather = unbound_shared::skill_level(c.gather_xp);
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

    let me_pos = local.single().ok().map(|t| t.translation);
    vitals.node_dist = 999.0;
    for node in conn.db().gather_node().iter() {
        if node.charges == 0 {
            continue;
        }
        if let Some(pos) = me_pos {
            let d = (pos.x - node.x).hypot(pos.z - node.z);
            if d < vitals.node_dist {
                vitals.node_dist = d;
                vitals.node_kind = node.kind;
            }
        }
    }

    let mut others = Vec::new();
    for p in conn.db().player().iter() {
        if p.identity == me {
            continue;
        }
        others.push(format!("{} {:3.0}hp", p.name, p.hp));
    }
    vitals.others = others.join("  ·  ");

    for msg in events.read() {
        vitals.log = match msg.row.kind {
            1 => format!("hit  {:+.0}", -msg.row.damage),
            2 => format!("kill  {:+.0}", -msg.row.damage),
            3 => "blocked".into(),
            4 => "dodged".into(),
            5 => format!("gathered  +{:.0} xp", msg.row.damage),
            _ => vitals.log.clone(),
        };
    }
}

pub fn flash_hits(
    mut events: ReadInsertMessage<CombatEvent>,
    mut local: Query<&mut Transform, With<LocalPlayer>>,
) {
    for msg in events.read() {
        if msg.row.kind == 1 || msg.row.kind == 2 {
            if let Ok(mut t) = local.single_mut() {
                t.translation.y = PLAYER_HEIGHT * 0.5 + 0.08;
            }
        }
    }
}

pub fn refresh_weapon(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    control: Res<crate::camera::ControlState>,
    mut state: ResMut<WeaponState>,
    local: Query<Entity, With<LocalPlayer>>,
    weapons: Query<(Entity, &WeaponVisual, Option<&ChildOf>)>,
) {
    if state.loadout == control.loadout && state.drawn == control.drawn {
        return;
    }
    state.loadout = control.loadout;
    state.drawn = control.drawn;
    let Ok(local_e) = local.single() else {
        return;
    };
    for (w, _, parent) in &weapons {
        if parent.map(|p| p.parent() == local_e).unwrap_or(false) {
            commands.entity(w).despawn();
        }
    }
    attach_weapon(
        &mut commands,
        &mut meshes,
        &mut materials,
        local_e,
        control.loadout,
        control.drawn,
    );
}

pub fn refresh_remote_weapons(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    remotes: Query<(Entity, &ServerPose), With<RemotePlayer>>,
    weapons: Query<(Entity, &WeaponVisual, Option<&ChildOf>)>,
) {
    for (entity, pose) in &remotes {
        let has_weapon = weapons
            .iter()
            .any(|(_, _, parent)| parent.map(|p| p.parent() == entity).unwrap_or(false));
        if pose.drawn && !has_weapon {
            attach_weapon(
                &mut commands,
                &mut meshes,
                &mut materials,
                entity,
                pose.loadout,
                true,
            );
        } else if !pose.drawn && has_weapon {
            for (w, _, parent) in &weapons {
                if parent.map(|p| p.parent() == entity).unwrap_or(false) {
                    commands.entity(w).despawn();
                }
            }
        }
    }
}

fn apply_player_vitals(vitals: &mut LocalVitals, p: &Player) {
    vitals.hp = p.hp;
    vitals.stamina = p.stamina;
    vitals.loadout = p.loadout;
    vitals.name = p.name.clone();
    vitals.alive = p.alive;
}

fn dummy_color(dummy: &Dummy) -> Color {
    if !dummy.alive {
        Color::srgb(0.15, 0.15, 0.15)
    } else if dummy.action == ACTION_LIGHT || dummy.action == ACTION_HEAVY {
        Color::srgb(0.95, 0.45, 0.12)
    } else if dummy.action == ACTION_HIT {
        Color::srgb(0.9, 0.35, 0.2)
    } else {
        Color::srgb(0.72, 0.22, 0.18)
    }
}

fn node_color(kind: u8, depleted: bool) -> Color {
    if depleted {
        Color::srgb(0.18, 0.16, 0.14)
    } else if kind == 1 {
        Color::srgb(0.45, 0.48, 0.52)
    } else {
        Color::srgb(0.38, 0.24, 0.12)
    }
}

fn attach_weapon(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    parent: Entity,
    loadout_id: u8,
    drawn: bool,
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
    let root = commands
        .spawn((
            Mesh3d(meshes.add(Capsule3d::new(0.42, 1.05))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: dummy_color(dummy),
                perceptual_roughness: 0.65,
                ..default()
            })),
            Transform::from_xyz(dummy.x, PLAYER_HEIGHT * 0.5, dummy.z)
                .with_rotation(Quat::from_rotation_y(dummy.yaw)),
            DummyPawn,
        ))
        .id();
    let club = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(0.16, 0.16, 1.1))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.28, 0.16, 0.08),
                perceptual_roughness: 0.9,
                ..default()
            })),
            Transform::from_xyz(0.45, 0.1, -0.35),
        ))
        .id();
    let bar = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(1.0, 0.08, 0.08))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.85, 0.18, 0.14),
                unlit: true,
                ..default()
            })),
            Transform::from_xyz(0.0, 1.25, 0.0).with_scale(Vec3::new(
                (dummy.hp / MAX_HP).clamp(0.05, 1.0),
                1.0,
                1.0,
            )),
            DummyHpBar,
        ))
        .id();
    commands.entity(root).add_child(club);
    commands.entity(root).add_child(bar);
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

fn spawn_node(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    node: &GatherNode,
) {
    let depleted = node.charges == 0;
    let (mesh, y) = if node.kind == 1 {
        (meshes.add(Cuboid::new(0.9, 0.7, 0.9)), 0.35)
    } else {
        (meshes.add(Cylinder::new(0.28, 1.6)), 0.8)
    };
    let root = commands
        .spawn((
            Mesh3d(mesh),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: node_color(node.kind, depleted),
                perceptual_roughness: 0.9,
                ..default()
            })),
            Transform::from_xyz(node.x, y, node.z).with_scale(if depleted {
                Vec3::new(1.0, 0.45, 1.0)
            } else {
                Vec3::ONE
            }),
            NodePawn { id: node.id },
        ))
        .id();
    if node.kind != 1 {
        let canopy = commands
            .spawn((
                Mesh3d(meshes.add(Sphere::new(0.85))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.18, 0.38, 0.16),
                    perceptual_roughness: 1.0,
                    ..default()
                })),
                Transform::from_xyz(0.0, 1.0, 0.0),
            ))
            .id();
        commands.entity(root).add_child(canopy);
    }
}
