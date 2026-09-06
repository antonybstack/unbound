use bevy::prelude::*;
use bevy_stdb::prelude::*;
use unbound_shared::{
    aim_dir, death_started, dodge_burst_dt, dodge_dir, dummy_body_scale, dummy_club_pitch,
    dummy_heavy_slammed, dummy_hp_bar_hit, dummy_telegraph_started, dummy_windup_ticks,
    hp_bar_tint, hyperarmor, hyperarmor_flash_emissive, hyperarmor_flash_scale, incoming_hit_shake,
    integrate, invulnerable_for, life_started, loadout, melee_lunge_dt, merge_input_buttons,
    nameplate_alpha, node_mesh_scale, node_respawned, node_restore_mix, predicted_busy_ticks,
    predicted_release_ticks, start_drawn_action, start_gather_action, wanderer_hp_bar_hit,
    wanderer_hp_bar_tint, weapon_extra_rotation,
    ACTION_BLOCK, ACTION_DEAD, ACTION_DODGE, ACTION_HEAVY, ACTION_HIT, ACTION_LIGHT, ACTION_NONE,
    BTN_BLOCK, BTN_DODGE, BTN_HEAVY, BTN_LIGHT, BTN_SPRINT, DODGE_SPEED, GATHER_RANGE,
    HP_FLASH_TIME, HYPERARMOR_FLASH_TIME, MAX_HP, MAX_STAMINA, MOVE_SPEED, PLAYER_HEIGHT,
    SHOT_CEILING_Y, SHOT_GROUND_Y, SHOT_SPAWN_Y, SPRINT_STAMINA_PER_SEC, STAMINA_REGEN_PER_SEC,
    TICK_HZ,
};

use crate::camera::ControlState;
use crate::module_bindings::{
    character_table::characterQueryTableAccess, combat_event_table::combat_eventQueryTableAccess,
    dummy_table::dummyQueryTableAccess, gather_node_table::gather_nodeQueryTableAccess,
    player_table::playerQueryTableAccess, projectile_table::projectileQueryTableAccess, Character,
    CharacterTableAccess, CombatEvent, Dummy, DummyTableAccess, GatherNode, GatherNodeTableAccess,
    Player, PlayerTableAccess, Projectile, ProjectileTableAccess,
};
use crate::net::{LocalPlayer, NetworkedIdentity, RemotePlayer, ServerPose};
use crate::{MainCamera, StdbConn, StdbSubs, SubKey};
use spacetimedb_sdk::Table;

#[derive(Component)]
pub struct DummyPawn;

#[derive(Component)]
pub struct HpBar {
    pub fade: f32,
    pub flash: f32,
}

#[derive(Component)]
pub struct Nameplate {
    pub target: Entity,
    pub alpha: f32,
}

#[derive(Component)]
pub struct ShotPawn {
    pub id: u32,
    pub vx: f32,
    pub vy: f32,
    pub vz: f32,
    pub skill: u8,
}

#[derive(Component)]
pub struct PredictedShot {
    pub vx: f32,
    pub vy: f32,
    pub vz: f32,
    pub skill: u8,
}

#[derive(Component)]
pub struct WeaponVisual {
    pub loadout: u8,
    pub rest: Transform,
    pub drawn: bool,
}

#[derive(Component)]
pub struct ShieldVisual {
    pub rest: Transform,
}

#[derive(Component)]
pub struct DummyClub {
    pub rest: Transform,
}

#[derive(Component)]
pub struct DustPuff {
    pub age: f32,
}

#[derive(Component)]
pub struct HitSpark {
    pub age: f32,
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
    pub pending_hit: bool,
}

#[derive(Component)]
pub struct DummyStep {
    pub accum: f32,
    pub last_x: f32,
    pub last_z: f32,
    pub speed: f32,
    pub since: f32,
}

impl DummyStep {
    pub fn new(x: f32, z: f32) -> Self {
        Self {
            accum: 0.18,
            last_x: x,
            last_z: z,
            speed: 0.0,
            since: 0.0,
        }
    }
}

#[derive(Resource, Default)]
pub struct HitFlash {
    pub t: f32,
}

#[derive(Resource, Default)]
pub struct StamFlash {
    pub t: f32,
}

#[derive(Resource, Default)]
pub struct GatherHintFlash {
    pub t: f32,
    pub was_in: bool,
}

#[derive(Resource, Default)]
pub struct DummyArmorFlash {
    pub t: f32,
}

#[derive(Resource, Default)]
pub struct DummyHpFlash {
    pub t: f32,
}

#[derive(Resource, Default)]
pub struct XpBarFlash {
    pub t: f32,
}

#[derive(Component)]
pub struct DamageFloater {
    pub age: f32,
}

