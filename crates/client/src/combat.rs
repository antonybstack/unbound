use bevy::prelude::*;
use bevy_stdb::prelude::*;
use unbound_shared::{
    ACTION_BLOCK, ACTION_DODGE, ACTION_HEAVY, ACTION_HIT, ACTION_LIGHT, ACTION_NONE, BTN_BLOCK,
    BTN_SPRINT, DODGE_SPEED, GATHER_RANGE, MAX_HP, MAX_STAMINA, PLAYER_HEIGHT,
    SPRINT_STAMINA_PER_SEC, STAMINA_REGEN_PER_SEC, TICK_HZ, dodge_burst_dt, dodge_dir,
    dummy_club_pitch, dummy_windup_ticks, integrate, loadout, merge_input_buttons,
    predicted_busy_ticks, start_drawn_action, start_gather_action, weapon_extra_rotation,
};

use crate::camera::ControlState;
use crate::module_bindings::{
    Character, CharacterTableAccess, CombatEvent, Dummy, DummyTableAccess, GatherNode,
    GatherNodeTableAccess, Player, PlayerTableAccess, Projectile, ProjectileTableAccess,
    character_table::characterQueryTableAccess, combat_event_table::combat_eventQueryTableAccess,
    dummy_table::dummyQueryTableAccess, gather_node_table::gather_nodeQueryTableAccess,
    player_table::playerQueryTableAccess, projectile_table::projectileQueryTableAccess,
};
use crate::net::{LocalPlayer, RemotePlayer, ServerPose};
use crate::{MainCamera, StdbConn, StdbSubs, SubKey};
use spacetimedb_sdk::Table;

#[derive(Component)]
pub struct DummyPawn;

#[derive(Component)]
pub struct HpBar;

#[derive(Component)]
pub struct Nameplate {
    pub target: Entity,
}

#[derive(Component)]
pub struct ShotPawn {
    pub id: u32,
    pub vx: f32,
    pub vz: f32,
}

#[derive(Component)]
pub struct WeaponVisual {
    pub loadout: u8,
    pub rest: Transform,
}

#[derive(Component)]
pub struct DummyClub {
    pub rest: Transform,
}

#[derive(Component)]
pub struct DummyPose {
    pub x: f32,
    pub z: f32,
    pub yaw: f32,
    pub action: u8,
    pub action_ticks: f32,
    pub hp: f32,
    pub alive: bool,
}

#[derive(Resource, Default)]
pub struct HitFlash {
    pub t: f32,
}

#[derive(Component)]
pub struct DamageFloater {
    pub age: f32,
}

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
    pub dummy_dist: f32,
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
            dummy_dist: 999.0,
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
    mut dummies: Query<(
        Entity,
        &mut DummyPose,
        &mut MeshMaterial3d<StandardMaterial>,
    )>,
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
        for (_e, mut pose, mat) in &mut dummies {
            pose.x = msg.new.x;
            pose.z = msg.new.z;
            pose.yaw = msg.new.yaw;
            pose.action = msg.new.action;
            pose.action_ticks = msg.new.action_ticks as f32;
            pose.hp = msg.new.hp;
            pose.alive = msg.new.alive;
            if let Some(mut m) = materials.get_mut(&mat.0) {
                m.base_color = dummy_color(pose.action, pose.alive);
            }
        }
    }
    for _ in deletes.read() {
        for (e, _, _) in &dummies {
            commands.entity(e).despawn();
        }
    }
}

