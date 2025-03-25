use clap::{ArgMatches, Command};
use lazy_static::lazy_static;
use log::{error, warn};
use shipyard::{EntityId, View};
use std::{collections::HashMap, sync::Arc};

use regex::Regex;

use crate::{
    ecs::components::unit::Unit,
    game::{map::Map, world_context::WorldContext},
    session::world_session::WorldSession,
};

mod debug;
mod movement;

pub struct ChatCommands {
    commands: CommandMap,
}

lazy_static! {
    static ref ANSI_ESCAPE_REGEX: Regex = Regex::new(r"\x1b\[(\d+)m").unwrap();
}

impl ChatCommands {
    pub fn build() -> Self {
        let mut commands = HashMap::new();
        commands.extend(debug::commands());
        commands.extend(movement::commands());

        Self { commands }
    }

    pub fn consume(
        &self,
        input: &str,
        session: Arc<WorldSession>,
        world_context: Arc<WorldContext>,
    ) -> bool {
        let input = shell_words::split(input).unwrap();
        if input.is_empty() {
            return false;
        }

        if let Some(handler) = self.commands.get(input[0].as_str()) {
            let Some(map) = session.current_map() else {
                error!("chat command: player has not current map");
                return false;
            };

            let Some(my_entity_id) = session.player_entity_id() else {
                error!("chat command: player has no entity id");
                return false;
            };

            let _ = handler(CommandContext {
                input,
                session,
                world_context,
                map: map.clone(),
                my_entity_id,
                target_entity_id: Self::extract_target_entity_id(map.clone(), my_entity_id),
            });

            return true;
        }

        false
    }

    fn replace_ansi_escape_sequences(input: String) -> String {
        let mut output = input.clone();
        for capture in ANSI_ESCAPE_REGEX.captures_iter(output.clone().as_str()) {
            let (s, [c]) = capture.extract();

            let color_code = match c.parse::<u32>() {
                Ok(0) => "|r",          // Reset
                Ok(1) => "",            // Bold
                Ok(4) => "",            // Underline
                Ok(31) => "|cffff0000", // Red
                Ok(32) => "|cff66ff00", // Green
                Ok(33) => "|cffffff00", // Yellow
                Ok(code) => {
                    warn!("unsupported ANSI escape code {code}");
                    ""
                }
                Err(_) => "",
            };

            output = output.replacen(s, color_code, 1);
        }

        output = output.replace("Usage:|r ", "Usage:|r .");
        output
    }

    fn process(
        command: Command,
        ctx: &CommandContext,
        f: &dyn Fn(ArgMatches) -> ChatCommandResult,
    ) -> ChatCommandResult {
        match command.try_get_matches_from(ctx.input.clone()) {
            Ok(matches) => f(matches),
            Err(err) => {
                let error_message = err.render().ansi().to_string();
                let error_message = ChatCommands::replace_ansi_escape_sequences(error_message);
                ctx.session.send_system_message(error_message.as_str());

                Err(ChatCommandError::InvalidArguments)
            }
        }
    }

    fn extract_target_entity_id(map: Arc<Map>, player_entity_id: EntityId) -> Option<EntityId> {
        map.world()
            .run(|v_unit: View<Unit>| v_unit[player_entity_id].target())
    }
}

type ChatCommandResult = Result<(), ChatCommandError>;

enum ChatCommandError {
    RequiresTarget,
    InvalidArguments,
    GenericError,
}

struct CommandContext {
    pub input: Vec<String>,
    pub session: Arc<WorldSession>,
    pub world_context: Arc<WorldContext>,
    pub map: Arc<Map>,
    pub my_entity_id: EntityId,
    target_entity_id: Option<EntityId>,
}

impl CommandContext {
    pub fn reply(&self, message: &str) {
        self.session.send_system_message(message);
    }

    pub fn reply_error(&self, message: &str) {
        self.session.send_error_system_message(message);
    }

    pub fn require_target(&self) -> Result<EntityId, ChatCommandError> {
        self.target_entity_id.ok_or_else(|| {
            self.reply_error("You must select a target");
            ChatCommandError::RequiresTarget
        })
    }
}

type CommandHandler = fn(CommandContext) -> ChatCommandResult;
type CommandMap = HashMap<&'static str, CommandHandler>;