#[derive(Component)]
pub struct NodePawn {
    pub id: u32,
    pub kind: u8,
    pub charges: u8,
    pub restore: f32,
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
    pub melee_xp: u64,
    pub ranged_xp: u64,
    pub magic_xp: u64,
    pub defence_xp: u64,
    pub hitpoints_xp: u64,
    pub gather_xp: u64,
    pub alive: bool,
    pub dummy_hp: f32,
    pub dummy_alive: bool,
    pub dummy_dist: f32,
    pub node_kind: u8,
    pub node_dist: f32,
    pub others: String,
    pub log: String,
    pub skills_primed: bool,
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
            melee_xp: 0,
            ranged_xp: 0,
            magic_xp: 0,
            defence_xp: 0,
            hitpoints_xp: 0,
            gather_xp: 0,
            alive: true,
            dummy_hp: MAX_HP,
            dummy_alive: true,
            dummy_dist: 999.0,
            node_kind: 0,
            node_dist: 999.0,
            others: String::new(),
            log: String::new(),
            skills_primed: false,
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
    mut control: ResMut<ControlState>,
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
            if let Some(kind) = dummy_telegraph_started(
                pose.action,
                pose.pending_hit,
                msg.new.action,
                msg.new.pending_hit,
            ) {
                control.sfx_dummy = if kind == ACTION_HEAVY { 2 } else { 1 };
                control.pip_pulse = unbound_shared::DUMMY_PIP_PULSE_TIME;
            }
            if dummy_heavy_slammed(
                pose.action,
                pose.pending_hit,
                msg.new.action,
                msg.new.pending_hit,
            ) {
                let at = Vec3::new(msg.new.x, 0.0, msg.new.z);
                spawn_dust(&mut commands, &mut meshes, &mut materials, at);
                spawn_hit_spark(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    at + Vec3::Y * 0.22,
                    Color::srgb(0.82, 0.62, 0.28),
                );
            }
            if death_started(pose.alive, msg.new.alive) {
                control.sfx_death = true;
            }
            if life_started(pose.alive, msg.new.alive) {
                control.sfx_rise = true;
            }
            pose.x = msg.new.x;
            pose.z = msg.new.z;
            pose.yaw = msg.new.yaw;
            pose.action = msg.new.action;
            pose.action_ticks = msg.new.action_ticks as f32;
            pose.hp = msg.new.hp;
            pose.alive = msg.new.alive;
            pose.pending_hit = msg.new.pending_hit;
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
    flash: Res<DummyArmorFlash>,
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
        let (sx, sy, sz) = dummy_body_scale(pose.action, pose.alive);
        let mut scale = Vec3::new(sx, sy, sz);
        if pose.alive {
            scale *= hyperarmor_flash_scale(flash.t);
        }
        transform.scale = transform.scale.lerp(scale, t);
    }
}

pub fn pose_hp_bars(
    time: Res<Time>,
    dummy_hp_flash: Res<DummyHpFlash>,
    camera: Query<&GlobalTransform, With<MainCamera>>,
    dummy: Query<(&DummyPose, &GlobalTransform), With<DummyPawn>>,
    remotes: Query<(&ServerPose, &GlobalTransform), With<RemotePlayer>>,
    mut bars: Query<(
        &ChildOf,
        &mut Transform,
        &mut HpBar,
        &MeshMaterial3d<StandardMaterial>,
    )>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Ok(cam) = camera.single() else {
        return;
    };
    let cam_pos = cam.translation();
    let dt = time.delta_secs();
    for (parent, mut tf, mut bar, mat) in &mut bars {
        let parent_e = parent.parent();
        let (hp, alive, parent_tf, is_dummy) = if let Ok((pose, g)) = dummy.get(parent_e) {
            (pose.hp, pose.alive, *g, true)
        } else if let Ok((pose, g)) = remotes.get(parent_e) {
            (pose.hp, pose.alive, *g, false)
        } else {
            continue;
        };
        let cam_local = parent_tf.affine().inverse().transform_point3(cam_pos);
        let ratio = if alive {
            (hp / MAX_HP).clamp(0.05, 1.0)
        } else {
            0.05
        };
        bar.fade = nameplate_alpha(alive, bar.fade, dt);
        bar.flash = (bar.flash - dt).max(0.0);
        *tf = Transform::from_xyz(0.0, 1.28, 0.0)
            .looking_at(cam_local, Vec3::Y)
            .with_scale(Vec3::new(ratio, 1.0, 1.0));
        if let Some(mut m) = materials.get_mut(&mat.0) {
            if is_dummy {
                let (r, g, b) = hp_bar_tint(dummy_hp_flash.t);
                let mut color = Color::srgb(r, g, b);
                color.set_alpha(bar.fade);
                m.base_color = color;
            } else {
                let (r, g, b) = wanderer_hp_bar_tint(bar.flash);
                let mut color = Color::srgb(r, g, b);
                color.set_alpha(bar.fade);
                m.base_color = color;
            }
            m.alpha_mode = if bar.fade < 0.999 {
                AlphaMode::Blend
            } else {
                AlphaMode::Opaque
            };
        }
    }
}

pub fn sync_nameplates(
    mut commands: Commands,
    dummy: Query<(Entity, &DummyPose), With<DummyPawn>>,
    remotes: Query<(Entity, &ServerPose), With<RemotePlayer>>,
    plates: Query<(Entity, &Nameplate)>,
) {
    let mut wanted: Vec<(Entity, bool)> = dummy.iter().map(|(e, p)| (e, p.alive)).collect();
    wanted.extend(remotes.iter().map(|(e, p)| (e, p.alive)));
    for (target, alive) in &wanted {
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
            Pickable::IGNORE,
            Nameplate {
                target: *target,
                alpha: if *alive { 1.0 } else { 0.0 },
            },
        ));
    }
    for (e, plate) in &plates {
        if !wanted.iter().any(|(t, _)| *t == plate.target) {
            commands.entity(e).despawn();
        }
    }
}

