use std::collections::HashMap;

use clap::{builder::BoolishValueParser, value_parser, Arg, Command};
use shipyard::{View, ViewMut};

use crate::{ecs::components::movement::Movement, entities::{player::Player, position::{Position, WorldPosition}}, game::map_manager::MapKey, protocol::{packets::{MsgMoveTeleportAck, SmsgNewWorld, SmsgTransferPending}, server::ServerMessage}};

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
    let command = Command::new(COMMAND_TELEPORT).arg(
        Arg::new("xyz")
            .long("xyz")
            .num_args(3..=4)
            .value_delimiter(' ')
            .value_parser(value_parser!(f32))
            .allow_hyphen_values(true)
    );

    ChatCommands::process(command, &ctx, &|matches| {
        let coords: Option<Vec<f32>> = matches.get_many("xyz").map(|values| values.copied().collect());

        if let Some(ref map) = ctx.session.current_map() {
            if let Some(player_ecs_entity_id) = ctx.session.player_entity_id() {
                map.world().run(|mut vm_player: ViewMut<Player>, v_movement: View<Movement>, v_wpos: View<WorldPosition>| {
                    let player = &mut vm_player[player_ecs_entity_id];
                    let movement = &v_movement[player_ecs_entity_id];
                    let player_curr_pos = &mut v_wpos[player_ecs_entity_id].clone();

                    if let Some(coords) = coords {
                        if coords.len() == 3 { // Teleport on the same map
                            let position = Position {
                                x: coords[0],
                                y: coords[1],
                                z: coords[2],
                                o: player_curr_pos.o,
                            };
                            player_curr_pos.update_local(&position);
                            player.set_teleport_destination(*player_curr_pos);

                            // Near teleport
                            let packet = ServerMessage::new(MsgMoveTeleportAck {
                                packed_guid: player.guid().as_packed(),
                                unk_counter: 0,
                                movement_info: movement.info(ctx.world_context.clone(), &position),
                            });

                            ctx.session.send(&packet).unwrap();
                        } else { // Teleport on another map
                            // For far teleport (on another map), use SmsgTransferPending for the loading
                            // screen then SmsgNewWorld for the actual teleport
                            // Then the response is MsgMoveWorldportAck
                            let position = Position {
                                x: coords[0],
                                y: coords[1],
                                z: coords[2],
                                o: player_curr_pos.o,
                            };
                            let map_key = MapKey::new(coords[3].round() as u32, None);
                            let destination_map_id = map_key.map_id;
                            player_curr_pos.update_local(&position);
                            player_curr_pos.map_key = map_key;
                            player.set_teleport_destination(*player_curr_pos);

                            let packet = ServerMessage::new(SmsgTransferPending {
                                destination_map_id,
                            });
                            ctx.session.send(&packet).unwrap();

                            let packet = ServerMessage::new(SmsgNewWorld {
                                map_id: destination_map_id,
                                x: coords[0],
                                y: coords[1],
                                z: coords[2],
                                o: player_curr_pos.o,
                            });
                            ctx.session.send(&packet).unwrap();

                            // TODO: implement player moving from one map to another
                            // ctx.world_context.map_manager.remove_player_from_map(&player.guid(), ctx.session.current_map().map(|m| m.key()));
                            // ctx.world_context.map_manager.get_map(MapKey::new(destination_map_id, None)).unwrap().add_player_on_login(ctx.session, char_data)
                            // TODO: implement MsgMoveWorldportAck
                        }
                    }
                });
            }
        }

        ChatCommandResult::ok()
    })
}
