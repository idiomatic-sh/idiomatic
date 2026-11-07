//! `query`: one module per command.

pub mod run;
pub mod validate;

/// The `query` commands.
#[derive(Debug, clap::Subcommand)]
pub enum QueryCommand {
    /// Run a RyoQL query over a repository and print the rows it gives.
    Run(run::command::RunArguments),
    /// Check a RyoQL query without running it, and say what is wrong with it.
    Validate(validate::command::ValidateArguments),
}

/// Runs one `query` command.
pub fn run(command: QueryCommand, edge: &crate::edge::Edge) -> crate::outcome::Outcome {
    match command {
        QueryCommand::Run(arguments) => run::command::run(arguments, edge),
        QueryCommand::Validate(arguments) => validate::command::run(arguments, edge),
    }
}
