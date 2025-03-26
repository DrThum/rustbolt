use std::collections::HashMap;

use clap::{builder::BoolishValueParser, Arg, ArgMatches, Command};
use shipyard::ViewMut;

use crate::ecs::components::movement::Movement;

use super::{ChatCommandResult, CommandContext, CommandHandler, CommandMap};

pub(super) fn commands() -> CommandMap {
    HashMap::from([setup_fly_command()])
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

        ctx.map.world().run(|mut vm_movement: ViewMut<Movement>| {
            vm_movement[ctx.my_entity_id].set_flying(flying, ctx.session.clone());
        });

        Ok(())
    }

    (command_name, (command, handler))
}