pub fn interpolate_dummy(
    time: Res<Time>,
    mut dummies: Query<(&DummyPose, &mut Transform), With<DummyPawn>>,
) {
    let t = (10.0 * time.delta_secs()).min(1.0);
    for (pose, mut transform) in &mut dummies {
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

pub fn pose_hp_bars(
    camera: Query<&GlobalTransform, With<MainCamera>>,
    dummy: Query<(&DummyPose, &GlobalTransform), With<DummyPawn>>,
    remotes: Query<(&ServerPose, &GlobalTransform), With<RemotePlayer>>,
    mut bars: Query<(&ChildOf, &mut Transform), With<HpBar>>,
) {
    let Ok(cam) = camera.single() else {
        return;
    };
    let cam_pos = cam.translation();
    for (parent, mut tf) in &mut bars {
        let parent_e = parent.parent();
        let (hp, alive, parent_tf) = if let Ok((pose, g)) = dummy.get(parent_e) {
            (pose.hp, pose.alive, *g)
        } else if let Ok((pose, g)) = remotes.get(parent_e) {
            (pose.hp, pose.alive, *g)
        } else {
            continue;
        };
        let cam_local = parent_tf.affine().inverse().transform_point3(cam_pos);
        let ratio = if alive {
            (hp / MAX_HP).clamp(0.05, 1.0)
        } else {
            0.05
        };
        *tf = Transform::from_xyz(0.0, 1.28, 0.0)
            .looking_at(cam_local, Vec3::Y)
            .with_scale(Vec3::new(ratio, 1.0, 1.0));
    }
}

pub fn sync_nameplates(
    mut commands: Commands,
    dummy: Query<Entity, With<DummyPawn>>,
    remotes: Query<Entity, With<RemotePlayer>>,
    plates: Query<(Entity, &Nameplate)>,
) {
    let mut wanted: Vec<Entity> = dummy.iter().collect();
    wanted.extend(remotes.iter());
    for target in &wanted {
        if plates.iter().any(|(_, p)| p.target == *target) {
            continue;
        }
        commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(-40.0),
                left: Val::Px(-40.0),
                ..default()
            },
            Text::new(""),
            TextFont::from_font_size(13.0),
            TextColor(Color::srgb(0.95, 0.93, 0.86)),
            TextLayout::no_wrap(),
            Nameplate { target: *target },
        ));
    }
    for (e, plate) in &plates {
        if !wanted.contains(&plate.target) {
            commands.entity(e).despawn();
        }
    }
}

pub fn update_nameplates(
    camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    dummy: Query<(&DummyPose, &GlobalTransform), With<DummyPawn>>,
    remotes: Query<(&ServerPose, &GlobalTransform), With<RemotePlayer>>,
    mut plates: Query<(&Nameplate, &mut Node, &mut Text, &mut TextColor)>,
) {
    let Ok((cam, cam_tf)) = camera.single() else {
        return;
    };
    for (plate, mut node, mut text, mut color) in &mut plates {
        let (world, label, hp, alive) = if let Ok((pose, g)) = dummy.get(plate.target) {
            (
                g.translation() + Vec3::Y * 1.15,
                "Dummy".to_string(),
                pose.hp,
                pose.alive,
            )
        } else if let Ok((pose, g)) = remotes.get(plate.target) {
            (
                g.translation() + Vec3::Y * 1.05,
                pose.name.clone(),
                pose.hp,
                pose.alive,
            )
        } else {
            node.top = Val::Px(-80.0);
            continue;
        };
        let Ok(screen) = cam.world_to_viewport(cam_tf, world) else {
            node.top = Val::Px(-80.0);
            continue;
        };
        // Keep plates out of the top-left HUD and off-screen.
        if screen.x < 400.0 && screen.y < 150.0 || screen.y < 0.0 || screen.x < 0.0 {
            node.top = Val::Px(-80.0);
            continue;
        }
        node.left = Val::Px(screen.x - 28.0);
        node.top = Val::Px(screen.y - 18.0);
        text.0 = if alive {
            format!("{label}  {hp:.0}")
        } else {
            format!("{label}  down")
        };
        color.0 = if alive {
            Color::srgb(0.95, 0.93, 0.86)
        } else {
            Color::srgb(0.55, 0.55, 0.55)
        };
    }
}

