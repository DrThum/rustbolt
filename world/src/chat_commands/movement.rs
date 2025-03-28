use std::collections::HashMap;

use clap::{builder::BoolishValueParser, Arg, ArgMatches, Command};
use shipyard::{View, ViewMut};

use crate::{
    chat_commands::ChatCommandError,
    ecs::components::movement::Movement,
    entities::{player::Player, position::WorldPosition},
    game::map_manager::MapKey,
    repositories::character::CharacterRepository,
};

use super::{ChatCommandResult, CommandContext, CommandHandler, CommandMap};

pub(super) fn commands() -> CommandMap {
    HashMap::from([setup_fly_command(), setup_teleport_command()])
}

fn setup_fly_command() -> (&'static str, (Command, CommandHandler)) {
    let command_name = "fly";
    let command = Command::new(command_name).arg(
        Arg::new("flying")
            .required(true)
            .value_parser(BoolishValueParser::new()),
    );

    fn handler(ctx: CommandContext, matches: ArgMatches) -> ChatCommandResult {
        let &flying = matches.get_one::<bool>("flying").unwrap();

        ctx.map.world().borrow().run(|mut vm_movement: ViewMut<Movement>| {
            vm_movement[ctx.my_entity_id].set_flying(flying, ctx.session.clone());
        });

        Ok(())
    }

    (command_name, (command, handler))
}

fn setup_teleport_command() -> (&'static str, (Command, CommandHandler)) {
    let command_name = "teleport";
    let command = Command::new(command_name)
        .arg(
            Arg::new("xyz")
                .long("xyz")
                .num_args(3..=4)
                .value_parser(clap::value_parser!(f32))
                .help("Coordinates as 'x y z' with optional map ID")
                .required_unless_present("player"),
        )
        // .arg(Arg::new("poi")
        //     .long("poi")
        //     .value_name("NAME")
        //     .help("Point of interest name from database"))
        .arg(
            Arg::new("player")
                .long("player")
                .short('p')
                .value_name("NAME")
                .help("Target player name")
                .required_unless_present("xyz"),
        )
        .arg(
            Arg::new("type")
                .long("type")
                .value_parser(["near", "far"])
                .default_value("near")
                .help("Teleport type: near or far"),
        );

    fn handler(ctx: CommandContext, matches: ArgMatches) -> ChatCommandResult {
        ctx.map.world().borrow().run(
            |mut vm_player: ViewMut<Player>, v_wpos: View<WorldPosition>| {
                let player = &mut vm_player[ctx.my_entity_id];
                let wpos = &mut v_wpos[ctx.my_entity_id].clone();

                if matches.contains_id("xyz") {
                    let map_key = MapKey::for_continent(
                        matches
                            .get_many::<f32>("xyz")
                            .unwrap()
                            .nth(3)
                            .map(|x| *x as u32)
                            .unwrap_or(wpos.map_key.map_id),
                    );
                    let x = *matches.get_many::<f32>("xyz").unwrap().nth(0).unwrap();
                    let y = *matches.get_many::<f32>("xyz").unwrap().nth(1).unwrap();
                    let z = *matches.get_many::<f32>("xyz").unwrap().nth(2).unwrap();
                    let o = wpos.o;
                    wpos.update(&WorldPosition {
                        map_key,
                        zone: 0, // TODO: get zone from DBC
                        x,
                        y,
                        z,
                        o,
                    });
                } else if matches.contains_id("player") {
                    let player_name = matches.get_one::<String>("player").unwrap();

                    let conn = ctx.world_context.database.characters.get().unwrap();
                    match CharacterRepository::fetch_guid_and_position_by_name(&conn, player_name) {
                        Some((guid, position)) => {
                            // Try to get the player position from the ECS first, in case the player is online
                            match ctx.map.lookup_entity_ecs(&guid) {
                                Some(target_entity_id) => {
                                    let target_wpos = &v_wpos[target_entity_id];
                                    wpos.update(target_wpos);
                                }
                                None => wpos.update(&position),
                            }
                        }
                        None => {
                            ctx.reply_error("Player not found");
                            return Err(ChatCommandError::GenericError);
                        }
                    }
                } else {
                    unreachable!()
                };

                let force_far = matches.get_one::<String>("type").unwrap() == "far";
                player.teleport_to(wpos, force_far);

                Ok(())
            },
        )
    }

    (command_name, (command, handler))
}
