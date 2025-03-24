use std::collections::HashMap;

use clap::{builder::BoolishValueParser, Arg, Command};
use shipyard::ViewMut;

use crate::ecs::components::movement::Movement;

use super::{ChatCommandResult, ChatCommands, CommandContext, CommandHandler, CommandMap};

pub(super) fn commands() -> CommandMap {
    HashMap::from([(COMMAND_FLY, handle_fly as CommandHandler)])
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

        if let Some(map) = ctx.session.current_map() {
            if let Some(player_ecs_entity) = ctx.session.player_entity_id() {
                map.world().run(|mut vm_movement: ViewMut<Movement>| {
                    vm_movement[player_ecs_entity].set_flying(*flying, ctx.session.clone());
                });
            }
        }

        ChatCommandResult::ok()
    })
}