pub fn tick_dummy_pose(time: Res<Time>, mut dummies: Query<&mut DummyPose>) {
    for mut pose in &mut dummies {
        if pose.action_ticks > 0.0 {
            pose.action_ticks = (pose.action_ticks - time.delta_secs() * TICK_HZ).max(0.0);
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
    mut shots: Query<(Entity, &mut ShotPawn, &mut Transform)>,
) {
    if let Some(conn) = conn.as_ref() {
        for row in conn.db().projectile().iter() {
            if shots.iter_mut().any(|(_, s, _)| s.id == row.id) {
                continue;
            }
            spawn_shot(&mut commands, &mut meshes, &mut materials, &row);
        }
    }
    for msg in inserts.read() {
        if shots.iter_mut().any(|(_, s, _)| s.id == msg.row.id) {
            continue;
        }
        spawn_shot(&mut commands, &mut meshes, &mut materials, &msg.row);
    }
    for msg in updates.read() {
        for (_, mut shot, mut transform) in &mut shots {
            if shot.id == msg.new.id {
                shot.vx = msg.new.vx;
                shot.vz = msg.new.vz;
                transform.translation = Vec3::new(msg.new.x, 1.1, msg.new.z);
                aim_shot(&mut transform, shot.vx, shot.vz);
            }
        }
    }
    for msg in deletes.read() {
        for (e, shot, _) in &mut shots {
            if shot.id == msg.row.id {
                commands.entity(e).despawn();
            }
        }
    }
}

pub fn fly_shots(time: Res<Time>, mut shots: Query<(&ShotPawn, &mut Transform)>) {
    let dt = time.delta_secs();
    for (shot, mut transform) in &mut shots {
        transform.translation.x += shot.vx * dt;
        transform.translation.z += shot.vz * dt;
        aim_shot(&mut transform, shot.vx, shot.vz);
    }
}

fn aim_shot(transform: &mut Transform, vx: f32, vz: f32) {
    if vx * vx + vz * vz < 1e-6 {
        return;
    }
    let yaw = (-vx).atan2(-vz);
    transform.rotation = Quat::from_rotation_y(yaw);
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
    vitals.dummy_dist = 999.0;
    let me_xz = local
        .single()
        .ok()
        .map(|t| (t.translation.x, t.translation.z));
    for d in conn.db().dummy().iter() {
        vitals.dummy_hp = d.hp;
        vitals.dummy_alive = d.alive;
        if let Some((x, z)) = me_xz {
            vitals.dummy_dist = (x - d.x).hypot(z - d.z);
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
    conn: Option<Res<StdbConn>>,
    mut flash: ResMut<HitFlash>,
    mut control: ResMut<ControlState>,
    mut commands: Commands,
    camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
) {
    let me = conn.and_then(|c| c.try_identity());
    let cam = camera.single().ok();
    for msg in events.read() {
        let row = &msg.row;
        let involved = me == Some(row.target) || me == Some(row.attacker);
        if (row.kind == 1 || row.kind == 2) && involved {
            if me == Some(row.target) && !row.target_is_dummy {
                if let Ok(mut t) = local.single_mut() {
                    t.translation.x = row.x;
                    t.translation.z = row.z;
                    t.translation.y = PLAYER_HEIGHT * 0.5 + 0.06;
                }
                flash.t = 0.16;
                control.shake = control.shake.max(0.16);
            } else if me == Some(row.attacker) && !row.attacker_is_dummy {
                control.shake = control.shake.max(0.08);
            }
        }
        if let Some((cam, cam_tf)) = cam {
            if let Ok(screen) = cam.world_to_viewport(cam_tf, Vec3::new(row.x, 1.9, row.z)) {
                let (text, color) = match row.kind {
                    1 => (format!("{:.0}", row.damage), Color::srgb(0.95, 0.82, 0.45)),
                    2 => ("KILL".into(), Color::srgb(0.95, 0.35, 0.22)),
                    3 => ("BLOCK".into(), Color::srgb(0.55, 0.75, 0.95)),
                    4 => ("DODGE".into(), Color::srgb(0.75, 0.9, 0.55)),
                    5 => (
                        format!("+{:.0}xp", row.damage),
                        Color::srgb(0.55, 0.85, 0.45),
                    ),
                    _ => continue,
                };
                commands.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(screen.x),
                        top: Val::Px(screen.y),
                        ..default()
                    },
                    Text::new(text),
                    TextFont::from_font_size(16.0),
                    TextColor(color),
                    DamageFloater { age: 0.0 },
                ));
            }
        }
    }
}

pub fn update_floaters(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut Node, &mut TextColor, &mut DamageFloater)>,
) {
    for (e, mut node, mut color, mut floater) in &mut q {
        floater.age += time.delta_secs();
        if let Val::Px(top) = node.top {
            node.top = Val::Px(top - 42.0 * time.delta_secs());
        }
        let alpha = (1.0 - floater.age / 0.85).clamp(0.0, 1.0);
        color.0.set_alpha(alpha);
        if floater.age > 0.85 {
            commands.entity(e).despawn();
        }
    }
}

