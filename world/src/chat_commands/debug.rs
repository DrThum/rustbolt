use std::collections::HashMap;

use clap::{Arg, ArgAction, ArgMatches, Command};
use log::info;
use shipyard::{Get, UniqueView, View, ViewMut};

use crate::{
    ecs::components::{guid::Guid, movement::Movement, threat_list::ThreatList},
    entities::{
        attribute_modifiers::AttributeModifiers, creature::Creature, player::Player,
        position::WorldPosition,
    },
    game::packet_broadcaster::WrappedPacketBroadcaster,
};

use super::{ChatCommandError, ChatCommandResult, CommandContext, CommandHandler, CommandMap};

pub(super) fn commands() -> CommandMap {
    HashMap::from([
        setup_gps_command(),
        setup_come_command(),
        setup_threat_command(),
        setup_item_command(),
    ])
}

fn setup_gps_command() -> (&'static str, (Command, CommandHandler)) {
    let command_name = "gps";
    let command = Command::new(command_name).arg(
        Arg::new("dump")
            .short('d')
            .long("dump")
            .action(ArgAction::SetTrue),
    );

    fn handler(ctx: CommandContext, matches: ArgMatches) -> ChatCommandResult {
        ctx.map.world().run(|v_wpos: View<WorldPosition>| {
            let wpos = v_wpos[ctx.my_entity_id];
            let output = format!(
                "Player position: {}, {}, {}, {}",
                wpos.x, wpos.y, wpos.z, wpos.o,
            );

            ctx.reply(output.as_str());

            if matches.get_flag("dump") {
                info!("GPS command output:\n {output}");
            }
        });

        Ok(())
    }

    (command_name, (command, handler))
}

fn setup_come_command() -> (&'static str, (Command, CommandHandler)) {
    let command_name = "come";
    let command = Command::new(command_name);

    fn handler(ctx: CommandContext, _matches: ArgMatches) -> ChatCommandResult {
        let player_target = ctx.require_target()?;

        ctx.map.world().run(
            |v_wpos: View<WorldPosition>,
             v_guid: View<Guid>,
             packet_broadcaster: UniqueView<WrappedPacketBroadcaster>,
             mut vm_movement: ViewMut<Movement>| {
                let player_wpos = v_wpos[ctx.my_entity_id];
                let target_wpos = v_wpos[player_target];
                let path = vec![player_wpos.vec3()];

                // TODO: Select speed depending on move flags (implement in Movement)
                let speed = vm_movement[player_target].speed_run;
                vm_movement[player_target].start_movement(
                    &v_guid[player_target].0,
                    (**packet_broadcaster).clone(),
                    &target_wpos.vec3(),
                    &path,
                    speed,
                    true,
                );

                Ok(())
            },
        )
    }

    (command_name, (command, handler))
}

fn setup_threat_command() -> (&'static str, (Command, CommandHandler)) {
    let command_name = "threat";
    let command = Command::new(command_name).arg(
        Arg::new("list")
            .short('l')
            .long("list")
            .action(ArgAction::SetTrue)
            .required(true), // Make this an ArgGroup when we implement threat modification
    );

    fn handler(ctx: CommandContext, matches: ArgMatches) -> ChatCommandResult {
        let player_target = ctx.require_target()?;

        ctx.map.world().run(
            |v_threat_list: View<ThreatList>,
             v_player: View<Player>,
             v_creature: View<Creature>| {
                if let Ok(tl) = v_threat_list.get(player_target) {
                    if matches.get_flag("list") {
                        ctx.reply("Threat list:");
                        for (entity_id, threat_level) in tl.threat_list() {
                            let mut target_name = "<unexpected entity type>";

                            if let Ok(player) = v_player.get(entity_id) {
                                target_name = player.name.as_str();
                            } else if let Ok(creature) = v_creature.get(entity_id) {
                                target_name = creature.name.as_str();
                            }

                            ctx.reply(format!("- {target_name} ({threat_level})").as_str());
                        }

                        return Ok(());
                    }
                }

                ctx.reply_error("Target has no threat list");
                Err(ChatCommandError::GenericError)
            },
        )
    }

    (command_name, (command, handler))
}

// TODO: Move to item.rs ?
fn setup_item_command() -> (&'static str, (Command, CommandHandler)) {
    let command_name = "item";
    let command: Command = Command::new(command_name)
        .subcommand_required(true)
        .subcommand(
            Command::new("add").args([
                Arg::new("id")
                    .short('i')
                    .long("id")
                    .required(true)
                    .value_parser(clap::value_parser!(u32)),
                Arg::new("count")
                    .short('c')
                    .long("count")
                    .value_parser(clap::value_parser!(u32))
                    .default_value("1"),
            ]),
        );

    fn handler(ctx: CommandContext, matches: ArgMatches) -> ChatCommandResult {
        if let Some(subcommand_add) = matches.subcommand_matches("add") {
            return ctx.map.world().run(
                |mut vm_player: ViewMut<Player>,
                 mut vm_attribute_modifiers: ViewMut<AttributeModifiers>| {
                    let Ok(mut player) = (&mut vm_player).get(ctx.my_entity_id) else {
                        return Err(ChatCommandError::GenericError);
                    };

                    let Ok(mut attribute_modifiers) =
                        (&mut vm_attribute_modifiers).get(ctx.my_entity_id)
                    else {
                        return Err(ChatCommandError::GenericError);
                    };

                    let item_id: &u32 = subcommand_add.get_one("id").unwrap();
                    let count: &u32 = subcommand_add.get_one("count").unwrap();

                    if ctx
                        .world_context
                        .data_store
                        .get_item_template(*item_id)
                        .is_none()
                    {
                        ctx.reply_error(format!("Item template {item_id} does not exist").as_str());
                        return Err(ChatCommandError::GenericError);
                    }

                    match player.auto_store_new_item(*item_id, *count, &mut attribute_modifiers) {
                        Ok(_) => Ok(()),
                        Err(err) => {
                            ctx.reply_error(format!("Unable to add item ({err:?})").as_str());
                            Err(ChatCommandError::GenericError)
                        }
                    }
                },
            );
        }

        Err(ChatCommandError::InvalidArguments)
    }

    (command_name, (command, handler))
}
