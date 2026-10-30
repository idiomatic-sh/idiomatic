//! `ratchet`: the one resource, implicit on the command line; one module per command.

pub mod check;
pub mod down;
pub mod init;
pub mod up;

/// The commands of the ratchet.
#[derive(Debug, clap::Subcommand)]
pub enum RatchetCommand {
    /// Run the product's tools and judge what they report against the lock.
    Check(check::command::CheckArguments),
    /// Accept unresolved violations.
    Up(up::command::UpArguments),
    /// Clear resolved violations.
    Down(down::command::DownArguments),
    /// Write a minimal asymptote.toml and git hook scripts where a repository has none.
    Init(init::command::InitArguments),
}

/// Runs one command; `check`, which runs tools, registers what stops them with `on_signal`.
pub fn run(
    command: RatchetCommand,
    edge: &crate::edge::Edge,
    on_signal: fn(Box<dyn Fn() + Send>),
) -> crate::outcome::Outcome {
    match command {
        RatchetCommand::Check(arguments) => check::command::run(arguments, edge, on_signal),
        RatchetCommand::Up(arguments) => up::command::run(arguments, edge),
        RatchetCommand::Down(arguments) => down::command::run(arguments, edge),
        RatchetCommand::Init(arguments) => init::command::run(arguments, edge),
    }
}
