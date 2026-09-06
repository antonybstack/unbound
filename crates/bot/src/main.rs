//! Headless SpacetimeDB client used to prove 1v1, dummy combat, gather, and persistence.

mod module_bindings;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use spacetimedb_sdk::{DbContext, Identity, Table};
use unbound_shared::{
    ACTION_HEAVY, ACTION_LIGHT, BTN_BLOCK, BTN_DODGE, BTN_INTERACT, BTN_LIGHT, BTN_SPRINT,
    GATHER_RANGE, LOADOUT_BOW, LOADOUT_SWORD, dist_xz,
};

use crate::module_bindings::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Dummy,
    Pvp,
    Gather,
    Bow,
}

fn main() {
    let args = Args::parse();
    let token = args
        .token
        .as_ref()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let token_path = args.token.clone();
    let name = args.name.clone();

    let conn = DbConnection::builder()
        .with_uri("http://127.0.0.1:3000")
        .with_database_name("unbound")
        .with_token(token)
        .on_connect(move |conn, identity, token| {
            if let Some(path) = &token_path {
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let _ = std::fs::write(path, token);
            }
            let short = identity_short(identity);
            eprintln!("bot {name} connected as {short}");
            let _ = conn.reducers().set_name(name.clone());
            conn.subscription_builder().subscribe_to_all_tables();
        })
        .on_connect_error(|_ctx, err| {
            eprintln!("connect error: {err}");
            std::process::exit(2);
        })
        .on_disconnect(|_ctx, err| {
            if let Some(err) = err {
                eprintln!("disconnected: {err}");
            }
        })
        .build()
        .expect("db connection");

    let _thread = conn.run_threaded();
    let started = Instant::now();
    let mut last_status = Instant::now();
    let mut ticks = 0u32;
    let mut saw_self = false;
    let mut max_melee = 0u64;
    let mut min_hp = 100.0f32;

    while started.elapsed() < Duration::from_secs(args.seconds) {
        std::thread::sleep(Duration::from_millis(50));
        ticks += 1;
        let Some(me) = conn.try_identity() else {
            continue;
        };
        let me_row = conn.db().player().iter().find(|p| p.identity == me);
        let Some(me_row) = me_row else {
            // Enter the yard.
            let _ = conn
                .reducers()
                .set_input(0.0, 0.0, 0.0, 0.0, false, 0, LOADOUT_SWORD);
            continue;
        };
        saw_self = true;
        min_hp = min_hp.min(me_row.hp);

        if let Some(c) = conn.db().character().iter().find(|c| c.identity == me) {
            max_melee = max_melee.max(c.melee_xp);
        }

        let (yaw, dir_z, drawn, buttons, loadout) = think(args.mode, &conn, &me_row);
        let _ = conn
            .reducers()
            .set_input(0.0, dir_z, yaw, 0.0, drawn, buttons, loadout);

        if last_status.elapsed() >= Duration::from_secs(1) {
            last_status = Instant::now();
            eprintln!(
                "bot {} t={} pos=({:.1},{:.1}) hp={:.0} stam={:.0} xp_melee={}",
                args.name, ticks, me_row.x, me_row.z, me_row.hp, me_row.stamina, max_melee
            );
        }
    }

    if let Some(me) = conn.try_identity() {
        if let Some(c) = conn.db().character().iter().find(|c| c.identity == me) {
            println!(
                "RESULT name={} identity={} melee_xp={} ranged_xp={} magic_xp={} defence_xp={} hitpoints_xp={} gather_xp={} min_hp={:.1} saw_self={}",
                c.name,
                identity_short(me),
                c.melee_xp,
                c.ranged_xp,
                c.magic_xp,
                c.defence_xp,
                c.hitpoints_xp,
                c.gather_xp,
                min_hp,
                saw_self
            );
        } else {
            println!(
                "RESULT name={} identity={} melee_xp=0 saw_self={saw_self} min_hp={min_hp:.1}",
                args.name,
                identity_short(me)
            );
        }
    }
    let _ = conn.disconnect();
}

fn think(mode: Mode, conn: &DbConnection, me: &Player) -> (f32, f32, bool, u32, u8) {
    if !me.alive {
        return (me.yaw, 0.0, false, 0, LOADOUT_SWORD);
    }
    match mode {
        Mode::Gather => think_gather(conn, me),
        Mode::Dummy => think_fight(conn, me, true, false),
        Mode::Pvp => think_fight(conn, me, false, false),
        Mode::Bow => think_fight(conn, me, true, true),
    }
}

