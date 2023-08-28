use std::collections::HashMap;

use clap::{Arg, ArgAction, Command};
use log::info;
use shipyard::{Get, View, ViewMut};

use crate::{
    ecs::components::{guid::Guid, movement::Movement, threat_list::ThreatList, unit::Unit},
    entities::{creature::Creature, player::Player, position::WorldPosition},
};

use super::{ChatCommandResult, ChatCommands, CommandContext, CommandHandler, CommandMap};

pub(super) fn commands() -> CommandMap {
    HashMap::from([
        (COMMAND_GPS, handle_gps as CommandHandler),
        (COMMAND_COME, handle_come as CommandHandler),
        (COMMAND_THREAT, handle_threat as CommandHandler),
    ])
}

static COMMAND_GPS: &str = "gps";
fn handle_gps(ctx: CommandContext) -> ChatCommandResult {
    let command = Command::new(COMMAND_GPS).arg(
        Arg::new("dump")
            .short('d')
            .long("dump")
            .action(ArgAction::SetTrue),
    );

    ChatCommands::process(command, &ctx, &|matches| {
        if let Some(ref map) = ctx.session.current_map() {
            if let Some(player_ecs_entity) = ctx.session.player_entity_id() {
                map.world().run(|v_wpos: View<WorldPosition>| {
                    let wpos = v_wpos[player_ecs_entity];
                    let output = format!(
                        "Player position: {}, {}, {}, {}",
                        wpos.x, wpos.y, wpos.z, wpos.o,
                    );

                    ctx.session.send_system_message(output.as_str());

                    if matches.get_flag("dump") {
                        info!("GPS command output:\n {output}");
                    }
                });
            }
        }

        ChatCommandResult::HandledOk
    })
}

static COMMAND_COME: &str = "come";
fn handle_come(ctx: CommandContext) -> ChatCommandResult {
    let command = Command::new(COMMAND_COME);

    ChatCommands::process(command, &ctx, &|_| {
        if let Some(ref map) = ctx.session.current_map() {
            if let Some(player_ecs_entity) = ctx.session.player_entity_id() {
                map.world().run(
                    |v_wpos: View<WorldPosition>,
                     v_unit: View<Unit>,
                     v_guid: View<Guid>,
                     mut vm_movement: ViewMut<Movement>| {
                        let player_wpos = v_wpos[player_ecs_entity];
                        let player_target = v_unit[player_ecs_entity].target();
                        if player_target.is_none() {
                            ctx.session
                                .send_error_system_message("You must select a target");
                            return ChatCommandResult::HandledWithError;
                        }

                        let target_entity_id = player_target.unwrap();
                        let target_wpos = v_wpos[target_entity_id];

                        let path = vec![player_wpos.vec3()];

                        // TODO: Select speed depending on move flags (implement in Movement)
                        let speed = vm_movement[target_entity_id].speed_run;
                        vm_movement[target_entity_id].start_movement(
                            &v_guid[target_entity_id].0,
                            map.clone(),
                            &target_wpos.vec3(),
                            &path,
                            speed,
                            true,
                        );

                        return ChatCommandResult::HandledOk;
                    },
                );
            }
        }

        ChatCommandResult::HandledWithError
    })
}

static COMMAND_THREAT: &str = "threat";
fn handle_threat(ctx: CommandContext) -> ChatCommandResult {
    let command = Command::new(COMMAND_THREAT).arg(
        Arg::new("list")
            .short('l')
            .long("list")
            .action(ArgAction::SetTrue)
            .required(true), // Make this an ArgGroup when we implement threat modification
    );

    ChatCommands::process(command, &ctx, &|matches| {
        if let Some(ref map) = ctx.session.current_map() {
            if let Some(player_ecs_entity) = ctx.session.player_entity_id() {
                map.world().run(
                    |v_unit: View<Unit>,
                     v_threat_list: View<ThreatList>,
                     v_player: View<Player>,
                     v_creature: View<Creature>| {
                        if let Some(my_target) = v_unit[player_ecs_entity].target() {
                            if let Ok(tl) = v_threat_list.get(my_target) {
                                if matches.get_flag("list") {
                                    ctx.session.send_system_message("Threat list:");
                                    for (entity_id, threat_level) in tl.threat_list() {
                                        let mut target_name = "<unexpected entity type>";

                                        if let Ok(player) = v_player.get(entity_id) {
                                            target_name = player.name.as_str();
                                        } else if let Ok(creature) = v_creature.get(entity_id) {
                                            target_name = creature.name.as_str();
                                        }

                                        ctx.session.send_system_message(
                                            format!("- {target_name} ({threat_level})").as_str(),
                                        );
                                    }

                                    return ChatCommandResult::HandledOk;
                                }
                            }
                        }

                        ctx.session
                            .send_error_system_message("You must select a creature target");
                        return ChatCommandResult::HandledWithError;
                    },
                );
            }
        }

        ChatCommandResult::HandledWithError
    })
}