pub fn tick_hit_flash(
    time: Res<Time>,
    mut flash: ResMut<HitFlash>,
    local: Query<&MeshMaterial3d<StandardMaterial>, With<LocalPlayer>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if flash.t <= 0.0 {
        return;
    }
    flash.t = (flash.t - time.delta_secs()).max(0.0);
    let mix = (flash.t / 0.16).clamp(0.0, 1.0);
    let Ok(mat) = local.single() else {
        return;
    };
    if let Some(mut m) = materials.get_mut(&mat.0) {
        let base = Color::srgb(0.82, 0.62, 0.28);
        m.base_color = base.mix(&Color::srgb(0.95, 0.25, 0.18), mix);
    }
}

pub fn apply_predicted_starts(
    mut control: ResMut<ControlState>,
    vitals: Res<LocalVitals>,
    mut local: Query<&mut Transform, With<LocalPlayer>>,
) {
    if control.pred_action == ACTION_BLOCK && (control.buttons & BTN_BLOCK) == 0 {
        control.pred_action = ACTION_NONE;
        control.pred_ticks = 0.0;
    }
    if control.drawn && control.pred_action == unbound_shared::ACTION_GATHER {
        control.pred_action = ACTION_NONE;
        control.pred_ticks = 0.0;
    }
    let buttons = merge_input_buttons(control.buttons, control.latched);
    let start = if control.drawn {
        start_drawn_action(
            control.pred_action,
            control.pred_loadout,
            control.loadout,
            control.pred_stamina,
            buttons,
        )
    } else {
        start_gather_action(
            control.pred_action,
            buttons,
            vitals.node_dist <= GATHER_RANGE,
        )
    };
    let Some(start) = start else {
        return;
    };
    control.pred_action = start.action;
    control.pred_ticks = predicted_busy_ticks(&start) as f32;
    control.pred_loadout = if start.action == unbound_shared::ACTION_GATHER {
        control.pred_loadout
    } else {
        start.loadout
    };
    if start.action != unbound_shared::ACTION_GATHER {
        control.pred_stamina = start.stamina;
    }
    if start.action == ACTION_DODGE {
        if let Ok(mut transform) = local.single_mut() {
            let (dx, dz) = dodge_dir(control.dir_x, control.dir_z);
            let (x, z) = integrate(
                transform.translation.x,
                transform.translation.z,
                control.yaw,
                dx,
                dz,
                dodge_burst_dt(),
                DODGE_SPEED,
            );
            transform.translation.x = x;
            transform.translation.z = z;
        }
    }
}

pub fn tick_prediction(time: Res<Time>, mut control: ResMut<ControlState>) {
    let dt = time.delta_secs();
    if control.pred_ticks > 0.0 {
        control.pred_ticks = (control.pred_ticks - dt * TICK_HZ).max(0.0);
        if control.pred_ticks <= 0.0 && control.pred_action != ACTION_BLOCK {
            control.pred_action = ACTION_NONE;
        }
    }
    let sprinting = (control.buttons & BTN_SPRINT) != 0
        && control.pred_stamina > 1.0
        && control.pred_action != ACTION_DODGE
        && !unbound_shared::move_lock(control.pred_action);
    if sprinting {
        control.pred_stamina = (control.pred_stamina - SPRINT_STAMINA_PER_SEC * dt).max(0.0);
    } else if control.pred_action != ACTION_DODGE && control.pred_action != ACTION_BLOCK {
        control.pred_stamina = (control.pred_stamina + STAMINA_REGEN_PER_SEC * dt).min(MAX_STAMINA);
    }
}