pub fn update_nameplates(
    time: Res<Time>,
    camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    dummy: Query<(&DummyPose, &GlobalTransform), With<DummyPawn>>,
    remotes: Query<(&ServerPose, &GlobalTransform), With<RemotePlayer>>,
    mut plates: Query<(&mut Nameplate, &mut Node, &mut Text, &mut TextColor)>,
) {
    let Ok((cam, cam_tf)) = camera.single() else {
        return;
    };
    let dt = time.delta_secs();
    for (mut plate, mut node, mut text, mut color) in &mut plates {
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
        plate.alpha = nameplate_alpha(alive, plate.alpha, dt);
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
            Color::srgba(0.95, 0.93, 0.86, plate.alpha)
        } else {
            Color::srgba(0.55, 0.55, 0.55, plate.alpha)
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
    ghosts: Query<(Entity, &Transform), (With<PredictedShot>, Without<ShotPawn>)>,
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
        // Authoritative bolt arrived; drop the local ghost so we don't draw two.
        for (e, ghost) in ghosts.iter() {
            let dx = ghost.translation.x - msg.row.x;
            let dz = ghost.translation.z - msg.row.z;
            if dx * dx + dz * dz < 16.0 {
                commands.entity(e).despawn();
            }
        }
    }
    for msg in updates.read() {
        for (_, mut shot, mut transform) in &mut shots {
            if shot.id == msg.new.id {
                shot.vx = msg.new.vx;
                shot.vy = msg.new.vy;
                shot.vz = msg.new.vz;
                transform.translation = Vec3::new(msg.new.x, msg.new.y, msg.new.z);
                aim_shot(&mut transform, shot.vx, shot.vy, shot.vz);
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

pub fn fly_shots(time: Res<Time>, mut shots: Query<(&mut ShotPawn, &mut Transform)>) {
    let dt = time.delta_secs();
    for (mut shot, mut transform) in &mut shots {
        shot.vy -= unbound_shared::shot_gravity(shot.skill) * dt;
        transform.translation.x += shot.vx * dt;
        transform.translation.y += shot.vy * dt;
        transform.translation.z += shot.vz * dt;
        aim_shot(&mut transform, shot.vx, shot.vy, shot.vz);
    }
}

fn aim_shot(transform: &mut Transform, vx: f32, vy: f32, vz: f32) {
    let dir = Vec3::new(vx, vy, vz);
    if dir.length_squared() < 1e-6 {
        return;
    }
    transform.look_to(dir, Vec3::Y);
}

pub fn sync_nodes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut inserts: ReadInsertMessage<GatherNode>,
    mut updates: ReadUpdateMessage<GatherNode>,
    mut deletes: ReadDeleteMessage<GatherNode>,
    conn: Option<Res<StdbConn>>,
    time: Res<Time>,
    mut control: ResMut<ControlState>,
    local: Query<&Transform, With<LocalPlayer>>,
    mut nodes: Query<
        (
            Entity,
            &mut NodePawn,
            &mut Transform,
            &mut MeshMaterial3d<StandardMaterial>,
        ),
        Without<LocalPlayer>,
    >,
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
        for (_e, mut node, transform, _mat) in &mut nodes {
            if node.id != msg.new.id {
                continue;
            }
            let emptied = node.charges > 0 && msg.new.charges == 0;
            let returned = node_respawned(node.charges, msg.new.charges);
            node.charges = msg.new.charges;
            if emptied || returned {
                let at = transform.translation;
                let color = if msg.new.kind == 1 {
                    Color::srgb(0.78, 0.84, 0.92)
                } else {
                    Color::srgb(0.62, 0.88, 0.42)
                };
                if emptied {
                    spawn_hit_spark(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        at + Vec3::Y * 0.35,
                        color,
                    );
                    spawn_hit_spark(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        at + Vec3::new(0.22, 0.12, 0.08),
                        color,
                    );
                    spawn_dust(&mut commands, &mut meshes, &mut materials, at);
                    control.sfx_deplete = if msg.new.kind == 1 { 2 } else { 1 };
                }
                if returned {
                    spawn_hit_spark(
                        &mut commands,
                        &mut meshes,
                        &mut materials,
                        at + Vec3::Y * 0.25,
                        color,
                    );
                    control.sfx_respawn = if msg.new.kind == 1 { 2 } else { 1 };
                }
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
    let gathering = control.pred_action == unbound_shared::ACTION_GATHER;
    let me = local.single().ok().map(|t| t.translation);
    let pulse = 1.0 + 0.07 * (time.elapsed_secs() * 10.0).sin();
    let dt = time.delta_secs();
    for (_e, mut node, mut transform, mat) in &mut nodes {
        let prev = node.restore;
        node.restore = node_restore_mix(node.charges, node.restore, dt);
        if node.restore < 1.0 || prev < 1.0 {
            let (sx, sy, sz) = node_mesh_scale(node.restore);
            transform.scale = Vec3::new(sx, sy, sz);
            if let Some(mut m) = materials.get_mut(&mat.0) {
                m.base_color = node_color(node.kind, node.restore);
            }
        }
        if node.restore < 1.0 {
            continue;
        }
        let near = me
            .map(|p| {
                (p.x - transform.translation.x).hypot(p.z - transform.translation.z) < GATHER_RANGE
            })
            .unwrap_or(false);
        if gathering && near {
            transform.scale = Vec3::splat(pulse);
        } else if transform.scale.x > 1.01 || transform.scale.x < 0.99 {
            transform.scale = Vec3::ONE;
        }
    }
}

pub fn sync_vitals(
    mut vitals: ResMut<LocalVitals>,
    mut control: ResMut<ControlState>,
    mut xp_flash: ResMut<XpBarFlash>,
    conn: Option<Res<StdbConn>>,
    mut characters: ReadInsertMessage<Character>,
    mut char_updates: ReadUpdateMessage<Character>,
    mut player_updates: ReadUpdateMessage<Player>,
    mut events: ReadInsertMessage<CombatEvent>,
    local: Query<&Transform, With<LocalPlayer>>,
    mut commands: Commands,
    camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
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

    let cam = camera.single().ok();
    let me_pos = local.single().ok().map(|t| t.translation);
    let mut apply_one = |c: &Character| {
        if c.identity != me {
            return;
        }
        let primed = vitals.skills_primed;
        let bump = |old: u8, xp: u64| -> (u8, Option<u8>) {
            let new = unbound_shared::skill_level(xp);
            (
                new,
                unbound_shared::xp_bar_levelled(primed, old, new).then_some(new),
            )
        };
        vitals.name = c.name.clone();
        let (melee, up_m) = bump(vitals.melee, c.melee_xp);
        let (ranged, up_r) = bump(vitals.ranged, c.ranged_xp);
        let (magic, up_g) = bump(vitals.magic, c.magic_xp);
        let (defence, up_d) = bump(vitals.defence, c.defence_xp);
        let (hitpoints, up_h) = bump(vitals.hitpoints, c.hitpoints_xp);
        let (gather, up_a) = bump(vitals.gather, c.gather_xp);
        vitals.melee = melee;
        vitals.ranged = ranged;
        vitals.magic = magic;
        vitals.defence = defence;
        vitals.hitpoints = hitpoints;
        vitals.gather = gather;
        vitals.melee_xp = c.melee_xp;
        vitals.ranged_xp = c.ranged_xp;
        vitals.magic_xp = c.magic_xp;
        vitals.defence_xp = c.defence_xp;
        vitals.hitpoints_xp = c.hitpoints_xp;
        vitals.gather_xp = c.gather_xp;
        vitals.skills_primed = true;
        let ups = [
            (unbound_shared::SKILL_MELEE, up_m),
            (unbound_shared::SKILL_RANGED, up_r),
            (unbound_shared::SKILL_MAGIC, up_g),
            (unbound_shared::SKILL_DEFENCE, up_d),
            (unbound_shared::SKILL_HITPOINTS, up_h),
            (unbound_shared::SKILL_GATHERING, up_a),
        ];
        for (skill, up) in ups {
            let Some(lvl) = up else { continue };
            control.sfx_level = true;
            xp_flash.t = unbound_shared::XP_FLASH_TIME;
            let text = format!("{} {}", unbound_shared::skill_label(skill), lvl);
            vitals.log = text.clone();
            if let (Some((cam, cam_tf)), Some(pos)) = (cam, me_pos) {
                spawn_world_floater(
                    &mut commands,
                    cam,
                    cam_tf,
                    pos + Vec3::Y * 1.2,
                    text,
                    Color::srgb(0.95, 0.85, 0.35),
                );
            }
        }
    };
    for c in conn.db().character().iter() {
        apply_one(&c);
    }
    for msg in characters.read() {
        apply_one(&msg.row);
    }
    for msg in char_updates.read() {
        apply_one(&msg.new);
    }

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
            6 => "guard break".into(),
            _ => vitals.log.clone(),
        };
    }
}

pub fn flash_hits(
    mut events: ReadInsertMessage<CombatEvent>,
    mut local: Query<&mut Transform, With<LocalPlayer>>,
    dummy: Query<&DummyPose, With<DummyPawn>>,
    remotes: Query<&NetworkedIdentity, With<RemotePlayer>>,
    mut bars: Query<(&ChildOf, &mut HpBar)>,
    conn: Option<Res<StdbConn>>,
    mut flash: ResMut<HitFlash>,
    mut armor: ResMut<DummyArmorFlash>,
    mut dummy_hp: ResMut<DummyHpFlash>,
    mut control: ResMut<ControlState>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    camera: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
) {
    let me = conn.and_then(|c| c.try_identity());
    let cam = camera.single().ok();
    for msg in events.read() {
        let row = &msg.row;
        let involved = me == Some(row.target) || me == Some(row.attacker);
        if (row.kind == 1 || row.kind == 2 || row.kind == 6) && involved {
            if me == Some(row.target) && !row.target_is_dummy {
                if let Ok(mut t) = local.single_mut() {
                    t.translation.x = row.x;
                    t.translation.z = row.z;
                    t.translation.y = PLAYER_HEIGHT * 0.5 + 0.06;
                }
                flash.t = HP_FLASH_TIME;
                control.shake = control.shake.max(incoming_hit_shake(
                    row.kind,
                    row.damage,
                    row.attacker_is_dummy,
                ));
            } else if me == Some(row.attacker) && !row.attacker_is_dummy {
                control.shake = control.shake.max(0.08);
                control.hitstop =
                    control
                        .hitstop
                        .max(if row.damage >= 20.0 { 0.10 } else { 0.045 });
            }
        }
        if dummy_hp_bar_hit(row.kind, row.target_is_dummy) {
            dummy_hp.t = HP_FLASH_TIME;
        }
        if wanderer_hp_bar_hit(row.kind, row.target_is_dummy) {
            for (parent, mut bar) in &mut bars {
                let Ok(id) = remotes.get(parent.parent()) else {
                    continue;
                };
                if id.identity == row.target {
                    bar.flash = HP_FLASH_TIME;
                }
            }
        }
        if row.kind == 1 && row.target_is_dummy {
            if let Ok(pose) = dummy.single() {
                if hyperarmor(pose.action, pose.pending_hit, true) {
                    armor.t = HYPERARMOR_FLASH_TIME;
                }
            }
        }
        let (text, color) = match row.kind {
            1 => (format!("{:.0}", row.damage), Color::srgb(0.95, 0.82, 0.45)),
            2 => ("KILL".into(), Color::srgb(0.95, 0.35, 0.22)),
            3 => ("BLOCK".into(), Color::srgb(0.55, 0.75, 0.95)),
            4 => ("DODGE".into(), Color::srgb(0.75, 0.9, 0.55)),
            5 => (
                format!("+{:.0}xp", row.damage),
                Color::srgb(0.55, 0.85, 0.45),
            ),
            6 => ("BREAK".into(), Color::srgb(0.95, 0.55, 0.22)),
            _ => continue,
        };
        if row.kind != 5 {
            spawn_hit_spark(
                &mut commands,
                &mut meshes,
                &mut materials,
                Vec3::new(row.x, 1.15, row.z),
                color,
            );
        }
        if let Some((cam, cam_tf)) = cam {
            spawn_world_floater(
                &mut commands,
                cam,
                cam_tf,
                Vec3::new(row.x, 1.9, row.z),
                text,
                color,
            );
        }
    }
}

fn spawn_world_floater(
    commands: &mut Commands,
    cam: &Camera,
    cam_tf: &GlobalTransform,
    world: Vec3,
    text: String,
    color: Color,
) {
    let Ok(screen) = cam.world_to_viewport(cam_tf, world) else {
        return;
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
    control: Res<ControlState>,
    local: Query<&MeshMaterial3d<StandardMaterial>, With<LocalPlayer>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if flash.t > 0.0 {
        flash.t = (flash.t - time.delta_secs()).max(0.0);
    }
    let Ok(mat) = local.single() else {
        return;
    };
    let Some(mut m) = materials.get_mut(&mat.0) else {
        return;
    };
    let ghost = invulnerable_for(
        control.pred_action,
        control.pred_ticks.round().clamp(0.0, 255.0) as u8,
        control.pred_loadout,
    );
    let mix = (flash.t / HP_FLASH_TIME).clamp(0.0, 1.0);
    let mut color = Color::srgb(0.82, 0.62, 0.28).mix(&Color::srgb(0.95, 0.25, 0.18), mix);
    if ghost {
        color = color.mix(&Color::srgb(0.95, 0.95, 1.0), 0.45);
        color.set_alpha(0.42);
        m.alpha_mode = AlphaMode::Blend;
    } else {
        color.set_alpha(1.0);
        m.alpha_mode = AlphaMode::Opaque;
    }
    m.base_color = color;
}

pub fn tick_dummy_armor_flash(
    time: Res<Time>,
    mut flash: ResMut<DummyArmorFlash>,
    mut dummy_hp: ResMut<DummyHpFlash>,
    dummy: Query<&MeshMaterial3d<StandardMaterial>, With<DummyPawn>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if flash.t > 0.0 {
        flash.t = (flash.t - time.delta_secs()).max(0.0);
    }
    if dummy_hp.t > 0.0 {
        dummy_hp.t = (dummy_hp.t - time.delta_secs()).max(0.0);
    }
    let Ok(mat) = dummy.single() else {
        return;
    };
    let Some(mut m) = materials.get_mut(&mat.0) else {
        return;
    };
    let e = hyperarmor_flash_emissive(flash.t);
    m.emissive = LinearRgba::rgb(1.0, 0.95, 0.82) * e;
}

pub fn tick_remote_ghost(
    remotes: Query<(&ServerPose, &MeshMaterial3d<StandardMaterial>), With<RemotePlayer>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (pose, mat) in &remotes {
        let Some(mut m) = materials.get_mut(&mat.0) else {
            continue;
        };
        let ghost = invulnerable_for(
            pose.action,
            pose.action_ticks.round().clamp(0.0, 255.0) as u8,
            pose.loadout,
        );
        let mut color = if pose.alive {
            Color::srgb(0.35, 0.48, 0.62)
        } else {
            Color::srgb(0.22, 0.24, 0.28)
        };
        if ghost {
            color = color.mix(&Color::srgb(0.95, 0.95, 1.0), 0.45);
            color.set_alpha(0.42);
            m.alpha_mode = AlphaMode::Blend;
        } else {
            color.set_alpha(1.0);
            m.alpha_mode = AlphaMode::Opaque;
        }
        m.base_color = color;
    }
}

pub fn apply_predicted_starts(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut control: ResMut<ControlState>,
    mut stam_flash: ResMut<StamFlash>,
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
        if (control.latched & (BTN_LIGHT | BTN_HEAVY | BTN_DODGE)) != 0 {
            stam_flash.t = 0.22;
            control.shake = control.shake.max(0.06);
        }
        return;
    };
    let prev = control.pred_action;
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
    if start.pending_hit && loadout(start.loadout).is_projectile {
        control.pred_shot = true;
    }
    if start.action == ACTION_LIGHT {
        control.sfx_swing = 1;
    } else if start.action == ACTION_HEAVY {
        control.sfx_swing = 2;
    }
    if start.action == unbound_shared::ACTION_GATHER && prev != unbound_shared::ACTION_GATHER {
        control.sfx_gather = true;
    }
    if start.action == ACTION_BLOCK && prev != ACTION_BLOCK {
        control.sfx_block = true;
    }
    if start.action == ACTION_DODGE {
        control.sfx_dodge = true;
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
            spawn_dust(
                &mut commands,
                &mut meshes,
                &mut materials,
                transform.translation,
            );
            transform.translation.x = x;
            transform.translation.z = z;
        }
    } else if start.pending_hit {
        let dt = melee_lunge_dt(start.action, loadout(start.loadout).is_projectile);
        if dt > 0.0 {
            if let Ok(mut transform) = local.single_mut() {
                let (x, z) = integrate(
                    transform.translation.x,
                    transform.translation.z,
                    control.yaw,
                    0.0,
                    1.0,
                    dt,
                    MOVE_SPEED,
                );
                transform.translation.x = x;
                transform.translation.z = z;
            }
        }
    }
}

fn spawn_hit_spark(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    at: Vec3,
    color: Color,
) {
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.16))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color.with_alpha(0.9),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            emissive: LinearRgba::from(color) * 3.0,
            ..default()
        })),
        Transform::from_translation(at),
        HitSpark { age: 0.0 },
    ));
}

