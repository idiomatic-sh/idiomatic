//! `relation`: one module per command.

pub mod describe;
pub mod list;

/// The `relation` commands.
#[derive(Debug, clap::Subcommand)]
pub enum RelationCommand {
    /// Describe one relation: what it holds, its columns and the languages that give it.
    Describe(describe::command::DescribeArguments),
    /// List every relation a query can read, each with what it holds.
    List(list::command::ListArguments),
}

/// Runs one `relation` command.
pub fn run(command: RelationCommand, edge: &crate::edge::Edge) -> crate::outcome::Outcome {
    match command {
        RelationCommand::Describe(arguments) => describe::command::run(arguments, edge),
        RelationCommand::List(arguments) => list::command::run(arguments, edge),
    }
}