pub fn pose_weapons(
    control: Res<ControlState>,
    remotes: Query<(Entity, &ServerPose), With<RemotePlayer>>,
    local: Query<Entity, With<LocalPlayer>>,
    mut weapons: Query<(&WeaponVisual, &mut Transform, Option<&ChildOf>)>,
) {
    let local_e = local.single().ok();
    for (visual, mut transform, parent) in &mut weapons {
        let Some(parent) = parent else {
            continue;
        };
        let (action, ticks, loadout) = if local_e == Some(parent.parent()) {
            (
                control.pred_action,
                control.pred_ticks,
                control.pred_loadout,
            )
        } else if let Some((_, pose)) = remotes.iter().find(|(e, _)| *e == parent.parent()) {
            (pose.action, pose.action_ticks as f32, pose.loadout)
        } else {
            continue;
        };
        let (pitch, yaw, roll) = weapon_extra_rotation(action, ticks, loadout);
        *transform = visual.rest
            * Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, pitch, yaw, roll));
    }
}

pub fn pose_dummy_club(
    dummies: Query<&DummyPose, With<DummyPawn>>,
    mut clubs: Query<(&DummyClub, &mut Transform, Option<&ChildOf>)>,
) {
    let dummy = dummies.single().ok();
    for (club, mut transform, parent) in &mut clubs {
        let Some(_parent) = parent else {
            continue;
        };
        let Some(pose) = dummy else {
            continue;
        };
        let pitch = dummy_club_pitch(
            pose.action,
            pose.action_ticks,
            dummy_windup_ticks(pose.action) as f32,
        );
        *transform = club.rest * Transform::from_rotation(Quat::from_rotation_x(pitch));
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
        let current = weapons
            .iter()
            .find(|(_, _vis, parent)| parent.map(|p| p.parent() == entity).unwrap_or(false));
        let mismatch = current
            .map(|(_, vis, _)| vis.loadout != pose.loadout)
            .unwrap_or(false);
        if !pose.drawn || mismatch {
            for (w, _, parent) in &weapons {
                if parent.map(|p| p.parent() == entity).unwrap_or(false) {
                    commands.entity(w).despawn();
                }
            }
        }
        if pose.drawn && (current.is_none() || mismatch) {
            attach_weapon(
                &mut commands,
                &mut meshes,
                &mut materials,
                entity,
                pose.loadout,
                true,
            );
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

fn dummy_color(action: u8, alive: bool) -> Color {
    if !alive {
        Color::srgb(0.15, 0.15, 0.15)
    } else if action == ACTION_HEAVY {
        Color::srgb(1.0, 0.55, 0.08)
    } else if action == ACTION_LIGHT {
        Color::srgb(0.95, 0.72, 0.18)
    } else if action == ACTION_HIT {
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
            WeaponVisual {
                loadout: loadout_id,
                rest: tf,
            },
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
    let club_rest = Transform::from_xyz(0.45, 0.1, -0.35);
    let root = commands
        .spawn((
            Mesh3d(meshes.add(Capsule3d::new(0.42, 1.05))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: dummy_color(dummy.action, dummy.alive),
                perceptual_roughness: 0.65,
                ..default()
            })),
            Transform::from_xyz(dummy.x, PLAYER_HEIGHT * 0.5, dummy.z)
                .with_rotation(Quat::from_rotation_y(dummy.yaw)),
            DummyPawn,
            DummyPose {
                x: dummy.x,
                z: dummy.z,
                yaw: dummy.yaw,
                action: dummy.action,
                action_ticks: dummy.action_ticks as f32,
                hp: dummy.hp,
                alive: dummy.alive,
            },
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
            club_rest,
            DummyClub { rest: club_rest },
        ))
        .id();
    let bar = commands
        .spawn((
            Mesh3d(meshes.add(Cuboid::new(1.15, 0.12, 0.04))),
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
            HpBar,
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
    let staff = shot.skill == 2;
    let color = if staff {
        Color::srgb(0.55, 0.35, 0.95)
    } else {
        Color::srgb(0.85, 0.7, 0.25)
    };
    let mesh = if staff {
        meshes.add(Sphere::new(0.14))
    } else {
        meshes.add(Cuboid::new(0.07, 0.07, 0.55))
    };
    let mut tf = Transform::from_xyz(shot.x, 1.1, shot.z);
    aim_shot(&mut tf, shot.vx, shot.vz);
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            emissive: LinearRgba::from(color) * 4.0,
            ..default()
        })),
        tf,
        ShotPawn {
            id: shot.id,
            vx: shot.vx,
            vz: shot.vz,
        },
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