pub fn tick_hit_sparks(
    time: Res<Time>,
    mut commands: Commands,
    mut sparks: Query<(
        Entity,
        &mut HitSpark,
        &mut Transform,
        &MeshMaterial3d<StandardMaterial>,
    )>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (e, mut spark, mut tf, mat) in &mut sparks {
        spark.age += time.delta_secs();
        let t = (spark.age / 0.18).clamp(0.0, 1.0);
        tf.scale = Vec3::splat(1.0 + t * 2.4);
        if let Some(mut m) = materials.get_mut(&mat.0) {
            m.base_color.set_alpha(0.9 * (1.0 - t));
        }
        if spark.age > 0.18 {
            commands.entity(e).despawn();
        }
    }
}

fn spawn_dust(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    at: Vec3,
) {
    commands.spawn((
        Mesh3d(meshes.add(Cylinder::new(0.45, 0.04))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgba(0.62, 0.55, 0.4, 0.55),
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        })),
        Transform::from_xyz(at.x, 0.04, at.z),
        DustPuff { age: 0.0 },
    ));
}

pub fn tick_dust(
    time: Res<Time>,
    mut commands: Commands,
    mut puffs: Query<(
        Entity,
        &mut DustPuff,
        &mut Transform,
        &MeshMaterial3d<StandardMaterial>,
    )>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (e, mut puff, mut tf, mat) in &mut puffs {
        puff.age += time.delta_secs();
        let t = (puff.age / 0.35).clamp(0.0, 1.0);
        tf.scale = Vec3::new(1.0 + t * 1.8, 1.0, 1.0 + t * 1.8);
        if let Some(mut m) = materials.get_mut(&mat.0) {
            m.base_color.set_alpha(0.5 * (1.0 - t));
        }
        if puff.age > 0.35 {
            commands.entity(e).despawn();
        }
    }
}

