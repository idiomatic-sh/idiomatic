//! `operator`: one module per command.

pub mod describe;
pub mod list;

/// The `operator` commands.
#[derive(Debug, clap::Subcommand)]
pub enum OperatorCommand {
    /// Describe one operator: what it does, the fields it takes and an example.
    Describe(describe::command::DescribeArguments),
    /// List every operator a query can use, each with what it does.
    List(list::command::ListArguments),
}

/// Runs one `operator` command.
pub fn run(command: OperatorCommand, edge: &crate::edge::Edge) -> crate::outcome::Outcome {
    match command {
        OperatorCommand::Describe(arguments) => describe::command::run(arguments, edge),
        OperatorCommand::List(arguments) => list::command::run(arguments, edge),
    }
}
