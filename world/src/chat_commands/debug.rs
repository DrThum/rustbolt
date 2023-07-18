use std::collections::HashMap;

use binrw::NullString;
use clap::{Arg, ArgAction, Command};
use log::info;
use shipyard::View;

use crate::{
    entities::position::WorldPosition,
    protocol::{packets::SmsgMessageChat, server::ServerMessage},
    shared::constants::{ChatMessageType, Language},
};

use super::{ChatCommandResult, ChatCommands, CommandContext, CommandHandler, CommandMap};

pub(super) fn commands() -> CommandMap {
    HashMap::from([(COMMAND_GPS, handle_gps as CommandHandler)])
}

static COMMAND_GPS: &str = "gps";
fn handle_gps(ctx: CommandContext) -> ChatCommandResult {
    let command = Command::new(COMMAND_GPS).arg(
        Arg::new("dump")
            .short('d')
            .long("dump")
            .action(ArgAction::SetTrue),
    );

    match command.try_get_matches_from(ctx.input) {
        Ok(matches) => {
            if let Some(ref map) = ctx.session.current_map() {
                if let Some(player_ecs_entity) = ctx.session.player_entity_id() {
                    map.world().run(|v_wpos: View<WorldPosition>| {
                        let wpos = v_wpos[player_ecs_entity];
                        let output = format!(
                            "Player position: {}, {}, {}, {}",
                            wpos.x, wpos.y, wpos.z, wpos.o,
                        );

                        let packet = ServerMessage::new(SmsgMessageChat::build(
                            ChatMessageType::System,
                            Language::Universal,
                            None,
                            None,
                            NullString::from(output.clone()),
                        ));
                        ctx.session.send(&packet).unwrap();

                        if matches.get_flag("dump") {
                            info!("GPS command output:\n {output}");
                        }
                    });
                }
            }

            return ChatCommandResult::HandledOk;
        }
        Err(err) => {
            let error_message = err.render().ansi().to_string();
            let error_message = ChatCommands::replace_ansi_escape_sequences(error_message);

            let packet = ServerMessage::new(SmsgMessageChat::build(
                ChatMessageType::System,
                Language::Universal,
                None,
                None,
                NullString::from(error_message),
            ));
            ctx.session.send(&packet).unwrap();

            return ChatCommandResult::HandledWithError;
        }
    }
}
