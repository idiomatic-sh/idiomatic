//! `explain`: parses, checks, composes, calls the service, presents.

use tastemaker_core::inbound::use_cases::explain::interface::ExplainUseCase as _;

use crate::outcome::Outcome;

/// The arguments of `explain`.
#[derive(Debug, clap::Args)]
#[expect(
    clippy::field_scoped_visibility_modifiers,
    reason = "clap fills these fields, the command hands them to the sibling `schemas.rs` and that file's tests read them; `pub` would publish them through the library's public arguments struct, and private fields would hide them from those tests"
)]
#[command(after_help = super::schemas::help())]
pub struct ExplainArguments {
    /// A path, else `<kind>/<name>`, else a topic; the whole workspace when left out.
    pub(super) subject: Option<String>,
    /// Write every topic and definition as Markdown, with an `llms.txt` index.
    #[arg(long, requires = "output_directory")]
    pub(super) all: bool,
    /// The directory `--all` writes to.
    #[arg(short = 'o', long = "output", id = "output_directory")]
    pub(super) output_directory: Option<std::path::PathBuf>,
}

/// Checks the arguments, composes the service, calls it once and shapes its answer.
pub(super) fn answer(
    arguments: ExplainArguments,
    edge: &crate::edge::Edge,
) -> Result<impl crate::output::Encoded, Outcome> {
    let request = super::schemas::request(
        arguments.subject,
        arguments.all,
        arguments.output_directory,
        edge,
    )
    .map_err(|error| crate::output::fail(&error, Outcome::Failure))?;
    super::composition::build(&edge.store)
        .explain(request)
        .map(super::schemas::answer)
        .map_err(|error| crate::output::fail(&error, Outcome::Error))
}

/// Runs `explain`.
pub fn run(arguments: ExplainArguments, edge: &crate::edge::Edge) -> Outcome {
    let format = edge.format;
    match answer(arguments, edge) {
        Ok(answer) => crate::output::present(format, &answer, Outcome::Success),
        Err(outcome) => outcome,
    }
}