fn think_fight(
    conn: &DbConnection,
    me: &Player,
    dummy_only: bool,
    ranged: bool,
) -> (f32, f32, bool, u32, u8) {
    let mut target: Option<(f32, f32, f32, u8)> = None; // x, z, dist, action
    if !dummy_only {
        for p in conn.db().player().iter() {
            if p.identity == me.identity || !p.alive {
                continue;
            }
            let d = dist_xz(me.x, me.z, p.x, p.z);
            if target.map(|(_, _, td, _)| d < td).unwrap_or(true) {
                target = Some((p.x, p.z, d, p.action));
            }
        }
    }
    if target.is_none() {
        for d in conn.db().dummy().iter() {
            if !d.alive {
                continue;
            }
            let dist = dist_xz(me.x, me.z, d.x, d.z);
            target = Some((d.x, d.z, dist, d.action));
        }
    }
    let Some((tx, tz, dist, action)) = target else {
        return (me.yaw, 0.0, true, 0, LOADOUT_SWORD);
    };
    let yaw = (-(tx - me.x)).atan2(-(tz - me.z));
    let swinging = action == ACTION_LIGHT || action == ACTION_HEAVY;
    let loadout = if ranged { LOADOUT_BOW } else { LOADOUT_SWORD };
    let in_range = if ranged { dist < 12.0 } else { dist < 2.15 };
    let dir_z = if ranged {
        if dist > 9.0 {
            1.0
        } else if dist < 5.5 {
            -1.0
        } else {
            0.0
        }
    } else if swinging || in_range {
        0.0
    } else {
        1.0
    };
    let mut buttons = 0u32;
    if !ranged && swinging && dist < 3.2 {
        if action == ACTION_HEAVY {
            buttons |= BTN_DODGE;
        } else {
            buttons |= BTN_BLOCK;
        }
    } else if in_range {
        buttons |= BTN_LIGHT;
    } else if dist > 6.0 {
        buttons |= BTN_SPRINT;
    }
    (yaw, dir_z, true, buttons, loadout)
}

fn think_gather(conn: &DbConnection, me: &Player) -> (f32, f32, bool, u32, u8) {
    let mut best: Option<(f32, f32, f32)> = None;
    for node in conn.db().gather_node().iter() {
        if node.charges == 0 {
            continue;
        }
        let d = dist_xz(me.x, me.z, node.x, node.z);
        if best.map(|(_, _, bd)| d < bd).unwrap_or(true) {
            best = Some((node.x, node.z, d));
        }
    }
    let Some((tx, tz, dist)) = best else {
        return (me.yaw, 0.0, false, 0, LOADOUT_SWORD);
    };
    let yaw = (-(tx - me.x)).atan2(-(tz - me.z));
    if dist <= GATHER_RANGE {
        (yaw, 0.0, false, BTN_INTERACT, LOADOUT_SWORD)
    } else {
        let sprint = if dist > 5.0 { BTN_SPRINT } else { 0 };
        (yaw, 1.0, false, sprint, LOADOUT_SWORD)
    }
}

fn identity_short(id: Identity) -> String {
    let b = id.to_byte_array();
    format!("{:04x}", unbound_shared::wanderer_tag(&b))
}

struct Args {
    name: String,
    token: Option<PathBuf>,
    mode: Mode,
    seconds: u64,
}

impl Args {
    fn parse() -> Self {
        let mut name = "Bot".to_string();
        let mut token = None;
        let mut mode = Mode::Pvp;
        let mut seconds = 12;
        let mut it = std::env::args().skip(1);
        while let Some(arg) = it.next() {
            match arg.as_str() {
                "--name" => name = it.next().unwrap_or(name),
                "--token" => token = it.next().map(PathBuf::from),
                "--mode" => {
                    mode = match it.next().unwrap_or_default().as_str() {
                        "dummy" => Mode::Dummy,
                        "gather" => Mode::Gather,
                        "bow" => Mode::Bow,
                        _ => Mode::Pvp,
                    }
                }
                "--seconds" => seconds = it.next().and_then(|s| s.parse().ok()).unwrap_or(seconds),
                "-h" | "--help" => {
                    eprintln!(
                        "unbound-bot --name NAME --token PATH --mode pvp|dummy|gather|bow --seconds N"
                    );
                    std::process::exit(0);
                }
                other => eprintln!("unknown arg {other}"),
            }
        }
        Self {
            name,
            token,
            mode,
            seconds,
        }
    }
}