pub fn tick_prediction(time: Res<Time>, mut control: ResMut<ControlState>) {
    let mut dt = time.delta_secs();
    if control.hitstop > 0.0 {
        control.hitstop = (control.hitstop - dt).max(0.0);
        dt = 0.0;
    }
    if control.pred_ticks > 0.0 {
        control.pred_ticks = (control.pred_ticks - dt * TICK_HZ).max(0.0);
        if control.pred_ticks <= 0.0 && control.pred_action != ACTION_BLOCK {
            control.pred_action = ACTION_NONE;
        }
    }
    let can_step = control.pred_action != ACTION_DODGE
        && control.pred_action != ACTION_DEAD
        && !unbound_shared::move_lock(control.pred_action);
    let sprinting = (control.buttons & BTN_SPRINT) != 0 && control.pred_stamina > 1.0 && can_step;
    let walking = can_step && !sprinting && control.dir_x.abs() + control.dir_z.abs() > 0.1;
    if sprinting {
        control.pred_stamina = (control.pred_stamina - SPRINT_STAMINA_PER_SEC * dt).max(0.0);
        control.foot_accum += dt;
        if control.foot_accum >= 0.28 {
            control.foot_accum = 0.0;
            control.sfx_foot = 2;
        }
    } else {
        if unbound_shared::stamina_regen_ok(control.pred_action) {
            control.pred_stamina =
                (control.pred_stamina + STAMINA_REGEN_PER_SEC * dt).min(MAX_STAMINA);
        }
        if walking {
            control.foot_accum += dt;
            if control.foot_accum >= 0.42 {
                control.foot_accum = 0.0;
                control.sfx_foot = 1;
            }
        } else {
            control.foot_accum = 0.18;
        }
    }
}

