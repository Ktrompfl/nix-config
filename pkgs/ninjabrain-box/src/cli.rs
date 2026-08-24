//! The command line, which is a thin wrapper over [`Request`].

use anyhow::{Result};
use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::Shell;

use crate::action::{Index, Request, Target};

#[derive(Parser, Debug)]
#[command(
    name = "ninjabrain-box",
    about = "Ninjabrain Bot on a display of its own, with a Wayland overlay for its panel",
    long_about = "Ninjabrain Bot on a display of its own, with a Wayland overlay for its panel.\n\
                  \n\
                  With no command, starts the calculator and its overlay and stays in the \
                  foreground. With one, sends it to the box already running.",
    version,
    allow_negative_numbers = true
)]
pub struct Cli {
    /// Run without an overlay, driven only through the socket
    #[arg(long)]
    pub headless: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Put something on screen
    Show {
        #[arg(value_enum, default_value_t)]
        target: Target,
    },
    /// Take something off screen
    Hide {
        #[arg(value_enum, default_value_t)]
        target: Target,
    },
    /// Show it if hidden, hide it if shown
    Toggle {
        #[arg(value_enum, default_value_t)]
        target: Target,
    },

    /// Throw every measurement away and start over
    Reset,
    /// Undo the last change, including ones the calculator cannot
    Undo,
    /// Redo the last undone change
    Redo,

    /// Remove an eye throw and put the rest of the list back
    ///
    /// Needs `mode = "unbound"` unless it is the last throw.
    Drop {
        /// Which throw, from 1 forwards or -1 back
        #[arg(default_value = "-1")]
        throw: Index,
    },
    /// Move an eye throw's angle by a number of adjustment steps
    ///
    /// With one argument that is the step count and the throw is the last, so
    /// `adjust +1` is the everyday spelling. With two, the throw comes first.
    /// Reaching a throw that is not the last needs `mode = "unbound"`.
    #[command(arg_required_else_help = true)]
    Adjust {
        /// A step count, or a throw followed by a step count
        #[arg(value_name = "[THROW] STEPS", num_args = 1..=2, allow_hyphen_values = true)]
        arguments: Vec<String>,
    },
    /// Feed in a line as if the game had just put it on the clipboard
    #[command(arg_required_else_help = true)]
    Measure {
        /// The F3+C line
        #[arg(trailing_var_arg = true, num_args = 1..)]
        line: Vec<String>,
    },
    /// Report what the box and the calculator think, as JSON
    Status,

    /// Stop the box, the calculator, and its display
    Quit,

    /// Print a shell completion script
    Completions {
        #[arg(value_name = "SHELL")]
        shell: Shell,
    },
}

/// What this invocation amounts to.
pub enum Invocation {
    /// Run the box, with an overlay unless `headless`.
    Run { headless: bool },
    Send(Request),
    Completions(Shell),
}

pub fn parse() -> Result<Invocation> {
    let cli = Cli::parse();
    let request = match cli.command {
        None => return Ok(Invocation::Run { headless: cli.headless }),
        Some(Command::Completions { shell }) => return Ok(Invocation::Completions(shell)),
        Some(Command::Show { target }) => Request::Show(target),
        Some(Command::Hide { target }) => Request::Hide(target),
        Some(Command::Toggle { target }) => Request::Toggle(target),
        Some(Command::Reset) => Request::Reset,
        Some(Command::Undo) => Request::Undo,
        Some(Command::Redo) => Request::Redo,
        Some(Command::Drop { throw }) => Request::Drop(throw),
        Some(Command::Status) => Request::Status,
        Some(Command::Quit) => Request::Quit,
        Some(Command::Measure { line }) => Request::Measure(line.join(" ")),
        // The one command whose arguments are positional and optional, so it
        // is handed to the same parser that reads it off the socket.
        Some(Command::Adjust { arguments }) => {
            Request::parse(&format!("adjust {}", arguments.join(" ")))?
        }
    };
    Ok(Invocation::Send(request))
}

pub fn completions(shell: Shell) {
    let mut command = Cli::command();
    let name = command.get_name().to_owned();
    clap_complete::generate(shell, &mut command, name, &mut std::io::stdout());
}
