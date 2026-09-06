use std::time::Duration;

use spacetimedb::{table, Identity, ReducerContext, ScheduleAt, Table};
use unbound_shared::{integrate, TICK_DT};

#[table(accessor = player, public)]
#[derive(Clone, Debug)]
pub struct Player {
    #[primary_key]
    pub identity: Identity,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub yaw: f32,
    pub drawn: bool,
}

#[table(accessor = player_input)]
#[derive(Clone, Debug)]
pub struct PlayerInput {
    #[primary_key]
    pub identity: Identity,
    pub dir_x: f32,
    pub dir_z: f32,
    pub yaw: f32,
    pub drawn: bool,
}

#[table(accessor = world_tick_timer, scheduled(world_tick))]
pub struct WorldTickTimer {
    #[primary_key]
    #[auto_inc]
    pub scheduled_id: u64,
    pub scheduled_at: ScheduleAt,
}

#[spacetimedb::reducer(init)]
pub fn init(ctx: &ReducerContext) -> Result<(), String> {
    ctx.db.world_tick_timer().try_insert(WorldTickTimer {
        scheduled_id: 0,
        scheduled_at: ScheduleAt::Interval(Duration::from_millis(33).into()),
    })?;
    log::info!("unbound module initialized (30 Hz tick)");
    Ok(())
}

#[spacetimedb::reducer(client_connected)]
pub fn connected(ctx: &ReducerContext) -> Result<(), String> {
    let identity = ctx.sender();
    if ctx.db.player().identity().find(&identity).is_some() {
        return Ok(());
    }
    ctx.db.player().insert(Player {
        identity,
        x: 0.0,
        y: 0.0,
        z: 0.0,
        yaw: 0.0,
        drawn: false,
    });
    ctx.db.player_input().insert(PlayerInput {
        identity,
        dir_x: 0.0,
        dir_z: 0.0,
        yaw: 0.0,
        drawn: false,
    });
    log::info!("player connected: {identity}");
    Ok(())
}

#[spacetimedb::reducer(client_disconnected)]
pub fn disconnected(ctx: &ReducerContext) -> Result<(), String> {
    let identity = ctx.sender();
    ctx.db.player().identity().delete(&identity);
    ctx.db.player_input().identity().delete(&identity);
    log::info!("player disconnected: {identity}");
    Ok(())
}

#[spacetimedb::reducer]
pub fn set_input(
    ctx: &ReducerContext,
    dir_x: f32,
    dir_z: f32,
    yaw: f32,
    drawn: bool,
) -> Result<(), String> {
    let identity = ctx.sender();
    let Some(mut input) = ctx.db.player_input().identity().find(&identity) else {
        return Err("no player input row".into());
    };
    input.dir_x = dir_x.clamp(-1.0, 1.0);
    input.dir_z = dir_z.clamp(-1.0, 1.0);
    input.yaw = yaw;
    input.drawn = drawn;
    ctx.db.player_input().identity().update(input);

    if let Some(mut player) = ctx.db.player().identity().find(&identity) {
        player.yaw = yaw;
        player.drawn = drawn;
        ctx.db.player().identity().update(player);
    }
    Ok(())
}

#[spacetimedb::reducer]
pub fn world_tick(ctx: &ReducerContext, _tick: WorldTickTimer) -> Result<(), String> {
    if ctx.sender() != ctx.database_identity() {
        return Err("world_tick is scheduler-only".into());
    }

    for input in ctx.db.player_input().iter() {
        let Some(mut player) = ctx.db.player().identity().find(&input.identity) else {
            continue;
        };
        let (x, z) = integrate(player.x, player.z, input.yaw, input.dir_x, input.dir_z, TICK_DT);
        player.x = x;
        player.z = z;
        player.yaw = input.yaw;
        player.drawn = input.drawn;
        ctx.db.player().identity().update(player);
    }
    Ok(())
}