pub fn spawn_predicted_shots(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut control: ResMut<ControlState>,
    local: Query<&Transform, With<LocalPlayer>>,
) {
    if !control.pred_shot {
        return;
    }
    let def = loadout(control.pred_loadout);
    if !def.is_projectile {
        control.pred_shot = false;
        return;
    }
    if control.pred_action != ACTION_LIGHT && control.pred_action != ACTION_HEAVY {
        return;
    }
    let release = predicted_release_ticks(control.pred_action, control.pred_loadout) as f32;
    if control.pred_ticks > release {
        return;
    }
    let Ok(tf) = local.single() else {
        return;
    };
    let (fx, fy, fz) = aim_dir(control.yaw, control.pitch);
    let speed = def.projectile_speed;
    let skill = if def.id == 2 { 2 } else { 1 };
    spawn_bolt(
        &mut commands,
        &mut meshes,
        &mut materials,
        tf.translation.x + fx * 0.9,
        SHOT_SPAWN_Y + fy * 0.4,
        tf.translation.z + fz * 0.9,
        fx * speed,
        fy * speed,
        fz * speed,
        skill,
        true,
        0,
    );
    // Latch once at release; pred_shot stays true through windup without retriggering.
    control.sfx_shot = skill;
    control.shot_kick = unbound_shared::CROSSHAIR_KICK_TIME;
    control.pred_shot = false;
}

