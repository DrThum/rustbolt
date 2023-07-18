use clap::{ArgMatches, Command};
use lazy_static::lazy_static;
use log::warn;
use std::{collections::HashMap, sync::Arc};

use regex::Regex;

use crate::session::world_session::WorldSession;

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

    pub fn try_process(&self, input: &str, session: Arc<WorldSession>) -> ChatCommandResult {
        let input = shell_words::split(input).unwrap();
        if input.is_empty() {
            return ChatCommandResult::Unhandled;
        }

        if let Some(handler) = self.commands.get(input[0].as_str()) {
            return handler(CommandContext { input, session });
        }

        ChatCommandResult::Unhandled
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

                ChatCommandResult::HandledWithError
            }
        }
    }
}

#[derive(PartialEq)]
pub enum ChatCommandResult {
    HandledOk,
    HandledWithError,
    Unhandled,
}

struct CommandContext {
    pub input: Vec<String>,
    pub session: Arc<WorldSession>,
}

type CommandHandler = fn(CommandContext) -> ChatCommandResult;
type CommandMap = HashMap<&'static str, CommandHandler>;
