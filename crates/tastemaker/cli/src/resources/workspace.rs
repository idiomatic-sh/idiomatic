//! `workspace`: the one resource, implicit on the command line; one module per command.

pub mod add;
pub mod check;
pub mod create;
pub mod explain;
pub mod init;
pub mod list;
pub mod scaffold;
pub mod update;
pub mod validate;

/// The commands of the workspace.
#[derive(Debug, clap::Subcommand)]
pub enum WorkspaceCommand {
    /// Initialize a project with its own local asset package.
    Init(init::command::InitArguments),
    /// Write one entry into the workspace's tastemaker.toml.
    Add(add::command::AddArguments),
    /// Write the skeleton of a new rule, spec or template.
    Create(create::command::CreateArguments),
    /// Write the files an entry places that do not exist yet.
    Scaffold(scaffold::command::ScaffoldArguments),
    /// Place the workspace and check it.
    Check(check::command::CheckArguments),
    /// Prove the version in use of every definition of the workspace's packages: its latest, and any the lock pins.
    Validate(validate::command::ValidateArguments),
    /// Move entries to the latest version their package offers.
    Update(update::command::UpdateArguments),
    /// Say what applies to a path, what a definition is, or a topic.
    Explain(explain::command::ExplainArguments),
    /// List definitions, relations and operators.
    List(list::command::ListArguments),
}

/// Runs one command.
pub fn run(command: WorkspaceCommand, edge: &crate::edge::Edge) -> crate::outcome::Outcome {
    match command {
        WorkspaceCommand::Init(arguments) => init::command::run(arguments, edge),
        WorkspaceCommand::Add(arguments) => add::command::run(arguments, edge),
        WorkspaceCommand::Create(arguments) => create::command::run(arguments, edge),
        WorkspaceCommand::Scaffold(arguments) => scaffold::command::run(arguments, edge),
        WorkspaceCommand::Check(arguments) => check::command::run(arguments, edge),
        WorkspaceCommand::Validate(arguments) => validate::command::run(arguments, edge),
        WorkspaceCommand::Update(arguments) => update::command::run(arguments, edge),
        WorkspaceCommand::Explain(arguments) => explain::command::run(arguments, edge),
        WorkspaceCommand::List(arguments) => list::command::run(arguments, edge),
    }
}
