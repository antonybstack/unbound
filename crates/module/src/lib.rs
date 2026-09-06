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
    // Do not spawn a pawn here. `spacetime sql` / `spacetime call` also connect,
    // which would flash a ghost capsule at the origin.
    log::info!("client connected: {}", ctx.sender());
    Ok(())
}

#[spacetimedb::reducer(client_disconnected)]
pub fn disconnected(ctx: &ReducerContext) -> Result<(), String> {
    let identity = ctx.sender();
    let had_player = ctx.db.player().identity().delete(&identity);
    ctx.db.player_input().identity().delete(&identity);
    if had_player {
        log::info!("player left: {identity}");
    }
    Ok(())
}

fn upsert_input(ctx: &ReducerContext, identity: Identity, dir_x: f32, dir_z: f32, yaw: f32, drawn: bool) {
    let row = PlayerInput {
        identity,
        dir_x,
        dir_z,
        yaw,
        drawn,
    };
    if ctx.db.player_input().identity().find(&identity).is_some() {
        ctx.db.player_input().identity().update(row);
    } else {
        ctx.db.player_input().insert(row);
    }
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
    let dir_x = dir_x.clamp(-1.0, 1.0);
    let dir_z = dir_z.clamp(-1.0, 1.0);
    upsert_input(ctx, identity, dir_x, dir_z, yaw, drawn);

    if let Some(mut player) = ctx.db.player().identity().find(&identity) {
        player.yaw = yaw;
        player.drawn = drawn;
        ctx.db.player().identity().update(player);
    } else {
        ctx.db.player().insert(Player {
            identity,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            yaw,
            drawn,
        });
        log::info!("player entered: {identity}");
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
