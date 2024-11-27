use std::collections::HashMap;

use clap::{builder::BoolishValueParser, Arg, Command};
use shipyard::{View, ViewMut};

use crate::{ecs::components::movement::Movement, entities::{player::Player, position::{Position, WorldPosition}}, protocol::{packets::MsgMoveTeleportAck, server::ServerMessage}};

use super::{ChatCommandResult, ChatCommands, CommandContext, CommandHandler, CommandMap};

pub(super) fn commands() -> CommandMap {
    HashMap::from([
        (COMMAND_FLY, handle_fly as CommandHandler),
        (COMMAND_TELEPORT, handle_teleport as CommandHandler),
    ])
}

static COMMAND_FLY: &str = "fly";
fn handle_fly(ctx: CommandContext) -> ChatCommandResult {
    let command = Command::new(COMMAND_FLY).arg(
        Arg::new("flying")
            .required(true)
            .value_parser(BoolishValueParser::new()),
    );

    ChatCommands::process(command, &ctx, &|matches| {
        let flying = matches.get_one::<bool>("flying").unwrap();

        if let Some(ref map) = ctx.session.current_map() {
            if let Some(player_ecs_entity) = ctx.session.player_entity_id() {
                map.world().run(|mut vm_movement: ViewMut<Movement>| {
                    vm_movement[player_ecs_entity].set_flying(*flying, ctx.session.clone());
                });
            }
        }

        ChatCommandResult::ok()
    })
}

static COMMAND_TELEPORT: &str = "teleport";
fn handle_teleport(ctx: CommandContext) -> ChatCommandResult {
    // TODO: params:
    // --xyz f32 f32 f32 <optional u32 mapId>, orientation is kept
    // --poi followed by a name, coords are kept in a database table
    // --player followed by a player name
    let command = Command::new(COMMAND_TELEPORT);

    ChatCommands::process(command, &ctx, &|_matches| {
        if let Some(ref map) = ctx.session.current_map() {
            if let Some(player_ecs_entity_id) = ctx.session.player_entity_id() {
                map.world().run(|mut vm_player: ViewMut<Player>, v_movement: View<Movement>, v_wpos: View<WorldPosition>| {
                    let player = &mut vm_player[player_ecs_entity_id];
                    let movement = &v_movement[player_ecs_entity_id];
                    let wpos = &mut v_wpos[player_ecs_entity_id].clone();
                    let position = Position { // Undercity entrance
                        x: 1968.12,
                        y: 309.62,
                        z: 41.57,
                        o: 1.67,
                    };
                    wpos.update_local(&position);
                    player.set_teleport_destination(*wpos);

                    // Near teleport
                    let packet = ServerMessage::new(MsgMoveTeleportAck {
                        packed_guid: player.guid().as_packed(),
                        unk_counter: 0,
                        movement_info: movement.info(ctx.world_context.clone(), &position),
                    });

                    ctx.session.send(&packet).unwrap();

                    // For far teleport (on another map), use SmsgTransferPending for the loading
                    // screen then SmsgNewWorld for the actual teleport
                    // Then the response is MsgMoveWorldportAck
                });
            }
        }

        ChatCommandResult::ok()
    })
}