pub fn fly_predicted_shots(
    time: Res<Time>,
    mut commands: Commands,
    mut shots: Query<(Entity, &mut PredictedShot, &mut Transform)>,
) {
    let dt = time.delta_secs();
    for (e, mut shot, mut transform) in &mut shots {
        shot.vy -= unbound_shared::shot_gravity(shot.skill) * dt;
        transform.translation.x += shot.vx * dt;
        transform.translation.y += shot.vy * dt;
        transform.translation.z += shot.vz * dt;
        aim_shot(&mut transform, shot.vx, shot.vy, shot.vz);
        if transform.translation.y < SHOT_GROUND_Y
            || transform.translation.y > SHOT_CEILING_Y
            || transform.translation.x.abs() > 40.0
            || transform.translation.z.abs() > 40.0
        {
            commands.entity(e).despawn();
        }
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
        if !visual.drawn {
            *transform = visual.rest;
            continue;
        }
        let (action, ticks, loadout) = if local_e == Some(parent.parent()) {
            (
                control.pred_action,
                control.pred_ticks,
                control.pred_loadout,
            )
        } else if let Some((_, pose)) = remotes.iter().find(|(e, _)| *e == parent.parent()) {
            (pose.action, pose.action_ticks, pose.loadout)
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
    shields: Query<(Entity, &ShieldVisual, Option<&ChildOf>)>,
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
    despawn_child_visuals(&mut commands, local_e, &shields);
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
    shields: Query<(Entity, &ShieldVisual, Option<&ChildOf>)>,
) {
    for (entity, pose) in &remotes {
        let current = weapons
            .iter()
            .find(|(_, _vis, parent)| parent.map(|p| p.parent() == entity).unwrap_or(false));
        let mismatch = current
            .map(|(_, vis, _)| vis.loadout != pose.loadout || vis.drawn != pose.drawn)
            .unwrap_or(current.is_none());
        if mismatch {
            for (w, _, parent) in &weapons {
                if parent.map(|p| p.parent() == entity).unwrap_or(false) {
                    commands.entity(w).despawn();
                }
            }
            despawn_child_visuals(&mut commands, entity, &shields);
            attach_weapon(
                &mut commands,
                &mut meshes,
                &mut materials,
                entity,
                pose.loadout,
                pose.drawn,
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

fn node_color(kind: u8, mix: f32) -> Color {
    let empty = Color::srgb(0.18, 0.16, 0.14);
    let live = if kind == 1 {
        Color::srgb(0.45, 0.48, 0.52)
    } else {
        Color::srgb(0.38, 0.24, 0.12)
    };
    empty.mix(&live, mix.clamp(0.0, 1.0))
}

fn attach_weapon(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    parent: Entity,
    loadout_id: u8,
    drawn: bool,
) {
    let def = loadout(loadout_id);
    let (mesh, color, tf) = match (def.id, drawn) {
        (1, true) => (
            meshes.add(Cuboid::new(0.08, 0.08, 1.15)),
            Color::srgb(0.45, 0.28, 0.12),
            Transform::from_xyz(0.25, 0.15, -0.55),
        ),
        (1, false) => (
            meshes.add(Cuboid::new(0.08, 0.08, 1.15)),
            Color::srgb(0.45, 0.28, 0.12),
            Transform::from_xyz(-0.05, 0.32, 0.38).with_rotation(Quat::from_rotation_x(-1.2)),
        ),
        (2, true) => (
            meshes.add(Cylinder::new(0.04, 1.4)),
            Color::srgb(0.35, 0.2, 0.55),
            Transform::from_xyz(0.28, 0.1, -0.2).with_rotation(Quat::from_rotation_x(0.2)),
        ),
        (2, false) => (
            meshes.add(Cylinder::new(0.04, 1.4)),
            Color::srgb(0.35, 0.2, 0.55),
            Transform::from_xyz(0.12, 0.15, 0.36).with_rotation(Quat::from_rotation_x(0.2)),
        ),
        (_, true) => (
            meshes.add(Cuboid::new(0.12, 0.04, 0.9)),
            Color::srgb(0.75, 0.75, 0.8),
            Transform::from_xyz(0.38, 0.15, -0.35),
        ),
        (_, false) => (
            meshes.add(Cuboid::new(0.12, 0.04, 0.9)),
            Color::srgb(0.75, 0.75, 0.8),
            Transform::from_xyz(0.08, 0.32, 0.36).with_rotation(Quat::from_rotation_x(-1.25)),
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
                drawn,
            },
        ))
        .id();
    commands.entity(parent).add_child(child);
    if def.id == unbound_shared::LOADOUT_SWORD {
        attach_shield(commands, meshes, materials, parent, drawn);
    }
}

fn attach_shield(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    parent: Entity,
    drawn: bool,
) {
    let tf = if drawn {
        Transform::from_xyz(-0.42, 0.12, -0.18)
            .with_rotation(Quat::from_rotation_y(1.35) * Quat::from_rotation_x(1.2))
    } else {
        Transform::from_xyz(-0.16, 0.18, 0.34).with_rotation(Quat::from_rotation_x(1.15))
    };
    let shield = commands
        .spawn((
            Mesh3d(meshes.add(Cylinder::new(0.34, 0.07))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.42, 0.26, 0.14),
                perceptual_roughness: 0.85,
                ..default()
            })),
            tf,
            ShieldVisual { rest: tf },
        ))
        .id();
    commands.entity(parent).add_child(shield);
}

fn despawn_child_visuals<T: Component>(
    commands: &mut Commands,
    parent: Entity,
    q: &Query<(Entity, &T, Option<&ChildOf>)>,
) {
    for (e, _, child_of) in q {
        if child_of.map(|p| p.parent() == parent).unwrap_or(false) {
            commands.entity(e).despawn();
        }
    }
}

pub fn pose_shields(
    control: Res<ControlState>,
    remotes: Query<(Entity, &ServerPose), With<RemotePlayer>>,
    local: Query<Entity, With<LocalPlayer>>,
    mut shields: Query<(&ShieldVisual, &mut Transform, Option<&ChildOf>)>,
) {
    let local_e = local.single().ok();
    for (visual, mut transform, parent) in &mut shields {
        let Some(parent) = parent else {
            continue;
        };
        let action = if local_e == Some(parent.parent()) {
            control.pred_action
        } else if let Some((_, pose)) = remotes.iter().find(|(e, _)| *e == parent.parent()) {
            pose.action
        } else {
            continue;
        };
        let lift = if action == ACTION_BLOCK {
            Transform::from_xyz(0.22, 0.08, -0.28)
                .with_rotation(Quat::from_rotation_y(-0.55) * Quat::from_rotation_x(-0.35))
        } else {
            Transform::IDENTITY
        };
        *transform = visual.rest * lift;
    }
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
            crate::camera::CamBlock::default(),
            DummyPose {
                x: dummy.x,
                z: dummy.z,
                yaw: dummy.yaw,
                action: dummy.action,
                action_ticks: dummy.action_ticks as f32,
                hp: dummy.hp,
                alive: dummy.alive,
                pending_hit: dummy.pending_hit,
            },
            DummyStep::new(dummy.x, dummy.z),
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
            HpBar {
                fade: if dummy.alive { 1.0 } else { 0.0 },
                flash: 0.0,
            },
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
    spawn_bolt(
        commands, meshes, materials, shot.x, shot.y, shot.z, shot.vx, shot.vy, shot.vz, shot.skill,
        false, shot.id,
    );
}

fn spawn_bolt(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    x: f32,
    y: f32,
    z: f32,
    vx: f32,
    vy: f32,
    vz: f32,
    skill: u8,
    predicted: bool,
    id: u32,
) {
    let staff = skill == 2;
    let color = if staff {
        Color::srgb(0.55, 0.35, 0.95)
    } else {
        Color::srgb(0.85, 0.7, 0.25)
    };
    let mesh = if staff {
        meshes.add(Sphere::new(0.22))
    } else {
        meshes.add(Cuboid::new(0.06, 0.06, 0.62))
    };
    let mut tf = Transform::from_xyz(x, y, z);
    aim_shot(&mut tf, vx, vy, vz);
    let mut e = commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: color,
            emissive: LinearRgba::from(color) * 4.0,
            ..default()
        })),
        tf,
    ));
    if predicted {
        e.insert(PredictedShot { vx, vy, vz, skill });
    } else {
        e.insert(ShotPawn {
            id,
            vx,
            vy,
            vz,
            skill,
        });
    }
}

fn spawn_node(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    node: &GatherNode,
) {
    let restore = if node.charges == 0 { 0.0 } else { 1.0 };
    let (sx, sy, sz) = node_mesh_scale(restore);
    let (mesh, y) = if node.kind == 1 {
        (meshes.add(Cuboid::new(0.9, 0.7, 0.9)), 0.35)
    } else {
        (meshes.add(Cylinder::new(0.28, 1.6)), 0.8)
    };
    let root = commands
        .spawn((
            Mesh3d(mesh),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: node_color(node.kind, restore),
                perceptual_roughness: 0.9,
                ..default()
            })),
            Transform::from_xyz(node.x, y, node.z).with_scale(Vec3::new(sx, sy, sz)),
            NodePawn {
                id: node.id,
                kind: node.kind,
                charges: node.charges,
                restore,
            },
            // Visual squash does not shrink camera collision.
            crate::camera::CamBlock {
                radius: if node.kind == 1 { 0.7 } else { 0.9 },
            },
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
