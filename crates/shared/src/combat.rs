use crate::{
    ACTION_BLOCK, ACTION_DEAD, ACTION_DODGE, ACTION_GATHER, ACTION_HEAVY, ACTION_HIT, ACTION_LIGHT,
    ACTION_SWAP, BTN_BLOCK, BTN_DODGE, BTN_HEAVY, BTN_INTERACT, BTN_LIGHT, LOADOUT_BOW,
    LOADOUT_STAFF, LOADOUT_SWORD, TICK_DT, TICK_HZ,
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

/// Buttons that are meaningful as a 1-frame press and must be latched until `set_input`.
pub const BTN_EDGE: u32 = BTN_LIGHT | BTN_HEAVY | BTN_DODGE | BTN_INTERACT;

pub const DUMMY_AGGRO_RANGE: f32 = 12.0;
pub const DUMMY_LEASH_RANGE: f32 = 14.0;
pub const DUMMY_STRIKE_RANGE: f32 = 2.55;
pub const DUMMY_MELEE_RANGE: f32 = 2.2;
pub const DUMMY_CHASE_SPEED: f32 = 3.2;
pub const DUMMY_HOME_SPEED: f32 = 3.6;
pub const DUMMY_LIGHT_DAMAGE: f32 = 11.0;
pub const DUMMY_HEAVY_DAMAGE: f32 = 20.0;
pub const DUMMY_KEEP_OUT: f32 = 1.75;
pub const DUMMY_APPROACH: f32 = 2.35;

pub const CAM_SHEATHED: f32 = 6.8;
pub const CAM_DRAWN: f32 = 4.35;
pub const CAM_LOCK: f32 = 5.15;
pub const CAM_LOCK_MIX: f32 = 0.32;
pub const CAM_SHOULDER: f32 = 0.42;
pub const CAM_SHAKE_TIME: f32 = 0.16;

pub fn dummy_light_windup() -> u8 {
    12
}

pub fn dummy_heavy_windup() -> u8 {
    18
}

pub fn dummy_cooldown_ticks() -> u8 {
    22
}

pub fn dummy_windup_ticks(action: u8) -> u8 {
    if action == ACTION_HEAVY {
        dummy_heavy_windup()
    } else {
        dummy_light_windup()
    }
}

/// +1 chase, 0 hold the pocket, -1 step back. Dummy should not glue to the player.
pub fn dummy_move_dir(dist: f32) -> f32 {
    if dist > DUMMY_APPROACH {
        1.0
    } else if dist < DUMMY_KEEP_OUT {
        -1.0
    } else {
        0.0
    }
}

pub fn camera_distance(drawn: bool, lock_on: bool) -> f32 {
    if lock_on {
        CAM_LOCK
    } else if drawn {
        CAM_DRAWN
    } else {
        CAM_SHEATHED
    }
}

pub fn lock_focus_xz(px: f32, pz: f32, tx: f32, tz: f32, mix: f32) -> (f32, f32) {
    let mix = mix.clamp(0.0, 1.0);
    (px + (tx - px) * mix, pz + (tz - pz) * mix)
}

pub fn camera_shake_amp(t: f32) -> f32 {
    let a = (t / CAM_SHAKE_TIME).clamp(0.0, 1.0);
    a * a * 0.16
}

pub fn hp_regen_ok(action: u8) -> bool {
    !action_busy(action) && action != ACTION_BLOCK
}

pub fn merge_input_buttons(held: u32, latched: u32) -> u32 {
    held | latched
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActionStart {
    pub action: u8,
    pub ticks: u8,
    pub stamina: f32,
    pub pending_hit: bool,
    pub loadout: u8,
}

/// Same rules the module uses to start a drawn-weapon action.
pub fn start_drawn_action(
    current_action: u8,
    current_loadout: u8,
    wanted_loadout: u8,
    stamina: f32,
    buttons: u32,
) -> Option<ActionStart> {
    if action_busy(current_action) {
        return None;
    }
    if wanted_loadout != current_loadout {
        return Some(ActionStart {
            action: ACTION_SWAP,
            ticks: swap_ticks(),
            stamina,
            pending_hit: false,
            loadout: wanted_loadout,
        });
    }
    if (buttons & BTN_DODGE) != 0 {
        let cost = loadout(current_loadout).dodge_stamina;
        if stamina_ok(stamina, cost) {
            return Some(ActionStart {
                action: ACTION_DODGE,
                ticks: dodge_ticks(),
                stamina: stamina - cost,
                pending_hit: false,
                loadout: current_loadout,
            });
        }
        return None;
    }
    if (buttons & BTN_BLOCK) != 0 && current_loadout == LOADOUT_SWORD {
        return Some(ActionStart {
            action: ACTION_BLOCK,
            ticks: 1,
            stamina,
            pending_hit: false,
            loadout: current_loadout,
        });
    }
    let def = loadout(current_loadout);
    if (buttons & BTN_HEAVY) != 0 && stamina_ok(stamina, def.heavy_stamina) {
        return Some(ActionStart {
            action: ACTION_HEAVY,
            ticks: def.heavy_windup_ticks,
            stamina: stamina - def.heavy_stamina,
            pending_hit: true,
            loadout: current_loadout,
        });
    }
    if (buttons & BTN_LIGHT) != 0 && stamina_ok(stamina, def.light_stamina) {
        return Some(ActionStart {
            action: ACTION_LIGHT,
            ticks: def.light_windup_ticks,
            stamina: stamina - def.light_stamina,
            pending_hit: true,
            loadout: current_loadout,
        });
    }
    None
}

pub fn start_gather_action(
    current_action: u8,
    buttons: u32,
    in_range: bool,
) -> Option<ActionStart> {
    if action_busy(current_action) {
        return None;
    }
    if (buttons & BTN_INTERACT) == 0 || !in_range {
        return None;
    }
    Some(ActionStart {
        action: ACTION_GATHER,
        ticks: gather_ticks(),
        stamina: 0.0,
        pending_hit: false,
        loadout: 0,
    })
}

pub fn dodge_dir(dir_x: f32, dir_z: f32) -> (f32, f32) {
    if dir_x.abs() + dir_z.abs() < 0.1 {
        (0.0, 1.0)
    } else {
        (dir_x, dir_z)
    }
}

pub fn dodge_burst_dt() -> f32 {
    TICK_DT * dodge_ticks() as f32 * 0.35
}

/// Client-side busy window: attacks include recover so the chop plays through.
pub fn predicted_busy_ticks(start: &ActionStart) -> u8 {
    let def = loadout(start.loadout);
    match start.action {
        ACTION_LIGHT => start.ticks.saturating_add(def.light_recover_ticks),
        ACTION_HEAVY => start.ticks.saturating_add(def.heavy_recover_ticks),
        _ => start.ticks,
    }
}

/// 0 at windup start, 1 at impact, >1 through recover.
pub fn swing_progress(action: u8, ticks_left: f32, loadout_id: u8) -> f32 {
    let def = loadout(loadout_id);
    let (windup, recover) = match action {
        ACTION_HEAVY => (
            def.heavy_windup_ticks as f32,
            def.heavy_recover_ticks as f32,
        ),
        ACTION_LIGHT => (
            def.light_windup_ticks as f32,
            def.light_recover_ticks as f32,
        ),
        _ => return 0.0,
    };
    let total = windup + recover;
    if total <= 1e-3 {
        return 1.0;
    }
    (total - ticks_left.max(0.0)) / windup.max(1.0)
}

pub fn weapon_extra_rotation(action: u8, ticks_left: f32, loadout_id: u8) -> (f32, f32, f32) {
    match action {
        ACTION_BLOCK => (0.85, 0.15, -0.9),
        ACTION_DODGE => (0.35, 0.0, 0.4),
        ACTION_SWAP => (-0.45, 0.2, 0.0),
        ACTION_HIT => (0.55, 0.0, 0.2),
        ACTION_GATHER => (0.4, 0.0, 0.15),
        ACTION_LIGHT | ACTION_HEAVY => {
            let p = swing_progress(action, ticks_left, loadout_id);
            if p < 1.0 {
                (-0.2 - p * 0.95, 0.0, p * 0.25)
            } else {
                let rec = (p - 1.0).clamp(0.0, 1.2);
                (-1.15 + rec * 1.85, 0.0, 0.25 + rec * 0.35)
            }
        }
        _ => (0.0, 0.0, 0.0),
    }
}

/// Dummy club pitch. Raises through most of the windup, slams in the last 30%.
pub fn dummy_club_pitch(action: u8, ticks_left: f32, windup_ticks: f32) -> f32 {
    if action == ACTION_HIT {
        return 0.4;
    }
    if action != ACTION_LIGHT && action != ACTION_HEAVY {
        return 0.0;
    }
    let t = if windup_ticks > 1e-3 {
        (1.0 - ticks_left / windup_ticks).clamp(0.0, 1.0)
    } else {
        1.0
    };
    if t < 0.7 {
        -(t / 0.7) * 1.25
    } else {
        -1.25 + ((t - 0.7) / 0.3) * 2.2
    }
}

pub fn action_label(action: u8) -> &'static str {
    match action {
        ACTION_LIGHT => "LIGHT",
        ACTION_HEAVY => "HEAVY",
        ACTION_DODGE => "DODGE",
        ACTION_BLOCK => "BLOCK",
        ACTION_SWAP => "SWAP",
        ACTION_HIT => "HIT",
        ACTION_DEAD => "DEAD",
        ACTION_GATHER => "GATHER",
        _ => "",
    }
}
