use crate::{
    ACTION_BLOCK, ACTION_DEAD, ACTION_DODGE, ACTION_GATHER, ACTION_HEAVY, ACTION_HIT, ACTION_LIGHT,
    ACTION_SWAP, LOADOUT_BOW, LOADOUT_STAFF, LOADOUT_SWORD, TICK_HZ,
};

#[derive(Clone, Copy, Debug)]
pub struct LoadoutDef {
    pub id: u8,
    pub name: &'static str,
    pub light_stamina: f32,
    pub heavy_stamina: f32,
    pub dodge_stamina: f32,
    pub light_damage: f32,
    pub heavy_damage: f32,
    pub light_windup_ticks: u8,
    pub heavy_windup_ticks: u8,
    pub light_recover_ticks: u8,
    pub heavy_recover_ticks: u8,
    pub melee_range: f32,
    pub projectile_speed: f32,
    pub is_projectile: bool,
}

pub const SWORD: LoadoutDef = LoadoutDef {
    id: LOADOUT_SWORD,
    name: "Sword & board",
    light_stamina: 12.0,
    heavy_stamina: 28.0,
    dodge_stamina: 22.0,
    light_damage: 14.0,
    heavy_damage: 28.0,
    light_windup_ticks: 5,
    heavy_windup_ticks: 11,
    light_recover_ticks: 6,
    heavy_recover_ticks: 10,
    melee_range: 2.3,
    projectile_speed: 0.0,
    is_projectile: false,
};

pub const BOW: LoadoutDef = LoadoutDef {
    id: LOADOUT_BOW,
    name: "Bow",
    light_stamina: 10.0,
    heavy_stamina: 24.0,
    dodge_stamina: 22.0,
    light_damage: 12.0,
    heavy_damage: 22.0,
    light_windup_ticks: 6,
    heavy_windup_ticks: 14,
    light_recover_ticks: 8,
    heavy_recover_ticks: 12,
    melee_range: 0.0,
    projectile_speed: 28.0,
    is_projectile: true,
};

pub const STAFF: LoadoutDef = LoadoutDef {
    id: LOADOUT_STAFF,
    name: "Staff",
    light_stamina: 14.0,
    heavy_stamina: 32.0,
    dodge_stamina: 22.0,
    light_damage: 13.0,
    heavy_damage: 26.0,
    light_windup_ticks: 7,
    heavy_windup_ticks: 13,
    light_recover_ticks: 8,
    heavy_recover_ticks: 12,
    melee_range: 0.0,
    projectile_speed: 22.0,
    is_projectile: true,
};

pub fn loadout(id: u8) -> LoadoutDef {
    match id {
        LOADOUT_BOW => BOW,
        LOADOUT_STAFF => STAFF,
        _ => SWORD,
    }
}

pub fn dodge_ticks() -> u8 {
    (0.28 * TICK_HZ) as u8
}

pub fn swap_ticks() -> u8 {
    (0.45 * TICK_HZ) as u8
}

pub fn hitstun_ticks() -> u8 {
    4
}

pub fn death_respawn_ticks() -> u8 {
    (3.0 * TICK_HZ) as u8
}

pub fn gather_ticks() -> u8 {
    (1.35 * TICK_HZ) as u8
}

pub fn node_respawn_ticks() -> u16 {
    (8.0 * TICK_HZ) as u16
}

pub fn stamina_ok(stamina: f32, cost: f32) -> bool {
    stamina + 0.01 >= cost
}

pub fn scaled_damage(base: f32, skill_level: u8) -> f32 {
    (base * (1.0 + 0.025 * (skill_level.saturating_sub(1) as f32))).round()
}

pub fn action_busy(action: u8) -> bool {
    action == ACTION_LIGHT
        || action == ACTION_HEAVY
        || action == ACTION_DODGE
        || action == ACTION_SWAP
        || action == ACTION_HIT
        || action == ACTION_GATHER
        || action == ACTION_DEAD
}

pub fn move_lock(action: u8) -> bool {
    action == ACTION_HEAVY
        || action == ACTION_SWAP
        || action == ACTION_GATHER
        || action == ACTION_DEAD
}

pub fn blocking(action: u8) -> bool {
    action == ACTION_BLOCK
}

pub fn invulnerable(action: u8) -> bool {
    action == ACTION_DODGE
}
